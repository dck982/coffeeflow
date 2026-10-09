#include "ble_central.h"

#include <cstdint>

#include "esp_log.h"
#include "esp_timer.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "host/ble_gap.h"
#include "host/ble_hs.h"
#include "host/util/util.h"
#include "nimble/nimble_port.h"
#include "nimble/nimble_port_freertos.h"

#include "ble_scace.h"
#include "ble_scale.h"
#include "ble_scan_policy.h"
#include "can_link.h"
#include "core/core.h"

namespace ble_central {
namespace {

constexpr const char* kTag = "ble_central";
constexpr uint32_t kTickMs = 100;
constexpr int32_t kConnectTimeoutMs = 30000;

using ble_scan_policy::Scan;

portMUX_TYPE g_lock = portMUX_INITIALIZER_UNLOCKED;
bool g_started = false;
bool g_synced = false;
bool g_active = false;
bool g_connecting = false;
bool g_probe_enabled = false;
Scan g_scan = Scan::kOff;  // scan en cours, kOff s'il n'y en a pas
int64_t g_probe_window_started_us = 0;
TaskHandle_t g_task = nullptr;

int scan_event(struct ble_gap_event* event, void* arg);

void log_error(int rc) {
  can_link::send_log(common::LogCode::kBleError, common::LogSeverity::kWarn, static_cast<uint16_t>(rc));
}

void set_scan(Scan scan) {
  portENTER_CRITICAL(&g_lock);
  g_scan = scan;
  portEXIT_CRITICAL(&g_lock);
}

bool start_scan(Scan scan, int64_t now) {
  uint8_t own_addr_type;
  int rc = ble_hs_id_infer_auto(0, &own_addr_type);
  if (rc != 0) {
    log_error(rc);
    return false;
  }
  struct ble_gap_disc_params params{};
  int32_t duration_ms = BLE_HS_FOREVER;
  if (scan == Scan::kContinuous) {
    // La Lunar peut diffuser l'UUID dans l'advertising et son nom dans la scan
    // response. Le contrôleur du S3 déduplique sinon par adresse, et masque le
    // second paquet avant que NimBLE nous le livre.
    params.filter_duplicates = 0;
    params.passive = 0;
  } else {
    // La sonde annonce son service dans le paquet d'advertising : un scan
    // passif, dédupliqué, suffit et n'émet rien.
    params.filter_duplicates = 1;
    params.passive = 1;
    duration_ms = static_cast<int32_t>(ble_scan_policy::kProbeScanWindowUs / 1000);
  }
  rc = ble_gap_disc(own_addr_type, duration_ms, &params, scan_event, nullptr);
  if (rc != 0) {
    if (rc != BLE_HS_EALREADY && rc != BLE_HS_EBUSY) {
      ESP_LOGW(kTag, "scan unavailable: %d", rc);
      log_error(rc);
    }
    return false;
  }
  set_scan(scan);
  if (scan == Scan::kContinuous) {
    can_link::send_log(common::LogCode::kBleScanStarted, common::LogSeverity::kDebug);
  } else {
    portENTER_CRITICAL(&g_lock);
    g_probe_window_started_us = now;
    portEXIT_CRITICAL(&g_lock);
  }
  return true;
}

void cancel_scan() {
  // NimBLE retire le callback du scan pendant l'annulation : aucun
  // BLE_GAP_EVENT_DISC_COMPLETE ne sera remis.
  int rc = ble_gap_disc_cancel();
  if (rc != 0 && rc != BLE_HS_EALREADY) log_error(rc);
  set_scan(Scan::kOff);
}

void connect(const ble_addr_t& address, ble_gap_event_fn* client_event) {
  // Marquée avant l'annulation : la tâche centrale ne relance pas le scan
  // entre les deux, ce qui ferait refuser la connexion.
  portENTER_CRITICAL(&g_lock);
  g_connecting = true;
  portEXIT_CRITICAL(&g_lock);
  cancel_scan();
  uint8_t own_addr_type;
  int rc = ble_hs_id_infer_auto(0, &own_addr_type);
  if (rc == 0) {
    // nullptr demande les paramètres de connexion valides par défaut de
    // NimBLE. Une structure value-initialized contient des zéros invalides.
    rc = ble_gap_connect(own_addr_type, &address, kConnectTimeoutMs, nullptr, client_event, nullptr);
  }
  if (rc != 0) {
    ESP_LOGW(kTag, "connect unavailable: %d", rc);
    log_error(rc);
    connection_attempt_finished();
  }
}

// Copie de scace.enabled tenue par la tâche centrale : la tâche hôte la lit
// à chaque annonce reçue, sans prendre le verrou de configuration.
bool probe_enabled() {
  portENTER_CRITICAL(&g_lock);
  bool enabled = g_probe_enabled;
  portEXIT_CRITICAL(&g_lock);
  return enabled;
}

int scan_event(struct ble_gap_event* event, void*) {
  switch (event->type) {
    case BLE_GAP_EVENT_DISC: {
      struct ble_hs_adv_fields fields{};
      if (ble_hs_adv_parse_fields(&fields, event->disc.data, event->disc.length_data) != 0) return 0;
      if (!ble_scale::connected() && ble_scale::matches(fields)) {
        can_link::send_log(common::LogCode::kBleScaleFound, common::LogSeverity::kInfo,
                           static_cast<uint16_t>(event->disc.rssi));
        connect(event->disc.addr, ble_scale::gap_event);
      } else if (probe_enabled() && !ble_scace::connected() && ble_scace::matches(fields)) {
        can_link::send_log(common::LogCode::kScaceFound, common::LogSeverity::kInfo,
                           static_cast<uint16_t>(event->disc.rssi));
        connect(event->disc.addr, ble_scace::gap_event);
      }
      return 0;
    }
    case BLE_GAP_EVENT_DISC_COMPLETE:
      set_scan(Scan::kOff);
      return 0;
    default:
      return 0;
  }
}

// Seule cette tâche démarre ou arrête le scan ; la tâche hôte ne fait que
// l'annuler pour se connecter.
void update_scan(int64_t now) {
  const bool enabled = core::get_config().scace_enabled;
  portENTER_CRITICAL(&g_lock);
  g_probe_enabled = enabled;
  portEXIT_CRITICAL(&g_lock);
  if (!enabled && ble_scace::connected()) ble_scace::disconnect();
  ble_scan_policy::Inputs in;
  Scan current;
  int64_t window_started;
  portENTER_CRITICAL(&g_lock);
  const bool synced = g_synced;
  in.connecting = g_connecting;
  in.cycle_active = g_active;
  current = g_scan;
  window_started = g_probe_window_started_us;
  portEXIT_CRITICAL(&g_lock);
  if (!synced) return;
  in.scale_wanted = !ble_scale::connected();
  in.probe_wanted = enabled && !ble_scace::connected();
  const Scan wanted = ble_scan_policy::decide(in);
  if (current == wanted) return;
  if (current != Scan::kOff) {
    cancel_scan();
    return;
  }
  if (wanted == Scan::kContinuous ||
      (wanted == Scan::kProbeWindow && ble_scan_policy::probe_window_due(now, window_started))) {
    start_scan(wanted, now);
  }
}

void central_task(void*) {
  for (;;) {
    const int64_t now = esp_timer_get_time();
    ble_scale::tick(now);
    update_scan(now);
    vTaskDelay(pdMS_TO_TICKS(kTickMs));
  }
}

void reset_state() {
  portENTER_CRITICAL(&g_lock);
  g_synced = false;
  g_connecting = false;
  g_scan = Scan::kOff;
  g_probe_window_started_us = 0;
  portEXIT_CRITICAL(&g_lock);
  ble_scale::reset();
  ble_scace::reset();
}

void on_reset(int reason) {
  ESP_LOGW(kTag, "NimBLE reset: %d", reason);
  log_error(reason);
  reset_state();
}

void on_sync() {
  int rc = ble_hs_util_ensure_addr(0);
  if (rc != 0) {
    log_error(rc);
    return;
  }
  portENTER_CRITICAL(&g_lock);
  g_synced = true;
  portEXIT_CRITICAL(&g_lock);
}

void host_task(void*) {
  nimble_port_run();
  nimble_port_freertos_deinit();
}

}  // namespace

void init() {
  if (g_started) return;
  esp_err_t init_err = nimble_port_init();
  if (init_err != ESP_OK) {
    ESP_LOGE(kTag, "nimble_port_init failed: %s", esp_err_to_name(init_err));
    log_error(init_err);
    return;
  }
  ble_hs_cfg.reset_cb = on_reset;
  ble_hs_cfg.sync_cb = on_sync;
  nimble_port_freertos_init(host_task);
  if (xTaskCreatePinnedToCore(central_task, "ble_central", 4096, nullptr, 5, &g_task, 0) != pdPASS) {
    ESP_LOGE(kTag, "central task creation failed");
    log_error(ESP_ERR_NO_MEM);
    nimble_port_stop();
    nimble_port_deinit();
    return;
  }
  g_started = true;
}

void stop() {
  if (!g_started) return;
  // La tâche hôte et la tâche centrale peuvent encore appeler NimBLE ; elles
  // doivent disparaître avant de libérer les allocations du contrôleur.
  if (g_task != nullptr) {
    vTaskDelete(g_task);
    g_task = nullptr;
  }
  set_active(false);
  reset_state();
  nimble_port_stop();
  nimble_port_deinit();
  g_started = false;
  ESP_LOGI(kTag, "NimBLE et controleur completement desinitialises");
}

void set_active(bool active) {
  portENTER_CRITICAL(&g_lock);
  g_active = active;
  portEXIT_CRITICAL(&g_lock);
  ble_scale::set_active(active);
}

void connection_attempt_finished() {
  portENTER_CRITICAL(&g_lock);
  g_connecting = false;
  portEXIT_CRITICAL(&g_lock);
}

}  // namespace ble_central
