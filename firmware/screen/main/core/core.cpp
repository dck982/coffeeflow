#include "core/core.h"

#include <cstring>

#include "esp_timer.h"
#include "esp_heap_caps.h"
#include "esp_attr.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"

#include "can_link.h"
#include "ble_scale.h"
#include "net_wifi.h"
#include "core/calibration_machine.h"
#include "core/machine.h"
#include "common/messages.hpp"
#include "common/version.hpp"
#include "log_codes.hpp"

namespace core {
namespace {

constexpr float kPressureFullScaleBar = calibration_machine::kPressureFullScaleBar;
constexpr float kFlowPulsesPerLiter = calibration_machine::kFlowPulsesPerLiter;
constexpr uint32_t kFlowWindowPulses = 10;  // réglage unique du lissage Digmesa
constexpr uint32_t kFlowSilenceMs = 3000;
constexpr uint32_t kTelemetryTickMs = 50;
constexpr uint32_t kScalePresentMs = 2000;

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
  int64_t scale_received_us = 0;
  FlowSample flow_history[kFlowHistoryCapacity]{};
  size_t flow_count = 0;
  TelemetryProfile profile = TelemetryProfile::kIdle;
  bool profile_dirty = true;
  int64_t last_request_us = 0;
  machine::Machine machine;
  int64_t last_cycle_set_us = 0;
  bool cycle_set_on = false;
  int64_t shot_start_unix_s = 0;
};
State g_state;
// Le seul résumé retenu ne consomme pas de SRAM interne utile aux radios/LCD.
struct LastShotStore { bool available; float weight_g; uint32_t duration_ms; float flow_ml_s; int64_t unix_s; };
EXT_RAM_BSS_ATTR LastShotStore g_last_shot{};
ForgetNetworkCallback g_forget_network_callback = nullptr;
bool g_flash_active = false;
bool g_radio_transition = false;

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

void send_set(bool ssr, uint8_t dimmer, uint16_t ttl_ms) {
  common::SetPayload command;
  command.set_ssr = true;
  command.set_dimmer = true;
  command.ssr = ssr;
  command.dimmer = dimmer;
  command.ttl_ms = ttl_ms;
  common::Frame frame = command.pack();
  can_link::send_message(common::MessageType::kSet, common::Dest::kSensors, frame.data(), 5);
}

uint16_t config_field_arg(const char* field) {
  if (field == nullptr) return 0;
  struct Entry { const char* name; uint16_t id; };
  static constexpr Entry kFields[] = {
      {"version", 1},
      {"brew.target_weight_g", 2},
      {"brew.target_time_s", 3},
      {"brew.pump_pct", 4},
      {"preinfusion.mode", 5},
      {"preinfusion.time_s", 6},
      {"preinfusion.pressure_bar", 7},
      {"preinfusion.pump_pct", 8},
      {"rampdown.mode", 9},
      {"rampdown.lead_time_s", 10},
      {"rampdown.lead_weight_g", 11},
      {"rampdown.pressure_drop_bar", 12},
      {"purge.pump_pct", 13},
      {"purge.max_s", 14},
      {"ui.dim_after_s", 15},
      {"ui.standby_after_s", 16},
      {"profiles", 17},
      {"brew", 18},
      {"preinfusion", 19},
      {"rampdown", 20},
      {"purge", 21},
      {"ui", 22},
  };
  for (const Entry& entry : kFields) {
    if (std::strcmp(entry.name, field) == 0) return entry.id;
  }
  return 0;
}

const char* action_reason(ActionStatus status) {
  switch (status) {
    case ActionStatus::kOk: return "ok";
    case ActionStatus::kUnavailable: return "unavailable";
    case ActionStatus::kBusLost: return "bus_lost";
    case ActionStatus::kLocked: return "locked";
    case ActionStatus::kDimmerNotReady: return "dimmer_not_ready";
    case ActionStatus::kCycleActive: return "cycle_active";
    case ActionStatus::kNoCycle: return "no_cycle";
    case ActionStatus::kInvalidValue: return "invalid_value";
  }
  return "unavailable";
}

ActionResult action_result(ActionStatus status) { return {status, action_reason(status)}; }

machine::Config machine_config(const Config& c) {
  return {c.target_weight_g, c.target_time_s,
          c.preinfusion_mode == PreinfusionMode::kPressure ? machine::PreinfusionMode::kPressure : machine::PreinfusionMode::kTime,
          c.preinfusion_time_s, c.preinfusion_pressure_bar, c.preinfusion_pump_pct,
          static_cast<machine::RampdownMode>(c.rampdown_mode), c.rampdown_lead_time_s,
          c.rampdown_lead_weight_g, c.rampdown_pressure_drop_bar, c.brew_pump_pct,
          c.purge_pump_pct, c.purge_max_s};
}

machine::Input machine_input(const Snapshot& s) { return {s.weight_g, s.scale_present, s.pressure_bar}; }

CycleState cycle_state(machine::State state) { return static_cast<CycleState>(state); }

void update_cycle_snapshot_locked(int64_t now) {
  g_state.snapshot.cycle_state = cycle_state(g_state.machine.state());
  g_state.snapshot.cycle_elapsed_ms = g_state.machine.elapsed_ms(static_cast<uint64_t>(now / 1000));
  g_state.snapshot.cycle_phase_elapsed_ms = g_state.machine.phase_elapsed_ms(static_cast<uint64_t>(now / 1000));
  g_state.snapshot.cycle_start_weight_g = g_state.machine.starting_weight_g();
  g_state.snapshot.cycle_weight_goal = g_state.machine.weight_goal();
}

void remember_completed_shot_locked(const Snapshot& before) {
  if (g_state.machine.state() != machine::State::kFinished ||
      g_state.machine.stop_reason() == machine::StopReason::kPurgeReleased ||
      g_state.machine.stop_reason() == machine::StopReason::kPurgeTimeout) return;
  g_state.snapshot.last_shot_available = true;
  g_state.snapshot.last_shot_weight_g = before.weight_g - g_state.machine.starting_weight_g();
  g_state.snapshot.last_shot_duration_ms = g_state.machine.elapsed_ms(static_cast<uint64_t>(now_us() / 1000));
  g_state.snapshot.last_shot_flow_ml_s = before.flow_ml_s;
  g_state.snapshot.last_shot_unix_s = g_state.shot_start_unix_s;
  g_last_shot = {true, g_state.snapshot.last_shot_weight_g, g_state.snapshot.last_shot_duration_ms,
                 g_state.snapshot.last_shot_flow_ml_s, g_state.snapshot.last_shot_unix_s};
}

void tick_machine() {
  const int64_t now = now_us();
  machine::Output output{};
  bool send = false;
  machine::StopReason stop_reason = machine::StopReason::kNone;
  portENTER_CRITICAL(&g_state.lock);
  const bool was_active = g_state.machine.active();
  output = g_state.machine.tick(static_cast<uint64_t>(now / 1000), machine_input(g_state.snapshot));
  const bool active = g_state.machine.active();
  if (was_active && !active) { remember_completed_shot_locked(g_state.snapshot); stop_reason = g_state.machine.stop_reason(); }
  update_cycle_snapshot_locked(now);
  if ((active && now - g_state.last_cycle_set_us >= 100000) || (!active && g_state.cycle_set_on)) {
    g_state.last_cycle_set_us = now;
    g_state.cycle_set_on = active;
    send = true;
  }
  portEXIT_CRITICAL(&g_state.lock);
  if (was_active && !active) {
    const auto reason = static_cast<uint16_t>(stop_reason);
    can_link::send_log(stop_reason == machine::StopReason::kPurgeReleased || stop_reason == machine::StopReason::kPurgeTimeout
                           ? common::LogCode::kPurgeStopped : common::LogCode::kBrewStopped,
                       common::LogSeverity::kInfo, reason);
  }
  if (send) send_set(output.ssr, output.dimmer, output.ttl_ms);
  set_telemetry_profile(active ? TelemetryProfile::kActive : TelemetryProfile::kIdle);
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
    tick_machine();
    vTaskDelay(pdMS_TO_TICKS(kTelemetryTickMs));
  }
}

void radio_transition_task(void* arg) {
  const RadioMode target = static_cast<RadioMode>(reinterpret_cast<uintptr_t>(arg));
  if (target == RadioMode::kWifi) {
    ble_scale::stop();
    net_wifi::start();
  } else if (target == RadioMode::kMachine) {
    net_wifi::stop();
    ble_scale::init();
  } else {
    // Mode diagnostic : aucune radio ne retient de SRAM interne.
    net_wifi::stop();
    ble_scale::stop();
  }
  portENTER_CRITICAL(&g_state.lock);
  g_state.snapshot.radio_mode = target;
  g_state.snapshot.radio_transition = false;
  g_radio_transition = false;
  portEXIT_CRITICAL(&g_state.lock);
  vTaskDelete(nullptr);
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
  // La balance suffit à une cadence sobre au repos. La machine du lot 9
  // sélectionnera kActive pendant une infusion ou une purge ; le client BLE
  // publiera alors chaque notification plutôt qu'un échantillon par seconde.
  ble_scale::set_active(profile == TelemetryProfile::kActive);
}

bool begin_flash(FlashTarget target, uint32_t total) {
  if (target == FlashTarget::kNone || total == 0 || g_flash_active || get_snapshot().cycle_state == CycleState::kPreinfusion ||
      get_snapshot().cycle_state == CycleState::kBrew || get_snapshot().cycle_state == CycleState::kRampdown ||
      get_snapshot().cycle_state == CycleState::kPurge) return false;
  Snapshot snapshot = get_snapshot();
  // Sans secteur le dimmer ne répond pas sur I2C : STATUS_ACTUATORS est alors
  // légitimement absent. Le CAN vivant reste indispensable, et un écho frais
  // qui dit SSR/pompe actifs interdit toujours le flash. L'arrêt est envoyé
  // juste après l'acceptation dans tous les cas.
  if (!snapshot.sensors_alive ||
      (snapshot.actuators_freshness == Freshness::kFresh &&
       (snapshot.valve_open || snapshot.dimmer_pct != 0))) return false;
  g_flash_active = true;
  portENTER_CRITICAL(&g_state.lock);
  g_state.snapshot.flash_active = true;
  g_state.snapshot.flash_target = target;
  g_state.snapshot.flash_bytes_done = 0;
  g_state.snapshot.flash_bytes_total = total;
  g_state.profile = TelemetryProfile::kSuspended;
  g_state.profile_dirty = true;
  portEXIT_CRITICAL(&g_state.lock);
  send_set(false, 0, 0);
  return true;
}

void update_flash_progress(uint32_t done) {
  portENTER_CRITICAL(&g_state.lock);
  if (g_state.snapshot.flash_active) g_state.snapshot.flash_bytes_done = done;
  portEXIT_CRITICAL(&g_state.lock);
}

void finish_flash() {
  g_flash_active = false;
  portENTER_CRITICAL(&g_state.lock);
  g_state.snapshot.flash_active = false;
  g_state.snapshot.flash_target = FlashTarget::kNone;
  g_state.profile = TelemetryProfile::kIdle;
  g_state.profile_dirty = true;
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
  g_state.snapshot.pressure_raw = payload.pressure_raw;
  g_state.snapshot.temperature_raw = payload.temperature_raw;
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

void update_scale_connection(bool connected) {
  portENTER_CRITICAL(&g_state.lock);
  g_state.snapshot.scale_connected = connected;
  if (!connected) g_state.scale_received_us = 0;
  portEXIT_CRITICAL(&g_state.lock);
}

void update_scale_weight(float weight_g) {
  int64_t now = now_us();
  portENTER_CRITICAL(&g_state.lock);
  g_state.snapshot.weight_g = weight_g;
  g_state.scale_received_us = now;
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

bool request_radio_mode(RadioMode mode) {
  const CycleState cycle = get_snapshot().cycle_state;
  if (cycle == CycleState::kPreinfusion || cycle == CycleState::kBrew ||
      cycle == CycleState::kRampdown || cycle == CycleState::kPurge || g_flash_active) return false;
  portENTER_CRITICAL(&g_state.lock);
  if (g_radio_transition) {
    portEXIT_CRITICAL(&g_state.lock);
    return false;
  }
  if (g_state.snapshot.radio_mode == mode) {
    portEXIT_CRITICAL(&g_state.lock);
    return true;
  }
  g_radio_transition = true;
  g_state.snapshot.radio_transition = true;
  portEXIT_CRITICAL(&g_state.lock);

  const auto arg = reinterpret_cast<void*>(static_cast<uintptr_t>(mode));
  if (xTaskCreatePinnedToCore(radio_transition_task, "radio_mode", 6144, arg, 5, nullptr, 0) != pdPASS) {
    portENTER_CRITICAL(&g_state.lock);
    g_radio_transition = false;
    g_state.snapshot.radio_transition = false;
    portEXIT_CRITICAL(&g_state.lock);
    return false;
  }
  return true;
}

RadioMode radio_mode() {
  portENTER_CRITICAL(&g_state.lock);
  RadioMode mode = g_state.snapshot.radio_mode;
  portEXIT_CRITICAL(&g_state.lock);
  return mode;
}

void register_forget_network_callback(ForgetNetworkCallback callback) { g_forget_network_callback = callback; }
void forget_network() { if (g_forget_network_callback != nullptr) g_forget_network_callback(); }

ConfigResult put_config(const Config& candidate) {
  if (get_snapshot().cycle_state == CycleState::kPreinfusion || get_snapshot().cycle_state == CycleState::kBrew ||
      get_snapshot().cycle_state == CycleState::kRampdown || get_snapshot().cycle_state == CycleState::kPurge) return {ConfigStatus::kBusy, "cycle"};
  for (int attempt = 0; attempt < 2; ++attempt) {
    ConfigResult result = apply_config(candidate, get_config().revision);
    if (result.status != ConfigStatus::kStaleRevision) return result;
  }
  return {ConfigStatus::kStaleRevision, "revision"};
}

void note_http_auth_refused() {
  can_link::send_log(common::LogCode::kHttpAuthRefused, common::LogSeverity::kWarn);
}

void note_config_rejected(const char* field) {
  can_link::send_log(common::LogCode::kConfigRejected, common::LogSeverity::kWarn, config_field_arg(field));
}

DiagnosticStatus set_diagnostic_purge(bool enabled, uint8_t pump_pct) {
  // Compatibilité temporaire avec l'écran de service : il devient lui aussi
  // un client de la face actions, sans chemin SET parallèle.
  ActionResult result = perform_action({enabled ? Action::kPurgePress : Action::kPurgeRelease,
                                        false, pump_pct, 0});
  switch (result.status) {
    case ActionStatus::kOk: return DiagnosticStatus::kOk;
    case ActionStatus::kBusLost: return DiagnosticStatus::kBusLost;
    case ActionStatus::kLocked: return DiagnosticStatus::kLocked;
    default: return DiagnosticStatus::kDimmerNotReady;
  }
}

ActionResult perform_action(const ActionCommand& command) {
  if (command.action == Action::kStartBrew && radio_mode() == RadioMode::kWifi) {
    return action_result(ActionStatus::kUnavailable);
  }
  if (command.action == Action::kTare) {
    if (!get_snapshot().scale_present) return action_result(ActionStatus::kUnavailable);
    return action_result(ble_scale::tare() ? ActionStatus::kOk : ActionStatus::kUnavailable);
  }
  if (command.action == Action::kDismissSummary) {
    portENTER_CRITICAL(&g_state.lock);
    bool ok = g_state.machine.dismiss();
    g_state.snapshot.last_shot_available = false;
    g_last_shot.available = false;
    update_cycle_snapshot_locked(now_us());
    portEXIT_CRITICAL(&g_state.lock);
    return action_result(ok ? ActionStatus::kOk : ActionStatus::kNoCycle);
  }
  Snapshot snapshot = get_snapshot();
  if (command.action == Action::kResetSensors) {
    if (!snapshot.sensors_alive) return action_result(ActionStatus::kBusLost);
    if (snapshot.cycle_state == CycleState::kPreinfusion ||
        snapshot.cycle_state == CycleState::kBrew ||
        snapshot.cycle_state == CycleState::kRampdown ||
        snapshot.cycle_state == CycleState::kPurge || g_flash_active)
      return action_result(ActionStatus::kCycleActive);
    send_set(false, 0, 0);
    can_link::reset_sensors();
    return action_result(ActionStatus::kOk);
  }
  if (command.action == Action::kStopBrew) {
    portENTER_CRITICAL(&g_state.lock); bool ok = g_state.machine.stop(static_cast<uint64_t>(now_us() / 1000)); if (ok) remember_completed_shot_locked(g_state.snapshot); update_cycle_snapshot_locked(now_us()); g_state.cycle_set_on = false; portEXIT_CRITICAL(&g_state.lock);
    if (ok) send_set(false, 0, 0);
    if (ok) can_link::send_log(common::LogCode::kBrewStopped, common::LogSeverity::kInfo,
                               static_cast<uint16_t>(machine::StopReason::kManual));
    return action_result(ok ? ActionStatus::kOk : ActionStatus::kNoCycle);
  }
  if (command.action == Action::kPurgeRelease) {
    portENTER_CRITICAL(&g_state.lock); bool ok = g_state.machine.purge_release(static_cast<uint64_t>(now_us() / 1000)); update_cycle_snapshot_locked(now_us()); g_state.cycle_set_on = false; portEXIT_CRITICAL(&g_state.lock);
    if (ok) send_set(false, 0, 0);
    if (ok) can_link::send_log(common::LogCode::kPurgeStopped, common::LogSeverity::kInfo,
                               static_cast<uint16_t>(machine::StopReason::kPurgeReleased));
    return action_result(ok ? ActionStatus::kOk : ActionStatus::kNoCycle);
  }
  if (command.action == Action::kStartBrew || command.action == Action::kPurgePress) {
    if (g_flash_active) return action_result(ActionStatus::kCycleActive);
    if (!snapshot.sensors_alive) return action_result(ActionStatus::kBusLost);
    if (snapshot.lockout) return action_result(ActionStatus::kLocked);
    if (!snapshot.dimmer_ready || !snapshot.dimmer_valid) return action_result(ActionStatus::kDimmerNotReady);
    portENTER_CRITICAL(&g_state.lock);
    bool ok = command.action == Action::kStartBrew
                  ? g_state.machine.start(static_cast<uint64_t>(now_us() / 1000), machine_config(get_config()), machine_input(g_state.snapshot))
                  : g_state.machine.purge_press(static_cast<uint64_t>(now_us() / 1000), machine_config(get_config()));
    if (ok && command.action == Action::kStartBrew) {
      g_state.shot_start_unix_s = g_state.snapshot.time_known ? g_state.snapshot.wall_time_unix_s : 0;
    }
    update_cycle_snapshot_locked(now_us());
    portEXIT_CRITICAL(&g_state.lock);
    if (ok) can_link::send_log(command.action == Action::kStartBrew ? common::LogCode::kBrewStarted : common::LogCode::kPurgeStarted,
                               common::LogSeverity::kInfo);
    return action_result(ok ? ActionStatus::kOk : ActionStatus::kCycleActive);
  }
  if (command.action != Action::kSetActuators) return action_result(ActionStatus::kUnavailable);
  if (command.dimmer > 100) return action_result(ActionStatus::kInvalidValue);
  snapshot = get_snapshot();
  if (!snapshot.sensors_alive) return action_result(ActionStatus::kBusLost);
  if (snapshot.lockout) return action_result(ActionStatus::kLocked);
  if (snapshot.cycle_state == CycleState::kPreinfusion || snapshot.cycle_state == CycleState::kBrew || snapshot.cycle_state == CycleState::kRampdown || snapshot.cycle_state == CycleState::kPurge || g_flash_active) return action_result(ActionStatus::kCycleActive);
  send_set(command.ssr, command.dimmer, command.ttl_ms);
  return action_result(ActionStatus::kOk);
}

Snapshot get_snapshot() {
  Snapshot result;
  int64_t pressure_received;
  int64_t flow_received;
  int64_t actuators_received;
  int64_t sensors_message;
  int64_t last_edge;
  int64_t scale_received;
  TelemetryProfile profile;
  portENTER_CRITICAL(&g_state.lock);
  result = g_state.snapshot;
  result.last_shot_available = g_last_shot.available;
  result.last_shot_weight_g = g_last_shot.weight_g;
  result.last_shot_duration_ms = g_last_shot.duration_ms;
  result.last_shot_flow_ml_s = g_last_shot.flow_ml_s;
  result.last_shot_unix_s = g_last_shot.unix_s;
  pressure_received = g_state.pressure_received_us;
  flow_received = g_state.flow_received_us;
  actuators_received = g_state.actuators_received_us;
  sensors_message = g_state.last_sensors_message_us;
  last_edge = g_state.last_flow_edge_received_us;
  scale_received = g_state.scale_received_us;
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
  result.scale_age_ms = age_ms(scale_received, now);
  result.scale_present = result.scale_connected && scale_received != 0 &&
                         now - scale_received <= static_cast<int64_t>(kScalePresentMs) * 1000;
  if (last_edge == 0 || now - last_edge > static_cast<int64_t>(kFlowSilenceMs) * 1000) result.flow_ml_s = 0.0f;
  result.screen_version_major = common::kFirmwareVersionMajor;
  result.screen_version_minor = common::kFirmwareVersionMinor;
  result.screen_version_patch = common::kFirmwareVersionPatch;
  result.screen_uptime_s = static_cast<uint32_t>(now / 1000000);
  result.internal_heap_free = heap_caps_get_free_size(MALLOC_CAP_INTERNAL | MALLOC_CAP_8BIT);
  result.internal_heap_largest = heap_caps_get_largest_free_block(MALLOC_CAP_INTERNAL | MALLOC_CAP_8BIT);
  result.internal_heap_minimum = heap_caps_get_minimum_free_size(MALLOC_CAP_INTERNAL | MALLOC_CAP_8BIT);
  return result;
}

}  // namespace core
