#pragma once

#include <algorithm>
#include <cmath>
#include <cstdint>

namespace core::thermal {

// Loi de chauffe de l'infusion : précharge, puis remplissage, pré-infusion et
// infusion. Elle est volontairement séparée du régulateur de repos
// (`Controller` dans thermal_control.h) : régler la stabilisation à la
// consigne ne doit pas modifier la compensation de l'eau admise.
//
// La commande ne dépend pas de la NTC, qui voit l'eau froide environ 11 s
// avant la chaleur (docs/chauffe-chaudiere.md, simulation du 29 septembre).
//
// La précharge est une avance sur l'appoint : dès l'entrée en infusion,
// l'appoint est retenu jusqu'à avoir rendu l'énergie de la précharge. Sans
// ce remboursement, l'eau du remplissage est chauffée deux fois, et l'excédent
// tombe en tasse d'autant plus que l'infusion est longue
// (docs/chauffe-infusion.md, remboursement de la précharge).
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
  // Sécurité seulement. Une coupure proche de la consigne recréerait le trou
  // de chauffe de 13 h 01 (28 septembre), provoqué par le pic de précharge.
  static constexpr float kSafetyAboveTargetC = 4.0f;

  // Une commande vaut au plus jusqu'à 1 s : au-delà, le pas manqué n'est pas
  // compté, comme dans le régulateur.
  static constexpr uint64_t kMaximumStepMs = 1000;

  // Début d'un cycle, avec ou sans précharge, et fin de l'écoulement.
  void reset() {
    debt_pct_s_ = 0.0f;
    last_ms_ = 0;
    last_preheat_pct_ = 0.0f;
    last_withheld_pct_ = 0.0f;
  }

  // Précharge : 90 % sauf si la NTC dépasse déjà la cible de 0,5 °C. La
  // commande envoyée s'ajoute à la dette jusqu'au pas suivant.
  float preheat_pct(uint64_t now_ms, float temperature_c, float target_c) {
    settle(now_ms);
    last_preheat_pct_ =
        temperature_c - target_c < kPreheatAboveTargetBandC ? kPreheatPowerPct : 0.0f;
    return last_preheat_pct_;
  }

  // Remplissage, pré-infusion (`infusing` faux) et infusion, rampe de fin
  // comprise. La chauffe suit le débit jusqu'à l'arrêt de la pompe.
  float flow_pct(uint64_t now_ms, float temperature_c, float target_c, float flow_ml_s,
                 bool flow_valid, bool infusing) {
    settle(now_ms);
    const float law_pct = law(temperature_c, target_c, flow_ml_s, flow_valid);
    // Seul l'appoint de la loi est retenu : la sécurité, qui donne déjà 0 %,
    // ne rembourse rien.
    last_withheld_pct_ = infusing && debt_pct_s_ > 0.0f ? law_pct : 0.0f;
    return law_pct - last_withheld_pct_;
  }

  // Énergie de précharge encore à rendre, en %·s (12 J par %·s).
  float debt_pct_s() const { return debt_pct_s_; }

 private:
  static float law(float temperature_c, float target_c, float flow_ml_s, bool flow_valid) {
    if (temperature_c - target_c > kSafetyAboveTargetC) return 0.0f;
    if (!flow_valid || !std::isfinite(flow_ml_s)) return kFlowFallbackPct;
    return std::min(kPowerLimitPct,
                    kHoldPowerPct + std::max(0.0f, flow_ml_s) * kWaterHeatPctPerMlS);
  }

  // Impute la commande du pas précédent sur la durée écoulée depuis : la
  // précharge augmente la dette, l'appoint retenu la rembourse. Le dernier pas
  // de remboursement peut rendre jusqu'à 250 ms de trop (≈ 0,27 kJ à 90 %).
  void settle(uint64_t now_ms) {
    if (last_ms_ != 0 && now_ms > last_ms_) {
      const float dt_s =
          static_cast<float>(std::min(now_ms - last_ms_, kMaximumStepMs)) / 1000.0f;
      debt_pct_s_ = std::max(0.0f, debt_pct_s_ + (last_preheat_pct_ - last_withheld_pct_) * dt_s);
    }
    last_ms_ = now_ms;
    last_preheat_pct_ = 0.0f;
    last_withheld_pct_ = 0.0f;
  }

  float debt_pct_s_ = 0.0f;
  uint64_t last_ms_ = 0;
  float last_preheat_pct_ = 0.0f;
  float last_withheld_pct_ = 0.0f;
};

}  // namespace core::thermal
