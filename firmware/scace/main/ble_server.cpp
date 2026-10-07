#include "ble_server.h"

#include <cstring>

#include "esp_log.h"
#include "freertos/FreeRTOS.h"
#include "host/ble_gap.h"
#include "host/ble_gatt.h"
#include "host/ble_hs.h"
#include "host/util/util.h"
#include "nimble/nimble_port.h"
#include "nimble/nimble_port_freertos.h"
#include "nvs_flash.h"
#include "os/os_mbuf.h"
#include "services/gap/ble_svc_gap.h"
#include "services/gatt/ble_svc_gatt.h"

namespace ble_server {
namespace {

using common::scace::kFrameSize;

constexpr const char* kTag = "ble_server";
constexpr int kMaxConnections = CONFIG_BT_NIMBLE_MAX_CONNECTIONS;

struct Peer {
  uint16_t handle;
  bool subscribed;
};

ble_uuid128_t make_uuid(const uint8_t (&le)[16]) {
  ble_uuid128_t uuid{};
  uuid.u.type = BLE_UUID_TYPE_128;
  std::memcpy(uuid.value, le, sizeof(uuid.value));
  return uuid;
}

const ble_uuid128_t kServiceUuid = make_uuid(common::scace::kServiceUuidLe);
const ble_uuid128_t kFrameUuid = make_uuid(common::scace::kFrameUuidLe);

// Partagés entre la tâche hôte NimBLE (événements GAP, lectures) et la
// boucle principale (publish).
portMUX_TYPE g_lock = portMUX_INITIALIZER_UNLOCKED;
Peer g_peers[kMaxConnections];
int g_peer_count = 0;
uint8_t g_frame[kFrameSize]{};

uint16_t g_value_handle = 0;
uint8_t g_own_addr_type = 0;

int access_frame(uint16_t, uint16_t, ble_gatt_access_ctxt* ctxt, void*) {
  if (ctxt->op != BLE_GATT_ACCESS_OP_READ_CHR) return BLE_ATT_ERR_UNLIKELY;
  uint8_t bytes[kFrameSize];
  portENTER_CRITICAL(&g_lock);
  std::memcpy(bytes, g_frame, sizeof(bytes));
  portEXIT_CRITICAL(&g_lock);
  return os_mbuf_append(ctxt->om, bytes, sizeof(bytes)) == 0 ? 0 : BLE_ATT_ERR_INSUFFICIENT_RES;
}

// NimBLE garde des pointeurs vers ces tables : elles vivent tout le programme.
// Remplies dans start(), un tableau se termine par une entrée nulle.
ble_gatt_chr_def g_characteristics[2]{};
ble_gatt_svc_def g_services[2]{};

void build_gatt_tables() {
  g_characteristics[0].uuid = &kFrameUuid.u;
  g_characteristics[0].access_cb = access_frame;
  g_characteristics[0].flags = BLE_GATT_CHR_F_READ | BLE_GATT_CHR_F_NOTIFY;
  g_characteristics[0].val_handle = &g_value_handle;
  g_services[0].type = BLE_GATT_SVC_TYPE_PRIMARY;
  g_services[0].uuid = &kServiceUuid.u;
  g_services[0].characteristics = g_characteristics;
}

int on_gap_event(ble_gap_event* event, void*);

void advertise() {
  if (ble_gap_adv_active()) return;
  portENTER_CRITICAL(&g_lock);
  const bool full = g_peer_count >= kMaxConnections;
  portEXIT_CRITICAL(&g_lock);
  if (full) return;

  // L'UUID 128 bits remplit le paquet d'annonce (21 octets sur 31) : le nom
  // part dans la réponse au scan.
  ble_hs_adv_fields fields{};
  fields.flags = BLE_HS_ADV_F_DISC_GEN | BLE_HS_ADV_F_BREDR_UNSUP;
  fields.uuids128 = &kServiceUuid;
  fields.num_uuids128 = 1;
  fields.uuids128_is_complete = 1;
  int rc = ble_gap_adv_set_fields(&fields);
  if (rc != 0) {
    ESP_LOGE(kTag, "champs d'annonce : %d", rc);
    return;
  }
  ble_hs_adv_fields response{};
  response.name = reinterpret_cast<const uint8_t*>(common::scace::kDeviceName);
  response.name_len = sizeof(common::scace::kDeviceName) - 1;
  response.name_is_complete = 1;
  rc = ble_gap_adv_rsp_set_fields(&response);
  if (rc != 0) {
    ESP_LOGE(kTag, "réponse au scan : %d", rc);
    return;
  }
  ble_gap_adv_params params{};
  params.conn_mode = BLE_GAP_CONN_MODE_UND;
  params.disc_mode = BLE_GAP_DISC_MODE_GEN;
  rc = ble_gap_adv_start(g_own_addr_type, nullptr, BLE_HS_FOREVER, &params, on_gap_event, nullptr);
  if (rc != 0) ESP_LOGE(kTag, "annonce : %d", rc);
}

void add_peer(uint16_t handle) {
  portENTER_CRITICAL(&g_lock);
  if (g_peer_count < kMaxConnections) g_peers[g_peer_count++] = {handle, false};
  portEXIT_CRITICAL(&g_lock);
}

void remove_peer(uint16_t handle) {
  portENTER_CRITICAL(&g_lock);
  for (int i = 0; i < g_peer_count; ++i) {
    if (g_peers[i].handle == handle) {
      g_peers[i] = g_peers[--g_peer_count];
      break;
    }
  }
  portEXIT_CRITICAL(&g_lock);
}

void set_subscribed(uint16_t handle, bool subscribed) {
  portENTER_CRITICAL(&g_lock);
  for (int i = 0; i < g_peer_count; ++i) {
    if (g_peers[i].handle == handle) g_peers[i].subscribed = subscribed;
  }
  portEXIT_CRITICAL(&g_lock);
}

int on_gap_event(ble_gap_event* event, void*) {
  switch (event->type) {
    case BLE_GAP_EVENT_CONNECT:
      if (event->connect.status == 0) add_peer(event->connect.conn_handle);
      // L'annonce s'arrête à chaque connexion : la relancer tant qu'il reste
      // une place.
      advertise();
      return 0;
    case BLE_GAP_EVENT_DISCONNECT:
      remove_peer(event->disconnect.conn.conn_handle);
      advertise();
      return 0;
    case BLE_GAP_EVENT_ADV_COMPLETE:
      advertise();
      return 0;
    case BLE_GAP_EVENT_SUBSCRIBE:
      if (event->subscribe.attr_handle == g_value_handle) {
        set_subscribed(event->subscribe.conn_handle, event->subscribe.cur_notify != 0);
      }
      return 0;
    default:
      return 0;
  }
}

void on_sync() {
  int rc = ble_hs_util_ensure_addr(0);
  if (rc == 0) rc = ble_hs_id_infer_auto(0, &g_own_addr_type);
  if (rc != 0) {
    ESP_LOGE(kTag, "adresse BLE : %d", rc);
    return;
  }
  advertise();
}

void on_reset(int reason) {
  ESP_LOGW(kTag, "pile BLE réinitialisée : %d", reason);
  portENTER_CRITICAL(&g_lock);
  g_peer_count = 0;
  portEXIT_CRITICAL(&g_lock);
}

void host_task(void*) {
  nimble_port_run();
  nimble_port_freertos_deinit();
}

}  // namespace

void start() {
  // Le contrôleur range sa calibration radio en NVS.
  esp_err_t err = nvs_flash_init();
  if (err == ESP_ERR_NVS_NO_FREE_PAGES || err == ESP_ERR_NVS_NEW_VERSION_FOUND) {
    ESP_ERROR_CHECK(nvs_flash_erase());
    err = nvs_flash_init();
  }
  ESP_ERROR_CHECK(err);
  ESP_ERROR_CHECK(nimble_port_init());

  ble_hs_cfg.sync_cb = on_sync;
  ble_hs_cfg.reset_cb = on_reset;
  ble_svc_gap_init();
  ble_svc_gatt_init();
  build_gatt_tables();
  ESP_ERROR_CHECK(ble_gatts_count_cfg(g_services) == 0 ? ESP_OK : ESP_FAIL);
  ESP_ERROR_CHECK(ble_gatts_add_svcs(g_services) == 0 ? ESP_OK : ESP_FAIL);
  ble_svc_gap_device_name_set(common::scace::kDeviceName);
  nimble_port_freertos_init(host_task);
}

void publish(const common::scace::Frame& frame) {
  uint8_t bytes[kFrameSize];
  common::scace::encode(frame, bytes);
  Peer peers[kMaxConnections];
  int count = 0;
  portENTER_CRITICAL(&g_lock);
  std::memcpy(g_frame, bytes, sizeof(bytes));
  for (int i = 0; i < g_peer_count; ++i) {
    if (g_peers[i].subscribed) peers[count++] = g_peers[i];
  }
  portEXIT_CRITICAL(&g_lock);

  for (int i = 0; i < count; ++i) {
    os_mbuf* om = ble_hs_mbuf_from_flat(bytes, sizeof(bytes));
    if (om == nullptr) return;
    // notify_custom libère om dans tous les cas. Une trame perdue faute de
    // tampon se voit au numéro de séquence côté central.
    ble_gatts_notify_custom(peers[i].handle, g_value_handle, om);
  }
}

int connections() {
  portENTER_CRITICAL(&g_lock);
  const int count = g_peer_count;
  portEXIT_CRITICAL(&g_lock);
  return count;
}

}  // namespace ble_server
