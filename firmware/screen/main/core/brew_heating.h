#pragma once

#include <algorithm>
#include <array>
#include <cmath>
#include <cstddef>
#include <cstdint>
#include <limits>

namespace core::thermal {

// Loi de chauffe de l'infusion : précharge, puis remplissage, pré-infusion et
// infusion. Elle est volontairement séparée du régulateur de repos
// (`Controller` dans thermal_control.h) : régler la stabilisation à la
// consigne ne doit pas modifier la compensation de l'eau admise.
//
// La commande ne dépend pas de la NTC, qui voit l'eau froide environ 11 s
// avant la chaleur (docs/chauffe-chaudiere.md, simulation du 29 septembre).
class BrewHeating {
 public:
  static constexpr float kPreheatPowerPct = 90.0f;
  // La précharge est supprimée si la NTC dépasse déjà la cible de 0,5 °C.
  static constexpr float kPreheatAboveTargetBandC = 0.5f;
  static constexpr float kHoldPowerPct = 3.5f;
  // 4,18 J/(g·K) × 70,5 K (eau à 24,5 °C portée à 95 °C réels) / 12 J par %·s.
  static constexpr float kWaterHeatPctPerMlS = 4.18f * 70.5f / 12.0f;
  static constexpr float kPowerLimitPct = 90.0f;
  // Sans mesure de débit fraîche, reprendre l'ancien plancher d'infusion.
  static constexpr float kFlowFallbackPct = 45.0f;
  // La chaleur envoyée pendant l'infusion atteint la NTC environ 11 s après
  // l'eau froide : dans les 11 dernières secondes, elle ne sert plus qu'à
  // l'état final.
  static constexpr float kEndCutLeadS = 11.0f;
  // Sécurité seulement. Une coupure proche de la consigne recréerait le trou
  // de chauffe de 13 h 01 (28 septembre), provoqué par le pic de précharge.
  static constexpr float kSafetyAboveTargetC = 4.0f;

  void reset() { end_cut_ = false; }

  static float preheat_pct(float temperature_c, float target_c) {
    return temperature_c - target_c < kPreheatAboveTargetBandC ? kPreheatPowerPct : 0.0f;
  }

  // `remaining_s` : temps estimé avant l'arrêt de la pompe, NaN si inconnu
  // (arrêt manuel, balance sans débit établi, phase avant l'infusion).
  float flow_pct(float temperature_c, float target_c, float flow_ml_s, bool flow_valid,
                 float remaining_s) {
    // Figée une fois atteinte : une estimation qui remonte ne relance pas la
    // chauffe avant l'arrêt de la pompe.
    if (std::isfinite(remaining_s) && remaining_s <= kEndCutLeadS) end_cut_ = true;
    if (end_cut_ || temperature_c - target_c > kSafetyAboveTargetC) return 0.0f;
    if (!flow_valid || !std::isfinite(flow_ml_s)) return kFlowFallbackPct;
    return std::min(kPowerLimitPct,
                    kHoldPowerPct + std::max(0.0f, flow_ml_s) * kWaterHeatPctPerMlS);
  }

  bool end_cut() const { return end_cut_; }

 private:
  bool end_cut_ = false;
};

// Temps restant avant l'arrêt de la pompe. Au poids, le débit en tasse est
// pris sur 2 s ; tant qu'il n'est pas établi, l'estimation reste inconnue et
// la chauffe continue : fin plus chaude plutôt que creux plus profond.
class BrewEndEstimator {
 public:
  static constexpr uint64_t kRateWindowMs = 2000;
  static constexpr float kMinimumCupWeightG = 3.0f;
  static constexpr float kMinimumCupRateGPerS = 0.3f;

  void reset() { count_ = next_ = 0; }

  static float by_time(uint32_t elapsed_ms, float stop_time_s) {
    return stop_time_s - static_cast<float>(elapsed_ms) / 1000.0f;
  }

  // `cup_weight_g` et `stop_weight_g` sont comptés depuis le début du cycle.
  float by_weight(uint64_t now_ms, float cup_weight_g, float stop_weight_g) {
    constexpr float kUnknown = std::numeric_limits<float>::quiet_NaN();
    if (!std::isfinite(cup_weight_g)) return kUnknown;
    history_[next_] = {now_ms, cup_weight_g};
    next_ = (next_ + 1) % kCapacity;
    if (count_ < kCapacity) ++count_;
    if (cup_weight_g < kMinimumCupWeightG) return kUnknown;
    for (size_t i = 1; i < count_; ++i) {
      const Sample& old = history_[(next_ + kCapacity - 1 - i) % kCapacity];
      if (now_ms < old.at_ms || now_ms - old.at_ms < kRateWindowMs) continue;
      const float rate = (cup_weight_g - old.weight_g) * 1000.0f /
                         static_cast<float>(now_ms - old.at_ms);
      if (rate < kMinimumCupRateGPerS) return kUnknown;
      return std::max(0.0f, stop_weight_g - cup_weight_g) / rate;
    }
    return kUnknown;
  }

 private:
  struct Sample {
    uint64_t at_ms = 0;
    float weight_g = 0.0f;
  };
  static constexpr size_t kCapacity = 16;
  std::array<Sample, kCapacity> history_{};
  size_t count_ = 0;
  size_t next_ = 0;
};

}  // namespace core::thermal
