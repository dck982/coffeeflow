// Configuration utilisateur persistante. Les calibrations sont compilées dans
// calibration_machine.h : elles ne passent ni par NVS ni par /config.
#pragma once

#include <cstdint>

namespace core {

inline constexpr uint16_t kConfigSchemaVersion = 1;

enum class PreinfusionMode : uint8_t { kTime, kPressure };
enum class RampdownMode : uint8_t { kNone, kTime, kWeight, kPressureDrop };

struct Config {
  uint16_t version = kConfigSchemaVersion;
  uint32_t revision = 0;
  float target_weight_g = 36.0f;
  uint16_t target_time_s = 28;
  PreinfusionMode preinfusion_mode = PreinfusionMode::kTime;
  uint16_t preinfusion_time_s = 6;
  float preinfusion_pressure_bar = 4.0f;
  uint8_t preinfusion_pump_pct = 30;
  RampdownMode rampdown_mode = RampdownMode::kNone;
  float rampdown_lead_time_s = 3.0f;
  float rampdown_lead_weight_g = 4.0f;
  float rampdown_pressure_drop_bar = 1.0f;
  uint8_t brew_pump_pct = 100;
  uint8_t purge_pump_pct = 100;
  uint16_t purge_max_s = 20;
  uint16_t dim_after_s = 240;
  uint16_t standby_after_s = 1800;
};

// `profiles` et les courbes de calibration ne sont délibérément pas encodés
// dans le blob v1. Ils auront leur propre sous-format versionné lorsqu'ils
// existeront ; ne pas leur réserver des tableaux arbitraires aujourd'hui.
inline constexpr bool kProfilesSupported = false;

enum class ConfigStatus : uint8_t { kOk, kVersionMismatch, kStaleRevision, kInvalidValue, kStorageError };
struct ConfigResult { ConfigStatus status = ConfigStatus::kOk; const char* field = nullptr; };

// Initialise/charge les deux emplacements transactionnels NVS. À appeler
// après nvs_flash_init(), avant tout consommateur de configuration.
void config_init();
Config get_config();
ConfigResult apply_config(const Config& candidate, uint32_t expected_revision);
ConfigResult reset_config();

}  // namespace core
