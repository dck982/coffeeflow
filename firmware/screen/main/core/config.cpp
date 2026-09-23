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

// Exact v5 layout, retained so the new thermal settings do not reset NVS.
struct ConfigV5 {
  uint16_t version;
  uint32_t revision;
  float target_weight_g;
  uint16_t target_time_s;
  float target_pressure_bar;
  uint16_t filling_time_s;
  float filling_pressure_target_bar;
  uint8_t filling_pump_pct;
  PreinfusionMode preinfusion_mode;
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
struct StoredConfigV5 { uint32_t magic; ConfigV5 config; uint32_t checksum; };

// Version 4 did not yet persist the infusion pressure target displayed on the
// first settings page. Keep its exact layout for migration.
struct ConfigV4 {
  uint16_t version;
  uint32_t revision;
  float target_weight_g;
  uint16_t target_time_s;
  uint16_t filling_time_s;
  float filling_pressure_target_bar;
  uint8_t filling_pump_pct;
  PreinfusionMode preinfusion_mode;
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
struct StoredConfigV4 { uint32_t magic; ConfigV4 config; uint32_t checksum; };

struct ConfigV2 {
  uint16_t version;
  uint32_t revision;
  float target_weight_g;
  uint16_t target_time_s;
  PreinfusionMode preinfusion_mode;
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
struct StoredConfigV2 { uint32_t magic; ConfigV2 config; uint32_t checksum; };

// Version 3 used a pressure delta relative to the first valid sample after
// the one-second filling guard. Keep its layout so existing NVS blobs can be
// migrated to the new absolute target semantics.
struct ConfigV3 {
  uint16_t version;
  uint32_t revision;
  float target_weight_g;
  uint16_t target_time_s;
  uint16_t filling_time_s;
  float filling_pressure_delta_bar;
  uint8_t filling_pump_pct;
  PreinfusionMode preinfusion_mode;
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
struct StoredConfigV3 { uint32_t magic; ConfigV3 config; uint32_t checksum; };

enum class PreinfusionModeV1 : uint8_t { kTime, kPressure };
struct ConfigV1 {
  uint16_t version;
  uint32_t revision;
  float target_weight_g;
  uint16_t target_time_s;
  PreinfusionModeV1 preinfusion_mode;
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
struct StoredConfigV1 { uint32_t magic; ConfigV1 config; uint32_t checksum; };
static_assert(sizeof(StoredConfigV1) == sizeof(StoredConfigV2), "v1 and v2 layouts must match");

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

uint32_t checksum_v2(const StoredConfigV2& stored) {
  return checksum_bytes(&stored, offsetof(StoredConfigV2, checksum));
}

uint32_t checksum_v5(const StoredConfigV5& stored) {
  return checksum_bytes(&stored, offsetof(StoredConfigV5, checksum));
}

uint32_t checksum_v4(const StoredConfigV4& stored) {
  return checksum_bytes(&stored, offsetof(StoredConfigV4, checksum));
}

uint32_t checksum_v3(const StoredConfigV3& stored) {
  return checksum_bytes(&stored, offsetof(StoredConfigV3, checksum));
}

uint32_t checksum_v1(const StoredConfigV1& stored) {
  return checksum_bytes(&stored, offsetof(StoredConfigV1, checksum));
}

bool read_slot(nvs_handle_t handle, const char* key, StoredConfig* out) {
  size_t size = sizeof(*out);
  if (nvs_get_blob(handle, key, out, &size) != ESP_OK || size != sizeof(*out)) return false;
  return out->magic == kMagic && out->config.version == kConfigSchemaVersion && out->checksum == checksum(*out);
}

bool read_v2_slot(nvs_handle_t handle, const char* key, StoredConfigV2* out) {
  size_t size = sizeof(*out);
  if (nvs_get_blob(handle, key, out, &size) != ESP_OK || size != sizeof(*out)) return false;
  return out->magic == kMagic && out->config.version == 2 && out->checksum == checksum_v2(*out);
}

bool read_v5_slot(nvs_handle_t handle, const char* key, StoredConfigV5* out) {
  size_t size = sizeof(*out);
  if (nvs_get_blob(handle, key, out, &size) != ESP_OK || size != sizeof(*out)) return false;
  return out->magic == kMagic && out->config.version == 5 && out->checksum == checksum_v5(*out);
}

bool read_v4_slot(nvs_handle_t handle, const char* key, StoredConfigV4* out) {
  size_t size = sizeof(*out);
  if (nvs_get_blob(handle, key, out, &size) != ESP_OK || size != sizeof(*out)) return false;
  return out->magic == kMagic && out->config.version == 4 && out->checksum == checksum_v4(*out);
}

bool read_v3_slot(nvs_handle_t handle, const char* key, StoredConfigV3* out) {
  size_t size = sizeof(*out);
  if (nvs_get_blob(handle, key, out, &size) != ESP_OK || size != sizeof(*out)) return false;
  return out->magic == kMagic && out->config.version == 3 && out->checksum == checksum_v3(*out);
}

bool read_v1_slot(nvs_handle_t handle, const char* key, StoredConfigV1* out) {
  size_t size = sizeof(*out);
  if (nvs_get_blob(handle, key, out, &size) != ESP_OK || size != sizeof(*out)) return false;
  return out->magic == kMagic && out->config.version == 1 && out->checksum == checksum_v1(*out);
}

Config migrate_v2(const ConfigV2& legacy) {
  Config migrated{};
  migrated.version = kConfigSchemaVersion;
  migrated.revision = legacy.revision;
  migrated.target_weight_g = legacy.target_weight_g;
  migrated.target_time_s = legacy.target_time_s;
  migrated.preinfusion_mode = legacy.preinfusion_mode;
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

Config migrate_v4(const ConfigV4& legacy) {
  Config migrated{};
  migrated.version = kConfigSchemaVersion;
  migrated.revision = legacy.revision;
  migrated.target_weight_g = legacy.target_weight_g;
  migrated.target_time_s = legacy.target_time_s;
  migrated.target_pressure_bar = Config{}.target_pressure_bar;
  migrated.filling_time_s = legacy.filling_time_s;
  migrated.filling_pressure_target_bar = legacy.filling_pressure_target_bar;
  migrated.filling_pump_pct = legacy.filling_pump_pct;
  migrated.preinfusion_mode = legacy.preinfusion_mode;
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

Config migrate_v5(const ConfigV5& legacy) {
  Config migrated{};
  migrated.revision = legacy.revision;
  migrated.target_weight_g = legacy.target_weight_g;
  migrated.target_time_s = legacy.target_time_s;
  migrated.target_pressure_bar = legacy.target_pressure_bar;
  migrated.filling_time_s = legacy.filling_time_s;
  migrated.filling_pressure_target_bar = legacy.filling_pressure_target_bar;
  migrated.filling_pump_pct = legacy.filling_pump_pct;
  migrated.preinfusion_mode = legacy.preinfusion_mode;
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

Config migrate_v3(const ConfigV3& legacy) {
  Config migrated{};
  migrated.version = kConfigSchemaVersion;
  migrated.revision = legacy.revision;
  migrated.target_weight_g = legacy.target_weight_g;
  migrated.target_time_s = legacy.target_time_s;
  migrated.filling_time_s = legacy.filling_time_s;
  // There is no lossless conversion from the old delta without the pressure
  // reference captured during a cycle; use the new requested default.
  migrated.filling_pressure_target_bar = Config{}.filling_pressure_target_bar;
  migrated.filling_pump_pct = legacy.filling_pump_pct;
  migrated.preinfusion_mode = legacy.preinfusion_mode;
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

Config migrate_v1(const ConfigV1& legacy) {
  ConfigV2 v2{};
  v2.version = 2;
  v2.revision = legacy.revision;
  v2.target_weight_g = legacy.target_weight_g;
  v2.target_time_s = legacy.target_time_s;
  v2.preinfusion_mode = legacy.preinfusion_mode == PreinfusionModeV1::kPressure
                            ? PreinfusionMode::kPressure
                            : PreinfusionMode::kTime;
  v2.preinfusion_time_s = legacy.preinfusion_time_s;
  v2.preinfusion_pressure_bar = legacy.preinfusion_pressure_bar;
  v2.preinfusion_pump_pct = legacy.preinfusion_pump_pct;
  v2.rampdown_mode = legacy.rampdown_mode;
  v2.rampdown_lead_time_s = legacy.rampdown_lead_time_s;
  v2.rampdown_lead_weight_g = legacy.rampdown_lead_weight_g;
  v2.rampdown_pressure_drop_bar = legacy.rampdown_pressure_drop_bar;
  v2.brew_pump_pct = legacy.brew_pump_pct;
  v2.purge_pump_pct = legacy.purge_pump_pct;
  v2.purge_max_s = legacy.purge_max_s;
  v2.dim_after_s = legacy.dim_after_s;
  v2.standby_after_s = legacy.standby_after_s;
  return migrate_v2(v2);
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
  if (!valid_step(c.target_pressure_bar, 6, 12, .1f)) return "brew.target_pressure_bar";
  if (!valid_step(c.brew_temperature_c, 80, 100, .5f)) return "heating.brew_temperature_c";
  if (c.filling_time_s < 1 || c.filling_time_s > 10) return "filling.time_s";
  if (!valid_step(c.filling_pressure_target_bar, .1f, 1.0f, .1f)) return "filling.pressure_target_bar";
  if (c.filling_pump_pct < 20 || c.filling_pump_pct > 100 || c.filling_pump_pct % 5) return "filling.pump_pct";
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
  if (c.brew_pump_pct < kMinimumBrewPumpPct || c.brew_pump_pct > 100 || c.brew_pump_pct % 5) return "brew.pump_pct";
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
      StoredConfigV5 v5_a{}, v5_b{};
      bool v5_a_valid = read_v5_slot(handle, "a", &v5_a);
      bool v5_b_valid = read_v5_slot(handle, "b", &v5_b);
      if (v5_a_valid || v5_b_valid) {
        const StoredConfigV5& legacy =
            (!v5_b_valid || (v5_a_valid && v5_a.config.revision >= v5_b.config.revision)) ? v5_a : v5_b;
        selected = migrate_v5(legacy.config);
        found = true;
        migrated = true;
      } else {
      StoredConfigV4 v4_a{}, v4_b{};
      bool v4_a_valid = read_v4_slot(handle, "a", &v4_a);
      bool v4_b_valid = read_v4_slot(handle, "b", &v4_b);
      if (v4_a_valid || v4_b_valid) {
        const StoredConfigV4& legacy =
            (!v4_b_valid || (v4_a_valid && v4_a.config.revision >= v4_b.config.revision)) ? v4_a : v4_b;
        selected = migrate_v4(legacy.config);
        found = true;
        migrated = true;
      } else {
        StoredConfigV3 v3_a{}, v3_b{};
        bool v3_a_valid = read_v3_slot(handle, "a", &v3_a);
        bool v3_b_valid = read_v3_slot(handle, "b", &v3_b);
        if (v3_a_valid || v3_b_valid) {
          const StoredConfigV3& legacy =
              (!v3_b_valid || (v3_a_valid && v3_a.config.revision >= v3_b.config.revision)) ? v3_a : v3_b;
          selected = migrate_v3(legacy.config);
          found = true;
          migrated = true;
        } else {
          StoredConfigV2 v2_a{}, v2_b{};
          bool v2_a_valid = read_v2_slot(handle, "a", &v2_a);
          bool v2_b_valid = read_v2_slot(handle, "b", &v2_b);
          if (v2_a_valid || v2_b_valid) {
            const StoredConfigV2& legacy =
                (!v2_b_valid || (v2_a_valid && v2_a.config.revision >= v2_b.config.revision)) ? v2_a : v2_b;
            selected = migrate_v2(legacy.config);
            found = true;
            migrated = true;
          } else {
            StoredConfigV1 v1_a{}, v1_b{};
            bool v1_a_valid = read_v1_slot(handle, "a", &v1_a);
            bool v1_b_valid = read_v1_slot(handle, "b", &v1_b);
            if (v1_a_valid || v1_b_valid) {
              const StoredConfigV1& legacy =
                  (!v1_b_valid || (v1_a_valid && v1_a.config.revision >= v1_b.config.revision)) ? v1_a : v1_b;
              selected = migrate_v1(legacy.config);
              found = true;
              migrated = true;
            }
          }
        }
      }
      }
    }
    nvs_close(handle);
  }
  // Une machine mise à jour ne doit pas chauffer avant un choix explicite.
  // Une installation sans ancienne configuration conserve le défaut true.
  if (migrated) selected.heating_enabled = false;
  const bool normalized_brew_pump = selected.brew_pump_pct < kMinimumBrewPumpPct;
  if (normalized_brew_pump) selected.brew_pump_pct = kMinimumBrewPumpPct;
  if ((migrated || (found && normalized_brew_pump)) && !persist(selected)) {
    ESP_LOGE(kTag, "cannot persist migrated or normalized configuration");
  }
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
