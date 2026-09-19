#include "core/machine.h"

namespace core::machine {
namespace {
constexpr uint16_t kLeaseMs = 500;
constexpr float kScaleBackwardsG = 5.0f;
constexpr uint64_t kFillingPressureGuardMs = 1000;
}

bool Machine::start(uint64_t now_ms, const Config& config, const Input& input) {
  if (active()) return false;
  config_ = config;
  started_ms_ = phase_started_ms_ = now_ms;
  finished_ms_ = 0;
  starting_weight_g_ = input.weight_g;
  preinfusion_start_weight_g_ = input.weight_g;
  preinfusion_pressure_start_bar_ = input.pressure_bar;
  weight_goal_ = input.scale_present;
  effective_preinfusion_mode_ = config_.preinfusion_mode;
  preinfusion_scale_armed_ = has_preinfusion_mode(effective_preinfusion_mode_, PreinfusionMode::kWeight) &&
                             input.scale_present;
  if (!preinfusion_scale_armed_) {
    effective_preinfusion_mode_ = static_cast<PreinfusionMode>(
        static_cast<uint8_t>(effective_preinfusion_mode_) &
        ~static_cast<uint8_t>(PreinfusionMode::kWeight));
  }
  stop_reason_ = StopReason::kNone;
  state_ = State::kFilling;
  return true;
}

bool Machine::stop(uint64_t now_ms) {
  if (!active() || state_ == State::kPurge) return false;
  finish(StopReason::kManual, now_ms);
  return true;
}

bool Machine::purge_press(uint64_t now_ms, const Config& config) {
  if (active()) return state_ == State::kPurge;
  config_ = config;
  started_ms_ = phase_started_ms_ = now_ms;
  finished_ms_ = 0;
  stop_reason_ = StopReason::kNone;
  state_ = State::kPurge;
  return true;
}

bool Machine::purge_release(uint64_t now_ms) {
  if (state_ != State::kPurge) return false;
  finish(StopReason::kPurgeReleased, now_ms);
  return true;
}

bool Machine::dismiss() {
  if (state_ != State::kFinished) return false;
  state_ = State::kIdle;
  stop_reason_ = StopReason::kNone;
  return true;
}

uint32_t Machine::elapsed_ms(uint64_t now_ms) const {
  if (state_ == State::kFinished) now_ms = finished_ms_;
  return started_ms_ == 0 || now_ms < started_ms_ ? 0 : static_cast<uint32_t>(now_ms - started_ms_);
}

void Machine::finish(StopReason reason, uint64_t now_ms) {
  finished_ms_ = now_ms;
  state_ = State::kFinished;
  stop_reason_ = reason;
}

Output Machine::tick(uint64_t now_ms, const Input& input) {
  if (state_ == State::kPurge) {
    if (now_ms - started_ms_ >= static_cast<uint64_t>(config_.purge_max_s) * 1000) finish(StopReason::kPurgeTimeout, now_ms);
    else return {config_.purge_pump_pct, kLeaseMs};
  }
  if (!active()) return {0, 0};

  if (weight_goal_) {
    const float delta = input.weight_g - starting_weight_g_;
    if (!input.scale_present || delta < -kScaleBackwardsG) {
      finish(StopReason::kScaleLost, now_ms);
      return {0, 0};
    }
    const float ramp_start = config_.target_weight_g - config_.rampdown_lead_weight_g;
    if (state_ == State::kBrew && config_.rampdown_mode == RampdownMode::kWeight && delta >= ramp_start) {
      state_ = State::kRampdown;
      phase_started_ms_ = now_ms;
    }
    if (delta >= (config_.rampdown_mode == RampdownMode::kWeight ? config_.target_weight_g : ramp_start)) {
      finish(StopReason::kTargetWeight, now_ms);
      return {0, 0};
    }
  } else if (now_ms - started_ms_ >= static_cast<uint64_t>(config_.target_time_s) * 1000) {
    finish(StopReason::kTargetTime, now_ms);
    return {0, 0};
  }

  if (state_ == State::kFilling) {
    const uint64_t elapsed = now_ms - started_ms_;
    if (elapsed < kFillingPressureGuardMs) return {config_.filling_pump_pct, kLeaseMs};

    const bool pressure_done = input.pressure_valid &&
                                input.pressure_bar > config_.filling_pressure_target_bar;

    const bool time_done = elapsed >= static_cast<uint64_t>(config_.filling_time_s) * 1000;
    if (!time_done && !pressure_done) return {config_.filling_pump_pct, kLeaseMs};

    state_ = (effective_preinfusion_mode_ == PreinfusionMode::kNone ||
              (effective_preinfusion_mode_ == PreinfusionMode::kTime &&
               config_.preinfusion_time_s == 0))
                 ? State::kBrew
                 : State::kPreinfusion;
    phase_started_ms_ = now_ms;
  }

  if (state_ == State::kPreinfusion) {
    bool done = false;
    if (has_preinfusion_mode(effective_preinfusion_mode_, PreinfusionMode::kTime)) {
      done |= now_ms - phase_started_ms_ >= static_cast<uint64_t>(config_.preinfusion_time_s) * 1000;
    }
    if (has_preinfusion_mode(effective_preinfusion_mode_, PreinfusionMode::kPressure)) {
      done |= input.pressure_bar >= config_.preinfusion_pressure_bar;
    }
    if (preinfusion_scale_armed_ &&
        input.weight_g - preinfusion_start_weight_g_ >= 0.1f) {
      done = true;
    }
    if (!done) return {config_.preinfusion_pump_pct, kLeaseMs};
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
  return {config_.brew_pump_pct, kLeaseMs};
}

}  // namespace core::machine
