#include "can_link.h"

#include <cstring>

#include "esp_log.h"
#include "esp_system.h"
#include "esp_timer.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"

#include "board.h"
#include "common/framing.hpp"
#include "common/messages.hpp"
#include "common/version.hpp"
#include "core/events.h"
#include "serial_bridge.h"

namespace can_link {

namespace {

constexpr const char* kTag = "screen";

// Même valeur que sensors/main.cpp (kPresenceTimeoutUs) : un PING ou PONG du
// pair suffit à réarmer, l'absence pendant ce délai est le repli sécurité.
constexpr int64_t kPresenceTimeoutUs = 3 * 1000 * 1000;

int64_t g_last_presence_rx_us = 0;
bool g_presence_lost = true;

int64_t now_us() { return esp_timer_get_time(); }

void mark_presence() {
  g_last_presence_rx_us = now_us();
  if (g_presence_lost) {
    g_presence_lost = false;
    send_log(common::LogCode::kPresenceRestored, common::LogSeverity::kInfo);
    core::events::push(core::EventKind::kCanPresenceRestored);
  }
}

void on_ping_received() {
  mark_presence();
  send_pong();
}

void on_pong_received() { mark_presence(); }

void on_reset_received() {
  send_log(common::LogCode::kRebootRequested, common::LogSeverity::kInfo);
  vTaskDelay(pdMS_TO_TICKS(50));  // laisser partir le LOG avant le reboot
  esp_restart();
}

}  // namespace

void init() {
  twai_general_config_t g_config = TWAI_GENERAL_CONFIG_DEFAULT(board::kCanTx, board::kCanRx, TWAI_MODE_NORMAL);
  twai_timing_config_t t_config = TWAI_TIMING_CONFIG_500KBITS();
  twai_filter_config_t f_config = TWAI_FILTER_CONFIG_ACCEPT_ALL();
  ESP_ERROR_CHECK(twai_driver_install(&g_config, &t_config, &f_config));
  ESP_ERROR_CHECK(twai_start());

  g_last_presence_rx_us = now_us();
}

void send_message(common::MessageType type, common::Dest dest, const uint8_t* data, uint8_t dlc) {
  common::CanId id{type, dest, common::Node::kScreen};
  common::RawFrame frame;
  frame.can_id = common::encode_can_id(id);
  frame.dlc = dlc;
  if (dlc > 0) {
    std::memcpy(frame.data.data(), data, dlc);
  }

  twai_message_t msg{};
  msg.identifier = frame.can_id;
  msg.data_length_code = dlc;
  if (dlc > 0) {
    std::memcpy(msg.data, data, dlc);
  }
  esp_err_t err = twai_transmit(&msg, pdMS_TO_TICKS(50));
  if (err != ESP_OK) {
    ESP_LOGW(kTag, "twai_transmit type=0x%02x échec: %s", static_cast<int>(type), esp_err_to_name(err));
  }

  uint8_t out[common::kMaxCobsSize + 1];
  size_t len = common::encode_framed(frame, out);
  if (len > 0) {
    serial_bridge::write_raw(out, len);
  }
}

void send_log(common::LogCode code, common::LogSeverity severity, uint16_t arg16, uint32_t arg32) {
  common::LogPayload payload;
  payload.code = static_cast<uint8_t>(code);
  payload.severity = static_cast<uint8_t>(severity);
  payload.arg16 = arg16;
  payload.arg32 = arg32;
  common::Frame f = payload.pack();
  send_message(common::MessageType::kLog, common::Dest::kBroadcast, f.data(), 8);
}

void send_pong() {
  common::PongPayload payload;
  payload.node = common::Node::kScreen;
  payload.version_major = common::kFirmwareVersionMajor;
  payload.version_minor = common::kFirmwareVersionMinor;
  payload.version_patch = common::kFirmwareVersionPatch;
  payload.uptime_s = static_cast<uint32_t>(now_us() / 1000000);
  common::Frame f = payload.pack();
  send_message(common::MessageType::kPong, common::Dest::kSensors, f.data(), 8);
}

bool presence_lost() { return g_presence_lost; }

void tick_presence() {
  if (!g_presence_lost && (now_us() - g_last_presence_rx_us) > kPresenceTimeoutUs) {
    g_presence_lost = true;
    send_log(common::LogCode::kPresenceLost, common::LogSeverity::kWarn);
    core::events::push(core::EventKind::kCanPresenceLost);
  }
}

void dispatch_own_protocol(const twai_message_t& msg) {
  common::CanId id = common::decode_can_id(static_cast<uint16_t>(msg.identifier));
  if (!common::is_known_message_type(static_cast<uint8_t>(id.type))) {
    return;
  }
  if (id.dest != common::Dest::kBroadcast && id.dest != common::Dest::kScreen) {
    return;
  }
  if (id.src != common::Node::kSensors) {
    return;
  }

  switch (id.type) {
    case common::MessageType::kPing:
      on_ping_received();
      break;
    case common::MessageType::kPong:
      on_pong_received();
      break;
    case common::MessageType::kReset:
      on_reset_received();
      break;
    default:
      // Le reste (STATUS_*, LOG, FLASH_*) ne nous concerne pas en tant que
      // nœud : coffeetool le voit déjà via le pont brut.
      break;
  }
}

}  // namespace can_link
