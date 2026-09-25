// Configuration utilisateur persistante. Les calibrations sont compilées dans
// calibration_machine.h : elles ne passent ni par NVS ni par /config.
#pragma once

#include <cstdint>

namespace core {

inline constexpr uint16_t kConfigSchemaVersion = 6;
inline constexpr float kMinimumBrewTemperatureC = 50.0f;
inline constexpr float kMaximumBrewTemperatureC = 100.0f;
inline constexpr float kBrewTemperatureToleranceC = 1.0f;
// Sous cette puissance, la pompe ne maintient plus une pression d'infusion
// sûre. Cette borne est distincte des puissances de remplissage et de purge.
inline constexpr uint8_t kMinimumBrewPumpPct = 50;

enum class PreinfusionMode : uint8_t {
  kNone = 0,
  kTime = 1 << 0,
  kPressure = 1 << 1,
  kWeight = 1 << 2,
};

constexpr PreinfusionMode operator|(PreinfusionMode lhs, PreinfusionMode rhs) {
  return static_cast<PreinfusionMode>(static_cast<uint8_t>(lhs) | static_cast<uint8_t>(rhs));
}

constexpr PreinfusionMode operator&(PreinfusionMode lhs, PreinfusionMode rhs) {
  return static_cast<PreinfusionMode>(static_cast<uint8_t>(lhs) & static_cast<uint8_t>(rhs));
}

constexpr bool has_preinfusion_mode(PreinfusionMode modes, PreinfusionMode mode) {
  return (modes & mode) != PreinfusionMode::kNone;
}
enum class RampdownMode : uint8_t { kNone, kTime, kWeight, kPressureDrop };

struct Config {
  uint16_t version = kConfigSchemaVersion;
  uint32_t revision = 0;
  float target_weight_g = 36.0f;
  uint16_t target_time_s = 28;
  float target_pressure_bar = 9.0f;
  float brew_temperature_c = 90.0f;
  bool heating_enabled = true;
  uint16_t filling_time_s = 3;
  float filling_pressure_target_bar = 0.3f;
  uint8_t filling_pump_pct = 100;
  PreinfusionMode preinfusion_mode = PreinfusionMode::kTime;
  uint16_t preinfusion_time_s = 4;
  float preinfusion_pressure_bar = 1.5f;
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

enum class ConfigStatus : uint8_t { kOk, kVersionMismatch, kStaleRevision, kInvalidValue, kStorageError, kBusy };
struct ConfigResult { ConfigStatus status = ConfigStatus::kOk; const char* field = nullptr; };

// Initialise/charge les deux emplacements transactionnels NVS. À appeler
// après nvs_flash_init(), avant tout consommateur de configuration.
void config_init();
Config get_config();
ConfigResult apply_config(const Config& candidate, uint32_t expected_revision);
ConfigResult reset_config();

}  // namespace core
