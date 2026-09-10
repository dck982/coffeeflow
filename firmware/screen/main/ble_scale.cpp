#include "ble_scale.h"

#include <cctype>
#include <cstdint>
#include <cstring>

#include "esp_log.h"
#include "esp_timer.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "host/ble_gap.h"
#include "host/ble_gatt.h"
#include "host/ble_hs.h"
#include "host/util/util.h"
#include "nimble/nimble_port.h"
#include "nimble/nimble_port_freertos.h"
#include "os/os_mbuf.h"

#include "core/core.h"

namespace ble_scale {
namespace {

constexpr const char* kTag = "ble_scale";
constexpr uint32_t kHeartbeatMs = 2500;
constexpr uint32_t kIdlePublishMs = 1000;
constexpr size_t kRxCapacity = 128;

// UUID Acaia Lunar issus de l'implémentation de référence. Ils doivent être
// confirmés avec la balance réelle avant qu'une infusion au poids soit livrée.
const ble_uuid128_t kServiceUuid = BLE_UUID128_INIT(
    0x55, 0xE4, 0x05, 0xD2, 0xAF, 0x9F, 0xA9, 0x8F,
    0xE5, 0x4A, 0x7D, 0xFE, 0x43, 0x53, 0x53, 0x49);
const ble_uuid128_t kCharacteristicUuid = BLE_UUID128_INIT(
    0xB3, 0x9B, 0x72, 0x34, 0xBE, 0xEC, 0xD4, 0xA8,
    0xF4, 0x43, 0x41, 0x88, 0x43, 0x53, 0x53, 0x49);
const ble_uuid16_t kCccdUuid = BLE_UUID16_INIT(BLE_GATT_DSC_CLT_CFG_UUID16);

portMUX_TYPE g_lock = portMUX_INITIALIZER_UNLOCKED;
uint16_t g_connection = BLE_HS_CONN_HANDLE_NONE;
uint16_t g_value_handle = 0;
uint16_t g_service_end_handle = 0;
bool g_active = false;
bool g_subscribed = false;
bool g_started = false;
TaskHandle_t g_heartbeat_task = nullptr;
int64_t g_last_heartbeat_us = 0;
int64_t g_last_publish_us = 0;
uint8_t g_rx[kRxCapacity]{};
size_t g_rx_len = 0;

int gap_event(struct ble_gap_event* event, void* arg);

bool ready(uint16_t* connection, uint16_t* value_handle) {
  portENTER_CRITICAL(&g_lock);
  *connection = g_connection;
  *value_handle = g_value_handle;
  bool result = g_subscribed && *connection != BLE_HS_CONN_HANDLE_NONE && *value_handle != 0;
  portEXIT_CRITICAL(&g_lock);
  return result;
}

void clear_connection() {
  portENTER_CRITICAL(&g_lock);
  g_connection = BLE_HS_CONN_HANDLE_NONE;
  g_value_handle = 0;
  g_service_end_handle = 0;
  g_subscribed = false;
  g_rx_len = 0;
  portEXIT_CRITICAL(&g_lock);
  core::update_scale_connection(false);
}

bool contains_acaia_name(const struct ble_gap_disc_desc& disc) {
  struct ble_hs_adv_fields fields{};
  if (ble_hs_adv_parse_fields(&fields, disc.data, disc.length_data) != 0 || fields.name == nullptr) return false;
  char name[33]{};
  size_t n = fields.name_len < sizeof(name) - 1 ? fields.name_len : sizeof(name) - 1;
  for (size_t i = 0; i < n; ++i) name[i] = static_cast<char>(std::toupper(fields.name[i]));
  return std::strstr(name, "ACAIA") != nullptr || std::strstr(name, "LUNAR") != nullptr;
}

void scan() {
  uint8_t own_addr_type;
  if (ble_hs_id_infer_auto(0, &own_addr_type) != 0) return;
  struct ble_gap_disc_params params{};
  params.filter_duplicates = 1;
  params.passive = 0;
  int rc = ble_gap_disc(own_addr_type, BLE_HS_FOREVER, &params, gap_event, nullptr);
  if (rc != 0 && rc != BLE_HS_EALREADY) ESP_LOGW(kTag, "scan unavailable: %d", rc);
}

void send_frame(uint8_t cmd, const uint8_t* payload, size_t payload_len, bool length_prefix) {
  uint16_t connection, value_handle;
  if (!ready(&connection, &value_handle) || payload_len + (length_prefix ? 6 : 5) > 64) return;
  uint8_t frame[64]{};
  size_t n = 0;
  frame[n++] = 0xEF;
  frame[n++] = 0xDD;
  frame[n++] = cmd;
  if (length_prefix) frame[n++] = static_cast<uint8_t>(payload_len + 1);
  std::memcpy(frame + n, payload, payload_len);
  n += payload_len;
  uint8_t even = 0, odd = 0;
  for (size_t i = 3; i < n; ++i) {
    if ((i - 3) % 2 == 0) even += frame[i]; else odd += frame[i];
  }
  frame[n++] = even;
  frame[n++] = odd;
  int rc = ble_gattc_write_no_rsp_flat(connection, value_handle, frame, n);
  if (rc != 0) ESP_LOGW(kTag, "Acaia write failed: %d", rc);
}

void request_notifications_and_heartbeat() {
  const uint8_t id[] = {0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37,
                        0x38, 0x39, 0x30, 0x31, 0x32, 0x33, 0x34};
  const uint8_t heartbeat[] = {0};
  const uint8_t request[] = {0x00, 0x01, 0x01, 0x02, 0x02, 0x05, 0x03, 0x04};
  send_frame(0x0B, id, sizeof(id), false);
  send_frame(0x00, heartbeat, sizeof(heartbeat), true);
  send_frame(0x0C, request, sizeof(request), true);
}

float decode_weight(const uint8_t* data) {
  uint32_t raw = static_cast<uint32_t>(data[0]) | (static_cast<uint32_t>(data[1]) << 8) |
                 (static_cast<uint32_t>(data[2]) << 16) | (static_cast<uint32_t>(data[3]) << 24);
  float weight = static_cast<float>(raw);
  switch (data[4]) {
    case 1: weight /= 10.0f; break;
    case 2: weight /= 100.0f; break;
    case 3: weight /= 1000.0f; break;
    case 4: weight /= 10000.0f; break;
    default: break;
  }
  // Bit à confirmer par tare puis masse négative sur la Lunar réelle.
  return (data[5] & 0x02) != 0 ? -weight : weight;
}

void process_notification(const uint8_t* payload, size_t length) {
  size_t offset = 0;
  while (offset < length) {
    uint8_t type = payload[offset++];
    if (type == 5 && offset + 6 <= length) {
      float weight = decode_weight(payload + offset);
      bool active;
      int64_t last_publish;
      portENTER_CRITICAL(&g_lock);
      active = g_active;
      last_publish = g_last_publish_us;
      portEXIT_CRITICAL(&g_lock);
      int64_t now = esp_timer_get_time();
      if (active || now - last_publish >= static_cast<int64_t>(kIdlePublishMs) * 1000) {
        core::update_scale_weight(weight);
        portENTER_CRITICAL(&g_lock);
        g_last_publish_us = now;
        portEXIT_CRITICAL(&g_lock);
      }
      offset += 6;
    } else if (type == 7 && offset + 3 <= length) {
      offset += 3;  // chrono balance : hors algorithme CoffeeFlow
    } else if (type == 8 && offset < length) {
      offset += 1;  // boutons bruts : pas encore une action d'interface
    } else {
      break;  // sous-message inconnu, taille non encodée : conserver le cadrage.
    }
  }
}

void consume_rx(const uint8_t* data, size_t length) {
  if (length > kRxCapacity) return;
  if (g_rx_len + length > sizeof(g_rx)) g_rx_len = 0;
  std::memcpy(g_rx + g_rx_len, data, length);
  g_rx_len += length;
  while (g_rx_len >= 3) {
    if (g_rx[0] != 0xEF || g_rx[1] != 0xDD) {
      std::memmove(g_rx, g_rx + 1, --g_rx_len);
      continue;
    }
    if (g_rx[2] == 0x20) { clear_connection(); return; }
    if (g_rx_len < 6) return;
    size_t payload_len = g_rx[3];
    size_t frame_len = payload_len + 5;
    if (payload_len == 0 || frame_len > sizeof(g_rx)) { g_rx_len = 0; return; }
    if (g_rx_len < frame_len) return;
    if (g_rx[2] == 12) process_notification(g_rx + 4, payload_len);
    std::memmove(g_rx, g_rx + frame_len, g_rx_len - frame_len);
    g_rx_len -= frame_len;
  }
}

int subscription_done(uint16_t, const struct ble_gatt_error* error, struct ble_gatt_attr*, void*) {
  if (error->status == 0) {
    portENTER_CRITICAL(&g_lock);
    g_subscribed = true;
    portEXIT_CRITICAL(&g_lock);
    request_notifications_and_heartbeat();
  }
  return 0;
}

int descriptor_discovered(uint16_t connection, const struct ble_gatt_error* error,
                          uint16_t, const struct ble_gatt_dsc* descriptor, void*) {
  if (error->status != 0) return 0;
  if (ble_uuid_cmp(&descriptor->uuid.u, &kCccdUuid.u) != 0) return 0;
  const uint8_t enable_notifications[] = {1, 0};
  return ble_gattc_write_flat(connection, descriptor->handle, enable_notifications,
                              sizeof(enable_notifications), subscription_done, nullptr);
}

int characteristic_discovered(uint16_t connection, const struct ble_gatt_error* error,
                              const struct ble_gatt_chr* characteristic, void*) {
  if (error->status != 0 || characteristic == nullptr) return 0;
  if ((characteristic->properties & BLE_GATT_CHR_PROP_NOTIFY) == 0 ||
      (characteristic->properties & (BLE_GATT_CHR_PROP_WRITE | BLE_GATT_CHR_PROP_WRITE_NO_RSP)) == 0) return 0;
  portENTER_CRITICAL(&g_lock);
  g_value_handle = characteristic->val_handle;
  uint16_t service_end = g_service_end_handle;
  portEXIT_CRITICAL(&g_lock);
  return ble_gattc_disc_all_dscs(connection, characteristic->val_handle, service_end,
                                 descriptor_discovered, nullptr);
}

int service_discovered(uint16_t connection, const struct ble_gatt_error* error,
                       const struct ble_gatt_svc* service, void*) {
  if (error->status != 0 || service == nullptr) return 0;
  portENTER_CRITICAL(&g_lock);
  g_service_end_handle = service->end_handle;
  portEXIT_CRITICAL(&g_lock);
  return ble_gattc_disc_chrs_by_uuid(connection, service->start_handle, service->end_handle,
                                     &kCharacteristicUuid.u, characteristic_discovered, nullptr);
}

int gap_event(struct ble_gap_event* event, void*) {
  switch (event->type) {
    case BLE_GAP_EVENT_DISC:
      if (!contains_acaia_name(event->disc)) return 0;
      ble_gap_disc_cancel();
      {
        uint8_t own_addr_type;
        if (ble_hs_id_infer_auto(0, &own_addr_type) == 0) {
          struct ble_gap_conn_params params{};
          int rc = ble_gap_connect(own_addr_type, &event->disc.addr, 30000, &params, gap_event, nullptr);
          if (rc != 0) scan();
        }
      }
      return 0;
    case BLE_GAP_EVENT_CONNECT:
      if (event->connect.status != 0) { scan(); return 0; }
      portENTER_CRITICAL(&g_lock);
      g_connection = event->connect.conn_handle;
      portEXIT_CRITICAL(&g_lock);
      core::update_scale_connection(true);
      if (ble_gattc_disc_svc_by_uuid(event->connect.conn_handle, &kServiceUuid.u,
                                     service_discovered, nullptr) != 0) {
        ble_gap_terminate(event->connect.conn_handle, BLE_ERR_REM_USER_CONN_TERM);
      }
      return 0;
    case BLE_GAP_EVENT_DISCONNECT:
      clear_connection();
      scan();
      return 0;
    case BLE_GAP_EVENT_NOTIFY_RX: {
      size_t length = OS_MBUF_PKTLEN(event->notify_rx.om);
      uint8_t buffer[kRxCapacity];
      if (length <= sizeof(buffer) && os_mbuf_copydata(event->notify_rx.om, 0, length, buffer) == 0) {
        consume_rx(buffer, length);
      }
      return 0;
    }
    default:
      return 0;
  }
}

void on_reset(int reason) { ESP_LOGW(kTag, "NimBLE reset: %d", reason); clear_connection(); }
void on_sync() { if (ble_hs_util_ensure_addr(0) == 0) scan(); }
void host_task(void*) { nimble_port_run(); nimble_port_freertos_deinit(); }

void heartbeat_task(void*) {
  for (;;) {
    int64_t now = esp_timer_get_time();
    int64_t last;
    portENTER_CRITICAL(&g_lock);
    last = g_last_heartbeat_us;
    portEXIT_CRITICAL(&g_lock);
    if (now - last >= static_cast<int64_t>(kHeartbeatMs) * 1000) {
      request_notifications_and_heartbeat();
      portENTER_CRITICAL(&g_lock);
      g_last_heartbeat_us = now;
      portEXIT_CRITICAL(&g_lock);
    }
    vTaskDelay(pdMS_TO_TICKS(100));
  }
}

}  // namespace

void init() {
  if (g_started) return;
  if (nimble_port_init() != ESP_OK) { ESP_LOGE(kTag, "nimble_port_init failed"); return; }
  ble_hs_cfg.reset_cb = on_reset;
  ble_hs_cfg.sync_cb = on_sync;
  nimble_port_freertos_init(host_task);
  if (xTaskCreatePinnedToCore(heartbeat_task, "ble_acaia", 4096, nullptr, 5,
                              &g_heartbeat_task, 0) != pdPASS) {
    ESP_LOGE(kTag, "heartbeat task creation failed");
    nimble_port_stop();
    nimble_port_deinit();
    return;
  }
  g_started = true;
}

void stop() {
  if (!g_started) return;
  // La tâche hôte et la tâche heartbeat peuvent encore appeler NimBLE ; elles
  // doivent disparaître avant de libérer les allocations du contrôleur.
  if (g_heartbeat_task != nullptr) {
    vTaskDelete(g_heartbeat_task);
    g_heartbeat_task = nullptr;
  }
  set_active(false);
  clear_connection();
  nimble_port_stop();
  nimble_port_deinit();
  g_started = false;
  ESP_LOGI(kTag, "NimBLE et controleur completement desinitialises");
}

void set_active(bool active) {
  portENTER_CRITICAL(&g_lock);
  g_active = active;
  portEXIT_CRITICAL(&g_lock);
}

bool tare() {
  uint16_t connection, value_handle;
  if (!ready(&connection, &value_handle)) return false;
  const uint8_t payload[] = {0};
  send_frame(0x0F, payload, sizeof(payload), true);
  return true;
}

}  // namespace ble_scale
