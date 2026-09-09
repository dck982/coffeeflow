// Module écran (Waveshare ESP32-S3-Touch-LCD-4.3) — phase 3 : pont USB <->
// CAN, voir docs/firmware-implementation.md et docs/firmware.md.
//
// Deux rôles à la fois, comme prévu par la doc :
//   - Pont transparent, même cadrage que firmware/can-monitor (COBS + PDU +
//     CRC16, common/framing.hpp) : toute trame CAN reçue est réémise telle
//     quelle sur l'USB, tout PDU décodé depuis l'USB est réémis tel quel sur
//     le bus. coffeetool s'en sert exactement comme de can-monitor.
//   - Nœud kScreen à part entière (contrairement à can-monitor qui n'a
//     aucune identité) : répond aux PING venus des capteurs, se reboote sur
//     RESET, LOG au boot. Nécessaire pour que la présence côté capteurs
//     (tick_presence(), sensors/main.cpp) ait un vrai répondant sur le bus.
//
// Le pont passe par UART_NUM_2 réaffecté sur GPIO43(TX)/44(RX) via la
// matrice GPIO, pas l'USB natif de l'ESP32-S3 : sur ce banc, le câble USB-C
// est branché sur le port "UART" de la carte (bridge WCH CH343P externe).
// GPIO43/44 sont les broches par défaut d'UART0 (celles du boot ROM), donc
// U2 doit les reprendre explicitement.
//
// Deux bugs de bring-up trouvés et corrigés ici (2026-09-09), tous deux
// silencieux — aucun message d'erreur, juste une réception qui ne marche
// jamais :
//   1. Régression IDF v5.x -> v6.1 : `uart_set_pin()` route bien GPIO44 vers
//      UART2 via la matrice GPIO, mais sa branche RX (gpio_hal_matrix_in())
//      ne force plus le pad en PIN_FUNC_GPIO comme le faisait v5.1 (la
//      branche TX, elle, le fait toujours) — sans forcer nous-mêmes le pad,
//      GPIO44 reste sur sa fonction IOMUX de reset (U0RXD), jamais lue.
//      Corrigé par un appel explicite à gpio_func_sel()/gpio_input_enable()/
//      gpio_pullup_en() juste après uart_set_pin().
//   2. `uart_read_bytes(..., portMAX_DELAY)` ne se réveille jamais sur cet
//      UART2 réaffecté, même une fois le bug 1 corrigé et la FIFO
//      effectivement pleine (confirmé par uart_get_buffered_data_len()) —
//      un timeout court (20 ms) en boucle, comme le fait l'exemple officiel
//      Waveshare (05_UART_Test), fonctionne à chaque essai.
// Les deux bugs ont été diagnostiqués en comparant notre code à
// waveshareteam/ESP32-S3-Touch-LCD-4.3 (cloné dans tmp/), et en confirmant
// par bissection matérielle (sonde directe de GPIO44, uart_get_buffered_data_len())
// que le signal physique et la FIFO UART étaient sains avant de soupçonner
// l'appel bloquant. Voir docs/firmware-implementation.md pour le détail.
//
// Ni LVGL, ni Wi-Fi, ni BLE, ni logique d'infusion : voir docs/firmware.md,
// "Le Waveshare reste atteignable en USB-C... image factory minuscule".

#include <cstring>

#include "driver/gpio.h"
#include "driver/i2c_master.h"
#include "driver/twai.h"
#include "driver/uart.h"
#include "esp_log.h"
#include "esp_ota_ops.h"
#include "esp_partition.h"
#include "esp_private/gpio.h"
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

// Brochage — voir docs/firmware.md, "Module écran", et
// tests/screen/hello_waveshare/ pour le bring-up CH422G/GT911 de référence
// (I2C partagé avec le tactile, non utilisé ici).
constexpr gpio_num_t kI2cSda = GPIO_NUM_8;
constexpr gpio_num_t kI2cScl = GPIO_NUM_9;
// GPIO20=TX, GPIO19=RX : confirmé contre les exemples officiels Waveshare
// (waveshareteam/ESP32-S3-Touch-LCD-4.3, examples/ESP-IDF/06_TWAItransmit et
// 07_TWAIreceive, sdkconfig.defaults) — pas 15/16 comme une première lecture
// de la fiche produit l'avait suggéré. GPIO19/20 sont les broches natives
// D+/D- de l'USB de l'ESP32-S3, basculées vers CAN_TX/CAN_RX par le mux
// analogique FSUSB42UMX quand CAN_SEL (EXIO5) passe haut — d'où le partage
// de bit avec EXIO_USB_SEL.
constexpr gpio_num_t kCanTx = GPIO_NUM_20;
constexpr gpio_num_t kCanRx = GPIO_NUM_19;

// UART2 réaffecté sur GPIO43/44 (le port "UART" réellement câblé sur ce
// banc, pas l'USB natif) — voir le commentaire en tête de fichier pour les
// deux bugs de bring-up et leurs correctifs.
constexpr uart_port_t kUartNum = UART_NUM_2;
constexpr gpio_num_t kUartTx = GPIO_NUM_43;
constexpr gpio_num_t kUartRx = GPIO_NUM_44;
// uart_read_bytes(..., portMAX_DELAY) ne se réveille jamais sur cet UART2 —
// voir le commentaire en tête de fichier, bug 2. Boucler avec ce délai court
// à la place.
constexpr TickType_t kUartReadTimeout = pdMS_TO_TICKS(20);

// CH422G : pas de registre au sens I2C classique, l'adresse elle-même
// sélectionne la fonction (voir tests/screen/hello_waveshare/, même
// convention). 0x24 = registre de mode (direction des EXIO), 0x38 =
// registre de sortie.
constexpr uint16_t kCh422gModeAddr = 0x24;
constexpr uint16_t kCh422gOutAddr = 0x38;
constexpr uint8_t kCh422gModeOutputs = 0x01;  // EXIO0-7 en push-pull
// CAN_SEL = EXIO5 (docs/cablage.md, docs/firmware.md) : tenu haut, sinon le
// mux analogique FSUSB42UMX reste en position USB (GPIO19/20 routés vers le
// connecteur USB-C natif) au lieu de CAN (GPIO19/20 routés vers le
// transceiver CAN) — voir waveshareteam/ESP32-S3-Touch-LCD-4.3,
// examples/ESP-IDF/06_TWAItransmit. Même bit que EXIO_USB_SEL vu par
// tests/screen/hello_waveshare/ (ligne partagée, deux noms selon la
// fonction regardée). Les autres EXIO (touch/LCD/SD) ne sont pas pilotés
// ici : cette image ne s'en sert pas.
constexpr uint8_t kCh422gOutCanSel = 1 << 5;

constexpr const char* kTag = "screen";

// Présence — voir docs/firmware.md §2 : un PING ou un PONG reçu du pair
// suffit, c'est le nœud qui répond qui doit remettre son propre compteur à
// zéro (TWAI ne boucle pas ses propres trames). Pas de bail/verrou ici :
// l'écran ne pilote aucun actionneur, cette logique est spécifique aux
// capteurs. Réutilisée aussi comme preuve de vie pour la validation OTA de
// l'écran lui-même, voir tick_ota_validation() plus bas.
int64_t g_last_presence_rx_us = 0;
bool g_presence_lost = true;

// OTA de l'écran lui-même — voir docs/firmware-implementation.md, phase 4
// point 3 côté écran : Mac -> USB -> screen directement, sans passer par le
// CAN (screen étant à la fois pont et destinataire). Même mécanique que
// sensors/main.cpp (PENDING_VERIFY + temporisateur d'invalidation, preuve de
// vie = un PING/PONG reçu de sensors sur le bus).
constexpr int64_t kOtaValidationTimeoutUs = 30 * 1000 * 1000;
constexpr size_t kFlashBlockSize = 2048;
bool g_ota_pending_verify = false;
int64_t g_ota_pending_since_us = 0;

// Flash — un seul transfert à la fois, comme côté sensors. `partition` non
// nul == transfert en cours (BEGIN reçu, END/ABORT pas encore traité).
struct FlashState {
  const esp_partition_t* partition = nullptr;
  esp_ota_handle_t ota_handle = 0;
  uint32_t image_size = 0;
  uint32_t bytes_written = 0;
  uint16_t block_number = 0;
  uint8_t block_buf[kFlashBlockSize];
  size_t block_buf_len = 0;
  common::Crc32Incremental image_crc;
};
FlashState g_flash;

int64_t now_us() { return esp_timer_get_time(); }

// CAN_SEL avant tout le reste : voir docs/firmware.md, "doit être tenu
// haut, sinon le transceiver n'est pas sélectionné".
void ch422g_select_can() {
  i2c_master_bus_config_t bus_config{};
  bus_config.i2c_port = -1;
  bus_config.sda_io_num = kI2cSda;
  bus_config.scl_io_num = kI2cScl;
  bus_config.clk_source = I2C_CLK_SRC_DEFAULT;
  bus_config.glitch_ignore_cnt = 7;
  bus_config.flags.enable_internal_pullup = true;
  i2c_master_bus_handle_t bus;
  ESP_ERROR_CHECK(i2c_new_master_bus(&bus_config, &bus));

  i2c_device_config_t mode_dev_cfg{};
  mode_dev_cfg.dev_addr_length = I2C_ADDR_BIT_LEN_7;
  mode_dev_cfg.device_address = kCh422gModeAddr;
  mode_dev_cfg.scl_speed_hz = 400000;
  i2c_master_dev_handle_t mode_dev;
  ESP_ERROR_CHECK(i2c_master_bus_add_device(bus, &mode_dev_cfg, &mode_dev));

  i2c_device_config_t out_dev_cfg{};
  out_dev_cfg.dev_addr_length = I2C_ADDR_BIT_LEN_7;
  out_dev_cfg.device_address = kCh422gOutAddr;
  out_dev_cfg.scl_speed_hz = 400000;
  i2c_master_dev_handle_t out_dev;
  ESP_ERROR_CHECK(i2c_master_bus_add_device(bus, &out_dev_cfg, &out_dev));

  uint8_t mode_val = kCh422gModeOutputs;
  ESP_ERROR_CHECK(i2c_master_transmit(mode_dev, &mode_val, 1, -1));
  uint8_t out_val = kCh422gOutCanSel;
  ESP_ERROR_CHECK(i2c_master_transmit(out_dev, &out_val, 1, -1));
}

// Transmet sur le bus ET mirroir sur l'USB : TWAI ne boucle pas nos propres
// trames, donc sans ce mirroir coffeetool ne verrait jamais un PONG/LOG que
// nous générons nous-mêmes (contrairement à ce qui transite par le pont,
// déjà vu deux fois : une fois à la réception CAN pour le pont, une fois ici
// pour nos propres messages).
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
    uart_write_bytes(kUartNum, reinterpret_cast<const char*>(out), len);
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
  payload.node = common::Node::kScreen;
  payload.version_major = common::kFirmwareVersionMajor;
  payload.version_minor = common::kFirmwareVersionMinor;
  payload.version_patch = common::kFirmwareVersionPatch;
  payload.uptime_s = static_cast<uint32_t>(now_us() / 1000000);
  common::Frame f = payload.pack();
  send_message(common::MessageType::kPong, common::Dest::kSensors, f.data(), 8);
}

// Réponses de flash — écrites uniquement sur l'UART (le lien Mac<->screen
// direct), pas sur le CAN : contrairement au flash de sensors (relayé à
// travers l'écran-pont), rien ici ne concerne le bus. `flash_client.py`
// n'inspecte que le champ type de l'identifiant CAN encodé dans le PDU, pas
// sa source/destination — voir _wait_flash_ctrl côté Mac.
void send_flash_ack_serial(common::FlashSubCmd subcmd, uint16_t block_number, uint16_t block_crc16) {
  common::FlashCtrlPayload payload;
  payload.subcmd = subcmd;
  payload.block_number = block_number;
  payload.block_crc16 = block_crc16;
  common::Frame f = payload.pack();

  common::CanId id{common::MessageType::kFlashCtrl, common::Dest::kSensors, common::Node::kScreen};
  common::RawFrame frame;
  frame.can_id = common::encode_can_id(id);
  frame.dlc = 8;
  std::memcpy(frame.data.data(), f.data(), 8);

  uint8_t out[common::kMaxCobsSize + 1];
  size_t len = common::encode_framed(frame, out);
  if (len > 0) {
    uart_write_bytes(kUartNum, reinterpret_cast<const char*>(out), len);
  }
}

// Remet l'état de flash à zéro sans toucher au handle OTA (à faire avant, si
// un handle est ouvert) — même logique que sensors/main.cpp.
void reset_flash_state() {
  g_flash.partition = nullptr;
  g_flash.ota_handle = 0;
  g_flash.image_size = 0;
  g_flash.bytes_written = 0;
  g_flash.block_number = 0;
  g_flash.block_buf_len = 0;
  g_flash.image_crc = common::Crc32Incremental{};
}

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
  // renvoyer — c'est l'effacement exigé avant d'acquitter le BEGIN.
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
  send_flash_ack_serial(common::FlashSubCmd::kBlockAck, 0, 0);
}

void on_flash_end(uint32_t expected_crc32) {
  if (g_flash.partition == nullptr) {
    send_log(common::LogCode::kFlashFailed, common::LogSeverity::kError);
    return;
  }
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
  send_flash_ack_serial(common::FlashSubCmd::kEnd, 0, 0);

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
      // BLOCK_ACK n'est émis que par nous (le récepteur) ; rien à faire si on
      // le reçoit.
      break;
  }
}

// FLASH_DATA (0x39) : la trame entière (jusqu'à 8 octets) est la donnée,
// aucun en-tête. Écrit au fil de l'eau, jamais l'image entière en RAM — même
// logique et même limite connue (rejeu de bloc non distinguable) que
// sensors/main.cpp, voir son commentaire pour le détail.
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
  send_flash_ack_serial(common::FlashSubCmd::kBlockAck, g_flash.block_number, block_crc16);
}

// Temporisateur d'invalidation OTA — voir docs/firmware-implementation.md,
// phase 4 point 3, et sensors/main.cpp::tick_ota_validation() (même
// mécanique). IDF ne redémarre jamais tout seul une image en
// PENDING_VERIFY ; preuve de vie = un PING/PONG reçu de sensors sur le CAN
// (mark_presence(), donc !g_presence_lost).
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
    // précédente valide, p.ex.) : pas de seconde tentative.
    ESP_LOGE(kTag, "esp_ota_mark_app_invalid_rollback_and_reboot a échoué");
    g_ota_pending_verify = false;
  }
}

void ota_validation_task(void*) {
  for (;;) {
    tick_ota_validation();
    vTaskDelay(pdMS_TO_TICKS(100));
  }
}

void mark_presence() {
  g_last_presence_rx_us = now_us();
  g_presence_lost = false;
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

// Notre propre rôle de nœud kScreen : ne réagit qu'à ce qui vient des
// capteurs et nous est adressé ou en broadcast. Tout le reste (y compris ce
// qui vient des capteurs mais ne nous concerne pas) est déjà passé sur l'USB
// par can_to_serial_task, sans repasser ici.
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

// CAN -> série : chaque trame reçue devient un PDU encadré écrit tel quel
// sur l'USB (pont, comme can-monitor), et est aussi soumise à notre propre
// protocole (nœud kScreen).
void can_to_serial_task(void*) {
  for (;;) {
    twai_message_t msg;
    if (twai_receive(&msg, portMAX_DELAY) != ESP_OK) {
      continue;
    }

    common::RawFrame frame;
    frame.can_id = static_cast<uint16_t>(msg.identifier);
    frame.dlc = msg.data_length_code;
    std::memcpy(frame.data.data(), msg.data, frame.dlc);

    uint8_t out[common::kMaxCobsSize + 1];
    size_t len = common::encode_framed(frame, out);
    if (len > 0) {
      uart_write_bytes(kUartNum, reinterpret_cast<const char*>(out), len);
    }

    dispatch_own_protocol(msg);
  }
}

// Série -> CAN : relais aveugle, comme can-monitor, SAUF pour notre propre
// OTA (FLASH_CTRL/FLASH_DATA adressé à kScreen) — c'est un échange direct
// Mac<->screen par USB, voir docs/firmware-implementation.md : rien à
// relayer sur le bus dans ce cas, et surtout pas les 256 trames FLASH_DATA
// par bloc qui n'ont aucun sens pour sensors. coffeetool émet déjà avec
// l'identité qu'il veut pour le reste, rien à réinterpréter ici.
void on_frame_from_serial(const common::RawFrame& frame, void* /*ctx*/) {
  common::CanId id = common::decode_can_id(frame.can_id);
  if (id.dest == common::Dest::kScreen &&
      (id.type == common::MessageType::kFlashCtrl || id.type == common::MessageType::kFlashData)) {
    if (id.type == common::MessageType::kFlashCtrl) {
      on_flash_ctrl_received(frame.data.data(), frame.dlc);
    } else {
      on_flash_data_received(frame.data.data(), frame.dlc);
    }
    return;
  }

  twai_message_t msg{};
  msg.identifier = frame.can_id;
  msg.data_length_code = frame.dlc;
  std::memcpy(msg.data, frame.data.data(), frame.dlc);
  twai_transmit(&msg, pdMS_TO_TICKS(50));
}

void serial_to_can_task(void*) {
  common::StreamDecoder decoder;
  uint8_t buf[64];
  for (;;) {
    // Timeout court, pas portMAX_DELAY : voir le commentaire en tête de
    // fichier, bug 2 — uart_read_bytes() bloquant indéfiniment ne se
    // réveille jamais sur cet UART2 réaffecté.
    int n = uart_read_bytes(kUartNum, buf, sizeof(buf), kUartReadTimeout);
    for (int i = 0; i < n; ++i) {
      decoder.push_byte(buf[i], on_frame_from_serial, nullptr);
    }
  }
}

}  // namespace

extern "C" void app_main() {
  uart_config_t uart_config{};
  uart_config.baud_rate = 115200;
  uart_config.data_bits = UART_DATA_8_BITS;
  uart_config.parity = UART_PARITY_DISABLE;
  uart_config.stop_bits = UART_STOP_BITS_1;
  uart_config.flow_ctrl = UART_HW_FLOWCTRL_DISABLE;
  uart_config.source_clk = UART_SCLK_DEFAULT;
  ESP_ERROR_CHECK(uart_driver_install(kUartNum, 2048, 0, 0, nullptr, 0));
  ESP_ERROR_CHECK(uart_param_config(kUartNum, &uart_config));
  ESP_ERROR_CHECK(uart_set_pin(kUartNum, kUartTx, kUartRx, UART_PIN_NO_CHANGE, UART_PIN_NO_CHANGE));
  // Voir le commentaire en tête de fichier, bug 1 : régression IDF v5->v6.1,
  // gpio_hal_matrix_in() (branche RX de uart_set_pin) ne force plus le pad
  // en PIN_FUNC_GPIO — sans ça GPIO44 reste sur sa fonction IOMUX de reset
  // (U0RXD) et ne reçoit jamais rien.
  gpio_func_sel(kUartRx, PIN_FUNC_GPIO);
  gpio_input_enable(kUartRx);
  gpio_pullup_en(kUartRx);

  // CAN_SEL avant toute initialisation TWAI, voir docs/firmware.md : sans
  // ça, le transceiver n'est pas sélectionné et le bus reste muet, sans
  // erreur visible.
  ch422g_select_can();

  // PENDING_VERIFY : voir docs/firmware-implementation.md, phase 4 point 3,
  // et sensors/main.cpp (même mécanique). Ne jamais valider l'image tout de
  // suite ici — tick_ota_validation() ne le fait qu'après un PING/PONG
  // confirmé sur le bus, ou rollback au bout de kOtaValidationTimeoutUs.
  const esp_partition_t* running = esp_ota_get_running_partition();
  esp_ota_img_states_t ota_state;
  if (running != nullptr && esp_ota_get_state_partition(running, &ota_state) == ESP_OK &&
      ota_state == ESP_OTA_IMG_PENDING_VERIFY) {
    g_ota_pending_verify = true;
    g_ota_pending_since_us = now_us();
  }

  twai_general_config_t g_config = TWAI_GENERAL_CONFIG_DEFAULT(kCanTx, kCanRx, TWAI_MODE_NORMAL);
  twai_timing_config_t t_config = TWAI_TIMING_CONFIG_500KBITS();
  twai_filter_config_t f_config = TWAI_FILTER_CONFIG_ACCEPT_ALL();
  ESP_ERROR_CHECK(twai_driver_install(&g_config, &t_config, &f_config));
  ESP_ERROR_CHECK(twai_start());

  g_last_presence_rx_us = now_us();

  send_log(common::LogCode::kBoot, common::LogSeverity::kInfo);
  if (g_ota_pending_verify) {
    send_log(common::LogCode::kOtaPendingVerify, common::LogSeverity::kInfo);
  }

  xTaskCreate(can_to_serial_task, "can2ser", 4096, nullptr, 10, nullptr);
  xTaskCreate(serial_to_can_task, "ser2can", 4096, nullptr, 10, nullptr);
  xTaskCreate(ota_validation_task, "ota_valid", 4096, nullptr, 5, nullptr);

  send_log(common::LogCode::kReady, common::LogSeverity::kInfo);
}
