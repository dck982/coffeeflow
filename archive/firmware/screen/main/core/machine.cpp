#include "core/machine.h"

#include <algorithm>
#include <cmath>

#include "core/config.h"

namespace core::machine {
namespace {
constexpr uint16_t kLeaseMs = 500;
constexpr float kScaleBackwardsG = 5.0f;
constexpr uint64_t kFillingPressureGuardMs = 1000;
// La pression arrive toutes les 100 ms et `tick` tourne toutes les 50 ms :
// 150 ms au-dessus du seuil exigent deux mesures consécutives, ce qui écarte
// un paquet isolé.
constexpr uint64_t kFillingPressureConfirmMs = 150;
constexpr uint64_t kBrewPressureControlPeriodMs = 200;
// Transition hydraulique douce après la pré-infusion. La période des pas est
// adaptée à l'écart afin que la montée complète dure environ 2,5 s.
constexpr uint8_t kBrewRampStepPct = 5;
constexpr uint64_t kBrewRampDurationMs = 2500;
// Au-delà, une indisponibilité prolongée ne doit pas provoquer un rattrapage
// d'intégrale brutal quand la pression réapparaît.
constexpr uint64_t kBrewPressureMaxIntegrationPeriodMs = 400;
constexpr float kBrewPressureActivationMarginBar = 3.0f;
// Réglage initial tiré de la première capture réelle : autour de 9 bar, le
// point de fonctionnement est proche de 70 %, avec 300 à 400 ms de retard.
constexpr float kBrewPressureKpPctPerBar = 15.0f;
constexpr float kBrewPressureKiPctPerBarSecond = 3.0f;
}

bool Machine::start(uint64_t now_ms, const Config& config, const Input& input) {
  if (active()) return false;
  config_ = config;
  phase_started_ms_ = now_ms;
  started_ms_ = config_.brew_preheat_time_s > 0.0f ? 0 : now_ms;
  finished_ms_ = 0;
  starting_weight_g_ = input.weight_g;
  preinfusion_start_weight_g_ = input.weight_g;
  preinfusion_pressure_start_bar_ = input.pressure_bar;
  filling_pressure_since_ms_ = 0;
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
  state_ = config_.brew_preheat_time_s > 0.0f ? State::kThermalPreheat : State::kFilling;
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

void Machine::enter_brew(uint64_t now_ms, bool ramp_from_preinfusion) {
  state_ = State::kBrew;
  phase_started_ms_ = now_ms;
  const uint8_t target_pct = config_.brew_pump_pct < core::kMinimumBrewPumpPct
                                 ? core::kMinimumBrewPumpPct
                                 : config_.brew_pump_pct;
  // Sans charge, la pompe ne débite rien sous 40 %. Après une pause à 35 %,
  // partir de kMinimumBrewPumpPct évite de perdre le début de la rampe.
  brew_ramp_start_pct_ = ramp_from_preinfusion
                             ? std::clamp(config_.preinfusion_pump_pct, core::kMinimumBrewPumpPct, target_pct)
                             : target_pct;
  brew_pump_pct_ = brew_ramp_start_pct_;
  brew_ramp_active_ = brew_pump_pct_ < target_pct;
  if (brew_ramp_active_) {
    const uint8_t difference_pct = target_pct - brew_ramp_start_pct_;
    const uint8_t step_count = (difference_pct + kBrewRampStepPct - 1) / kBrewRampStepPct;
    brew_ramp_step_period_ms_ = static_cast<uint16_t>(kBrewRampDurationMs / step_count);
  } else {
    brew_ramp_step_period_ms_ = 0;
  }
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

  if (state_ == State::kThermalPreheat) {
    const uint64_t duration_ms = static_cast<uint64_t>(config_.brew_preheat_time_s * 1000.0f);
    if (now_ms - phase_started_ms_ < duration_ms) return {0, kLeaseMs};
    state_ = State::kFilling;
    started_ms_ = phase_started_ms_ = now_ms;
    starting_weight_g_ = input.weight_g;
    preinfusion_start_weight_g_ = input.weight_g;
    preinfusion_pressure_start_bar_ = input.pressure_bar;
  }

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
    if (delta >= stop_weight_g()) {
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

    // Pression absolue : headspace plein et galette mouillée. Le volume à
    // 1 bar ne suit pas la mouture (23,5 à 30,7 ml du 26/09 au 04/10). La
    // pression au repos, jusqu'à 1,44 bar, retombe pendant la garde.
    bool pressure_done = false;
    if (input.pressure_valid && input.pressure_bar >= config_.filling_pressure_bar) {
      if (filling_pressure_since_ms_ == 0) filling_pressure_since_ms_ = now_ms;
      pressure_done = now_ms - filling_pressure_since_ms_ >= kFillingPressureConfirmMs;
    } else {
      filling_pressure_since_ms_ = 0;
    }

    const bool time_done = elapsed >= static_cast<uint64_t>(config_.filling_time_s) * 1000;
    if (!time_done && !pressure_done) return {config_.filling_pump_pct, kLeaseMs};

    if (effective_preinfusion_mode_ == PreinfusionMode::kNone ||
        (effective_preinfusion_mode_ == PreinfusionMode::kTime &&
         config_.preinfusion_time_s == 0)) {
      enter_brew(now_ms, false);
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
    if (preinfusion_scale_armed_ &&
        input.weight_g - preinfusion_start_weight_g_ >= 0.1f) {
      done = true;
    }
    if (!done) return {config_.preinfusion_pump_pct, kLeaseMs};
    enter_brew(now_ms, true);
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
  if (state_ == State::kBrew && brew_ramp_active_) {
    const uint8_t target_pct = config_.brew_pump_pct < core::kMinimumBrewPumpPct
                                   ? core::kMinimumBrewPumpPct
                                   : config_.brew_pump_pct;
    const uint64_t completed_steps = (now_ms - phase_started_ms_) / brew_ramp_step_period_ms_;
    const uint64_t requested_pct = static_cast<uint64_t>(brew_ramp_start_pct_) +
                                   completed_steps * kBrewRampStepPct;
    brew_pump_pct_ = static_cast<uint8_t>(std::min<uint64_t>(requested_pct, target_pct));
    const float pressure_error_bar = config_.target_pressure_bar - input.pressure_bar;
    const float minimum_bumpless_command_pct = static_cast<float>(core::kMinimumBrewPumpPct) +
        kBrewPressureKpPctPerBar * pressure_error_bar;
    const bool pressure_control_ready = input.pressure_valid &&
        input.pressure_bar >= config_.target_pressure_bar - kBrewPressureActivationMarginBar &&
        static_cast<float>(brew_pump_pct_) >= minimum_bumpless_command_pct;
    if (pressure_control_ready) {
      // Une galette restrictive peut approcher la cible avant la fin de la
      // rampe. Laisser alors le PI prendre la main depuis la commande courante
      // évite de poursuivre mécaniquement jusqu'au plafond.
      brew_ramp_active_ = false;
    } else if (brew_pump_pct_ >= target_pct) {
      brew_ramp_active_ = false;
      brew_pressure_integral_pct_ = static_cast<float>(brew_pump_pct_);
      last_brew_pressure_control_ms_ = now_ms;
    }
  }
  if (state_ == State::kBrew && !brew_ramp_active_ &&
      now_ms - last_brew_pressure_control_ms_ >= kBrewPressureControlPeriodMs) {
    if (input.pressure_valid) {
      // Cette date n'avance que lorsqu'une commande PI est réellement
      // calculée. Une courte absence de mesure ne réduit donc pas Ki.
      const uint64_t control_elapsed_ms = std::min(
          now_ms - last_brew_pressure_control_ms_, kBrewPressureMaxIntegrationPeriodMs);
      last_brew_pressure_control_ms_ = now_ms;
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
                (static_cast<float>(control_elapsed_ms) / 1000.0f);
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
