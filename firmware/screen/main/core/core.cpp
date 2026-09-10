#include "core/core.h"

#include <cstring>

#include "esp_timer.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"

#include "can_link.h"
#include "core/calibration_machine.h"
#include "common/messages.hpp"

namespace core {
namespace {

constexpr float kPressureFullScaleBar = calibration_machine::kPressureFullScaleBar;
constexpr float kFlowPulsesPerLiter = calibration_machine::kFlowPulsesPerLiter;
constexpr uint32_t kFlowWindowPulses = 10;  // réglage unique du lissage Digmesa
constexpr uint32_t kFlowSilenceMs = 3000;
constexpr uint32_t kTelemetryTickMs = 50;

struct Periods { uint16_t pressure; uint16_t flow; uint16_t actuators; };
constexpr Periods periods_for(TelemetryProfile profile) {
  switch (profile) {
    case TelemetryProfile::kIdle: return {500, 1000, 1000};
    case TelemetryProfile::kActive: return {100, 100, 200};
    case TelemetryProfile::kSuspended: return {0, 0, 0};
  }
  return {0, 0, 0};
}

struct FlowSample { uint32_t pulses; uint16_t edge_ms; int64_t received_us; };
constexpr size_t kFlowHistoryCapacity = 16;

struct State {
  portMUX_TYPE lock = portMUX_INITIALIZER_UNLOCKED;
  Snapshot snapshot;
  int64_t pressure_received_us = 0;
  int64_t flow_received_us = 0;
  int64_t actuators_received_us = 0;
  int64_t last_sensors_message_us = 0;
  int64_t last_flow_edge_received_us = 0;
  FlowSample flow_history[kFlowHistoryCapacity]{};
  size_t flow_count = 0;
  TelemetryProfile profile = TelemetryProfile::kIdle;
  bool profile_dirty = true;
  int64_t last_request_us = 0;
};
State g_state;
ForgetNetworkCallback g_forget_network_callback = nullptr;

int64_t now_us() { return esp_timer_get_time(); }

uint32_t age_ms(int64_t received_us, int64_t now) {
  return received_us == 0 ? UINT32_MAX : static_cast<uint32_t>((now - received_us) / 1000);
}

Freshness freshness(int64_t received_us, uint16_t period_ms, int64_t now) {
  if (received_us == 0 || now - received_us > 3 * 1000 * 1000) return Freshness::kMissing;
  return now - received_us > static_cast<int64_t>(2 * period_ms) * 1000 ? Freshness::kStale : Freshness::kFresh;
}

float decode_pressure_bar(uint32_t raw) {
  uint32_t be = ((raw & 0xFF) << 16) | (raw & 0xFF00) | ((raw >> 16) & 0xFF);
  int32_t signed_raw = (be & 0x800000) ? static_cast<int32_t>(be | 0xFF000000) : static_cast<int32_t>(be);
  return static_cast<float>(signed_raw) * kPressureFullScaleBar / 8388608.0f;
}

float decode_temperature_c(uint16_t raw) {
  uint16_t be = static_cast<uint16_t>((raw << 8) | (raw >> 8));
  return static_cast<float>(static_cast<int16_t>(be)) / 256.0f;
}

void remember_flow_sample(uint32_t pulses, uint16_t edge_ms, int64_t received_us) {
  if (g_state.flow_count == kFlowHistoryCapacity) {
    std::memmove(g_state.flow_history, g_state.flow_history + 1,
                 sizeof(FlowSample) * (kFlowHistoryCapacity - 1));
    g_state.flow_count--;
  }
  g_state.flow_history[g_state.flow_count++] = {pulses, edge_ms, received_us};
}

float compute_flow_ml_s() {
  if (g_state.flow_count < 2) return 0.0f;
  const FlowSample& latest = g_state.flow_history[g_state.flow_count - 1];
  const FlowSample* anchor = nullptr;
  for (size_t i = g_state.flow_count - 1; i-- > 0;) {
    if (latest.pulses - g_state.flow_history[i].pulses >= kFlowWindowPulses) {
      anchor = &g_state.flow_history[i];
      break;
    }
  }
  if (anchor == nullptr) return 0.0f;
  uint32_t delta_pulses = latest.pulses - anchor->pulses;
  uint16_t delta_ms = static_cast<uint16_t>(latest.edge_ms - anchor->edge_ms);
  if (delta_ms == 0) return 0.0f;
  return static_cast<float>(delta_pulses) * 1000.0f / (kFlowPulsesPerLiter * static_cast<float>(delta_ms));
}

void send_request(common::MessageType target, uint16_t period_ms) {
  common::ReqStatusPayload payload{target, period_ms};
  common::Frame frame = payload.pack();
  can_link::send_message(common::MessageType::kReqStatus, common::Dest::kSensors, frame.data(), 3);
}

void telemetry_task(void*) {
  for (;;) {
    TelemetryProfile profile;
    bool dirty;
    int64_t last_request;
    portENTER_CRITICAL(&g_state.lock);
    profile = g_state.profile;
    dirty = g_state.profile_dirty;
    last_request = g_state.last_request_us;
    portEXIT_CRITICAL(&g_state.lock);

    int64_t now = now_us();
    if (dirty || now - last_request >= 1000 * 1000) {
      Periods p = periods_for(profile);
      send_request(common::MessageType::kStatusPressure, p.pressure);
      send_request(common::MessageType::kStatusFlow, p.flow);
      send_request(common::MessageType::kStatusActuators, p.actuators);
      portENTER_CRITICAL(&g_state.lock);
      g_state.profile_dirty = false;
      g_state.last_request_us = now;
      portEXIT_CRITICAL(&g_state.lock);
    }
    can_link::tick_presence();
    vTaskDelay(pdMS_TO_TICKS(kTelemetryTickMs));
  }
}

}  // namespace

void init() { config_init(); }

void start_telemetry_task() {
  xTaskCreatePinnedToCore(telemetry_task, "telemetry", 4096, nullptr, 6, nullptr, 0);
}

void set_telemetry_profile(TelemetryProfile profile) {
  portENTER_CRITICAL(&g_state.lock);
  if (g_state.profile != profile) {
    g_state.profile = profile;
    g_state.profile_dirty = true;
  }
  portEXIT_CRITICAL(&g_state.lock);
}

void on_status_pressure(const uint8_t* data, uint8_t len) {
  common::StatusPressurePayload payload;
  if (!common::StatusPressurePayload::unpack(data, len, &payload)) return;
  int64_t now = now_us();
  portENTER_CRITICAL(&g_state.lock);
  g_state.last_sensors_message_us = now;
  g_state.pressure_received_us = now;
  g_state.snapshot.pressure_valid = (payload.flags & 0x01) != 0;
  if (g_state.snapshot.pressure_valid) {
    g_state.snapshot.pressure_bar = decode_pressure_bar(payload.pressure_raw);
    g_state.snapshot.temperature_c = decode_temperature_c(payload.temperature_raw);
  }
  portEXIT_CRITICAL(&g_state.lock);
}

void on_status_flow(const uint8_t* data, uint8_t len) {
  common::StatusFlowPayload payload;
  if (!common::StatusFlowPayload::unpack(data, len, &payload)) return;
  int64_t now = now_us();
  portENTER_CRITICAL(&g_state.lock);
  g_state.last_sensors_message_us = now;
  g_state.flow_received_us = now;
  g_state.snapshot.flow_valid = (payload.flags & 0x01) != 0;
  bool changed = g_state.flow_count == 0 || payload.pulse_count != g_state.snapshot.flow_pulse_count ||
                 payload.last_edge_ms != g_state.flow_history[g_state.flow_count - 1].edge_ms;
  if (g_state.flow_count > 0 && payload.pulse_count < g_state.snapshot.flow_pulse_count) {
    g_state.flow_count = 0;  // redémarrage du XIAO
  }
  g_state.snapshot.flow_pulse_count = payload.pulse_count;
  g_state.snapshot.volume_ml = static_cast<float>(payload.pulse_count) * 1000.0f / kFlowPulsesPerLiter;
  if (changed) {
    remember_flow_sample(payload.pulse_count, payload.last_edge_ms, now);
    g_state.last_flow_edge_received_us = now;
    g_state.snapshot.flow_ml_s = compute_flow_ml_s();
  }
  portEXIT_CRITICAL(&g_state.lock);
}

void on_status_actuators(const uint8_t* data, uint8_t len) {
  common::StatusActuatorsPayload payload;
  if (!common::StatusActuatorsPayload::unpack(data, len, &payload)) return;
  int64_t now = now_us();
  portENTER_CRITICAL(&g_state.lock);
  g_state.last_sensors_message_us = now;
  g_state.actuators_received_us = now;
  g_state.snapshot.valve_open = payload.ssr;
  g_state.snapshot.dimmer_pct = payload.dimmer;
  g_state.snapshot.lease_remaining_ms = payload.lease_remaining_ms;
  g_state.snapshot.continuous_on_ms = payload.continuous_on_ms;
  g_state.snapshot.lockout = (payload.flags & 0x01) != 0;
  g_state.snapshot.dimmer_ready = (payload.flags & 0x02) != 0;
  g_state.snapshot.dimmer_valid = (payload.flags & 0x04) != 0;
  g_state.snapshot.dimmer_error_active = (payload.flags & 0x08) != 0;
  portEXIT_CRITICAL(&g_state.lock);
}

void on_pong(const uint8_t* data, uint8_t len) {
  common::PongPayload payload;
  if (!common::PongPayload::unpack(data, len, &payload) || payload.node != common::Node::kSensors) return;
  int64_t now = now_us();
  portENTER_CRITICAL(&g_state.lock);
  g_state.last_sensors_message_us = now;
  g_state.snapshot.sensors_version_major = payload.version_major;
  g_state.snapshot.sensors_version_minor = payload.version_minor;
  g_state.snapshot.sensors_version_patch = payload.version_patch;
  g_state.snapshot.sensors_uptime_s = payload.uptime_s;
  portEXIT_CRITICAL(&g_state.lock);
}

void on_log(const uint8_t* data, uint8_t len) {
  common::LogPayload payload;
  if (!common::LogPayload::unpack(data, len, &payload)) return;
  if (payload.code != static_cast<uint8_t>(common::LogCode::kTwaiErrorCounters)) return;
  portENTER_CRITICAL(&g_state.lock);
  g_state.snapshot.sensors_twai_rx_errors = static_cast<uint8_t>(payload.arg16 >> 8);
  g_state.snapshot.sensors_twai_tx_errors = static_cast<uint8_t>(payload.arg16 & 0xFF);
  g_state.snapshot.sensors_twai_bus_errors = payload.arg32;
  portEXIT_CRITICAL(&g_state.lock);
}

void update_network_status(NetworkState state, uint32_t ipv4_address) {
  portENTER_CRITICAL(&g_state.lock);
  g_state.snapshot.network_state = static_cast<uint8_t>(state);
  g_state.snapshot.ipv4_address = ipv4_address;
  portEXIT_CRITICAL(&g_state.lock);
}

void mark_wall_time_known(int64_t unix_s) {
  portENTER_CRITICAL(&g_state.lock);
  g_state.snapshot.time_known = unix_s > 0;
  g_state.snapshot.wall_time_unix_s = unix_s;
  portEXIT_CRITICAL(&g_state.lock);
}

void register_forget_network_callback(ForgetNetworkCallback callback) { g_forget_network_callback = callback; }
void forget_network() { if (g_forget_network_callback != nullptr) g_forget_network_callback(); }

DiagnosticStatus set_diagnostic_purge(bool enabled, uint8_t pump_pct) {
  Snapshot snapshot = get_snapshot();
  if (!snapshot.sensors_alive) return DiagnosticStatus::kBusLost;
  if (snapshot.lockout) return DiagnosticStatus::kLocked;
  if (!snapshot.dimmer_ready || !snapshot.dimmer_valid) return DiagnosticStatus::kDimmerNotReady;
  common::SetPayload command;
  command.set_ssr = true;
  command.set_dimmer = true;
  command.ssr = enabled;
  command.dimmer = enabled ? pump_pct : 0;
  command.ttl_ms = enabled ? 750 : 0;
  common::Frame frame = command.pack();
  can_link::send_message(common::MessageType::kSet, common::Dest::kSensors, frame.data(), 5);
  return DiagnosticStatus::kOk;
}

Snapshot get_snapshot() {
  Snapshot result;
  int64_t pressure_received;
  int64_t flow_received;
  int64_t actuators_received;
  int64_t sensors_message;
  int64_t last_edge;
  TelemetryProfile profile;
  portENTER_CRITICAL(&g_state.lock);
  result = g_state.snapshot;
  pressure_received = g_state.pressure_received_us;
  flow_received = g_state.flow_received_us;
  actuators_received = g_state.actuators_received_us;
  sensors_message = g_state.last_sensors_message_us;
  last_edge = g_state.last_flow_edge_received_us;
  profile = g_state.profile;
  portEXIT_CRITICAL(&g_state.lock);

  int64_t now = now_us();
  Periods periods = periods_for(profile);
  result.pressure_age_ms = age_ms(pressure_received, now);
  result.flow_age_ms = age_ms(flow_received, now);
  result.actuators_age_ms = age_ms(actuators_received, now);
  result.pressure_freshness = result.pressure_valid ? freshness(pressure_received, periods.pressure, now) : Freshness::kMissing;
  result.flow_freshness = result.flow_valid ? freshness(flow_received, periods.flow, now) : Freshness::kMissing;
  result.actuators_freshness = freshness(actuators_received, periods.actuators, now);
  result.sensors_alive = sensors_message != 0 && now - sensors_message <= 3 * 1000 * 1000;
  result.flow_last_edge_age_ms = age_ms(last_edge, now);
  if (last_edge == 0 || now - last_edge > static_cast<int64_t>(kFlowSilenceMs) * 1000) result.flow_ml_s = 0.0f;
  return result;
}

}  // namespace core
