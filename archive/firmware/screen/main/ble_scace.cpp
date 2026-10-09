#include "ble_scace.h"

#include <cstdint>
#include <cstring>

#include "esp_log.h"
#include "freertos/FreeRTOS.h"
#include "host/ble_gatt.h"
#include "host/ble_hs.h"
#include "os/os_mbuf.h"

#include "ble_central.h"
#include "can_link.h"
#include "common/scace_ble.hpp"
#include "core/core.h"

namespace ble_scace {
namespace {

constexpr const char* kTag = "ble_scace";

ble_uuid128_t make_uuid(const uint8_t (&le)[16]) {
  ble_uuid128_t uuid{};
  uuid.u.type = BLE_UUID_TYPE_128;
  std::memcpy(uuid.value, le, sizeof(uuid.value));
  return uuid;
}

const ble_uuid128_t kServiceUuid = make_uuid(common::scace::kServiceUuidLe);
const ble_uuid128_t kFrameUuid = make_uuid(common::scace::kFrameUuidLe);
const ble_uuid16_t kCccdUuid = BLE_UUID16_INIT(BLE_GATT_DSC_CLT_CFG_UUID16);

portMUX_TYPE g_lock = portMUX_INITIALIZER_UNLOCKED;
uint16_t g_connection = BLE_HS_CONN_HANDLE_NONE;
uint16_t g_service_start_handle = 0;
uint16_t g_service_end_handle = 0;
uint16_t g_frame_handle = 0;
uint16_t g_cccd_handle = 0;

void clear_connection() {
  bool was_connected;
  portENTER_CRITICAL(&g_lock);
  was_connected = g_connection != BLE_HS_CONN_HANDLE_NONE;
  g_connection = BLE_HS_CONN_HANDLE_NONE;
  g_service_start_handle = 0;
  g_service_end_handle = 0;
  g_frame_handle = 0;
  g_cccd_handle = 0;
  portEXIT_CRITICAL(&g_lock);
  core::update_scace_connection(false);
  if (was_connected) can_link::send_log(common::LogCode::kScaceDisconnected, common::LogSeverity::kInfo);
}

void discovery_failed(uint16_t connection, const char* stage, int status) {
  ESP_LOGW(kTag, "SCACE %s failed: %d", stage, status);
  can_link::send_log(common::LogCode::kScaceError, common::LogSeverity::kWarn, static_cast<uint16_t>(status));
  ble_gap_terminate(connection, BLE_ERR_REM_USER_CONN_TERM);
}

int subscription_done(uint16_t connection, const struct ble_gatt_error* error, struct ble_gatt_attr*, void*) {
  if (error->status != 0) {
    discovery_failed(connection, "notification subscription", error->status);
    return 0;
  }
  core::update_scace_connection(true);
  can_link::send_log(common::LogCode::kScaceConnected, common::LogSeverity::kInfo);
  return 0;
}

int descriptor_discovered(uint16_t connection, const struct ble_gatt_error* error, uint16_t,
                          const struct ble_gatt_dsc* descriptor, void*) {
  if (error->status == BLE_HS_EDONE) {
    portENTER_CRITICAL(&g_lock);
    uint16_t cccd_handle = g_cccd_handle;
    portEXIT_CRITICAL(&g_lock);
    if (cccd_handle == 0) {
      discovery_failed(connection, "CCCD discovery", error->status);
      return 0;
    }
    const uint8_t enable_notifications[] = {1, 0};
    int rc = ble_gattc_write_flat(connection, cccd_handle, enable_notifications, sizeof(enable_notifications),
                                  subscription_done, nullptr);
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
    uint16_t frame_handle = g_frame_handle;
    uint16_t service_end = g_service_end_handle;
    portEXIT_CRITICAL(&g_lock);
    if (frame_handle == 0) {
      discovery_failed(connection, "characteristic discovery", error->status);
      return 0;
    }
    int rc = ble_gattc_disc_all_dscs(connection, frame_handle, service_end, descriptor_discovered, nullptr);
    if (rc != 0) discovery_failed(connection, "CCCD discovery start", rc);
    return rc;
  }
  if (error->status != 0 || characteristic == nullptr) {
    discovery_failed(connection, "characteristic discovery", error->status);
    return 0;
  }
  if ((characteristic->properties & BLE_GATT_CHR_PROP_NOTIFY) == 0) return 0;
  portENTER_CRITICAL(&g_lock);
  g_frame_handle = characteristic->val_handle;
  portEXIT_CRITICAL(&g_lock);
  return 0;
}

int service_discovered(uint16_t connection, const struct ble_gatt_error* error, const struct ble_gatt_svc* service,
                       void*) {
  if (error->status == BLE_HS_EDONE) {
    portENTER_CRITICAL(&g_lock);
    uint16_t start = g_service_start_handle;
    uint16_t end = g_service_end_handle;
    portEXIT_CRITICAL(&g_lock);
    if (start == 0) {
      discovery_failed(connection, "service discovery", error->status);
      return 0;
    }
    int rc = ble_gattc_disc_chrs_by_uuid(connection, start, end, &kFrameUuid.u, characteristic_discovered, nullptr);
    if (rc != 0) discovery_failed(connection, "characteristic discovery start", rc);
    return rc;
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

void process_frame(const uint8_t* data, size_t length) {
  common::scace::Frame frame;
  if (!common::scace::decode(data, length, &frame)) return;
  const bool ok = frame.status == common::scace::Status::kOk && frame.centi_c != common::scace::kNoTemperature;
  core::update_scace_temperature(frame.centi_c, ok);
}

}  // namespace

bool matches(const ble_hs_adv_fields& fields) {
  for (uint8_t i = 0; i < fields.num_uuids128; ++i) {
    if (ble_uuid_cmp(&fields.uuids128[i].u, &kServiceUuid.u) == 0) return true;
  }
  constexpr size_t kNameLength = sizeof(common::scace::kDeviceName) - 1;
  return fields.name != nullptr && fields.name_len == kNameLength &&
         std::memcmp(fields.name, common::scace::kDeviceName, kNameLength) == 0;
}

bool connected() {
  portENTER_CRITICAL(&g_lock);
  bool result = g_connection != BLE_HS_CONN_HANDLE_NONE;
  portEXIT_CRITICAL(&g_lock);
  return result;
}

int gap_event(struct ble_gap_event* event, void*) {
  switch (event->type) {
    case BLE_GAP_EVENT_CONNECT: {
      ble_central::connection_attempt_finished();
      if (event->connect.status != 0) {
        can_link::send_log(common::LogCode::kScaceError, common::LogSeverity::kWarn,
                           static_cast<uint16_t>(event->connect.status));
        return 0;
      }
      portENTER_CRITICAL(&g_lock);
      g_connection = event->connect.conn_handle;
      g_service_start_handle = 0;
      g_service_end_handle = 0;
      g_frame_handle = 0;
      g_cccd_handle = 0;
      portEXIT_CRITICAL(&g_lock);
      int rc = ble_gattc_disc_svc_by_uuid(event->connect.conn_handle, &kServiceUuid.u, service_discovered, nullptr);
      if (rc != 0) discovery_failed(event->connect.conn_handle, "service discovery start", rc);
      return 0;
    }
    case BLE_GAP_EVENT_DISCONNECT:
      clear_connection();
      return 0;
    case BLE_GAP_EVENT_NOTIFY_RX: {
      uint8_t buffer[common::scace::kFrameSize];
      // Un décodeur ignore les octets au-delà de la trame : une sonde plus
      // récente qui l'allonge reste lisible.
      if (OS_MBUF_PKTLEN(event->notify_rx.om) >= sizeof(buffer) &&
          os_mbuf_copydata(event->notify_rx.om, 0, sizeof(buffer), buffer) == 0) {
        process_frame(buffer, sizeof(buffer));
      }
      return 0;
    }
    default:
      return 0;
  }
}

void disconnect() {
  portENTER_CRITICAL(&g_lock);
  uint16_t connection = g_connection;
  portEXIT_CRITICAL(&g_lock);
  if (connection != BLE_HS_CONN_HANDLE_NONE) ble_gap_terminate(connection, BLE_ERR_REM_USER_CONN_TERM);
}

void reset() { clear_connection(); }

}  // namespace ble_scace
