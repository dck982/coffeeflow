#include "net_http.h"

#include <cmath>
#include <cstdio>
#include <cstring>
#include <limits>

#include "cJSON.h"
#include "esp_http_server.h"
#include "esp_log.h"
#include "esp_ota_ops.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"

#include "core/core.h"
#include "common/crc.hpp"
#include "net_ws.h"
#include "ota_proxy.h"

#if __has_include("secrets.h")
#include "secrets.h"
#else
#error "Missing firmware/screen/main/secrets.h: copy secrets.example.h and set non-placeholder secrets."
namespace secrets { inline constexpr char kHttpBearerToken[] = "invalid"; }
#endif

namespace net_http {
namespace {

constexpr const char* kTag = "net_http";
constexpr size_t kMaxBody = 2048;
httpd_handle_t g_server = nullptr;
esp_ota_handle_t g_http_ota = 0;
const esp_partition_t* g_http_ota_partition = nullptr;
common::Crc32Incremental g_http_ota_crc;

void reboot_after_http_response(void*) { vTaskDelay(pdMS_TO_TICKS(250)); esp_restart(); }

esp_err_t send_status_json(httpd_req_t* request, const char* status, const char* body) {
  httpd_resp_set_status(request, status);
  httpd_resp_set_type(request, "application/json");
  httpd_resp_set_hdr(request, "Cache-Control", "no-store");
  return httpd_resp_send(request, body, HTTPD_RESP_USE_STRLEN);
}

esp_err_t send_json_object(httpd_req_t* request, const char* status, cJSON* object) {
  char* printed = cJSON_PrintUnformatted(object);
  cJSON_Delete(object);
  if (printed == nullptr) return send_status_json(request, "500 Internal Server Error", "{\"error\":\"encode\"}");
  esp_err_t err = send_status_json(request, status, printed);
  cJSON_free(printed);
  return err;
}

esp_err_t send_error(httpd_req_t* request, const char* status, const char* error, const char* field) {
  cJSON* object = cJSON_CreateObject();
  cJSON_AddStringToObject(object, "error", error);
  if (field != nullptr) cJSON_AddStringToObject(object, "field", field);
  return send_json_object(request, status, object);
}

bool authorized(httpd_req_t* request) {
  char header[128]{};
  if (httpd_req_get_hdr_value_str(request, "Authorization", header, sizeof(header)) != ESP_OK) return false;
  static constexpr char kPrefix[] = "Bearer ";
  constexpr size_t kPrefixLen = sizeof(kPrefix) - 1;
  const size_t token_len = std::strlen(secrets::kHttpBearerToken);
  const size_t header_len = std::strlen(header);
  if (header_len != kPrefixLen + token_len) return false;
  unsigned diff = 0;
  for (size_t i = 0; i < kPrefixLen; ++i) {
    diff |= static_cast<unsigned char>(header[i]) ^ static_cast<unsigned char>(kPrefix[i]);
  }
  for (size_t i = 0; i < token_len; ++i) {
    diff |= static_cast<unsigned char>(header[kPrefixLen + i]) ^
            static_cast<unsigned char>(secrets::kHttpBearerToken[i]);
  }
  return diff == 0;
}

bool require_auth(httpd_req_t* request) {
  if (authorized(request)) return true;
  core::note_http_auth_refused();
  send_status_json(request, "401 Unauthorized", "{\"error\":\"unauthorized\"}");
  return false;
}

bool read_body(httpd_req_t* request, char* body, size_t capacity) {
  if (request->content_len <= 0 || static_cast<size_t>(request->content_len) >= capacity) return false;
  int received = 0;
  while (received < request->content_len) {
    int chunk = httpd_req_recv(request, body + received, request->content_len - received);
    if (chunk <= 0) return false;
    received += chunk;
  }
  body[received] = '\0';
  return true;
}

const char* freshness_text(core::Freshness freshness) {
  switch (freshness) {
    case core::Freshness::kFresh: return "fresh";
    case core::Freshness::kStale: return "stale";
    case core::Freshness::kMissing: return "missing";
  }
  return "missing";
}

const char* network_text(core::NetworkState state) {
  switch (state) {
    case core::NetworkState::kOff: return "off";
    case core::NetworkState::kApProvisioning: return "ap_provisioning";
    case core::NetworkState::kStaConnecting: return "sta_connecting";
    case core::NetworkState::kStaConnected: return "sta_connected";
    case core::NetworkState::kStaDisconnected: return "sta_disconnected";
  }
  return "unknown";
}

const char* preinfusion_mode_text(core::PreinfusionMode mode) {
  return mode == core::PreinfusionMode::kPressure ? "pressure" : "time";
}

const char* rampdown_mode_text(core::RampdownMode mode) {
  switch (mode) {
    case core::RampdownMode::kNone: return "none";
    case core::RampdownMode::kTime: return "time";
    case core::RampdownMode::kWeight: return "weight";
    case core::RampdownMode::kPressureDrop: return "pressure_drop";
  }
  return "none";
}

void add_age(cJSON* object, const char* key, uint32_t age_ms) {
  if (age_ms == std::numeric_limits<uint32_t>::max()) {
    cJSON_AddNullToObject(object, key);
    return;
  }
  cJSON_AddNumberToObject(object, key, age_ms);
}

cJSON* encode_config(const core::Config& config) {
  cJSON* root = cJSON_CreateObject();
  cJSON_AddNumberToObject(root, "version", config.version);
  cJSON* brew = cJSON_AddObjectToObject(root, "brew");
  cJSON_AddNumberToObject(brew, "target_weight_g", config.target_weight_g);
  cJSON_AddNumberToObject(brew, "target_time_s", config.target_time_s);
  cJSON_AddNumberToObject(brew, "pump_pct", config.brew_pump_pct);
  cJSON* preinfusion = cJSON_AddObjectToObject(root, "preinfusion");
  cJSON_AddStringToObject(preinfusion, "mode", preinfusion_mode_text(config.preinfusion_mode));
  cJSON_AddNumberToObject(preinfusion, "time_s", config.preinfusion_time_s);
  cJSON_AddNumberToObject(preinfusion, "pressure_bar", config.preinfusion_pressure_bar);
  cJSON_AddNumberToObject(preinfusion, "pump_pct", config.preinfusion_pump_pct);
  cJSON* rampdown = cJSON_AddObjectToObject(root, "rampdown");
  cJSON_AddStringToObject(rampdown, "mode", rampdown_mode_text(config.rampdown_mode));
  cJSON_AddNumberToObject(rampdown, "lead_time_s", config.rampdown_lead_time_s);
  cJSON_AddNumberToObject(rampdown, "lead_weight_g", config.rampdown_lead_weight_g);
  cJSON_AddNumberToObject(rampdown, "pressure_drop_bar", config.rampdown_pressure_drop_bar);
  cJSON* purge = cJSON_AddObjectToObject(root, "purge");
  cJSON_AddNumberToObject(purge, "pump_pct", config.purge_pump_pct);
  cJSON_AddNumberToObject(purge, "max_s", config.purge_max_s);
  cJSON* ui = cJSON_AddObjectToObject(root, "ui");
  cJSON_AddNumberToObject(ui, "dim_after_s", config.dim_after_s);
  cJSON_AddNumberToObject(ui, "standby_after_s", config.standby_after_s);
  cJSON_AddItemToObject(root, "profiles", cJSON_CreateArray());
  return root;
}

cJSON* encode_telemetry(const core::Snapshot& snapshot) {
  cJSON* root = cJSON_CreateObject();
  cJSON_AddNumberToObject(root, "pressure_bar", snapshot.pressure_bar);
  cJSON_AddNumberToObject(root, "temperature_c", snapshot.temperature_c);
  cJSON_AddNumberToObject(root, "flow_ml_s", snapshot.flow_ml_s);
  cJSON_AddNumberToObject(root, "volume_ml", snapshot.volume_ml);
  cJSON_AddNumberToObject(root, "pressure_raw", snapshot.pressure_raw);
  cJSON_AddNumberToObject(root, "temperature_raw", snapshot.temperature_raw);
  cJSON_AddNumberToObject(root, "flow_pulse_count", snapshot.flow_pulse_count);
  cJSON_AddBoolToObject(root, "pressure_valid", snapshot.pressure_valid);
  cJSON_AddBoolToObject(root, "flow_valid", snapshot.flow_valid);
  cJSON_AddStringToObject(root, "pressure_freshness", freshness_text(snapshot.pressure_freshness));
  cJSON_AddStringToObject(root, "flow_freshness", freshness_text(snapshot.flow_freshness));
  cJSON_AddStringToObject(root, "actuators_freshness", freshness_text(snapshot.actuators_freshness));
  add_age(root, "pressure_age_ms", snapshot.pressure_age_ms);
  add_age(root, "flow_age_ms", snapshot.flow_age_ms);
  add_age(root, "actuators_age_ms", snapshot.actuators_age_ms);
  add_age(root, "flow_last_edge_age_ms", snapshot.flow_last_edge_age_ms);
  cJSON_AddBoolToObject(root, "sensors_alive", snapshot.sensors_alive);
  cJSON_AddBoolToObject(root, "valve_open", snapshot.valve_open);
  cJSON_AddNumberToObject(root, "dimmer_pct", snapshot.dimmer_pct);
  cJSON_AddBoolToObject(root, "dimmer_ready", snapshot.dimmer_ready);
  cJSON_AddBoolToObject(root, "dimmer_valid", snapshot.dimmer_valid);
  cJSON_AddBoolToObject(root, "dimmer_error_active", snapshot.dimmer_error_active);
  cJSON_AddBoolToObject(root, "lockout", snapshot.lockout);
  cJSON_AddNumberToObject(root, "lease_remaining_ms", snapshot.lease_remaining_ms);
  cJSON_AddNumberToObject(root, "continuous_on_ms", snapshot.continuous_on_ms);
  cJSON_AddNumberToObject(root, "weight_g", snapshot.weight_g);
  cJSON_AddBoolToObject(root, "scale_connected", snapshot.scale_connected);
  cJSON_AddBoolToObject(root, "scale_present", snapshot.scale_present);
  add_age(root, "scale_age_ms", snapshot.scale_age_ms);
  const char* cycle = "idle";
  switch (snapshot.cycle_state) {
    case core::CycleState::kPreinfusion: cycle = "preinfusion"; break;
    case core::CycleState::kBrew: cycle = "brew"; break;
    case core::CycleState::kRampdown: cycle = "rampdown"; break;
    case core::CycleState::kFinished: cycle = "finished"; break;
    case core::CycleState::kPurge: cycle = "purge"; break;
    case core::CycleState::kIdle: break;
  }
  cJSON_AddStringToObject(root, "cycle", cycle);
  cJSON_AddNumberToObject(root, "cycle_elapsed_ms", snapshot.cycle_elapsed_ms);
  cJSON_AddBoolToObject(root, "cycle_weight_goal", snapshot.cycle_weight_goal);
  cJSON* last_shot = cJSON_AddObjectToObject(root, "last_shot");
  cJSON_AddBoolToObject(last_shot, "available", snapshot.last_shot_available);
  if (snapshot.last_shot_available) {
    cJSON_AddNumberToObject(last_shot, "weight_g", snapshot.last_shot_weight_g);
    cJSON_AddNumberToObject(last_shot, "duration_ms", snapshot.last_shot_duration_ms);
    cJSON_AddNumberToObject(last_shot, "flow_ml_s", snapshot.last_shot_flow_ml_s);
    if (snapshot.last_shot_unix_s != 0) cJSON_AddNumberToObject(last_shot, "unix_s", static_cast<double>(snapshot.last_shot_unix_s));
    else cJSON_AddNullToObject(last_shot, "unix_s");
  }
  cJSON* flash = cJSON_AddObjectToObject(root, "flash");
  cJSON_AddBoolToObject(flash, "active", snapshot.flash_active);
  cJSON_AddStringToObject(flash, "target", snapshot.flash_target == core::FlashTarget::kScreen ? "screen" : snapshot.flash_target == core::FlashTarget::kSensors ? "sensors" : "none");
  cJSON_AddNumberToObject(flash, "bytes_done", snapshot.flash_bytes_done);
  cJSON_AddNumberToObject(flash, "bytes_total", snapshot.flash_bytes_total);

  cJSON* network = cJSON_AddObjectToObject(root, "network");
  cJSON_AddStringToObject(network, "radio_mode",
                          snapshot.radio_mode == core::RadioMode::kWifi ? "wifi" :
                          snapshot.radio_mode == core::RadioMode::kMachine ? "machine" : "off");
  cJSON_AddBoolToObject(network, "transition", snapshot.radio_transition);
  cJSON_AddStringToObject(network, "state", network_text(static_cast<core::NetworkState>(snapshot.network_state)));
  if (snapshot.ipv4_address == 0) {
    cJSON_AddNullToObject(network, "ipv4");
  } else {
    char ip[16];
    std::snprintf(ip, sizeof(ip), "%u.%u.%u.%u",
                  static_cast<unsigned>(snapshot.ipv4_address & 0xFFu),
                  static_cast<unsigned>((snapshot.ipv4_address >> 8) & 0xFFu),
                  static_cast<unsigned>((snapshot.ipv4_address >> 16) & 0xFFu),
                  static_cast<unsigned>((snapshot.ipv4_address >> 24) & 0xFFu));
    cJSON_AddStringToObject(network, "ipv4", ip);
  }
  cJSON_AddBoolToObject(network, "time_known", snapshot.time_known);
  if (snapshot.time_known) {
    cJSON_AddNumberToObject(network, "wall_time_unix_s", static_cast<double>(snapshot.wall_time_unix_s));
  } else {
    cJSON_AddNullToObject(network, "wall_time_unix_s");
  }

  cJSON* screen = cJSON_AddObjectToObject(root, "screen");
  cJSON_AddNumberToObject(screen, "version_major", snapshot.screen_version_major);
  cJSON_AddNumberToObject(screen, "version_minor", snapshot.screen_version_minor);
  cJSON_AddNumberToObject(screen, "version_patch", snapshot.screen_version_patch);
  cJSON_AddNumberToObject(screen, "uptime_s", snapshot.screen_uptime_s);

  cJSON* sensors = cJSON_AddObjectToObject(root, "sensors");
  cJSON_AddNumberToObject(sensors, "version_major", snapshot.sensors_version_major);
  cJSON_AddNumberToObject(sensors, "version_minor", snapshot.sensors_version_minor);
  cJSON_AddNumberToObject(sensors, "version_patch", snapshot.sensors_version_patch);
  cJSON_AddNumberToObject(sensors, "uptime_s", snapshot.sensors_uptime_s);
  cJSON_AddNumberToObject(sensors, "twai_rx_errors", snapshot.sensors_twai_rx_errors);
  cJSON_AddNumberToObject(sensors, "twai_tx_errors", snapshot.sensors_twai_tx_errors);
  cJSON_AddNumberToObject(sensors, "twai_bus_errors", snapshot.sensors_twai_bus_errors);
  return root;
}

bool is_integer_number(const cJSON* node) {
  if (!cJSON_IsNumber(node)) return false;
  double value = node->valuedouble;
  return std::isfinite(value) && value == std::floor(value);
}

bool as_u16(const cJSON* node, uint16_t* out) {
  if (!is_integer_number(node) || node->valuedouble < 0 || node->valuedouble > 65535) return false;
  *out = static_cast<uint16_t>(node->valuedouble);
  return true;
}

bool as_u8(const cJSON* node, uint8_t* out) {
  if (!is_integer_number(node) || node->valuedouble < 0 || node->valuedouble > 255) return false;
  *out = static_cast<uint8_t>(node->valuedouble);
  return true;
}

bool as_float(const cJSON* node, float* out) {
  if (!cJSON_IsNumber(node) || !std::isfinite(node->valuedouble)) return false;
  *out = static_cast<float>(node->valuedouble);
  return true;
}

char g_field[48];

const char* join_field(const char* parent, const char* key) {
  std::snprintf(g_field, sizeof(g_field), "%s.%s", parent, key);
  return g_field;
}

const char* store_field(const char* name) {
  std::snprintf(g_field, sizeof(g_field), "%s", name != nullptr ? name : "");
  return g_field;
}

bool overlay_number_u16(cJSON* value, const char* field, uint16_t* dest, const char** error_field) {
  if (!as_u16(value, dest)) {
    *error_field = field;
    return false;
  }
  return true;
}

bool overlay_number_u8(cJSON* value, const char* field, uint8_t* dest, const char** error_field) {
  if (!as_u8(value, dest)) {
    *error_field = field;
    return false;
  }
  return true;
}

bool overlay_number_float(cJSON* value, const char* field, float* dest, const char** error_field) {
  if (!as_float(value, dest)) {
    *error_field = field;
    return false;
  }
  return true;
}

bool overlay_object(cJSON* object, const char* parent, core::Config* config, const char** error_field,
                    bool (*apply_key)(const char* key, cJSON* value, core::Config* config, const char** error_field)) {
  if (!cJSON_IsObject(object)) {
    *error_field = parent;
    return false;
  }
  for (cJSON* child = object->child; child != nullptr; child = child->next) {
    if (child->string == nullptr || !apply_key(child->string, child, config, error_field)) {
      if (*error_field == nullptr) *error_field = join_field(parent, child->string != nullptr ? child->string : "?");
      return false;
    }
  }
  return true;
}

bool apply_brew_key(const char* key, cJSON* value, core::Config* config, const char** error_field) {
  if (std::strcmp(key, "target_weight_g") == 0) {
    return overlay_number_float(value, "brew.target_weight_g", &config->target_weight_g, error_field);
  }
  if (std::strcmp(key, "target_time_s") == 0) {
    return overlay_number_u16(value, "brew.target_time_s", &config->target_time_s, error_field);
  }
  if (std::strcmp(key, "pump_pct") == 0) {
    return overlay_number_u8(value, "brew.pump_pct", &config->brew_pump_pct, error_field);
  }
  *error_field = join_field("brew", key);
  return false;
}

bool apply_preinfusion_key(const char* key, cJSON* value, core::Config* config, const char** error_field) {
  if (std::strcmp(key, "mode") == 0) {
    if (!cJSON_IsString(value)) {
      *error_field = "preinfusion.mode";
      return false;
    }
    if (std::strcmp(value->valuestring, "time") == 0) {
      config->preinfusion_mode = core::PreinfusionMode::kTime;
      return true;
    }
    if (std::strcmp(value->valuestring, "pressure") == 0) {
      config->preinfusion_mode = core::PreinfusionMode::kPressure;
      return true;
    }
    *error_field = "preinfusion.mode";
    return false;
  }
  if (std::strcmp(key, "time_s") == 0) {
    return overlay_number_u16(value, "preinfusion.time_s", &config->preinfusion_time_s, error_field);
  }
  if (std::strcmp(key, "pressure_bar") == 0) {
    return overlay_number_float(value, "preinfusion.pressure_bar", &config->preinfusion_pressure_bar, error_field);
  }
  if (std::strcmp(key, "pump_pct") == 0) {
    return overlay_number_u8(value, "preinfusion.pump_pct", &config->preinfusion_pump_pct, error_field);
  }
  *error_field = join_field("preinfusion", key);
  return false;
}

bool apply_rampdown_key(const char* key, cJSON* value, core::Config* config, const char** error_field) {
  if (std::strcmp(key, "mode") == 0) {
    if (!cJSON_IsString(value)) {
      *error_field = "rampdown.mode";
      return false;
    }
    if (std::strcmp(value->valuestring, "none") == 0) config->rampdown_mode = core::RampdownMode::kNone;
    else if (std::strcmp(value->valuestring, "time") == 0) config->rampdown_mode = core::RampdownMode::kTime;
    else if (std::strcmp(value->valuestring, "weight") == 0) config->rampdown_mode = core::RampdownMode::kWeight;
    else if (std::strcmp(value->valuestring, "pressure_drop") == 0) {
      config->rampdown_mode = core::RampdownMode::kPressureDrop;
    } else {
      *error_field = "rampdown.mode";
      return false;
    }
    return true;
  }
  if (std::strcmp(key, "lead_time_s") == 0) {
    return overlay_number_float(value, "rampdown.lead_time_s", &config->rampdown_lead_time_s, error_field);
  }
  if (std::strcmp(key, "lead_weight_g") == 0) {
    return overlay_number_float(value, "rampdown.lead_weight_g", &config->rampdown_lead_weight_g, error_field);
  }
  if (std::strcmp(key, "pressure_drop_bar") == 0) {
    return overlay_number_float(value, "rampdown.pressure_drop_bar", &config->rampdown_pressure_drop_bar,
                                error_field);
  }
  *error_field = join_field("rampdown", key);
  return false;
}

bool apply_purge_key(const char* key, cJSON* value, core::Config* config, const char** error_field) {
  if (std::strcmp(key, "pump_pct") == 0) {
    return overlay_number_u8(value, "purge.pump_pct", &config->purge_pump_pct, error_field);
  }
  if (std::strcmp(key, "max_s") == 0) {
    return overlay_number_u16(value, "purge.max_s", &config->purge_max_s, error_field);
  }
  *error_field = join_field("purge", key);
  return false;
}

bool apply_ui_key(const char* key, cJSON* value, core::Config* config, const char** error_field) {
  if (std::strcmp(key, "dim_after_s") == 0) {
    return overlay_number_u16(value, "ui.dim_after_s", &config->dim_after_s, error_field);
  }
  if (std::strcmp(key, "standby_after_s") == 0) {
    return overlay_number_u16(value, "ui.standby_after_s", &config->standby_after_s, error_field);
  }
  *error_field = join_field("ui", key);
  return false;
}

bool apply_json_patch(cJSON* root, core::Config* config, const char** error_field) {
  if (!cJSON_IsObject(root)) {
    *error_field = "version";
    return false;
  }
  cJSON* version = cJSON_GetObjectItemCaseSensitive(root, "version");
  if (version == nullptr) {
    *error_field = "version";
    return false;
  }
  uint16_t parsed_version = 0;
  if (!as_u16(version, &parsed_version)) {
    *error_field = "version";
    return false;
  }
  config->version = parsed_version;

  for (cJSON* child = root->child; child != nullptr; child = child->next) {
    if (child->string == nullptr) {
      *error_field = "version";
      return false;
    }
    if (std::strcmp(child->string, "version") == 0) continue;
    if (std::strcmp(child->string, "brew") == 0) {
      if (!overlay_object(child, "brew", config, error_field, apply_brew_key)) return false;
      continue;
    }
    if (std::strcmp(child->string, "preinfusion") == 0) {
      if (!overlay_object(child, "preinfusion", config, error_field, apply_preinfusion_key)) return false;
      continue;
    }
    if (std::strcmp(child->string, "rampdown") == 0) {
      if (!overlay_object(child, "rampdown", config, error_field, apply_rampdown_key)) return false;
      continue;
    }
    if (std::strcmp(child->string, "purge") == 0) {
      if (!overlay_object(child, "purge", config, error_field, apply_purge_key)) return false;
      continue;
    }
    if (std::strcmp(child->string, "ui") == 0) {
      if (!overlay_object(child, "ui", config, error_field, apply_ui_key)) return false;
      continue;
    }
    if (std::strcmp(child->string, "profiles") == 0) {
      if (!cJSON_IsArray(child) || cJSON_GetArraySize(child) != 0) {
        *error_field = "profiles";
        return false;
      }
      continue;
    }
    *error_field = store_field(child->string);
    return false;
  }
  return true;
}

esp_err_t reject_config(httpd_req_t* request, const char* error, const char* field) {
  core::note_config_rejected(field);
  return send_error(request, "400 Bad Request", error, field);
}

esp_err_t telemetry_handler(httpd_req_t* request) {
  if (!require_auth(request)) return ESP_OK;
  return send_json_object(request, "200 OK", encode_telemetry(core::get_snapshot()));
}

esp_err_t get_config_handler(httpd_req_t* request) {
  if (!require_auth(request)) return ESP_OK;
  return send_json_object(request, "200 OK", encode_config(core::get_config()));
}

esp_err_t post_config_handler(httpd_req_t* request) {
  if (!require_auth(request)) return ESP_OK;
  char body[kMaxBody + 1]{};
  if (!read_body(request, body, sizeof(body))) return send_error(request, "400 Bad Request", "invalid_json", nullptr);
  cJSON* root = cJSON_Parse(body);
  if (root == nullptr) return send_error(request, "400 Bad Request", "invalid_json", nullptr);
  core::Config config = core::get_config();
  const char* field = nullptr;
  bool ok = apply_json_patch(root, &config, &field);
  cJSON_Delete(root);
  if (!ok) return reject_config(request, "invalid_value", field);

  core::ConfigResult result = core::put_config(config);
  if (result.status == core::ConfigStatus::kOk) {
    return send_json_object(request, "200 OK", encode_config(core::get_config()));
  }
  if (result.status == core::ConfigStatus::kBusy) {
    return send_error(request, "409 Conflict", "busy", result.field);
  }
  if (result.status == core::ConfigStatus::kStorageError) {
    return send_error(request, "500 Internal Server Error", "storage", result.field);
  }
  if (result.status == core::ConfigStatus::kStaleRevision) {
    return send_error(request, "409 Conflict", "conflict", result.field);
  }
  const char* error = result.status == core::ConfigStatus::kVersionMismatch ? "unknown_version" : "invalid_value";
  return reject_config(request, error, result.field);
}

bool parse_action(cJSON* root, core::ActionCommand* command, const char** field) {
  if (!cJSON_IsObject(root)) {
    *field = "action";
    return false;
  }
  cJSON* action = cJSON_GetObjectItemCaseSensitive(root, "action");
  if (!cJSON_IsString(action)) {
    *field = "action";
    return false;
  }
  if (std::strcmp(action->valuestring, "set_actuators") == 0) command->action = core::Action::kSetActuators;
  else if (std::strcmp(action->valuestring, "start_brew") == 0) command->action = core::Action::kStartBrew;
  else if (std::strcmp(action->valuestring, "stop_brew") == 0) command->action = core::Action::kStopBrew;
  else if (std::strcmp(action->valuestring, "purge_press") == 0) command->action = core::Action::kPurgePress;
  else if (std::strcmp(action->valuestring, "purge_release") == 0) command->action = core::Action::kPurgeRelease;
  else if (std::strcmp(action->valuestring, "tare") == 0) command->action = core::Action::kTare;
  else if (std::strcmp(action->valuestring, "dismiss_summary") == 0) command->action = core::Action::kDismissSummary;
  else if (std::strcmp(action->valuestring, "start_flash") == 0) command->action = core::Action::kStartFlash;
  else {
    *field = "action";
    return false;
  }

  for (cJSON* child = root->child; child != nullptr; child = child->next) {
    if (child->string == nullptr) {
      *field = "action";
      return false;
    }
    if (std::strcmp(child->string, "action") == 0) continue;
    if (command->action != core::Action::kSetActuators) {
      *field = store_field(child->string);
      return false;
    }
    if (std::strcmp(child->string, "ssr") == 0) {
      if (!cJSON_IsBool(child)) {
        *field = "ssr";
        return false;
      }
      command->ssr = cJSON_IsTrue(child);
      continue;
    }
    if (std::strcmp(child->string, "dimmer") == 0) {
      if (!as_u8(child, &command->dimmer)) {
        *field = "dimmer";
        return false;
      }
      continue;
    }
    if (std::strcmp(child->string, "ttl_ms") == 0) {
      if (!as_u16(child, &command->ttl_ms)) {
        *field = "ttl_ms";
        return false;
      }
      continue;
    }
    *field = store_field(child->string);
    return false;
  }

  if (command->action == core::Action::kSetActuators) {
    if (cJSON_GetObjectItemCaseSensitive(root, "ssr") == nullptr) {
      *field = "ssr";
      return false;
    }
    if (cJSON_GetObjectItemCaseSensitive(root, "dimmer") == nullptr) {
      *field = "dimmer";
      return false;
    }
  }
  return true;
}

esp_err_t post_action_handler(httpd_req_t* request) {
  if (!require_auth(request)) return ESP_OK;
  char body[kMaxBody + 1]{};
  if (!read_body(request, body, sizeof(body))) return send_error(request, "400 Bad Request", "invalid_json", nullptr);
  cJSON* root = cJSON_Parse(body);
  if (root == nullptr) return send_error(request, "400 Bad Request", "invalid_json", nullptr);
  core::ActionCommand command;
  const char* field = nullptr;
  bool ok = parse_action(root, &command, &field);
  cJSON_Delete(root);
  if (!ok) return send_error(request, "400 Bad Request", "invalid_value", field);

  core::ActionResult result = core::perform_action(command);
  cJSON* response = cJSON_CreateObject();
  cJSON_AddBoolToObject(response, "ok", result.status == core::ActionStatus::kOk);
  cJSON_AddStringToObject(response, "reason", result.reason);
  if (result.status == core::ActionStatus::kOk) return send_json_object(request, "200 OK", response);
  if (result.status == core::ActionStatus::kInvalidValue) {
    cJSON_Delete(response);
    return send_error(request, "400 Bad Request", "invalid_value", "dimmer");
  }
  return send_json_object(request, "409 Conflict", response);
}

bool query_target(httpd_req_t* request, char* target, size_t size) {
  char query[48]{};
  if (httpd_req_get_url_query_str(request, query, sizeof(query)) != ESP_OK) return false;
  return httpd_query_key_value(query, "target", target, size) == ESP_OK;
}

esp_err_t firmware_handler(httpd_req_t* request) {
  if (!require_auth(request)) return ESP_OK;
  char target[12]{};
  if (!query_target(request, target, sizeof(target)) || request->content_len <= 0) return send_error(request, "400 Bad Request", "invalid_firmware", "target");
  const uint32_t size = static_cast<uint32_t>(request->content_len);
  const bool screen = std::strcmp(target, "screen") == 0;
  const bool sensors = std::strcmp(target, "sensors") == 0;
  if (!screen && !sensors) return send_error(request, "400 Bad Request", "invalid_firmware", "target");
  if (!core::begin_flash(screen ? core::FlashTarget::kScreen : core::FlashTarget::kSensors, size)) return send_error(request, "409 Conflict", "busy", nullptr);

  bool started = false;
  if (screen) {
    g_http_ota_partition = esp_ota_get_next_update_partition(nullptr);
    started = g_http_ota_partition != nullptr && size <= g_http_ota_partition->size &&
              esp_ota_begin(g_http_ota_partition, size, &g_http_ota) == ESP_OK;
    g_http_ota_crc = common::Crc32Incremental{};
  } else started = ota_proxy::begin_upload(size);
  if (!started) { core::finish_flash(); return send_error(request, "400 Bad Request", "invalid_firmware", "size"); }

  uint8_t chunk[1024]; uint32_t done = 0;
  while (done < size) {
    int got = httpd_req_recv(request, reinterpret_cast<char*>(chunk), (size - done < sizeof(chunk)) ? size - done : sizeof(chunk));
    if (got <= 0) { if (screen) esp_ota_abort(g_http_ota); else ota_proxy::abort_upload(); core::finish_flash(); return send_error(request, "400 Bad Request", "incomplete_upload", nullptr); }
    bool ok = screen ? esp_ota_write(g_http_ota, chunk, got) == ESP_OK : ota_proxy::write_upload(chunk, got);
    if (!ok) { if (screen) esp_ota_abort(g_http_ota); else ota_proxy::abort_upload(); core::finish_flash(); return send_error(request, "500 Internal Server Error", "write_failed", nullptr); }
    if (screen) g_http_ota_crc.update(chunk, got);
    done += got; core::update_flash_progress(done);
  }
  if (screen) {
    bool ok = esp_ota_end(g_http_ota) == ESP_OK && esp_ota_set_boot_partition(g_http_ota_partition) == ESP_OK;
    if (!ok) { core::finish_flash(); return send_error(request, "400 Bad Request", "invalid_firmware", nullptr); }
    send_status_json(request, "202 Accepted", "{\"ok\":true,\"rebooting\":true}");
    xTaskCreatePinnedToCore(reboot_after_http_response, "http_reboot", 2048, nullptr, 4, nullptr, 0);
    return ESP_OK;
  }
  if (!ota_proxy::commit_upload()) { ota_proxy::abort_upload(); return send_error(request, "500 Internal Server Error", "queue_failed", nullptr); }
  return send_status_json(request, "202 Accepted", "{\"ok\":true,\"queued\":true}");
}

}  // namespace

void start() {
  if (g_server != nullptr) return;
  httpd_config_t cfg = HTTPD_DEFAULT_CONFIG();
  cfg.core_id = 0;
  cfg.max_open_sockets = 4;
  cfg.stack_size = 8192;
  cfg.lru_purge_enable = true;
  if (httpd_start(&g_server, &cfg) != ESP_OK) {
    g_server = nullptr;
    ESP_LOGE(kTag, "httpd_start failed");
    return;
  }
  const httpd_uri_t telemetry{.uri = "/telemetry", .method = HTTP_GET, .handler = telemetry_handler, .user_ctx = nullptr,
                             .is_websocket = false, .handle_ws_control_frames = false, .supported_subprotocol = nullptr,
                             .ws_pre_handshake_cb = nullptr, .ws_post_handshake_cb = nullptr};
  const httpd_uri_t get_config{.uri = "/config", .method = HTTP_GET, .handler = get_config_handler, .user_ctx = nullptr,
                              .is_websocket = false, .handle_ws_control_frames = false, .supported_subprotocol = nullptr,
                              .ws_pre_handshake_cb = nullptr, .ws_post_handshake_cb = nullptr};
  const httpd_uri_t post_config{.uri = "/config", .method = HTTP_POST, .handler = post_config_handler, .user_ctx = nullptr,
                               .is_websocket = false, .handle_ws_control_frames = false, .supported_subprotocol = nullptr,
                               .ws_pre_handshake_cb = nullptr, .ws_post_handshake_cb = nullptr};
  const httpd_uri_t post_action{.uri = "/action", .method = HTTP_POST, .handler = post_action_handler, .user_ctx = nullptr,
                               .is_websocket = false, .handle_ws_control_frames = false, .supported_subprotocol = nullptr,
                             .ws_pre_handshake_cb = nullptr, .ws_post_handshake_cb = nullptr};
  const httpd_uri_t firmware{.uri = "/firmware", .method = HTTP_POST, .handler = firmware_handler, .user_ctx = nullptr,
                             .is_websocket = false, .handle_ws_control_frames = false, .supported_subprotocol = nullptr,
                             .ws_pre_handshake_cb = nullptr, .ws_post_handshake_cb = nullptr};
  httpd_register_uri_handler(g_server, &telemetry);
  httpd_register_uri_handler(g_server, &get_config);
  httpd_register_uri_handler(g_server, &post_config);
  httpd_register_uri_handler(g_server, &post_action);
  httpd_register_uri_handler(g_server, &firmware);
  net_ws::start(g_server, require_auth);
}

void stop() {
  if (g_server == nullptr) return;
  net_ws::stop();
  httpd_stop(g_server);
  g_server = nullptr;
}

}  // namespace net_http
