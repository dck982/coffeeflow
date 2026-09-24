#pragma once

#include <algorithm>
#include <array>
#include <cmath>
#include <cstddef>
#include <cstdint>

namespace core::thermal {

// Réglage initial. Les gains et l'avance doivent être recalibrés sur
// une capture de chauffe/refroidissement de la machine réelle.
class Controller {
 public:
  struct Output { uint16_t power_permille; bool ready; };
  static constexpr float kPredictionHorizonS = 20.0f;
  // The measured command-to-temperature response is about 22 s on the
  // current machine. Recent commands therefore contribute heat that is not
  // visible in the temperature slope yet.
  static constexpr float kCommandTransitWindowS = 22.0f;
  static constexpr float kCommandHeatGainCPerPctSecond = 0.007f;
  static constexpr float kPowerFilterTimeConstantS = 5.0f;
  static constexpr float kSlopeFilterTimeConstantS = 8.0f;
  static constexpr float kHoldPowerPct = 3.5f;

  Output step(uint64_t now_ms, float temperature_c, float target_c,
              bool valid, bool enabled, bool brewing) {
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
    const float dt_s = std::min(static_cast<float>(now_ms - last_ms_) / 1000.0f, 1.0f);
    if (dt_s > 0) {
      const float measured_slope = (temperature_c - last_temperature_c_) / dt_s;
      const float alpha = dt_s / (kSlopeFilterTimeConstantS + dt_s);
      slope_c_per_s_ += alpha * (measured_slope - slope_c_per_s_);
      last_temperature_c_ = temperature_c;
      last_ms_ = now_ms;
    }

    const float error = target_c - temperature_c;
    if (std::fabs(error) <= 0.5f) {
      if (ready_since_ms_ == 0) ready_since_ms_ = now_ms;
    } else {
      ready_since_ms_ = 0;
    }
    const bool ready = ready_since_ms_ != 0 && now_ms - ready_since_ms_ >= 3000;

    // Anticiper la chaleur encore en route, mais aussi la baisse avant que
    // la mesure ne passe sous la cible. Le terme de maintien donne de petites
    // impulsions au SSR quand la température est stable à la consigne ; la
    // prédiction peut toujours ramener la puissance à zéro si elle monte.
    const float transit_heat_c = recent_command_heat_c(now_ms);
    const float predicted_c = temperature_c + slope_c_per_s_ * kPredictionHorizonS + transit_heat_c;
    const float predicted_error = target_c - predicted_c;
    if (std::fabs(error) < 8.0f && dt_s > 0) {
      // Ne pas accumuler l'erreur déjà expliquée par la chaleur en transit.
      integral_pct_ = std::clamp(integral_pct_ + 0.18f * predicted_error * dt_s, 0.0f, 35.0f);
    }
    if (error < -0.5f) integral_pct_ = 0;
    float power = kHoldPowerPct + 8.0f * predicted_error + integral_pct_;
    if (brewing && error > -0.5f) power += 15.0f;

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
    } else if (brewing && limited_power > filtered_power_pct_) {
      // Ne pas ralentir l'appoint prévu pendant l'infusion.
      filtered_power_pct_ = limited_power;
    } else if (dt_s > 0.0f) {
      const float alpha = dt_s / (kPowerFilterTimeConstantS + dt_s);
      filtered_power_pct_ += alpha * (limited_power - filtered_power_pct_);
    }
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

  float recent_command_heat_c(uint64_t now_ms) const {
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
    return std::min(1.5f, pct_seconds * kCommandHeatGainCPerPctSecond);
  }

  uint64_t last_ms_ = 0;
  uint64_t ready_since_ms_ = 0;
  float last_temperature_c_ = 0;
  float slope_c_per_s_ = 0;
  float integral_pct_ = 0;
  float target_c_ = 0;
  float filtered_power_pct_ = 0;
  bool has_filtered_power_ = false;
  std::array<CommandSample, kCommandHistoryCapacity> command_history_{};
  size_t command_history_count_ = 0;
  size_t command_history_next_ = 0;
};

}  // namespace core::thermal
