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
#include "esp_ota_ops.h"
#include "esp_partition.h"
#include "esp_system.h"
#include "esp_timer.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"

#include "common/crc.hpp"
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

// OTA — voir docs/firmware-implementation.md, phase 4. IDF ne redémarre
// jamais tout seul une image en PENDING_VERIFY : ce délai est le
// temporisateur d'invalidation explicite qui s'en charge. Réutilise le même
// signal que la présence (mark_presence(), donc un PING/PONG venu de
// l'écran/pont) comme preuve que l'image tourne assez pour parler au bus.
constexpr int64_t kOtaValidationTimeoutUs = 30 * 1000 * 1000;
constexpr size_t kFlashBlockSize = 2048;

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

// Validation OTA — voir docs/firmware-implementation.md, phase 4.
bool g_ota_pending_verify = false;
int64_t g_ota_pending_since_us = 0;

// Flash — un seul transfert à la fois, pas de file d'attente. `partition`
// non nul == transfert en cours (BEGIN reçu, END/ABORT pas encore traité).
struct FlashState {
  const esp_partition_t* partition = nullptr;
  esp_ota_handle_t ota_handle = 0;
  uint32_t image_size = 0;
  uint32_t bytes_written = 0;
  uint16_t block_number = 0;  // nombre de blocs déjà acquittés
  uint8_t block_buf[kFlashBlockSize];
  size_t block_buf_len = 0;
  common::Crc32Incremental image_crc;
};
FlashState g_flash;

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

void send_flash_ack(common::FlashSubCmd subcmd, uint16_t block_number, uint16_t block_crc16) {
  common::FlashCtrlPayload payload;
  payload.subcmd = subcmd;
  payload.block_number = block_number;
  payload.block_crc16 = block_crc16;
  common::Frame f = payload.pack();
  send_message(common::MessageType::kFlashCtrl, common::Dest::kScreen, f.data(), 8);
}

// Remet l'état de flash à zéro sans toucher au handle OTA (à faire avant, si
// un handle est ouvert).
void reset_flash_state() {
  g_flash.partition = nullptr;
  g_flash.ota_handle = 0;
  g_flash.image_size = 0;
  g_flash.bytes_written = 0;
  g_flash.block_number = 0;
  g_flash.block_buf_len = 0;
  g_flash.image_crc = common::Crc32Incremental{};
}

// Abandonne un transfert en cours (BEGIN suivi d'un autre BEGIN, ABORT
// explicite, ou échec de vérification en END).
void flash_abort(const char* reason) {
  if (g_flash.partition != nullptr) {
    esp_ota_abort(g_flash.ota_handle);
    ESP_LOGW(kTag, "flash abandonné : %s", reason);
  }
  reset_flash_state();
}

void on_flash_begin(uint32_t image_size) {
  if (g_flash.partition != nullptr) {
    flash_abort("nouveau BEGIN reçu avant la fin du précédent transfert");
  }

  const esp_partition_t* partition = esp_ota_get_next_update_partition(nullptr);
  if (partition == nullptr || image_size == 0 || image_size > partition->size) {
    send_log(common::LogCode::kFlashFailed, common::LogSeverity::kError, 0, image_size);
    return;
  }

  // esp_ota_begin() efface les secteurs nécessaires à `image_size` avant de
  // renvoyer — c'est l'effacement exigé avant d'acquitter le BEGIN (voir
  // docs/firmware-implementation.md, phase 4, point 1).
  esp_ota_handle_t handle;
  esp_err_t err = esp_ota_begin(partition, image_size, &handle);
  if (err != ESP_OK) {
    ESP_LOGW(kTag, "esp_ota_begin échec: %s", esp_err_to_name(err));
    send_log(common::LogCode::kFlashFailed, common::LogSeverity::kError, 0, image_size);
    return;
  }

  reset_flash_state();
  g_flash.partition = partition;
  g_flash.ota_handle = handle;
  g_flash.image_size = image_size;

  send_log(common::LogCode::kFlashBegin, common::LogSeverity::kInfo, 0, image_size);
  send_flash_ack(common::FlashSubCmd::kBlockAck, 0, 0);
}

void on_flash_end(uint32_t expected_crc32) {
  if (g_flash.partition == nullptr) {
    send_log(common::LogCode::kFlashFailed, common::LogSeverity::kError);
    return;
  }
  // Bloc final incomplet : END n'aurait pas dû arriver avant le dernier
  // BLOCK_ACK. Traité comme un échec, pas rattrapable ici.
  if (g_flash.block_buf_len != 0 || g_flash.bytes_written != g_flash.image_size ||
      g_flash.image_crc.finish() != expected_crc32) {
    flash_abort("CRC32 global ou taille reçue incohérente au END");
    send_log(common::LogCode::kFlashFailed, common::LogSeverity::kError, 0, g_flash.bytes_written);
    return;
  }

  esp_err_t err = esp_ota_end(g_flash.ota_handle);
  if (err != ESP_OK) {
    ESP_LOGW(kTag, "esp_ota_end échec: %s", esp_err_to_name(err));
    reset_flash_state();
    send_log(common::LogCode::kFlashFailed, common::LogSeverity::kError);
    return;
  }
  err = esp_ota_set_boot_partition(g_flash.partition);
  if (err != ESP_OK) {
    ESP_LOGW(kTag, "esp_ota_set_boot_partition échec: %s", esp_err_to_name(err));
    reset_flash_state();
    send_log(common::LogCode::kFlashFailed, common::LogSeverity::kError);
    return;
  }

  reset_flash_state();
  send_log(common::LogCode::kFlashDone, common::LogSeverity::kInfo);
  // flash_client.py n'attend qu'un FLASH_CTRL de sous-commande END en retour
  // (le contenu importe peu) pour considérer le transfert confirmé.
  send_flash_ack(common::FlashSubCmd::kEnd, 0, 0);

  vTaskDelay(pdMS_TO_TICKS(50));  // laisser partir le LOG/ACK avant le reboot
  esp_restart();
}

void on_flash_ctrl_received(const uint8_t* data, size_t len) {
  common::FlashCtrlPayload payload;
  if (!common::FlashCtrlPayload::unpack(data, len, &payload)) {
    return;
  }
  switch (payload.subcmd) {
    case common::FlashSubCmd::kBegin:
      on_flash_begin(payload.image_size);
      break;
    case common::FlashSubCmd::kEnd:
      on_flash_end(payload.image_crc32);
      break;
    case common::FlashSubCmd::kAbort:
      if (g_flash.partition != nullptr) {
        flash_abort("ABORT reçu");
        send_log(common::LogCode::kFlashFailed, common::LogSeverity::kWarn);
      }
      break;
    case common::FlashSubCmd::kBlockAck:
      // BLOCK_ACK n'est émis que par le récepteur (nous) ; rien à faire si
      // on le reçoit (écran mal aligné sur son propre rôle).
      break;
  }
}

// FLASH_DATA (0x39) : la trame CAN entière (jusqu'à 8 octets) est la
// donnée, aucun en-tête. Écrit au fil de l'eau, jamais l'image entière en
// RAM — voir docs/firmware-implementation.md, phase 4, point 2.
//
// Limite connue : si un bloc est rejoué par flash_client.py après un
// BLOCK_ACK dont le CRC ne correspondait pas à ce qu'il a envoyé, ce
// firmware n'a aucun moyen de distinguer ce rejeu d'un bloc suivant — les
// octets rejoués sont alors traités comme la suite de l'image, ce qui la
// corrompt. Le CRC16 par bloc protège contre une corruption silencieuse
// (le transfert échouera au CRC32 global du END plutôt que de flasher une
// image fausse), mais ne permet pas un vrai rattrapage bloc par bloc tant
// que le protocole n'a pas de signal explicite de rejeu. Non exercé sur le
// vrai bus avant cette session : le CAN a son propre CRC/ACK matériel, donc
// le cas ne devrait se déclencher qu'en cas de bug logiciel, pas de bruit
// électrique — mais c'est un point ouvert, pas une garantie.
void on_flash_data_received(const uint8_t* data, size_t len) {
  if (g_flash.partition == nullptr || len == 0) {
    return;
  }

  size_t remaining_in_image = g_flash.image_size - g_flash.bytes_written - g_flash.block_buf_len;
  size_t to_copy = len < remaining_in_image ? len : remaining_in_image;
  if (to_copy == 0) {
    return;
  }
  std::memcpy(&g_flash.block_buf[g_flash.block_buf_len], data, to_copy);
  g_flash.block_buf_len += to_copy;

  size_t bytes_left_total = g_flash.image_size - g_flash.bytes_written;
  size_t expected_block_size = bytes_left_total < kFlashBlockSize ? bytes_left_total : kFlashBlockSize;
  if (g_flash.block_buf_len < expected_block_size) {
    return;
  }

  esp_err_t err = esp_ota_write(g_flash.ota_handle, g_flash.block_buf, g_flash.block_buf_len);
  if (err != ESP_OK) {
    ESP_LOGW(kTag, "esp_ota_write échec: %s", esp_err_to_name(err));
    flash_abort("esp_ota_write échec");
    send_log(common::LogCode::kFlashFailed, common::LogSeverity::kError, 0, g_flash.bytes_written);
    return;
  }
  g_flash.image_crc.update(g_flash.block_buf, g_flash.block_buf_len);
  uint16_t block_crc16 = common::crc16_ccitt(g_flash.block_buf, g_flash.block_buf_len);
  g_flash.bytes_written += g_flash.block_buf_len;
  g_flash.block_buf_len = 0;
  g_flash.block_number += 1;

  send_log(common::LogCode::kFlashProgress, common::LogSeverity::kDebug, 0, g_flash.bytes_written);
  send_flash_ack(common::FlashSubCmd::kBlockAck, g_flash.block_number, block_crc16);
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
    case common::MessageType::kFlashCtrl:
      on_flash_ctrl_received(msg.data, msg.data_length_code);
      break;
    case common::MessageType::kFlashData:
      on_flash_data_received(msg.data, msg.data_length_code);
      break;
    default:
      // STATUS_* venant du bus : rien à faire ici avant la phase 5.
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

// Temporisateur d'invalidation OTA — voir docs/firmware-implementation.md,
// phase 4, point 3 : une image en PENDING_VERIFY qui ne reçoit jamais de
// PING/PONG doit rollback elle-même, IDF ne le fait pas tout seul.
// `mark_presence()` (donc `!g_presence_lost`) sert de preuve : n'importe
// quel PING/PONG reçu de l'écran/pont confirme que l'image tourne assez
// pour parler sur le bus.
void tick_ota_validation() {
  if (!g_ota_pending_verify) return;

  if (!g_presence_lost) {
    esp_ota_mark_app_valid_cancel_rollback();
    g_ota_pending_verify = false;
    send_log(common::LogCode::kOtaValidated, common::LogSeverity::kInfo);
    return;
  }

  if (now_us() - g_ota_pending_since_us > kOtaValidationTimeoutUs) {
    send_log(common::LogCode::kOtaRollback, common::LogSeverity::kError);
    vTaskDelay(pdMS_TO_TICKS(50));  // laisser partir le LOG avant le reboot
    esp_ota_mark_app_invalid_rollback_and_reboot();
    // N'atteint ce point que si l'appel ci-dessus a échoué (pas d'image
    // précédente valide, p.ex.) : pas de seconde tentative, la carte reste
    // en PENDING_VERIFY jusqu'au prochain reset.
    ESP_LOGE(kTag, "esp_ota_mark_app_invalid_rollback_and_reboot a échoué");
    g_ota_pending_verify = false;
  }
}

void safety_task(void*) {
  for (;;) {
    tick_lease();
    tick_presence();
    tick_runtime_lockout();
    tick_twai_counters();
    tick_ota_validation();
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

  // PENDING_VERIFY : voir docs/firmware-implementation.md, phase 4, point 3.
  // Ne jamais valider l'image tout de suite ici — tick_ota_validation() ne
  // le fait qu'après un PING/PONG confirmé sur le bus, ou rollback au bout
  // de kOtaValidationTimeoutUs.
  const esp_partition_t* running = esp_ota_get_running_partition();
  esp_ota_img_states_t ota_state;
  if (running != nullptr && esp_ota_get_state_partition(running, &ota_state) == ESP_OK &&
      ota_state == ESP_OTA_IMG_PENDING_VERIFY) {
    g_ota_pending_verify = true;
    g_ota_pending_since_us = now_us();
  }

  twai_general_config_t g_config = TWAI_GENERAL_CONFIG_DEFAULT(kGpioCanTx, kGpioCanRx, TWAI_MODE_NORMAL);
  twai_timing_config_t t_config = TWAI_TIMING_CONFIG_500KBITS();
  twai_filter_config_t f_config = TWAI_FILTER_CONFIG_ACCEPT_ALL();
  ESP_ERROR_CHECK(twai_driver_install(&g_config, &t_config, &f_config));
  ESP_ERROR_CHECK(twai_start());

  int64_t t0 = now_us();
  g_last_presence_rx_us = t0 - kPresenceTimeoutUs;  // en attente, pas encore vu de pair
  g_last_own_ping_us = 0;

  send_log(common::LogCode::kBoot, common::LogSeverity::kInfo);
  if (g_ota_pending_verify) {
    send_log(common::LogCode::kOtaPendingVerify, common::LogSeverity::kInfo);
  }
  ESP_LOGI(kTag, "boot, version %d.%d.%d, verrou=%s", common::kFirmwareVersionMajor,
           common::kFirmwareVersionMinor, common::kFirmwareVersionPatch,
           s_lockout_active ? "actif" : "libre");

  xTaskCreate(twai_rx_task, "twai_rx", 4096, nullptr, 10, nullptr);
  xTaskCreate(safety_task, "safety", 4096, nullptr, 5, nullptr);

  send_log(common::LogCode::kReady, common::LogSeverity::kInfo);
}
