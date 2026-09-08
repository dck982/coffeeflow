// Module capteurs (XIAO ESP32-S3) — phase 2 : capteurs factory, sur la
// table. Voir docs/firmware-implementation.md et docs/firmware.md.
//
// Rien d'autre que le bus, PING/PONG, LOG, RESET, et la machine à états de
// sécurité (bail / présence / verrou 60 s) sur la seule sortie GPIO
// existante (SSR). Pas d'I2C, pas de débitmètre, pas de logique d'infusion :
// ça viendra en phase 5.

#include <cstring>

#include "driver/gpio.h"
#include "driver/twai.h"
#include "esp_log.h"
#include "esp_system.h"
#include "esp_timer.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"

#include "common/framing.hpp"
#include "common/messages.hpp"
#include "common/protocol.hpp"
#include "common/version.hpp"
#include "log_codes.hpp"

namespace {

constexpr const char* kTag = "sensors";

// Brochage — voir docs/firmware.md, "Module capteurs". GPIO natif de
// l'ESP32-S3, pas le D-number imprimé sur le silkscreen du Grove Shield
// XIAO (D8/D9/D10 ≠ GPIO8/9/10 : voir la note dans docs/cablage.md).
//
// Module M5Stack Unit CAN (CA-IS3050G isolé, docs.m5stack.com/en/unit/can) :
// Grove HY2.0-4P, jaune = CAN_TX (entrée du transceiver, à driver depuis le
// contrôleur), blanc = CAN_RX (sortie du transceiver, à lire par le
// contrôleur). Fil blanc → D8/GPIO7, donc GPIO7 = RX contrôleur, GPIO8 = TX
// contrôleur. Confirmé par self-test TWAI en boucle isolée (firmware/
// can-selftest) le 2026-09-08 : BUS_OFF immédiat avec TX/RX dans l'autre
// sens (deux sorties en collision sur le même fil), 18/18 PASS une fois
// inversé.
constexpr gpio_num_t kGpioSsr = GPIO_NUM_9;    // D10
constexpr gpio_num_t kGpioCanTx = GPIO_NUM_8;  // D9
constexpr gpio_num_t kGpioCanRx = GPIO_NUM_7;  // D8

// Sécurité — voir docs/firmware.md, section "Sécurité".
constexpr int64_t kLeaseDefaultUs = 500 * 1000;
constexpr int64_t kPresenceTimeoutUs = 3000 * 1000;
constexpr int64_t kPresencePingIntervalUs = 1500 * 1000;
constexpr int64_t kRuntimeLockoutUs = 60 * 1000 * 1000;
constexpr int64_t kRuntimeRearmGapUs = 2000 * 1000;
constexpr int64_t kTwaiCountersLogPeriodUs = 5000 * 1000;
constexpr int64_t kTickPeriodUs = 100 * 1000;

// Verrou 60 s — tenu en mémoire RTC, ne survit qu'à un démarrage à froid.
// Voir docs/firmware.md §3 : une commande RESET venue du bus ne le lève pas.
constexpr uint32_t kRtcMagic = 0xC0FFEE02;
RTC_NOINIT_ATTR uint32_t s_rtc_magic;
RTC_NOINIT_ATTR bool s_lockout_active;

// État des actionneurs — RAM ordinaire, remis à zéro à chaque redémarrage,
// logiciel ou non (seul le verrou ci-dessus doit survivre).
bool g_ssr = false;
uint8_t g_dimmer = 0;
int64_t g_lease_deadline_us = 0;  // 0 = pas de bail actif

// Plafond de marche continue — voir docs/firmware.md §3, "Rearmement".
int64_t g_run_start_us = 0;   // 0 = pas de marche en cours
int64_t g_off_since_us = 0;   // 0 = pas actuellement à l'arrêt

// Présence — voir docs/firmware.md §2.
int64_t g_last_presence_rx_us = 0;
int64_t g_last_own_ping_us = 0;
bool g_presence_lost = true;  // état de repos correct, carte seule sur la table

int64_t now_us() { return esp_timer_get_time(); }

void apply_ssr() { gpio_set_level(kGpioSsr, g_ssr ? 1 : 0); }

void send_message(common::MessageType type, common::Dest dest, const uint8_t* data, uint8_t dlc) {
  common::CanId id{type, dest, common::Node::kSensors};
  twai_message_t msg{};
  msg.identifier = common::encode_can_id(id);
  msg.data_length_code = dlc;
  if (dlc > 0) {
    std::memcpy(msg.data, data, dlc);
  }
  esp_err_t err = twai_transmit(&msg, pdMS_TO_TICKS(50));
  if (err != ESP_OK) {
    ESP_LOGW(kTag, "twai_transmit type=0x%02x échec: %s", static_cast<int>(type), esp_err_to_name(err));
  }
}

void send_log(common::LogCode code, common::LogSeverity severity, uint16_t arg16 = 0, uint32_t arg32 = 0) {
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
  payload.node = common::Node::kSensors;
  payload.version_major = common::kFirmwareVersionMajor;
  payload.version_minor = common::kFirmwareVersionMinor;
  payload.version_patch = common::kFirmwareVersionPatch;
  payload.uptime_s = static_cast<uint32_t>(now_us() / 1000000);
  common::Frame f = payload.pack();
  send_message(common::MessageType::kPong, common::Dest::kScreen, f.data(), 8);
}

void send_status_actuators() {
  common::StatusActuatorsPayload payload;
  payload.ssr = g_ssr;
  payload.dimmer = g_dimmer;
  int64_t t = now_us();
  int64_t lease_remaining = g_lease_deadline_us > t ? (g_lease_deadline_us - t) / 1000 : 0;
  payload.lease_remaining_ms = static_cast<uint16_t>(lease_remaining > 0xFFFF ? 0xFFFF : lease_remaining);
  int64_t continuous_ms = g_run_start_us != 0 ? (t - g_run_start_us) / 1000 : 0;
  payload.continuous_on_ms = static_cast<uint16_t>(continuous_ms > 0xFFFF ? 0xFFFF : continuous_ms);
  payload.flags = static_cast<uint8_t>((s_lockout_active ? 0x01 : 0) | 0x02 /* dimmer prêt : pas de dimmer réel avant la phase 5 */);
  common::Frame f = payload.pack();
  send_message(common::MessageType::kStatusActuators, common::Dest::kScreen, f.data(), 7);
}

// Coupe les actionneurs immédiatement, sans toucher au verrou lui-même.
void force_actuators_off() {
  g_ssr = false;
  g_dimmer = 0;
  g_lease_deadline_us = 0;
  apply_ssr();
}

void mark_presence() {
  bool was_lost = g_presence_lost;
  g_last_presence_rx_us = now_us();
  g_presence_lost = false;
  if (was_lost) {
    ESP_LOGI(kTag, "présence retrouvée");
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

void on_set_received(const uint8_t* data, size_t len) {
  common::SetPayload payload;
  if (!common::SetPayload::unpack(data, len, &payload)) {
    return;
  }
  if (s_lockout_active) {
    send_log(common::LogCode::kCommandRefusedLocked, common::LogSeverity::kWarn);
    send_status_actuators();
    return;
  }
  if (payload.set_ssr) {
    g_ssr = payload.ssr;
  }
  if (payload.set_dimmer) {
    g_dimmer = payload.dimmer;
  }
  apply_ssr();
  int64_t ttl_us = (payload.ttl_ms == 0 ? kLeaseDefaultUs : static_cast<int64_t>(payload.ttl_ms) * 1000);
  g_lease_deadline_us = now_us() + ttl_us;
  send_status_actuators();
}

void on_stop_received() {
  force_actuators_off();
  send_status_actuators();
}

void on_reqstatus_received(const uint8_t* data, size_t len) {
  // Types capteurs (pression/débit) pas encore câblés en phase 2 : rien à
  // streamer pour eux. STATUS_ACTUATORS part déjà à chaque SET traité.
  common::ReqStatusPayload payload;
  common::ReqStatusPayload::unpack(data, len, &payload);
}

void dispatch_frame(const twai_message_t& msg) {
  common::CanId id = common::decode_can_id(static_cast<uint16_t>(msg.identifier));
  if (!common::is_known_message_type(static_cast<uint8_t>(id.type))) {
    return;
  }
  // On n'écoute que ce qui nous est adressé ou en broadcast, et qui vient de
  // l'écran (self-filtering : TWAI ne boucle pas nos propres trames, mais un
  // troisième émetteur hypothétique serait ignoré ici aussi).
  if (id.dest != common::Dest::kBroadcast && id.dest != common::Dest::kSensors) {
    return;
  }
  if (id.src != common::Node::kScreen) {
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
    case common::MessageType::kStop:
      on_stop_received();
      break;
    case common::MessageType::kSet:
      on_set_received(msg.data, msg.data_length_code);
      break;
    case common::MessageType::kReqStatus:
      on_reqstatus_received(msg.data, msg.data_length_code);
      break;
    default:
      // FLASH_*, STATUS_* venant du bus : rien à faire ici avant la phase 4/5.
      break;
  }
}

void twai_rx_task(void*) {
  for (;;) {
    twai_message_t msg;
    if (twai_receive(&msg, portMAX_DELAY) == ESP_OK) {
      dispatch_frame(msg);
    }
  }
}

// Bail : coupe les actionneurs si le SET le plus récent a expiré.
void tick_lease() {
  if (g_lease_deadline_us == 0) return;
  if (now_us() < g_lease_deadline_us) return;
  if (!g_ssr && g_dimmer == 0) {
    g_lease_deadline_us = 0;
    return;
  }
  force_actuators_off();
  send_log(common::LogCode::kLeaseExpired, common::LogSeverity::kWarn);
  send_status_actuators();
}

// Présence : pingue si silence depuis kPresenceTimeoutUs, et coupe les
// actionneurs (repli sécurité) tant que personne ne répond.
void tick_presence() {
  int64_t t = now_us();
  if (t - g_last_presence_rx_us > kPresenceTimeoutUs) {
    if (!g_presence_lost) {
      g_presence_lost = true;
      force_actuators_off();
      send_log(common::LogCode::kPresenceLost, common::LogSeverity::kWarn);
    }
    if (t - g_last_own_ping_us > kPresencePingIntervalUs) {
      g_last_own_ping_us = t;
      send_message(common::MessageType::kPing, common::Dest::kBroadcast, nullptr, 0);
    }
  }
}

// Plafond 60 s — voir docs/firmware.md §3. Le compteur de marche continue
// n'est remis à zéro que par un arrêt d'au moins kRuntimeRearmGapUs : un
// écran qui commuterait juste avant l'échéance ne rearme rien.
void tick_runtime_lockout() {
  bool running = g_ssr || g_dimmer > 0;
  int64_t t = now_us();

  if (running) {
    if (g_off_since_us != 0 && (t - g_off_since_us) >= kRuntimeRearmGapUs) {
      g_run_start_us = t;  // coupure assez longue : nouveau cycle
    } else if (g_run_start_us == 0) {
      g_run_start_us = t;
    }
    g_off_since_us = 0;

    if (!s_lockout_active && g_run_start_us != 0 && (t - g_run_start_us) > kRuntimeLockoutUs) {
      uint32_t activated_ms = static_cast<uint32_t>((t - g_run_start_us) / 1000);
      s_lockout_active = true;
      force_actuators_off();
      send_log(common::LogCode::kRuntimeLockoutTriggered, common::LogSeverity::kError, 0, activated_ms);
      send_status_actuators();
    }
  } else {
    if (g_off_since_us == 0) {
      g_off_since_us = t;
    } else if ((t - g_off_since_us) >= kRuntimeRearmGapUs) {
      g_run_start_us = 0;
    }
  }
}

// Compteurs d'erreur TWAI — seul diagnostic de câblage sans oscilloscope,
// voir docs/firmware.md. Republiés périodiquement en LOG.
void tick_twai_counters() {
  static int64_t s_last_log_us = 0;
  int64_t t = now_us();
  if (t - s_last_log_us < kTwaiCountersLogPeriodUs) return;
  s_last_log_us = t;

  twai_status_info_t status;
  if (twai_get_status_info(&status) != ESP_OK) return;

  uint16_t arg16 = static_cast<uint16_t>(((status.rx_error_counter & 0xFF) << 8) | (status.tx_error_counter & 0xFF));
  send_log(common::LogCode::kTwaiErrorCounters, common::LogSeverity::kDebug, arg16, status.bus_error_count);
}

void safety_task(void*) {
  for (;;) {
    tick_lease();
    tick_presence();
    tick_runtime_lockout();
    tick_twai_counters();
    vTaskDelay(pdMS_TO_TICKS(kTickPeriodUs / 1000));
  }
}

void init_lockout_state() {
  esp_reset_reason_t reason = esp_reset_reason();
  if (reason == ESP_RST_POWERON || s_rtc_magic != kRtcMagic) {
    // Démarrage à froid, ou mémoire RTC jamais initialisée : seul cas où le
    // verrou 60 s se lève. Voir docs/firmware.md §3.
    s_rtc_magic = kRtcMagic;
    s_lockout_active = false;
  }
  // Tout autre motif de reset (logiciel, watchdog, brownout...) : le verrou
  // déjà posé, le cas échéant, reste posé.
}

}  // namespace

extern "C" void app_main() {
  // GPIO 10 tenu bas avant toute autre initialisation, y compris le CAN —
  // voir docs/firmware.md, "Le SSR est bas au boot, avant toute
  // initialisation du CAN."
  gpio_config_t ssr_cfg{};
  ssr_cfg.pin_bit_mask = 1ULL << kGpioSsr;
  ssr_cfg.mode = GPIO_MODE_OUTPUT;
  ssr_cfg.pull_up_en = GPIO_PULLUP_DISABLE;
  ssr_cfg.pull_down_en = GPIO_PULLDOWN_DISABLE;
  ssr_cfg.intr_type = GPIO_INTR_DISABLE;
  gpio_config(&ssr_cfg);
  gpio_set_level(kGpioSsr, 0);

  init_lockout_state();

  twai_general_config_t g_config = TWAI_GENERAL_CONFIG_DEFAULT(kGpioCanTx, kGpioCanRx, TWAI_MODE_NORMAL);
  twai_timing_config_t t_config = TWAI_TIMING_CONFIG_500KBITS();
  twai_filter_config_t f_config = TWAI_FILTER_CONFIG_ACCEPT_ALL();
  ESP_ERROR_CHECK(twai_driver_install(&g_config, &t_config, &f_config));
  ESP_ERROR_CHECK(twai_start());

  int64_t t0 = now_us();
  g_last_presence_rx_us = t0 - kPresenceTimeoutUs;  // en attente, pas encore vu de pair
  g_last_own_ping_us = 0;

  send_log(common::LogCode::kBoot, common::LogSeverity::kInfo);
  ESP_LOGI(kTag, "boot, version %d.%d.%d, verrou=%s", common::kFirmwareVersionMajor,
           common::kFirmwareVersionMinor, common::kFirmwareVersionPatch,
           s_lockout_active ? "actif" : "libre");

  xTaskCreate(twai_rx_task, "twai_rx", 4096, nullptr, 10, nullptr);
  xTaskCreate(safety_task, "safety", 4096, nullptr, 5, nullptr);

  send_log(common::LogCode::kReady, common::LogSeverity::kInfo);
}
