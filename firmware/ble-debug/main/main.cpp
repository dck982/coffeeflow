// Test protocole Acaia Lunar legacy pour M5Stack AtomS3.
// Topologie confirmee le 2026-09-11 : service 0x1820, 0x2a80 WRITE_NR|NOTIFY,
// CCCD 0x2902. Ce programme n'envoie que les trames d'initialisation Acaia et
// imprime les notifications brutes recues.

#include <cctype>
#include <cinttypes>
#include <cstdio>
#include <cstring>

#include "esp_idf_version.h"
#include "esp_log.h"
#include "esp_system.h"
#include "esp_timer.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "host/ble_gap.h"
#include "host/ble_gatt.h"
#include "host/ble_hs.h"
#include "host/util/util.h"
#include "nimble/nimble_port.h"
#include "nimble/nimble_port_freertos.h"
#include "nvs_flash.h"
#include "os/os_mbuf.h"

namespace {

constexpr char kTag[] = "acaia-test";
const ble_uuid16_t kAcaiaService = BLE_UUID16_INIT(0x1820);
const ble_uuid16_t kAcaiaCharacteristic = BLE_UUID16_INIT(0x2a80);
const ble_uuid16_t kCccd = BLE_UUID16_INIT(BLE_GATT_DSC_CLT_CFG_UUID16);

uint16_t g_connection = BLE_HS_CONN_HANDLE_NONE;
uint16_t g_service_start = 0;
uint16_t g_service_end = 0;
uint16_t g_value_handle = 0;
uint16_t g_cccd_handle = 0;

int gap_event(ble_gap_event* event, void* arg);

uint32_t elapsed_ms() { return static_cast<uint32_t>(esp_timer_get_time() / 1000); }
void print_hex(const uint8_t* data, size_t length) {
  for (size_t i = 0; i < length; ++i) std::printf("%02x", data[i]);
}
void report(const char* stage, int status) {
  ESP_LOGI(kTag, "%s status=%d%s", stage, status,
            status == BLE_HS_EDONE ? " (done)" : status == BLE_HS_EBUSY ? " (busy)" : "");
}
bool acaia_name(const ble_hs_adv_fields& fields) {
  if (!fields.name) return false;
  char name[48]{};
  const size_t n = fields.name_len < sizeof(name) - 1 ? fields.name_len : sizeof(name) - 1;
  for (size_t i = 0; i < n; ++i) name[i] = static_cast<char>(std::toupper(fields.name[i]));
  return std::strstr(name, "ACAIA") != nullptr || std::strstr(name, "LUNAR") != nullptr;
}
void start_scan() {
  uint8_t own_address_type;
  int rc = ble_hs_id_infer_auto(0, &own_address_type);
  if (rc == 0) {
    ble_gap_disc_params params{};
    params.passive = 0;
    params.filter_duplicates = 0;
    rc = ble_gap_disc(own_address_type, BLE_HS_FOREVER, &params, gap_event, nullptr);
  }
  report("SCAN", rc);
}
void clear_connection() {
  g_connection = BLE_HS_CONN_HANDLE_NONE;
  g_service_start = 0;
  g_service_end = 0;
  g_value_handle = 0;
  g_cccd_handle = 0;
}
void send_frame(uint8_t command, const uint8_t* payload, size_t payload_length, bool length_prefix) {
  if (g_connection == BLE_HS_CONN_HANDLE_NONE || g_value_handle == 0) return;
  uint8_t frame[64]{};
  size_t length = 0;
  frame[length++] = 0xef;
  frame[length++] = 0xdd;
  frame[length++] = command;
  if (length_prefix) frame[length++] = static_cast<uint8_t>(payload_length + 1);
  std::memcpy(frame + length, payload, payload_length);
  length += payload_length;
  uint8_t even = 0;
  uint8_t odd = 0;
  for (size_t i = 3; i < length; ++i) {
    if ((i - 3) % 2 == 0) even += frame[i]; else odd += frame[i];
  }
  frame[length++] = even;
  frame[length++] = odd;
  const int rc = ble_gattc_write_no_rsp_flat(g_connection, g_value_handle, frame, length);
  std::printf("TX cmd=0x%02x len=%u rc=%d raw=", command, static_cast<unsigned>(length), rc);
  print_hex(frame, length);
  std::printf("\n");
}
void start_acaia_protocol() {
  // Identifiant / heartbeat, puis demande des notifications poids/chrono/boutons.
  const uint8_t identifier[] = {0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37,
                                0x38, 0x39, 0x30, 0x31, 0x32, 0x33, 0x34};
  const uint8_t heartbeat[] = {0x00};
  const uint8_t notifications[] = {0x00, 0x01, 0x01, 0x02, 0x02, 0x05, 0x03, 0x04};
  send_frame(0x0b, identifier, sizeof(identifier), false);
  send_frame(0x00, heartbeat, sizeof(heartbeat), true);
  send_frame(0x0c, notifications, sizeof(notifications), true);
}
int subscription_done(uint16_t, const ble_gatt_error* error, ble_gatt_attr*, void*) {
  report("CCCD_WRITE", error->status);
  if (error->status == 0) start_acaia_protocol();
  return 0;
}
int descriptor_discovered(uint16_t connection, const ble_gatt_error* error, uint16_t,
                          const ble_gatt_dsc* descriptor, void*) {
  if (error->status == BLE_HS_EDONE) {
    report("CCCD_DISC", error->status);
    if (g_cccd_handle == 0) { ESP_LOGW(kTag, "CCCD not found"); return 0; }
    const uint8_t enable_notifications[] = {0x01, 0x00};
    const int rc = ble_gattc_write_flat(connection, g_cccd_handle, enable_notifications,
                                        sizeof(enable_notifications), subscription_done, nullptr);
    std::printf("CCCD handle=0x%04x enable_notify rc=%d\n", g_cccd_handle, rc);
    return rc;
  }
  if (error->status != 0) { report("CCCD_DISC", error->status); return 0; }
  if (ble_uuid_cmp(&descriptor->uuid.u, &kCccd.u) != 0) return 0;
  g_cccd_handle = descriptor->handle;
  return 0;
}
int characteristic_discovered(uint16_t connection, const ble_gatt_error* error,
                              const ble_gatt_chr* characteristic, void*) {
  if (error->status == BLE_HS_EDONE) {
    report("CHAR_DISC", error->status);
    if (g_value_handle == 0) { ESP_LOGW(kTag, "Acaia characteristic not found"); return 0; }
    const int rc = ble_gattc_disc_all_dscs(connection, g_value_handle, g_service_end,
                                           descriptor_discovered, nullptr);
    report("CCCD_DISC_START", rc);
    return rc;
  }
  if (error->status != 0) { report("CHAR_DISC", error->status); return 0; }
  g_value_handle = characteristic->val_handle;
  std::printf("ACAIA_CHANNEL value=0x%04x props=0x%02x\n", g_value_handle, characteristic->properties);
  return 0;
}
int service_discovered(uint16_t connection, const ble_gatt_error* error,
                       const ble_gatt_svc* service, void*) {
  if (error->status == BLE_HS_EDONE) {
    report("SERVICE_DISC", error->status);
    if (g_service_start == 0) { ESP_LOGW(kTag, "Acaia service not found"); return 0; }
    const int rc = ble_gattc_disc_chrs_by_uuid(connection, g_service_start, g_service_end,
                                               &kAcaiaCharacteristic.u, characteristic_discovered, nullptr);
    report("CHAR_DISC_START", rc);
    return rc;
  }
  if (error->status != 0) { report("SERVICE_DISC", error->status); return 0; }
  g_service_start = service->start_handle;
  g_service_end = service->end_handle;
  std::printf("ACAIA_SERVICE start=0x%04x end=0x%04x\n", service->start_handle, service->end_handle);
  return 0;
}
void connect_to(const ble_addr_t& address) {
  uint8_t own_address_type;
  int rc = ble_hs_id_infer_auto(0, &own_address_type);
  if (rc == 0) rc = ble_gap_connect(own_address_type, &address, 30000, nullptr, gap_event, nullptr);
  report("CONNECT_START", rc);
  if (rc != 0) start_scan();
}
int gap_event(ble_gap_event* event, void*) {
  switch (event->type) {
    case BLE_GAP_EVENT_DISC: {
      ble_hs_adv_fields fields{};
      if (ble_hs_adv_parse_fields(&fields, event->disc.data, event->disc.length_data) != 0 || !acaia_name(fields)) return 0;
      std::printf("TARGET t=%" PRIu32 "ms name=%.*s rssi=%d\n", elapsed_ms(), fields.name_len, fields.name, event->disc.rssi);
      const int rc = ble_gap_disc_cancel();
      report("SCAN_CANCEL", rc);
      if (rc == 0) connect_to(event->disc.addr);
      return 0;
    }
    case BLE_GAP_EVENT_CONNECT:
      report("CONNECT", event->connect.status);
      if (event->connect.status != 0) { start_scan(); return 0; }
      g_connection = event->connect.conn_handle;
      { const int rc = ble_gattc_disc_svc_by_uuid(g_connection, &kAcaiaService.u, service_discovered, nullptr); report("SERVICE_DISC_START", rc); }
      return 0;
    case BLE_GAP_EVENT_NOTIFY_RX: {
      const auto& notification = event->notify_rx;
      const uint16_t length = OS_MBUF_PKTLEN(notification.om);
      uint8_t data[256];
      if (length > sizeof(data) || os_mbuf_copydata(notification.om, 0, length, data) != 0) return 0;
      std::printf("RX t=%" PRIu32 "ms handle=0x%04x indication=%u len=%u raw=", elapsed_ms(), notification.attr_handle, notification.indication, length);
      print_hex(data, length);
      std::printf("\n");
      return 0;
    }
    case BLE_GAP_EVENT_DISCONNECT:
      report("DISCONNECT", event->disconnect.reason);
      clear_connection();
      start_scan();
      return 0;
    default:
      return 0;
  }
}
void on_reset(int reason) { report("NIMBLE_RESET", reason); }
void on_sync() {
  const int rc = ble_hs_util_ensure_addr(0);
  report("NIMBLE_SYNC", rc);
  if (rc == 0) start_scan();
}
void host_task(void*) { nimble_port_run(); nimble_port_freertos_deinit(); }

}  // namespace

extern "C" void app_main() {
  const esp_err_t nvs_rc = nvs_flash_init();
  if (nvs_rc != ESP_OK) ESP_LOGW(kTag, "NVS init: %s", esp_err_to_name(nvs_rc));
  vTaskDelay(pdMS_TO_TICKS(1000));
  ESP_LOGI(kTag, "Acaia legacy protocol test | ESP-IDF %s | reset=%d", esp_get_idf_version(), esp_reset_reason());
  nimble_port_init();
  ble_hs_cfg.reset_cb = on_reset;
  ble_hs_cfg.sync_cb = on_sync;
  nimble_port_freertos_init(host_task);
}
