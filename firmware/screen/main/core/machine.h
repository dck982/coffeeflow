// Logique pure du cycle café : aucune dépendance ESP-IDF, CAN ou LVGL.
#pragma once

#include <cstdint>

namespace core::machine {

enum class State : uint8_t { kIdle, kPreinfusion, kBrew, kRampdown, kFinished, kPurge };
enum class StopReason : uint8_t { kNone, kTargetTime, kTargetWeight, kManual, kScaleLost, kPurgeReleased, kPurgeTimeout };
enum class PreinfusionMode : uint8_t { kTime, kPressure };
enum class RampdownMode : uint8_t { kNone, kTime, kWeight, kPressureDrop };

struct Config {
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
};

struct Input { float weight_g; bool scale_present; float pressure_bar; };
struct Output { bool ssr; uint8_t dimmer; uint16_t ttl_ms; };

class Machine {
 public:
  bool start(uint64_t now_ms, const Config& config, const Input& input);
  bool stop(uint64_t now_ms);
  bool purge_press(uint64_t now_ms, const Config& config);
  bool purge_release(uint64_t now_ms);
  bool dismiss();
  Output tick(uint64_t now_ms, const Input& input);

  State state() const { return state_; }
  StopReason stop_reason() const { return stop_reason_; }
  bool active() const { return state_ == State::kPreinfusion || state_ == State::kBrew || state_ == State::kRampdown || state_ == State::kPurge; }
  bool weight_goal() const { return weight_goal_; }
  float starting_weight_g() const { return starting_weight_g_; }
  uint32_t elapsed_ms(uint64_t now_ms) const;
  uint32_t phase_elapsed_ms(uint64_t now_ms) const {
    return phase_started_ms_ == 0 || now_ms < phase_started_ms_ ? 0 : static_cast<uint32_t>(now_ms - phase_started_ms_);
  }

 private:
  void finish(StopReason reason);
  State state_ = State::kIdle;
  StopReason stop_reason_ = StopReason::kNone;
  Config config_{};
  uint64_t started_ms_ = 0;
  uint64_t phase_started_ms_ = 0;
  float starting_weight_g_ = 0.0f;
  float preinfusion_pressure_start_bar_ = 0.0f;
  bool weight_goal_ = false;
};

}  // namespace core::machine
