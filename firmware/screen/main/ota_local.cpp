#include "ota_local.h"

#include <cstring>

#include "esp_log.h"
#include "esp_ota_ops.h"
#include "esp_partition.h"
#include "esp_timer.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"

#include "can_link.h"
#include "common/crc.hpp"
#include "common/framing.hpp"
#include "common/messages.hpp"
#include "common/protocol.hpp"
#include "serial_bridge.h"

namespace ota_local {

namespace {

constexpr const char* kTag = "screen";

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
    serial_bridge::write_raw(out, len);
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
    can_link::send_log(common::LogCode::kFlashFailed, common::LogSeverity::kError, 0, image_size);
    return;
  }

  // esp_ota_begin() efface les secteurs nécessaires à `image_size` avant de
  // renvoyer — c'est l'effacement exigé avant d'acquitter le BEGIN.
  esp_ota_handle_t handle;
  esp_err_t err = esp_ota_begin(partition, image_size, &handle);
  if (err != ESP_OK) {
    ESP_LOGW(kTag, "esp_ota_begin échec: %s", esp_err_to_name(err));
    can_link::send_log(common::LogCode::kFlashFailed, common::LogSeverity::kError, 0, image_size);
    return;
  }

  reset_flash_state();
  g_flash.partition = partition;
  g_flash.ota_handle = handle;
  g_flash.image_size = image_size;

  can_link::send_log(common::LogCode::kFlashBegin, common::LogSeverity::kInfo, 0, image_size);
  send_flash_ack_serial(common::FlashSubCmd::kBlockAck, 0, 0);
}

void on_flash_end(uint32_t expected_crc32) {
  if (g_flash.partition == nullptr) {
    can_link::send_log(common::LogCode::kFlashFailed, common::LogSeverity::kError);
    return;
  }
  if (g_flash.block_buf_len != 0 || g_flash.bytes_written != g_flash.image_size ||
      g_flash.image_crc.finish() != expected_crc32) {
    flash_abort("CRC32 global ou taille reçue incohérente au END");
    can_link::send_log(common::LogCode::kFlashFailed, common::LogSeverity::kError, 0, g_flash.bytes_written);
    return;
  }

  esp_err_t err = esp_ota_end(g_flash.ota_handle);
  if (err != ESP_OK) {
    ESP_LOGW(kTag, "esp_ota_end échec: %s", esp_err_to_name(err));
    reset_flash_state();
    can_link::send_log(common::LogCode::kFlashFailed, common::LogSeverity::kError);
    return;
  }
  err = esp_ota_set_boot_partition(g_flash.partition);
  if (err != ESP_OK) {
    ESP_LOGW(kTag, "esp_ota_set_boot_partition échec: %s", esp_err_to_name(err));
    reset_flash_state();
    can_link::send_log(common::LogCode::kFlashFailed, common::LogSeverity::kError);
    return;
  }

  reset_flash_state();
  can_link::send_log(common::LogCode::kFlashDone, common::LogSeverity::kInfo);
  // flash_client.py n'attend qu'un FLASH_CTRL de sous-commande END en retour
  // (le contenu importe peu) pour considérer le transfert confirmé.
  send_flash_ack_serial(common::FlashSubCmd::kEnd, 0, 0);

  vTaskDelay(pdMS_TO_TICKS(50));  // laisser partir le LOG/ACK avant le reboot
  esp_restart();
}

// Temporisateur d'invalidation OTA — voir docs/firmware-implementation.md,
// phase 4 point 3, et sensors/main.cpp::tick_ota_validation() (même
// mécanique). IDF ne redémarre jamais tout seul une image en
// PENDING_VERIFY ; preuve de vie = un PING/PONG reçu de sensors sur le CAN
// (can_link::presence_lost() à faux).
void tick_ota_validation() {
  if (!g_ota_pending_verify) return;

  if (!can_link::presence_lost()) {
    esp_ota_mark_app_valid_cancel_rollback();
    g_ota_pending_verify = false;
    can_link::send_log(common::LogCode::kOtaValidated, common::LogSeverity::kInfo);
    return;
  }

  if (now_us() - g_ota_pending_since_us > kOtaValidationTimeoutUs) {
    can_link::send_log(common::LogCode::kOtaRollback, common::LogSeverity::kError);
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

}  // namespace

void init_pending_verify() {
  const esp_partition_t* running = esp_ota_get_running_partition();
  esp_ota_img_states_t ota_state;
  if (running != nullptr && esp_ota_get_state_partition(running, &ota_state) == ESP_OK &&
      ota_state == ESP_OTA_IMG_PENDING_VERIFY) {
    g_ota_pending_verify = true;
    g_ota_pending_since_us = now_us();
  }
}

bool pending_verify() { return g_ota_pending_verify; }

void start_validation_task() {
  xTaskCreatePinnedToCore(ota_validation_task, "ota_valid", 4096, nullptr, 5, nullptr, 0);
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
        can_link::send_log(common::LogCode::kFlashFailed, common::LogSeverity::kWarn);
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
    can_link::send_log(common::LogCode::kFlashFailed, common::LogSeverity::kError, 0, g_flash.bytes_written);
    return;
  }
  g_flash.image_crc.update(g_flash.block_buf, g_flash.block_buf_len);
  uint16_t block_crc16 = common::crc16_ccitt(g_flash.block_buf, g_flash.block_buf_len);
  g_flash.bytes_written += g_flash.block_buf_len;
  g_flash.block_buf_len = 0;
  g_flash.block_number += 1;

  can_link::send_log(common::LogCode::kFlashProgress, common::LogSeverity::kDebug, 0, g_flash.bytes_written);
  send_flash_ack_serial(common::FlashSubCmd::kBlockAck, g_flash.block_number, block_crc16);
}

}  // namespace ota_local
