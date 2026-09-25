#pragma once

#include <algorithm>
#include <array>
#include <cmath>
#include <cstddef>
#include <cstdint>

#include "core/config.h"

namespace core::thermal {

// Réglage initial. Les gains et l'avance doivent être recalibrés sur
// une capture de chauffe/refroidissement de la machine réelle.
class Controller {
 public:
  enum class Mode { kIdle, kBrew, kPurge };
  struct Output { uint16_t power_permille; bool ready; };
  static constexpr float kPredictionHorizonS = 20.0f;
  static constexpr float kRecoveryPredictionHorizonS = 10.0f;
  // The measured command-to-temperature response is about 22 s on the
  // current machine. Recent commands therefore contribute heat that is not
  // visible in the temperature slope yet.
  static constexpr float kCommandTransitWindowS = 22.0f;
  static constexpr float kCommandHeatGainCPerPctSecond = 0.007f;
  static constexpr float kPowerFilterTimeConstantS = 5.0f;
  static constexpr float kSlopeFilterTimeConstantS = 8.0f;
  static constexpr float kHoldPowerPct = 3.5f;
  static constexpr float kFlowFeedforwardPct = 18.0f;
  static constexpr float kFlowPowerLimitPct = 35.0f;
  static constexpr float kPurgeCompensationBandC = 5.0f;
  static constexpr float kRecoveryPowerLimitPct = 35.0f;
  static constexpr uint64_t kRecoveryDurationMs = 30000;

  Output step(uint64_t now_ms, float temperature_c, float target_c,
              bool valid, bool enabled, Mode mode) {
    if (!valid || !enabled || !std::isfinite(temperature_c) ||
        !std::isfinite(target_c) || temperature_c > 105.0f) {
      reset();
      return {0, false};
    }
    if (last_ms_ != 0 && target_c != target_c_) reset();
    target_c_ = target_c;
    if (last_ms_ == 0 || now_ms <= last_ms_ || now_ms - last_ms_ > 2000) {
      slope_c_per_s_ = 0;
      integral_pct_ = 0;
      ready_since_ms_ = 0;
      clear_command_history();
      last_temperature_c_ = temperature_c;
      last_ms_ = now_ms;
    }
    const bool flowing = mode != Mode::kIdle;
    if (flowing != was_flowing_) {
      // The temperature slope during water exchange does not predict the
      // boiler's slope once the flow stops (or starts).
      slope_c_per_s_ = 0;
      integral_pct_ = 0;
      last_temperature_c_ = temperature_c;
      last_ms_ = now_ms;
      if (!flowing) recovery_until_ms_ = now_ms + kRecoveryDurationMs;
      else uncompensated_purge_ = mode == Mode::kPurge &&
          std::fabs(temperature_c - target_c) >= kPurgeCompensationBandC;
      was_flowing_ = flowing;
    }
    const bool recovering = !flowing && now_ms < recovery_until_ms_;
    const float dt_s = std::min(static_cast<float>(now_ms - last_ms_) / 1000.0f, 1.0f);
    if (dt_s > 0) {
      const float measured_slope = (temperature_c - last_temperature_c_) / dt_s;
      const float alpha = dt_s / (kSlopeFilterTimeConstantS + dt_s);
      slope_c_per_s_ += alpha * (measured_slope - slope_c_per_s_);
      last_temperature_c_ = temperature_c;
      last_ms_ = now_ms;
    }

    const float error = target_c - temperature_c;
    if (std::fabs(error) <= kBrewTemperatureToleranceC) {
      if (ready_since_ms_ == 0) ready_since_ms_ = now_ms;
    } else {
      ready_since_ms_ = 0;
    }
    const bool ready = ready_since_ms_ != 0 && now_ms - ready_since_ms_ >= 3000;

    // Latch the decision at purge start: a purge far from setpoint may be
    // intended to cool the boiler, even if the NTC crosses the setpoint.
    if (mode == Mode::kPurge && uncompensated_purge_) {
      filtered_power_pct_ = 0.0f;
      integral_pct_ = 0.0f;
      record_command(now_ms, 0.0f);
      return {0, ready};
    }

    // Anticiper la chaleur encore en route, mais aussi la baisse avant que
    // la mesure ne passe sous la cible. Le terme de maintien donne de petites
    // impulsions au SSR quand la température est stable à la consigne ; la
    // prédiction peut toujours ramener la puissance à zéro si elle monte.
    const float transit_heat_c = recent_command_heat_c(now_ms, recovering ? 6.0f : 1.5f);
    const float predictive_slope = (flowing || recovering)
        ? std::max(0.0f, slope_c_per_s_) : slope_c_per_s_;
    const float horizon_s = recovering ? kRecoveryPredictionHorizonS : kPredictionHorizonS;
    // During recovery the rising slope already contains some of the heat
    // from recent commands. Adding both terms would count it twice.
    const float predicted_rise_c = recovering
        ? std::max(predictive_slope * horizon_s, transit_heat_c)
        : predictive_slope * horizon_s + transit_heat_c;
    const float predicted_c = temperature_c + predicted_rise_c;
    const float predicted_error = target_c - predicted_c;
    if (!flowing && !recovering && std::fabs(error) < 8.0f && dt_s > 0) {
      // Ne pas accumuler l'erreur déjà expliquée par la chaleur en transit.
      integral_pct_ = std::clamp(integral_pct_ + 0.18f * predicted_error * dt_s, 0.0f, 35.0f);
    }
    if (error < -0.5f) integral_pct_ = 0;
    float power = kHoldPowerPct + 8.0f * predicted_error + integral_pct_;
    if (flowing) {
      if (error > -0.5f) power = std::max(power, kFlowFeedforwardPct);
      power = std::min(power, kFlowPowerLimitPct);
    } else if (recovering) {
      power = std::min(power, kRecoveryPowerLimitPct);
    }

    // Un dépassement prédit coupe immédiatement. Lisser les variations
    // positives restantes évite de poursuivre le bruit de la dérivée.
    // La commande effectivement envoyée alimente l'estimation d'inertie.
    if (power <= 0.0f) {
      filtered_power_pct_ = 0.0f;
      record_command(now_ms, 0.0f);
      return {0, ready};
    }
    const float limited_power = std::clamp(power, 0.0f, 100.0f);
    if (!has_filtered_power_) {
      filtered_power_pct_ = limited_power;
      has_filtered_power_ = true;
    } else if (flowing && limited_power > filtered_power_pct_) {
      // Apply the modest flow feedforward without output-filter delay.
      filtered_power_pct_ = limited_power;
    } else if (dt_s > 0.0f) {
      const float alpha = dt_s / (kPowerFilterTimeConstantS + dt_s);
      filtered_power_pct_ += alpha * (limited_power - filtered_power_pct_);
    }
    if (flowing) filtered_power_pct_ = std::min(filtered_power_pct_, kFlowPowerLimitPct);
    if (recovering) filtered_power_pct_ = std::min(filtered_power_pct_, kRecoveryPowerLimitPct);
    const uint16_t power_permille = static_cast<uint16_t>(
        std::lround(std::clamp(filtered_power_pct_, 0.0f, 100.0f) * 10.0f));
    record_command(now_ms, power_permille / 10.0f);
    return {power_permille, ready};
  }

  void reset() {
    last_ms_ = 0;
    ready_since_ms_ = 0;
    integral_pct_ = 0;
    slope_c_per_s_ = 0;
    filtered_power_pct_ = 0;
    has_filtered_power_ = false;
    was_flowing_ = false;
    uncompensated_purge_ = false;
    recovery_until_ms_ = 0;
    clear_command_history();
  }

 private:
  struct CommandSample {
    uint64_t at_ms = 0;
    float power_pct = 0.0f;
  };

  static constexpr size_t kCommandHistoryCapacity = 96;

  void clear_command_history() {
    command_history_count_ = 0;
    command_history_next_ = 0;
  }

  void record_command(uint64_t now_ms, float power_pct) {
    command_history_[command_history_next_] = {now_ms, power_pct};
    command_history_next_ = (command_history_next_ + 1) % kCommandHistoryCapacity;
    if (command_history_count_ < kCommandHistoryCapacity) ++command_history_count_;
  }

  float recent_command_heat_c(uint64_t now_ms, float limit_c) const {
    if (command_history_count_ == 0) return 0.0f;
    const uint64_t window_ms = static_cast<uint64_t>(kCommandTransitWindowS * 1000.0f);
    const uint64_t cutoff_ms = now_ms > window_ms ? now_ms - window_ms : 0;
    float pct_seconds = 0.0f;
    uint64_t previous_ms = 0;
    float previous_power = 0.0f;
    bool have_previous = false;
    const size_t oldest = (command_history_next_ + kCommandHistoryCapacity -
                           command_history_count_) % kCommandHistoryCapacity;
    for (size_t i = 0; i < command_history_count_; ++i) {
      const CommandSample& sample = command_history_[(oldest + i) % kCommandHistoryCapacity];
      if (have_previous && sample.at_ms > previous_ms) {
        const uint64_t start_ms = std::max(previous_ms, cutoff_ms);
        const uint64_t end_ms = std::min(sample.at_ms, now_ms);
        if (end_ms > start_ms)
          pct_seconds += std::max(0.0f, previous_power - kHoldPowerPct) *
                         static_cast<float>(end_ms - start_ms) / 1000.0f;
      }
      previous_ms = sample.at_ms;
      previous_power = sample.power_pct;
      have_previous = true;
    }
    if (have_previous && now_ms > previous_ms) {
      const uint64_t start_ms = std::max(previous_ms, cutoff_ms);
      if (now_ms > start_ms)
        pct_seconds += std::max(0.0f, previous_power - kHoldPowerPct) *
                       static_cast<float>(now_ms - start_ms) / 1000.0f;
    }
    // Au-delà de 22 s, la réponse devient visible dans la pente mesurée.
    // La borne évite qu'une longue chauffe à 100 % domine la régulation fine.
    return std::min(limit_c, pct_seconds * kCommandHeatGainCPerPctSecond);
  }

  uint64_t last_ms_ = 0;
  uint64_t ready_since_ms_ = 0;
  float last_temperature_c_ = 0;
  float slope_c_per_s_ = 0;
  float integral_pct_ = 0;
  float target_c_ = 0;
  float filtered_power_pct_ = 0;
  bool has_filtered_power_ = false;
  bool was_flowing_ = false;
  bool uncompensated_purge_ = false;
  uint64_t recovery_until_ms_ = 0;
  std::array<CommandSample, kCommandHistoryCapacity> command_history_{};
  size_t command_history_count_ = 0;
  size_t command_history_next_ = 0;
};

}  // namespace core::thermal
