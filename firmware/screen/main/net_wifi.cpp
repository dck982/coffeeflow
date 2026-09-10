#include "net_wifi.h"

#include <cstdio>
#include <cstring>
#include <ctime>
#include <sys/time.h>

#include "esp_event.h"
#include "esp_http_server.h"
#include "esp_log.h"
#include "esp_mac.h"
#include "esp_netif.h"
#include "esp_netif_sntp.h"
#include "esp_wifi.h"
#include "nvs.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"

#include "core/core.h"
#include "core/events.h"
#include "can_link.h"
#include "net_http.h"

#if __has_include("secrets.h")
#include "secrets.h"
#else
#error "Missing firmware/screen/main/secrets.h: copy secrets.example.h and set non-placeholder secrets."
namespace secrets { inline constexpr char kProvisioningApPassword[] = "invalid"; }
#endif

namespace net_wifi {
namespace {
constexpr const char* kTag = "net_wifi";
constexpr const char* kNamespace = "wifi";
constexpr const char* kCredentialKey = "credentials";
constexpr size_t kMaxSsid = 32;
constexpr size_t kMaxPassword = 63;

struct Credentials { char ssid[kMaxSsid + 1]; char password[kMaxPassword + 1]; };
esp_netif_t* g_sta_netif = nullptr;
esp_netif_t* g_ap_netif = nullptr;
httpd_handle_t g_provisioning_server = nullptr;
bool g_time_started = false;
bool g_sntp_initialized = false;
bool g_network_base_ready = false;
bool g_wifi_initialized = false;
esp_event_handler_instance_t g_wifi_event_instance = nullptr;
esp_event_handler_instance_t g_ip_event_instance = nullptr;
esp_event_handler_instance_t g_sntp_event_instance = nullptr;

bool credentials_present(Credentials* out) {
  nvs_handle_t handle;
  if (nvs_open(kNamespace, NVS_READONLY, &handle) != ESP_OK) return false;
  size_t size = sizeof(*out);
  esp_err_t err = nvs_get_blob(handle, kCredentialKey, out, &size);
  nvs_close(handle);
  return err == ESP_OK && size == sizeof(*out) && out->ssid[0] != '\0' && out->password[0] != '\0';
}

void stop_provisioning();

esp_err_t root_handler(httpd_req_t* request) {
  static const char kPage[] =
      "<!doctype html><meta name=viewport content='width=device-width,initial-scale=1'>"
      "<title>CoffeeFlow</title><h1>Configurer le réseau</h1>"
      "<button type=button onclick=\"fetch('/scan').then(r=>r.text()).then(t=>document.getElementById('networks').textContent=t)\">Voir les reseaux</button>"
      "<pre id=networks></pre>"
      "<form method=post action=/provision><label>SSID <input name=ssid maxlength=32 required></label><br>"
      "<label>Mot de passe <input name=password type=password minlength=8 maxlength=63 required></label><br>"
      "<button>Enregistrer</button></form>";
  httpd_resp_set_type(request, "text/html; charset=utf-8");
  return httpd_resp_send(request, kPage, HTTPD_RESP_USE_STRLEN);
}

esp_err_t scan_handler(httpd_req_t* request) {
  // Réponse texte, jamais interpolée comme HTML : un SSID hostile ne peut pas
  // injecter la page de provisioning. Le scan est opportuniste et ne bloque
  // que cette petite requête AP, jamais la tâche CAN/LVGL.
  wifi_scan_config_t config{};
  if (esp_wifi_scan_start(&config, true) != ESP_OK) return httpd_resp_send_err(request, HTTPD_500_INTERNAL_SERVER_ERROR, "scan unavailable");
  uint16_t count = 10;
  wifi_ap_record_t records[10]{};
  ESP_ERROR_CHECK(esp_wifi_scan_get_ap_records(&count, records));
  char response[512]{};
  size_t used = 0;
  for (uint16_t i = 0; i < count; ++i) {
    const char* ssid = reinterpret_cast<const char*>(records[i].ssid);
    int written = std::snprintf(response + used, sizeof(response) - used, "%.*s\n", static_cast<int>(kMaxSsid), ssid);
    if (written <= 0 || static_cast<size_t>(written) >= sizeof(response) - used) break;
    used += static_cast<size_t>(written);
  }
  httpd_resp_set_type(request, "text/plain; charset=utf-8");
  return httpd_resp_send(request, response, used);
}

bool decode_form(httpd_req_t* request, Credentials* credentials) {
  if (request->content_len <= 0 || request->content_len > 256) return false;
  char body[257]{};
  int received = httpd_req_recv(request, body, request->content_len);
  if (received != request->content_len) return false;
  char ssid[kMaxSsid + 1]{};
  char password[kMaxPassword + 1]{};
  if (httpd_query_key_value(body, "ssid", ssid, sizeof(ssid)) != ESP_OK ||
      httpd_query_key_value(body, "password", password, sizeof(password)) != ESP_OK) return false;
  size_t ssid_len = std::strlen(ssid), password_len = std::strlen(password);
  if (ssid_len == 0 || ssid_len > kMaxSsid || password_len < 8 || password_len > kMaxPassword) return false;
  std::strncpy(credentials->ssid, ssid, sizeof(credentials->ssid) - 1);
  std::strncpy(credentials->password, password, sizeof(credentials->password) - 1);
  return true;
}

void start_sta();

void delayed_start_sta(void*) {
  // Laisser le handler HTTP rendre sa réponse avant httpd_stop(). Arrêter un
  // serveur depuis l'une de ses propres tâches est une source de deadlock.
  vTaskDelay(pdMS_TO_TICKS(100));
  start_sta();
  vTaskDelete(nullptr);
}

esp_err_t provision_handler(httpd_req_t* request) {
  Credentials credentials{};
  if (!decode_form(request, &credentials)) { httpd_resp_send_err(request, HTTPD_400_BAD_REQUEST, "invalid form"); return ESP_FAIL; }
  nvs_handle_t handle;
  esp_err_t err = nvs_open(kNamespace, NVS_READWRITE, &handle);
  if (err == ESP_OK) { err = nvs_set_blob(handle, kCredentialKey, &credentials, sizeof(credentials)); if (err == ESP_OK) err = nvs_commit(handle); nvs_close(handle); }
  if (err != ESP_OK) { httpd_resp_send_err(request, HTTPD_500_INTERNAL_SERVER_ERROR, "storage failed"); return ESP_FAIL; }
  // Ne jamais refléter le mot de passe ; envoyer la réponse avant de couper AP.
  httpd_resp_sendstr(request, "Enregistre. Connexion au reseau…");
  xTaskCreatePinnedToCore(delayed_start_sta, "wifi_connect", 3072, nullptr, 4, nullptr, 0);
  return ESP_OK;
}

void start_provisioning() {
  if (g_provisioning_server != nullptr) return;
  httpd_config_t cfg = HTTPD_DEFAULT_CONFIG();
  cfg.max_open_sockets = 2;
  if (httpd_start(&g_provisioning_server, &cfg) != ESP_OK) { g_provisioning_server = nullptr; return; }
  httpd_uri_t root{.uri = "/", .method = HTTP_GET, .handler = root_handler, .user_ctx = nullptr,
                   .is_websocket = false, .handle_ws_control_frames = false, .supported_subprotocol = nullptr,
                   .ws_pre_handshake_cb = nullptr, .ws_post_handshake_cb = nullptr};
  httpd_uri_t scan{.uri = "/scan", .method = HTTP_GET, .handler = scan_handler, .user_ctx = nullptr,
                   .is_websocket = false, .handle_ws_control_frames = false, .supported_subprotocol = nullptr,
                   .ws_pre_handshake_cb = nullptr, .ws_post_handshake_cb = nullptr};
  httpd_uri_t post{.uri = "/provision", .method = HTTP_POST, .handler = provision_handler, .user_ctx = nullptr,
                   .is_websocket = false, .handle_ws_control_frames = false, .supported_subprotocol = nullptr,
                   .ws_pre_handshake_cb = nullptr, .ws_post_handshake_cb = nullptr};
  httpd_register_uri_handler(g_provisioning_server, &root);
  httpd_register_uri_handler(g_provisioning_server, &scan);
  httpd_register_uri_handler(g_provisioning_server, &post);
}
void stop_provisioning() { if (g_provisioning_server != nullptr) { httpd_stop(g_provisioning_server); g_provisioning_server = nullptr; } }

void start_ap() {
  net_http::stop();
  uint8_t mac[6]; esp_read_mac(mac, ESP_MAC_WIFI_SOFTAP);
  wifi_config_t ap{};
  std::snprintf(reinterpret_cast<char*>(ap.ap.ssid), sizeof(ap.ap.ssid), "CoffeeFlow-%02X%02X", mac[4], mac[5]);
  std::strncpy(reinterpret_cast<char*>(ap.ap.password), secrets::kProvisioningApPassword, sizeof(ap.ap.password) - 1);
  ap.ap.ssid_len = std::strlen(reinterpret_cast<const char*>(ap.ap.ssid));
  ap.ap.channel = 1; ap.ap.max_connection = 2; ap.ap.authmode = WIFI_AUTH_WPA2_PSK;
  ESP_ERROR_CHECK(esp_wifi_set_mode(WIFI_MODE_AP));
  ESP_ERROR_CHECK(esp_wifi_set_config(WIFI_IF_AP, &ap));
  ESP_ERROR_CHECK(esp_wifi_start());
  start_provisioning();
  core::update_network_status(core::NetworkState::kApProvisioning, 0);
  core::events::push(core::EventKind::kWifiApStarted);
  can_link::send_log(common::LogCode::kWifiApStarted, common::LogSeverity::kInfo);
}

void start_sta() {
  Credentials credentials{};
  if (!credentials_present(&credentials)) { start_ap(); return; }
  stop_provisioning();
  esp_wifi_stop();
  wifi_config_t sta{};
  std::strncpy(reinterpret_cast<char*>(sta.sta.ssid), credentials.ssid, sizeof(sta.sta.ssid) - 1);
  std::strncpy(reinterpret_cast<char*>(sta.sta.password), credentials.password, sizeof(sta.sta.password) - 1);
  ESP_ERROR_CHECK(esp_wifi_set_mode(WIFI_MODE_STA));
  ESP_ERROR_CHECK(esp_wifi_set_config(WIFI_IF_STA, &sta));
  ESP_ERROR_CHECK(esp_wifi_start());
  core::update_network_status(core::NetworkState::kStaConnecting, 0);
}

void forget_network_task(void*) {
  nvs_handle_t handle;
  if (nvs_open(kNamespace, NVS_READWRITE, &handle) == ESP_OK) { nvs_erase_key(handle, kCredentialKey); nvs_commit(handle); nvs_close(handle); }
  g_time_started = false;
  esp_wifi_stop();
  start_ap();
  core::events::push(core::EventKind::kNetworkForgotten);
  can_link::send_log(common::LogCode::kNetworkForgotten, common::LogSeverity::kInfo);
  vTaskDelete(nullptr);
}

void forget_network_impl() {
  // Le callback peut provenir de LVGL (cœur 1). NVS et transitions Wi-Fi se
  // font sur une tâche de contrôle cœur 0, jamais dans le callback UI.
  xTaskCreatePinnedToCore(forget_network_task, "wifi_forget", 3072, nullptr, 4, nullptr, 0);
}

void start_sntp_once() {
  if (g_time_started) return;
  g_time_started = true;
  setenv("TZ", "CET-1CEST,M3.5.0,M10.5.0/3", 1); tzset();
  esp_sntp_config_t config = ESP_NETIF_SNTP_DEFAULT_CONFIG("pool.ntp.org");
  esp_netif_sntp_init(&config);
  g_sntp_initialized = true;
}

void on_sntp_event(void*, esp_event_base_t, int32_t, void* event_data) {
  auto* event = static_cast<esp_netif_sntp_time_sync_t*>(event_data);
  if (event == nullptr || event->tv.tv_sec < 1700000000) return;
  core::mark_wall_time_known(event->tv.tv_sec);
  core::events::push(core::EventKind::kTimeKnown);
  // One-shot : l'heure est valide pour la session, les mesures restent
  // monotones. Arrêter ici interdit toute resynchronisation périodique.
  esp_netif_sntp_deinit();
  g_sntp_initialized = false;
}

void on_wifi_event(void*, esp_event_base_t, int32_t event_id, void*) {
  if (event_id == WIFI_EVENT_STA_START) { esp_wifi_connect(); return; }
  if (event_id == WIFI_EVENT_STA_DISCONNECTED) {
    // Pas de repli AP : des credentials existent. L'écran expose le diagnostic.
    net_http::stop();
    core::update_network_status(core::NetworkState::kStaDisconnected, 0);
    core::events::push(core::EventKind::kWifiDisconnected);
    can_link::send_log(common::LogCode::kWifiDisconnected, common::LogSeverity::kWarn);
    esp_wifi_connect();
  }
}
void on_ip_event(void*, esp_event_base_t, int32_t, void* event_data) {
  auto* event = static_cast<ip_event_got_ip_t*>(event_data);
  core::update_network_status(core::NetworkState::kStaConnected, event->ip_info.ip.addr);
  core::events::push(core::EventKind::kWifiConnected);
  can_link::send_log(common::LogCode::kWifiConnected, common::LogSeverity::kInfo);
  start_sntp_once();
  net_http::start();
}
}  // namespace

void start() {
  if (g_wifi_initialized) return;
  if (!g_network_base_ready) {
    ESP_ERROR_CHECK(esp_netif_init());
    ESP_ERROR_CHECK(esp_event_loop_create_default());
    g_network_base_ready = true;
  }
  g_sta_netif = esp_netif_create_default_wifi_sta();
  g_ap_netif = esp_netif_create_default_wifi_ap();
  ESP_ERROR_CHECK(g_sta_netif != nullptr && g_ap_netif != nullptr ? ESP_OK : ESP_ERR_NO_MEM);
  wifi_init_config_t cfg = WIFI_INIT_CONFIG_DEFAULT();
  ESP_ERROR_CHECK(esp_wifi_init(&cfg));
  g_wifi_initialized = true;
  ESP_ERROR_CHECK(esp_wifi_set_storage(WIFI_STORAGE_RAM));
  ESP_ERROR_CHECK(esp_event_handler_instance_register(WIFI_EVENT, ESP_EVENT_ANY_ID, &on_wifi_event, nullptr, &g_wifi_event_instance));
  ESP_ERROR_CHECK(esp_event_handler_instance_register(IP_EVENT, IP_EVENT_STA_GOT_IP, &on_ip_event, nullptr, &g_ip_event_instance));
  ESP_ERROR_CHECK(esp_event_handler_instance_register(NETIF_SNTP_EVENT, NETIF_SNTP_TIME_SYNC, &on_sntp_event, nullptr, &g_sntp_event_instance));
  core::register_forget_network_callback(&forget_network_impl);
  Credentials credentials{};
  if (credentials_present(&credentials)) start_sta(); else start_ap();
}

void stop() {
  if (!g_wifi_initialized) return;
  net_http::stop();
  stop_provisioning();
  if (g_sntp_initialized) {
    esp_netif_sntp_deinit();
    g_sntp_initialized = false;
  }
  if (g_wifi_event_instance != nullptr) {
    esp_event_handler_instance_unregister(WIFI_EVENT, ESP_EVENT_ANY_ID, g_wifi_event_instance);
    g_wifi_event_instance = nullptr;
  }
  if (g_ip_event_instance != nullptr) {
    esp_event_handler_instance_unregister(IP_EVENT, IP_EVENT_STA_GOT_IP, g_ip_event_instance);
    g_ip_event_instance = nullptr;
  }
  if (g_sntp_event_instance != nullptr) {
    esp_event_handler_instance_unregister(NETIF_SNTP_EVENT, NETIF_SNTP_TIME_SYNC, g_sntp_event_instance);
    g_sntp_event_instance = nullptr;
  }
  esp_wifi_stop();
  ESP_ERROR_CHECK(esp_wifi_deinit());
  ESP_ERROR_CHECK(esp_wifi_clear_default_wifi_driver_and_handlers(g_sta_netif));
  ESP_ERROR_CHECK(esp_wifi_clear_default_wifi_driver_and_handlers(g_ap_netif));
  esp_netif_destroy(g_sta_netif);
  esp_netif_destroy(g_ap_netif);
  g_sta_netif = nullptr;
  g_ap_netif = nullptr;
  g_wifi_initialized = false;
  core::register_forget_network_callback(nullptr);
  // Une tentative interrompue avant synchronisation doit être rejouable à la
  // prochaine entrée dans le mode Wi-Fi.
  g_time_started = false;
  core::update_network_status(core::NetworkState::kOff, 0);
  ESP_LOGI(kTag, "Wi-Fi/httpd/netif completement desinitialises");
}
}  // namespace net_wifi
