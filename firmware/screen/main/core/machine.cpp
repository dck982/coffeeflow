#include "core/machine.h"

namespace core::machine {
namespace {
constexpr uint16_t kLeaseMs = 500;
constexpr float kScaleBackwardsG = 5.0f;
}

bool Machine::start(uint64_t now_ms, const Config& config, const Input& input) {
  if (active()) return false;
  config_ = config;
  started_ms_ = phase_started_ms_ = now_ms;
  starting_weight_g_ = input.weight_g;
  preinfusion_pressure_start_bar_ = input.pressure_bar;
  weight_goal_ = input.scale_present;
  stop_reason_ = StopReason::kNone;
  state_ = (config_.preinfusion_mode == PreinfusionMode::kTime && config_.preinfusion_time_s == 0)
               ? State::kBrew : State::kPreinfusion;
  return true;
}

bool Machine::stop(uint64_t) {
  if (!active() || state_ == State::kPurge) return false;
  finish(StopReason::kManual);
  return true;
}

bool Machine::purge_press(uint64_t now_ms, const Config& config) {
  if (active()) return state_ == State::kPurge;
  config_ = config;
  started_ms_ = phase_started_ms_ = now_ms;
  stop_reason_ = StopReason::kNone;
  state_ = State::kPurge;
  return true;
}

bool Machine::purge_release(uint64_t) {
  if (state_ != State::kPurge) return false;
  finish(StopReason::kPurgeReleased);
  return true;
}

bool Machine::dismiss() {
  if (state_ != State::kFinished) return false;
  state_ = State::kIdle;
  stop_reason_ = StopReason::kNone;
  return true;
}

uint32_t Machine::elapsed_ms(uint64_t now_ms) const {
  return started_ms_ == 0 || now_ms < started_ms_ ? 0 : static_cast<uint32_t>(now_ms - started_ms_);
}

void Machine::finish(StopReason reason) { state_ = State::kFinished; stop_reason_ = reason; }

Output Machine::tick(uint64_t now_ms, const Input& input) {
  if (state_ == State::kPurge) {
    if (now_ms - started_ms_ >= static_cast<uint64_t>(config_.purge_max_s) * 1000) finish(StopReason::kPurgeTimeout);
    else return {true, config_.purge_pump_pct, kLeaseMs};
  }
  if (!active()) return {false, 0, 0};

  if (weight_goal_) {
    const float delta = input.weight_g - starting_weight_g_;
    if (!input.scale_present || delta < -kScaleBackwardsG) {
      finish(StopReason::kScaleLost);
      return {false, 0, 0};
    }
    const float ramp_start = config_.target_weight_g - config_.rampdown_lead_weight_g;
    if (state_ == State::kBrew && config_.rampdown_mode == RampdownMode::kWeight && delta >= ramp_start) {
      state_ = State::kRampdown;
      phase_started_ms_ = now_ms;
    }
    if (delta >= (config_.rampdown_mode == RampdownMode::kWeight ? config_.target_weight_g : ramp_start)) {
      finish(StopReason::kTargetWeight);
      return {false, 0, 0};
    }
  } else if (now_ms - started_ms_ >= static_cast<uint64_t>(config_.target_time_s) * 1000) {
    finish(StopReason::kTargetTime);
    return {false, 0, 0};
  }

  if (state_ == State::kPreinfusion) {
    bool done = config_.preinfusion_mode == PreinfusionMode::kTime
                    ? now_ms - phase_started_ms_ >= static_cast<uint64_t>(config_.preinfusion_time_s) * 1000
                    : input.pressure_bar >= config_.preinfusion_pressure_bar;
    if (!done) return {true, config_.preinfusion_pump_pct, kLeaseMs};
    state_ = State::kBrew;
    phase_started_ms_ = now_ms;
  }
  if (state_ == State::kBrew) {
    bool ramp = config_.rampdown_mode == RampdownMode::kTime &&
                now_ms - started_ms_ >= static_cast<uint64_t>((config_.target_time_s - config_.rampdown_lead_time_s) * 1000.0f);
    if (ramp || (config_.rampdown_mode == RampdownMode::kPressureDrop &&
                 preinfusion_pressure_start_bar_ - input.pressure_bar >= config_.rampdown_pressure_drop_bar)) {
      state_ = State::kRampdown;
      phase_started_ms_ = now_ms;
    }
  }
  // La calibration déterminera la vraie pente; le premier cycle conserve le
  // niveau nominal, mais expose explicitement la phase pour l'UI et les traces.
  return {true, config_.brew_pump_pct, kLeaseMs};
}

}  // namespace core::machine
