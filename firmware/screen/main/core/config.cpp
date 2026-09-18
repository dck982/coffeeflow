#include "core/config.h"

#include <cmath>
#include <cstddef>
#include <cstring>

#include "esp_log.h"
#include "nvs.h"
#include "freertos/FreeRTOS.h"

namespace core {
namespace {
constexpr const char* kTag = "core_config";
constexpr const char* kNamespace = "ui";
constexpr uint32_t kMagic = 0x43464731;  // CFG1

struct StoredConfig { uint32_t magic; Config config; uint32_t checksum; };
static_assert(sizeof(StoredConfig) < 512, "configuration must remain a small NVS blob");

enum class LegacyPreinfusionMode : uint8_t { kTime, kPressure };
struct LegacyConfig {
  uint16_t version;
  uint32_t revision;
  float target_weight_g;
  uint16_t target_time_s;
  LegacyPreinfusionMode preinfusion_mode;
  uint16_t preinfusion_time_s;
  float preinfusion_pressure_bar;
  uint8_t preinfusion_pump_pct;
  RampdownMode rampdown_mode;
  float rampdown_lead_time_s;
  float rampdown_lead_weight_g;
  float rampdown_pressure_drop_bar;
  uint8_t brew_pump_pct;
  uint8_t purge_pump_pct;
  uint16_t purge_max_s;
  uint16_t dim_after_s;
  uint16_t standby_after_s;
};
struct LegacyStoredConfig { uint32_t magic; LegacyConfig config; uint32_t checksum; };
static_assert(sizeof(LegacyStoredConfig) == sizeof(StoredConfig), "configuration layout changed unexpectedly");

portMUX_TYPE g_lock = portMUX_INITIALIZER_UNLOCKED;
Config g_config{};

uint32_t checksum_bytes(const void* value, size_t size) {
  const auto* bytes = static_cast<const uint8_t*>(value);
  uint32_t result = 2166136261u;
  for (size_t i = 0; i < size; ++i) result = (result ^ bytes[i]) * 16777619u;
  return result;
}

uint32_t checksum(const StoredConfig& stored) {
  return checksum_bytes(&stored, offsetof(StoredConfig, checksum));
}

uint32_t legacy_checksum(const LegacyStoredConfig& stored) {
  return checksum_bytes(&stored, offsetof(LegacyStoredConfig, checksum));
}

bool read_slot(nvs_handle_t handle, const char* key, StoredConfig* out) {
  size_t size = sizeof(*out);
  if (nvs_get_blob(handle, key, out, &size) != ESP_OK || size != sizeof(*out)) return false;
  return out->magic == kMagic && out->config.version == kConfigSchemaVersion && out->checksum == checksum(*out);
}

bool read_legacy_slot(nvs_handle_t handle, const char* key, LegacyStoredConfig* out) {
  size_t size = sizeof(*out);
  if (nvs_get_blob(handle, key, out, &size) != ESP_OK || size != sizeof(*out)) return false;
  return out->magic == kMagic && out->config.version == 1 && out->checksum == legacy_checksum(*out);
}

Config migrate_legacy(const LegacyConfig& legacy) {
  Config migrated{};
  migrated.version = kConfigSchemaVersion;
  migrated.revision = legacy.revision;
  migrated.target_weight_g = legacy.target_weight_g;
  migrated.target_time_s = legacy.target_time_s;
  migrated.preinfusion_mode = legacy.preinfusion_mode == LegacyPreinfusionMode::kPressure
                                  ? PreinfusionMode::kPressure
                                  : PreinfusionMode::kTime;
  migrated.preinfusion_time_s = legacy.preinfusion_time_s;
  migrated.preinfusion_pressure_bar = legacy.preinfusion_pressure_bar;
  migrated.preinfusion_pump_pct = legacy.preinfusion_pump_pct;
  migrated.rampdown_mode = legacy.rampdown_mode;
  migrated.rampdown_lead_time_s = legacy.rampdown_lead_time_s;
  migrated.rampdown_lead_weight_g = legacy.rampdown_lead_weight_g;
  migrated.rampdown_pressure_drop_bar = legacy.rampdown_pressure_drop_bar;
  migrated.brew_pump_pct = legacy.brew_pump_pct;
  migrated.purge_pump_pct = legacy.purge_pump_pct;
  migrated.purge_max_s = legacy.purge_max_s;
  migrated.dim_after_s = legacy.dim_after_s;
  migrated.standby_after_s = legacy.standby_after_s;
  return migrated;
}

bool valid_step(float value, float min, float max, float step) {
  if (!std::isfinite(value) || value < min || value > max) return false;
  float scaled = (value - min) / step;
  return std::fabs(scaled - std::round(scaled)) < 0.001f;
}

const char* validate(const Config& c) {
  if (c.version != kConfigSchemaVersion) return "version";
  if (!valid_step(c.target_weight_g, 10, 100, .5f)) return "brew.target_weight_g";
  if (c.target_time_s < 5 || c.target_time_s > 60) return "brew.target_time_s";
  if ((static_cast<uint8_t>(c.preinfusion_mode) & ~0x07u) != 0) {
    return "preinfusion.mode";
  }
  if (c.preinfusion_time_s > 20) return "preinfusion.time_s";
  if (!valid_step(c.preinfusion_pressure_bar, 1, 9, .5f)) return "preinfusion.pressure_bar";
  if (c.preinfusion_pump_pct > 100 || c.preinfusion_pump_pct % 5) return "preinfusion.pump_pct";
  if (c.rampdown_mode > RampdownMode::kPressureDrop) return "rampdown.mode";
  if (!valid_step(c.rampdown_lead_time_s, 0, 15, .5f)) return "rampdown.lead_time_s";
  if (!valid_step(c.rampdown_lead_weight_g, 0, 20, .5f)) return "rampdown.lead_weight_g";
  if (!valid_step(c.rampdown_pressure_drop_bar, .5f, 4, .5f)) return "rampdown.pressure_drop_bar";
  if (c.brew_pump_pct < 20 || c.brew_pump_pct > 100 || c.brew_pump_pct % 5) return "brew.pump_pct";
  if (c.purge_pump_pct < 20 || c.purge_pump_pct > 100 || c.purge_pump_pct % 5) return "purge.pump_pct";
  if (c.purge_max_s < 5 || c.purge_max_s > 60 || c.purge_max_s % 5) return "purge.max_s";
  if (c.dim_after_s < 60 || c.dim_after_s > 1800 || c.dim_after_s % 60) return "ui.dim_after_s";
  if (c.standby_after_s < 300 || c.standby_after_s > 3600 || c.standby_after_s % 300) return "ui.standby_after_s";
  if (c.standby_after_s < c.dim_after_s) return "ui.standby_after_s";
  return nullptr;
}

bool persist(const Config& config) {
  nvs_handle_t handle;
  if (nvs_open(kNamespace, NVS_READWRITE, &handle) != ESP_OK) return false;
  StoredConfig a{}, b{};
  bool a_valid = read_slot(handle, "a", &a);
  bool b_valid = read_slot(handle, "b", &b);
  const char* key = (!a_valid || (b_valid && b.config.revision > a.config.revision)) ? "a" : "b";
  StoredConfig out{kMagic, config, 0};
  out.checksum = checksum(out);
  esp_err_t err = nvs_set_blob(handle, key, &out, sizeof(out));
  if (err == ESP_OK) err = nvs_commit(handle);
  nvs_close(handle);
  return err == ESP_OK;
}
}  // namespace

void config_init() {
  nvs_handle_t handle;
  Config selected{};
  bool found = false;
  bool migrated = false;
  if (nvs_open(kNamespace, NVS_READWRITE, &handle) == ESP_OK) {
    StoredConfig a{}, b{};
    bool a_valid = read_slot(handle, "a", &a);
    bool b_valid = read_slot(handle, "b", &b);
    if (a_valid || b_valid) {
      selected = (!b_valid || (a_valid && a.config.revision >= b.config.revision)) ? a.config : b.config;
      found = true;
    } else {
      LegacyStoredConfig legacy_a{}, legacy_b{};
      bool legacy_a_valid = read_legacy_slot(handle, "a", &legacy_a);
      bool legacy_b_valid = read_legacy_slot(handle, "b", &legacy_b);
      if (legacy_a_valid || legacy_b_valid) {
        const LegacyStoredConfig& legacy =
            (!legacy_b_valid || (legacy_a_valid && legacy_a.config.revision >= legacy_b.config.revision))
                ? legacy_a : legacy_b;
        selected = migrate_legacy(legacy.config);
        found = true;
        migrated = true;
      }
    }
    nvs_close(handle);
  }
  if (migrated && !persist(selected)) ESP_LOGE(kTag, "cannot persist migrated configuration");
  if (!found) { selected = Config{}; selected.revision = 1; if (!persist(selected)) ESP_LOGE(kTag, "cannot persist defaults"); }
  portENTER_CRITICAL(&g_lock); g_config = selected; portEXIT_CRITICAL(&g_lock);
}

Config get_config() { Config copy; portENTER_CRITICAL(&g_lock); copy = g_config; portEXIT_CRITICAL(&g_lock); return copy; }

ConfigResult apply_config(const Config& candidate, uint32_t expected_revision) {
  const char* invalid = validate(candidate);
  if (candidate.version != kConfigSchemaVersion) return {ConfigStatus::kVersionMismatch, invalid};
  if (invalid != nullptr) return {ConfigStatus::kInvalidValue, invalid};
  Config next = candidate;
  portENTER_CRITICAL(&g_lock);
  if (g_config.revision != expected_revision) { portEXIT_CRITICAL(&g_lock); return {ConfigStatus::kStaleRevision, "revision"}; }
  next.revision = g_config.revision + 1;
  portEXIT_CRITICAL(&g_lock);
  if (!persist(next)) return {ConfigStatus::kStorageError, "nvs"};
  portENTER_CRITICAL(&g_lock); g_config = next; portEXIT_CRITICAL(&g_lock);
  return {};
}

ConfigResult reset_config() { Config defaults{}; return apply_config(defaults, get_config().revision); }
}  // namespace core
