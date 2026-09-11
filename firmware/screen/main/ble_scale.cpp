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
#include "can_link.h"

namespace ble_scale {
namespace {

constexpr const char* kTag = "ble_scale";
constexpr uint32_t kHeartbeatMs = 2500;
constexpr uint32_t kIdlePublishMs = 1000;
constexpr size_t kRxCapacity = 128;

// Lunar recentes : canaux ecriture et notification distincts.
const ble_uuid128_t kModernServiceUuid = BLE_UUID128_INIT(
    0x55, 0xE4, 0x05, 0xD2, 0xAF, 0x9F, 0xA9, 0x8F,
    0xE5, 0x4A, 0x7D, 0xFE, 0x43, 0x53, 0x53, 0x49);
const ble_uuid128_t kModernWriteUuid = BLE_UUID128_INIT(
    0xB3, 0x9B, 0x72, 0x34, 0xBE, 0xEC, 0xD4, 0xA8,
    0xF4, 0x43, 0x41, 0x88, 0x43, 0x53, 0x53, 0x49);
const ble_uuid128_t kModernNotifyUuid = BLE_UUID128_INIT(
    0x16, 0x96, 0x24, 0x47, 0xC6, 0x23, 0x61, 0xBA,
    0xD9, 0x4B, 0x4D, 0x1E, 0x43, 0x53, 0x53, 0x49);
// Lunar legacy confirmee sur ACAIAL-C1EF4C9 le 2026-09-11.
const ble_uuid16_t kLegacyServiceUuid = BLE_UUID16_INIT(0x1820);
const ble_uuid16_t kLegacyCharacteristicUuid = BLE_UUID16_INIT(0x2A80);
const ble_uuid16_t kCccdUuid = BLE_UUID16_INIT(BLE_GATT_DSC_CLT_CFG_UUID16);

enum class DiscoveryMode : uint8_t { kLegacy, kModern };

portMUX_TYPE g_lock = portMUX_INITIALIZER_UNLOCKED;
uint16_t g_connection = BLE_HS_CONN_HANDLE_NONE;
uint16_t g_service_start_handle = 0;
uint16_t g_value_handle = 0;
uint16_t g_notify_handle = 0;
uint16_t g_service_end_handle = 0;
uint16_t g_cccd_handle = 0;
DiscoveryMode g_discovery_mode = DiscoveryMode::kLegacy;
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
  bool was_connected;
  portENTER_CRITICAL(&g_lock);
  was_connected = g_connection != BLE_HS_CONN_HANDLE_NONE;
  g_connection = BLE_HS_CONN_HANDLE_NONE;
  g_service_start_handle = 0;
  g_value_handle = 0;
  g_notify_handle = 0;
  g_service_end_handle = 0;
  g_cccd_handle = 0;
  g_subscribed = false;
  g_rx_len = 0;
  portEXIT_CRITICAL(&g_lock);
  core::update_scale_connection(false);
  if (was_connected) can_link::send_log(common::LogCode::kBleDisconnected, common::LogSeverity::kWarn);
}

bool is_acaia_advertisement(const struct ble_gap_disc_desc& disc) {
  struct ble_hs_adv_fields fields{};
  if (ble_hs_adv_parse_fields(&fields, disc.data, disc.length_data) != 0) return false;

  // La Lunar annonce normalement ce service. Le nom est un repli utile pour
  // les versions qui ne mettent l'UUID que dans la scan response.
  for (uint8_t i = 0; i < fields.num_uuids128; ++i) {
    if (ble_uuid_cmp(&fields.uuids128[i].u, &kModernServiceUuid.u) == 0) return true;
  }
  for (uint8_t i = 0; i < fields.num_uuids16; ++i)
    if (ble_uuid_cmp(&fields.uuids16[i].u, &kLegacyServiceUuid.u) == 0) return true;

  if (fields.name == nullptr) return false;
  char name[33]{};
  size_t n = fields.name_len < sizeof(name) - 1 ? fields.name_len : sizeof(name) - 1;
  for (size_t i = 0; i < n; ++i) name[i] = static_cast<char>(std::toupper(fields.name[i]));
  return std::strstr(name, "ACAIA") != nullptr || std::strstr(name, "LUNAR") != nullptr;
}

void discovery_failed(uint16_t connection, const char* stage, int status) {
  ESP_LOGW(kTag, "Acaia %s failed: %d", stage, status);
  can_link::send_log(common::LogCode::kBleError, common::LogSeverity::kWarn,
                     static_cast<uint16_t>(status));
  ble_gap_terminate(connection, BLE_ERR_REM_USER_CONN_TERM);
}

void scan() {
  uint8_t own_addr_type;
  int rc = ble_hs_id_infer_auto(0, &own_addr_type);
  if (rc != 0) {
    can_link::send_log(common::LogCode::kBleError, common::LogSeverity::kWarn,
                       static_cast<uint16_t>(rc));
    return;
  }
  struct ble_gap_disc_params params{};
  // La Lunar peut diffuser l'UUID dans l'advertising et son nom dans la scan
  // response. Le contrôleur du S3 déduplique sinon par adresse, et masque le
  // second paquet avant que NimBLE nous le livre.
  params.filter_duplicates = 0;
  params.passive = 0;
  rc = ble_gap_disc(own_addr_type, BLE_HS_FOREVER, &params, gap_event, nullptr);
  if (rc == 0) {
    can_link::send_log(common::LogCode::kBleScanStarted, common::LogSeverity::kDebug);
  } else if (rc != BLE_HS_EALREADY) {
    ESP_LOGW(kTag, "scan unavailable: %d", rc);
    can_link::send_log(common::LogCode::kBleError, common::LogSeverity::kWarn,
                       static_cast<uint16_t>(rc));
  }
}

void connect_to_device(const ble_addr_t& address) {
  uint8_t own_addr_type;
  int rc = ble_hs_id_infer_auto(0, &own_addr_type);
  if (rc == 0) {
    // nullptr demande les paramètres de connexion valides par défaut de
    // NimBLE. Une structure value-initialized contient des zéros invalides.
    rc = ble_gap_connect(own_addr_type, &address, 30000, nullptr, gap_event, nullptr);
  }
  if (rc != 0) {
    ESP_LOGW(kTag, "Acaia connect unavailable: %d", rc);
    can_link::send_log(common::LogCode::kBleError, common::LogSeverity::kWarn,
                       static_cast<uint16_t>(rc));
    scan();
  }
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

int subscription_done(uint16_t connection, const struct ble_gatt_error* error,
                      struct ble_gatt_attr*, void*) {
  if (error->status == 0) {
    portENTER_CRITICAL(&g_lock);
    g_subscribed = true;
    portEXIT_CRITICAL(&g_lock);
    can_link::send_log(common::LogCode::kBleSubscribed, common::LogSeverity::kInfo);
    request_notifications_and_heartbeat();
  } else {
    ESP_LOGW(kTag, "Acaia notification subscription failed: %d", error->status);
    can_link::send_log(common::LogCode::kBleError, common::LogSeverity::kWarn,
                       static_cast<uint16_t>(error->status));
    ble_gap_terminate(connection, BLE_ERR_REM_USER_CONN_TERM);
  }
  return 0;
}

int start_characteristic_discovery(uint16_t connection);
int start_descriptor_discovery(uint16_t connection);

int descriptor_discovered(uint16_t connection, const struct ble_gatt_error* error,
                          uint16_t, const struct ble_gatt_dsc* descriptor, void*) {
  if (error->status == BLE_HS_EDONE) {
    portENTER_CRITICAL(&g_lock);
    uint16_t cccd_handle = g_cccd_handle;
    portEXIT_CRITICAL(&g_lock);
    if (cccd_handle == 0) {
      discovery_failed(connection, "CCCD discovery", error->status);
      return 0;
    }
    const uint8_t enable_notifications[] = {1, 0};
    int rc = ble_gattc_write_flat(connection, cccd_handle, enable_notifications,
                                  sizeof(enable_notifications), subscription_done, nullptr);
    if (rc != 0) discovery_failed(connection, "CCCD subscription start", rc);
    return rc;
  }
  if (error->status != 0) {
    discovery_failed(connection, "descriptor discovery", error->status);
    return 0;
  }
  if (ble_uuid_cmp(&descriptor->uuid.u, &kCccdUuid.u) != 0) return 0;
  portENTER_CRITICAL(&g_lock);
  g_cccd_handle = descriptor->handle;
  portEXIT_CRITICAL(&g_lock);
  return 0;
}

int characteristic_discovered(uint16_t connection, const struct ble_gatt_error* error,
                              const struct ble_gatt_chr* characteristic, void*) {
  if (error->status == BLE_HS_EDONE) {
    portENTER_CRITICAL(&g_lock);
    bool found = g_value_handle != 0 && g_notify_handle != 0;
    portEXIT_CRITICAL(&g_lock);
    if (!found) {
      discovery_failed(connection, "characteristic discovery", error->status);
      return 0;
    }
    return start_descriptor_discovery(connection);
  }
  if (error->status != 0 || characteristic == nullptr) {
    discovery_failed(connection, "characteristic discovery", error->status);
    return 0;
  }
  if (g_discovery_mode == DiscoveryMode::kModern) {
    portENTER_CRITICAL(&g_lock);
    if (ble_uuid_cmp(&characteristic->uuid.u, &kModernWriteUuid.u) == 0 &&
        (characteristic->properties & (BLE_GATT_CHR_PROP_WRITE | BLE_GATT_CHR_PROP_WRITE_NO_RSP)) != 0) {
      g_value_handle = characteristic->val_handle;
    }
    if (ble_uuid_cmp(&characteristic->uuid.u, &kModernNotifyUuid.u) == 0 &&
        (characteristic->properties & BLE_GATT_CHR_PROP_NOTIFY) != 0) {
      g_notify_handle = characteristic->val_handle;
    }
    portEXIT_CRITICAL(&g_lock);
    return 0;
  }
  if ((characteristic->properties & BLE_GATT_CHR_PROP_NOTIFY) == 0 ||
      (characteristic->properties & (BLE_GATT_CHR_PROP_WRITE | BLE_GATT_CHR_PROP_WRITE_NO_RSP)) == 0) return 0;
  portENTER_CRITICAL(&g_lock);
  g_value_handle = characteristic->val_handle;
  g_notify_handle = characteristic->val_handle;
  portEXIT_CRITICAL(&g_lock);
  return 0;
}

int service_discovered(uint16_t connection, const struct ble_gatt_error* error,
                       const struct ble_gatt_svc* service, void*) {
  if (error->status == BLE_HS_EDONE) {
    portENTER_CRITICAL(&g_lock);
    bool found = g_service_start_handle != 0;
    portEXIT_CRITICAL(&g_lock);
    if (found) return start_characteristic_discovery(connection);
    if (g_discovery_mode == DiscoveryMode::kLegacy) {
      g_discovery_mode = DiscoveryMode::kModern;
      int rc = ble_gattc_disc_svc_by_uuid(connection, &kModernServiceUuid.u,
                                          service_discovered, nullptr);
      if (rc != 0) discovery_failed(connection, "modern service discovery start", rc);
      return rc;
    }
    discovery_failed(connection, "service discovery", error->status);
    return 0;
  }
  if (error->status != 0 || service == nullptr) {
    discovery_failed(connection, "service discovery", error->status);
    return 0;
  }
  portENTER_CRITICAL(&g_lock);
  g_service_start_handle = service->start_handle;
  g_service_end_handle = service->end_handle;
  portEXIT_CRITICAL(&g_lock);
  return 0;
}

int start_characteristic_discovery(uint16_t connection) {
  portENTER_CRITICAL(&g_lock);
  uint16_t start = g_service_start_handle;
  uint16_t end = g_service_end_handle;
  portEXIT_CRITICAL(&g_lock);
  int rc;
  if (g_discovery_mode == DiscoveryMode::kLegacy) {
    rc = ble_gattc_disc_chrs_by_uuid(connection, start, end, &kLegacyCharacteristicUuid.u,
                                    characteristic_discovered, nullptr);
  } else {
    rc = ble_gattc_disc_all_chrs(connection, start, end, characteristic_discovered, nullptr);
  }
  if (rc != 0) discovery_failed(connection, "characteristic discovery start", rc);
  return rc;
}

int start_descriptor_discovery(uint16_t connection) {
  portENTER_CRITICAL(&g_lock);
  uint16_t notify_handle = g_notify_handle;
  uint16_t service_end = g_service_end_handle;
  portEXIT_CRITICAL(&g_lock);
  int rc = ble_gattc_disc_all_dscs(connection, notify_handle, service_end,
                                   descriptor_discovered, nullptr);
  if (rc != 0) discovery_failed(connection, "CCCD discovery start", rc);
  return rc;
}

int gap_event(struct ble_gap_event* event, void*) {
  switch (event->type) {
    case BLE_GAP_EVENT_DISC:
      if (!is_acaia_advertisement(event->disc)) return 0;
      can_link::send_log(common::LogCode::kBleScaleFound, common::LogSeverity::kInfo,
                         static_cast<uint16_t>(event->disc.rssi));
      {
        int rc = ble_gap_disc_cancel();
        if (rc != 0) {
          can_link::send_log(common::LogCode::kBleError, common::LogSeverity::kWarn,
                             static_cast<uint16_t>(rc));
          return 0;
        }
        // NimBLE retire le callback du scan pendant l'annulation : aucun
        // BLE_GAP_EVENT_DISC_COMPLETE ne nous sera remis. La connexion doit
        // donc être demandée immédiatement après l'annulation réussie.
        connect_to_device(event->disc.addr);
      }
      return 0;
    case BLE_GAP_EVENT_CONNECT:
      if (event->connect.status != 0) {
        can_link::send_log(common::LogCode::kBleError, common::LogSeverity::kWarn,
                           static_cast<uint16_t>(event->connect.status));
        scan();
        return 0;
      }
      portENTER_CRITICAL(&g_lock);
      g_connection = event->connect.conn_handle;
      g_service_start_handle = 0;
      g_service_end_handle = 0;
      g_value_handle = 0;
      g_notify_handle = 0;
      g_cccd_handle = 0;
      portEXIT_CRITICAL(&g_lock);
      // La Lunar testee est legacy ; le repli moderne conserve la prise en
      // charge des generations recentes qui publient les UUID 128 bits.
      g_discovery_mode = DiscoveryMode::kLegacy;
      core::update_scale_connection(true);
      can_link::send_log(common::LogCode::kBleConnected, common::LogSeverity::kInfo);
      {
        int rc = ble_gattc_disc_svc_by_uuid(event->connect.conn_handle, &kLegacyServiceUuid.u,
                                            service_discovered, nullptr);
        if (rc != 0) discovery_failed(event->connect.conn_handle, "service discovery start", rc);
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

void on_reset(int reason) {
  ESP_LOGW(kTag, "NimBLE reset: %d", reason);
  can_link::send_log(common::LogCode::kBleError, common::LogSeverity::kWarn,
                     static_cast<uint16_t>(reason));
  clear_connection();
}

void on_sync() {
  int rc = ble_hs_util_ensure_addr(0);
  if (rc == 0) {
    scan();
  } else {
    can_link::send_log(common::LogCode::kBleError, common::LogSeverity::kWarn,
                       static_cast<uint16_t>(rc));
  }
}
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
  esp_err_t init_err = nimble_port_init();
  if (init_err != ESP_OK) {
    ESP_LOGE(kTag, "nimble_port_init failed: %s", esp_err_to_name(init_err));
    can_link::send_log(common::LogCode::kBleError, common::LogSeverity::kWarn,
                       static_cast<uint16_t>(init_err));
    return;
  }
  ble_hs_cfg.reset_cb = on_reset;
  ble_hs_cfg.sync_cb = on_sync;
  nimble_port_freertos_init(host_task);
  if (xTaskCreatePinnedToCore(heartbeat_task, "ble_acaia", 4096, nullptr, 5,
                              &g_heartbeat_task, 0) != pdPASS) {
    ESP_LOGE(kTag, "heartbeat task creation failed");
    can_link::send_log(common::LogCode::kBleError, common::LogSeverity::kWarn,
                       static_cast<uint16_t>(ESP_ERR_NO_MEM));
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
