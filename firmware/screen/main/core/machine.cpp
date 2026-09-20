#include "core/machine.h"

#include <algorithm>
#include <cmath>

#include "core/config.h"

namespace core::machine {
namespace {
constexpr uint16_t kLeaseMs = 500;
constexpr float kScaleBackwardsG = 5.0f;
constexpr uint64_t kFillingPressureGuardMs = 1000;
constexpr uint64_t kBrewPressureControlPeriodMs = 200;
constexpr float kBrewPressureActivationMarginBar = 2.0f;
// Réglage initial tiré de la première capture réelle : autour de 9 bar, le
// point de fonctionnement est proche de 70 %, avec 300 à 400 ms de retard.
constexpr float kBrewPressureKpPctPerBar = 15.0f;
constexpr float kBrewPressureKiPctPerBarSecond = 3.0f;
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

void Machine::enter_brew(uint64_t now_ms) {
  state_ = State::kBrew;
  phase_started_ms_ = now_ms;
  brew_pump_pct_ = config_.brew_pump_pct < core::kMinimumBrewPumpPct
                       ? core::kMinimumBrewPumpPct
                       : config_.brew_pump_pct;
  brew_pressure_control_active_ = false;
  brew_pressure_integral_pct_ = static_cast<float>(brew_pump_pct_);
  last_brew_pressure_control_ms_ = now_ms;
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

    if (effective_preinfusion_mode_ == PreinfusionMode::kNone ||
        (effective_preinfusion_mode_ == PreinfusionMode::kTime &&
         config_.preinfusion_time_s == 0)) {
      enter_brew(now_ms);
    } else {
      state_ = State::kPreinfusion;
      phase_started_ms_ = now_ms;
    }
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
    enter_brew(now_ms);
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
  if (state_ == State::kBrew &&
      now_ms - last_brew_pressure_control_ms_ >= kBrewPressureControlPeriodMs) {
    last_brew_pressure_control_ms_ = now_ms;
    if (input.pressure_valid) {
      const float error_bar = config_.target_pressure_bar - input.pressure_bar;
      const float pump_floor = static_cast<float>(core::kMinimumBrewPumpPct);
      const float pump_ceiling = static_cast<float>(config_.brew_pump_pct < core::kMinimumBrewPumpPct
                                                        ? core::kMinimumBrewPumpPct
                                                        : config_.brew_pump_pct);

      if (!brew_pressure_control_active_ &&
          input.pressure_bar >= config_.target_pressure_bar - kBrewPressureActivationMarginBar) {
        // Initialiser I pour que P + I reproduise la commande courante : le
        // passage en boucle fermée ne crée ainsi aucun saut de puissance.
        brew_pressure_control_active_ = true;
        brew_pressure_integral_pct_ = std::clamp(
            static_cast<float>(brew_pump_pct_) - kBrewPressureKpPctPerBar * error_bar,
            pump_floor, pump_ceiling);
      } else if (brew_pressure_control_active_) {
        const float proportional_pct = kBrewPressureKpPctPerBar * error_bar;
        const float candidate_integral_pct = brew_pressure_integral_pct_ +
            kBrewPressureKiPctPerBarSecond * error_bar *
                (static_cast<float>(kBrewPressureControlPeriodMs) / 1000.0f);
        const float candidate_output_pct = proportional_pct + candidate_integral_pct;

        // Anti-windup conditionnel : ne pas pousser davantage l'intégrale
        // lorsqu'une saturation empêche déjà la commande demandée.
        const bool winds_up_high = candidate_output_pct > pump_ceiling && error_bar > 0.0f;
        const bool winds_up_low = candidate_output_pct < pump_floor && error_bar < 0.0f;
        if (!winds_up_high && !winds_up_low) {
          brew_pressure_integral_pct_ = std::clamp(candidate_integral_pct,
                                                   pump_floor, pump_ceiling);
        }
      }

      if (brew_pressure_control_active_) {
        const float output_pct = std::clamp(
            kBrewPressureKpPctPerBar * error_bar + brew_pressure_integral_pct_,
            pump_floor, pump_ceiling);
        brew_pump_pct_ = static_cast<uint8_t>(std::lround(output_pct));
      }
    }
  }
  return {brew_pump_pct_, kLeaseMs};
}

}  // namespace core::machine
