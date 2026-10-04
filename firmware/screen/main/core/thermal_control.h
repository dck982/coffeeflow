#pragma once

#include <algorithm>
#include <array>
#include <cmath>
#include <cstddef>
#include <cstdint>
#include <limits>

#include "core/brew_heating.h"
#include "core/config.h"

namespace core::thermal {

// Régulateur de repos, de purge et de récupération. Pendant la précharge et
// l'écoulement d'une infusion, il délègue la commande à `BrewHeating`
// (brew_heating.h) et ne fait que tenir à jour son propre état.
//
// Au repos, un prédicteur de retard (voir docs/chauffe-repos.md) ; en purge
// et en récupération, l'ancienne prédiction par la pente et la chaleur en
// transit, à recalibrer sur capture.
class Controller {
 public:
  // `kBrew` : remplissage et pré-infusion ; `kInfusion` : infusion, rampe de
  // fin comprise. Le remboursement de la précharge commence avec `kInfusion`.
  enum class Mode { kIdle, kThermalPreheat, kBrew, kInfusion, kPurge };
  // `restart_window` : première commande non nulle de la précharge ou de
  // l'écoulement d'une infusion. Le module capteurs ouvre alors une nouvelle
  // période SSR au lieu d'attendre la fin de la période de repos en cours
  // (jusqu'à 1 s perdue, voir docs/chauffe-chaudiere.md, point 9).
  struct Output { uint16_t power_permille; bool ready; bool restart_window = false; };

  // Repos. Modèle ajusté sur les captures de repos du 29 septembre : retard
  // pur de 23,5 s entre commande et NTC, 1,28 kJ/K, pertes de 0,45 W/K.
  static constexpr float kIdleDelayS = 23.5f;
  static constexpr float kIdleHeatGainCPerPctSecond = 12.0f / 1280.0f;
  static constexpr float kIdleLossPctPerK = 0.45f / 12.0f;
  static constexpr float kAmbientC = 24.5f;
  static constexpr float kIdleProportionalPctPerC = 6.0f;
  static constexpr float kIdleIntegralPctPerCSecond = 0.05f;
  // L'intégrale corrige l'erreur du modèle de pertes : elle peut être négative.
  static constexpr float kIdleIntegralMinPct = -10.0f;
  static constexpr float kIdleIntegralMaxPct = 35.0f;
  static constexpr float kIdleIntegralBandC = 8.0f;

  // Purge et récupération.
  static constexpr float kPredictionHorizonS = 20.0f;
  static constexpr float kRecoveryPredictionHorizonS = 10.0f;
  static constexpr float kCommandTransitWindowS = 22.0f;
  static constexpr float kCommandHeatGainCPerPctSecond = 0.007f;
  static constexpr float kPowerFilterTimeConstantS = 5.0f;
  static constexpr float kSlopeFilterTimeConstantS = 8.0f;
  static constexpr float kHoldPowerPct = 3.5f;
  static constexpr float kFlowFeedforwardPct = 18.0f;
  static constexpr float kFlowPowerLimitPct = 35.0f;
  static constexpr float kFlowBonusStartMlS = 2.0f;
  static constexpr float kFlowBonusFullMlS = 4.0f;
  static constexpr float kFlowBonusMaxPct = 10.0f;
  static constexpr float kFlowBonusAboveTargetBandC = 2.0f;
  static constexpr float kPurgeFeedforwardAboveTargetBandC = 2.0f;
  static constexpr float kPurgeCompensationBandC = 5.0f;
  static constexpr float kRecoveryPowerLimitPct = 35.0f;
  static constexpr uint64_t kRecoveryDurationMs = 30000;
  // L'entrée est dans le domaine utilisateur : 105 °C ici = 115,5 °C à la
  // sonde avec l'offset de -10,5 °C appliqué lors de la lecture NTC.
  static constexpr float kMaximumBoilerUserTemperatureC = 105.0f;

  Output step(uint64_t now_ms, float temperature_c, float target_c,
              bool valid, bool enabled, Mode mode,
              float flow_ml_s = 0.0f, bool flow_valid = false,
              float brew_remaining_s = std::numeric_limits<float>::quiet_NaN()) {
    if (!valid || !enabled || !std::isfinite(temperature_c) ||
        !std::isfinite(target_c) || temperature_c > kMaximumBoilerUserTemperatureC) {
      reset();
      return {0, false};
    }
    if (last_ms_ != 0 && target_c != target_c_) reset();
    target_c_ = target_c;
    if (last_ms_ == 0 || now_ms <= last_ms_ || now_ms - last_ms_ > 2000) {
      slope_c_per_s_ = 0;
      ready_since_ms_ = 0;
      clear_command_history();
      last_temperature_c_ = temperature_c;
      last_ms_ = now_ms;
    }
    const bool preheating = mode == Mode::kThermalPreheat;
    const bool brewing = mode == Mode::kBrew || mode == Mode::kInfusion;
    const bool flowing = brewing || mode == Mode::kPurge;
    // La dette de précharge passe à l'écoulement qui la suit ; tout autre
    // début de précharge ou d'écoulement part d'une loi d'infusion neuve.
    if (preheating && !was_preheating_) brew_.reset();
    if (flowing != was_flowing_) {
      // The temperature slope during water exchange does not predict the
      // boiler's slope once the flow stops (or starts).
      slope_c_per_s_ = 0;
      last_temperature_c_ = temperature_c;
      last_ms_ = now_ms;
      if (!flowing || !was_preheating_) brew_.reset();
      if (!flowing) recovery_until_ms_ = now_ms + kRecoveryDurationMs;
      else uncompensated_purge_ = mode == Mode::kPurge &&
          std::fabs(temperature_c - target_c) >= kPurgeCompensationBandC;
      was_flowing_ = flowing;
    }
    was_preheating_ = preheating;
    // Une nouvelle précharge est une phase active à part entière : elle ne
    // doit pas hériter du plafond de récupération du cycle précédent.
    const bool recovering = !flowing && !preheating && now_ms < recovery_until_ms_;
    const float dt_s = std::min(static_cast<float>(now_ms - last_ms_) / 1000.0f, 1.0f);
    if (dt_s > 0) {
      const float measured_slope = (temperature_c - last_temperature_c_) / dt_s;
      const float alpha = dt_s / (kSlopeFilterTimeConstantS + dt_s);
      slope_c_per_s_ += alpha * (measured_slope - slope_c_per_s_);
      last_temperature_c_ = temperature_c;
      last_ms_ = now_ms;
    }

    const float error = target_c - temperature_c;
    // En purge, réduire progressivement les appoints entre la consigne et
    // consigne + 2 °C, sans couper net dès le premier dépassement.
    const float purge_taper = mode == Mode::kPurge
        ? std::clamp(1.0f + error / kPurgeFeedforwardAboveTargetBandC, 0.0f, 1.0f)
        : 1.0f;
    // En purge, l'appoint suit le débit et disparaît dès que celui-ci baisse.
    // Ne pas utiliser une mesure absente ou périmée.
    const float flow_bonus_pct = mode == Mode::kPurge && flow_valid && std::isfinite(flow_ml_s) &&
                                 error > -kFlowBonusAboveTargetBandC
        ? std::clamp((flow_ml_s - kFlowBonusStartMlS) /
                         (kFlowBonusFullMlS - kFlowBonusStartMlS), 0.0f, 1.0f) *
              kFlowBonusMaxPct * purge_taper
        : 0.0f;
    if (std::fabs(error) <= kBrewTemperatureToleranceC) {
      if (ready_since_ms_ == 0) ready_since_ms_ = now_ms;
    } else {
      ready_since_ms_ = 0;
    }
    const bool ready = ready_since_ms_ != 0 && now_ms - ready_since_ms_ >= 3000;

    if (preheating || brewing) {
      // Loi d'infusion séparée : ni la prédiction, ni l'intégrale, ni le
      // filtre de sortie du régulateur de repos n'interviennent. La commande
      // est enregistrée pour l'estimation de chaleur en transit de la reprise.
      const float power = preheating
          ? brew_.preheat_pct(now_ms, temperature_c, target_c)
          : brew_.flow_pct(now_ms, temperature_c, target_c, flow_ml_s, flow_valid,
                           brew_remaining_s, mode == Mode::kInfusion);
      filtered_power_pct_ = power;
      has_filtered_power_ = true;
      const uint16_t power_permille = static_cast<uint16_t>(
          std::lround(std::clamp(power, 0.0f, 100.0f) * 10.0f));
      record_command(now_ms, power_permille / 10.0f);
      // Une seule fois par phase : répété à chaque trame, le flag relancerait
      // la période toutes les 500 ms et chaufferait en continu.
      const WindowPhase phase = preheating ? WindowPhase::kPreheat : WindowPhase::kFlow;
      const bool restart_window = power_permille > 0 && restarted_phase_ != phase;
      if (restart_window) restarted_phase_ = phase;
      return {power_permille, ready, restart_window};
    }
    restarted_phase_ = WindowPhase::kNone;

    if (!flowing && !recovering) {
      // Repos : prédicteur de retard. La chaleur commandée pendant les
      // `kIdleDelayS` dernières secondes n'est pas encore visible à la NTC ;
      // elle est ajoutée à la mesure au lieu d'extrapoler la pente, qui
      // reflète des commandes passées. La récupération dure plus longtemps que
      // ce retard : l'historique vu ici ne contient jamais d'écoulement.
      const float loss_pct = idle_loss_pct(temperature_c);
      const float pending_c = command_pct_seconds(now_ms, kIdleDelayS, loss_pct, false) *
                              kIdleHeatGainCPerPctSecond;
      const float predicted_error = target_c - (temperature_c + pending_c);
      const float unclamped_pct =
          loss_pct + kIdleProportionalPctPerC * predicted_error + idle_integral_pct_;
      // Anti-emballement : ne pas intégrer dans le sens d'une sortie saturée.
      const bool saturated = (unclamped_pct >= 100.0f && predicted_error > 0.0f) ||
                             (unclamped_pct <= 0.0f && predicted_error < 0.0f);
      if (std::fabs(error) < kIdleIntegralBandC && dt_s > 0.0f && !saturated)
        idle_integral_pct_ = std::clamp(
            idle_integral_pct_ + kIdleIntegralPctPerCSecond * predicted_error * dt_s,
            kIdleIntegralMinPct, kIdleIntegralMaxPct);
      const float power = std::clamp(
          loss_pct + kIdleProportionalPctPerC * predicted_error + idle_integral_pct_,
          0.0f, 100.0f);
      // Le filtre de la purge et de la récupération repart de cette commande ;
      // une commande nulle le laisse s'initialiser au premier pas suivant.
      filtered_power_pct_ = power;
      if (power > 0.0f) has_filtered_power_ = true;
      const uint16_t power_permille = static_cast<uint16_t>(std::lround(power * 10.0f));
      record_command(now_ms, power_permille / 10.0f);
      return {power_permille, ready};
    }

    // Purge et récupération : prédiction par la pente et la chaleur en transit.
    // Latch the decision at purge start: a purge far from setpoint may be
    // intended to cool the boiler, even if the NTC crosses the setpoint.
    if (mode == Mode::kPurge && uncompensated_purge_) {
      filtered_power_pct_ = 0.0f;
      record_command(now_ms, 0.0f);
      return {0, ready};
    }

    // Anticiper la chaleur encore en route. Pendant l'écoulement et la
    // récupération, une pente négative n'est pas extrapolée.
    const float transit_heat_c = std::min(
        recovering ? 6.0f : 1.5f,
        command_pct_seconds(now_ms, kCommandTransitWindowS, kHoldPowerPct, true) *
            kCommandHeatGainCPerPctSecond);
    const float predictive_slope = std::max(0.0f, slope_c_per_s_);
    const float horizon_s = recovering ? kRecoveryPredictionHorizonS : kPredictionHorizonS;
    // During recovery the rising slope already contains some of the heat
    // from recent commands. Adding both terms would count it twice.
    const float predicted_rise_c = recovering
        ? std::max(predictive_slope * horizon_s, transit_heat_c)
        : predictive_slope * horizon_s + transit_heat_c;
    const float predicted_c = temperature_c + predicted_rise_c;
    const float predicted_error = target_c - predicted_c;
    float power = kHoldPowerPct + 8.0f * predicted_error;
    if (flowing) {
      power = std::min(power, kFlowPowerLimitPct);
    } else if (recovering) {
      power = std::min(power, kRecoveryPowerLimitPct);
    }

    // Un dépassement prédit coupe immédiatement. Lisser les variations
    // positives restantes évite de poursuivre le bruit de la dérivée.
    // La commande effectivement envoyée alimente l'estimation d'inertie.
    if (power <= 0.0f) {
      filtered_power_pct_ = 0.0f;
    } else {
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
    }
    if (flowing) filtered_power_pct_ = std::min(filtered_power_pct_, kFlowPowerLimitPct);
    if (recovering) filtered_power_pct_ = std::min(filtered_power_pct_, kRecoveryPowerLimitPct);
    // Les appoints de purge et de débit suivent directement leurs conditions
    // courantes : le filtre de 5 s ne doit pas prolonger leur réduction.
    const float purge_feedforward_pct = mode == Mode::kPurge
        ? kFlowFeedforwardPct * purge_taper : 0.0f;
    const float commanded_power_pct =
        std::max(filtered_power_pct_, purge_feedforward_pct) + flow_bonus_pct;
    const uint16_t power_permille = static_cast<uint16_t>(
        std::lround(std::clamp(commanded_power_pct, 0.0f, 100.0f) * 10.0f));
    record_command(now_ms, power_permille / 10.0f);
    return {power_permille, ready};
  }

  void reset() {
    last_ms_ = 0;
    ready_since_ms_ = 0;
    idle_integral_pct_ = 0;
    slope_c_per_s_ = 0;
    filtered_power_pct_ = 0;
    has_filtered_power_ = false;
    was_flowing_ = false;
    was_preheating_ = false;
    uncompensated_purge_ = false;
    brew_.reset();
    restarted_phase_ = WindowPhase::kNone;
    recovery_until_ms_ = 0;
    clear_command_history();
  }

 private:
  enum class WindowPhase : uint8_t { kNone, kPreheat, kFlow };

  struct CommandSample {
    uint64_t at_ms = 0;
    float power_pct = 0.0f;
  };

  // 32 s à un pas de 250 ms : couvre le retard du repos et la fenêtre de 22 s.
  static constexpr size_t kCommandHistoryCapacity = 128;

  static float idle_loss_pct(float temperature_c) {
    return std::max(0.0f, kIdleLossPctPerK * (temperature_c - kAmbientC));
  }

  void clear_command_history() {
    command_history_count_ = 0;
    command_history_next_ = 0;
  }

  void record_command(uint64_t now_ms, float power_pct) {
    command_history_[command_history_next_] = {now_ms, power_pct};
    command_history_next_ = (command_history_next_ + 1) % kCommandHistoryCapacity;
    if (command_history_count_ < kCommandHistoryCapacity) ++command_history_count_;
  }

  // Somme, sur les `window_s` dernières secondes, de la commande moins
  // `offset_pct`, en %·s. `positive_only` ignore les commandes sous l'offset.
  // Chaque commande vaut jusqu'à la suivante ; la dernière, jusqu'à `now_ms`.
  float command_pct_seconds(uint64_t now_ms, float window_s, float offset_pct,
                            bool positive_only) const {
    if (command_history_count_ == 0) return 0.0f;
    const uint64_t window_ms = static_cast<uint64_t>(window_s * 1000.0f);
    const uint64_t cutoff_ms = now_ms > window_ms ? now_ms - window_ms : 0;
    const auto excess = [&](float power_pct) {
      const float value = power_pct - offset_pct;
      return positive_only ? std::max(0.0f, value) : value;
    };
    float pct_seconds = 0.0f;
    const size_t oldest = (command_history_next_ + kCommandHistoryCapacity -
                           command_history_count_) % kCommandHistoryCapacity;
    for (size_t i = 0; i < command_history_count_; ++i) {
      const CommandSample& sample = command_history_[(oldest + i) % kCommandHistoryCapacity];
      const uint64_t end_ms = i + 1 < command_history_count_
          ? command_history_[(oldest + i + 1) % kCommandHistoryCapacity].at_ms
          : now_ms;
      const uint64_t start_ms = std::max(sample.at_ms, cutoff_ms);
      const uint64_t clipped_end_ms = std::min(end_ms, now_ms);
      if (clipped_end_ms > start_ms)
        pct_seconds += excess(sample.power_pct) *
                       static_cast<float>(clipped_end_ms - start_ms) / 1000.0f;
    }
    return pct_seconds;
  }

  uint64_t last_ms_ = 0;
  uint64_t ready_since_ms_ = 0;
  float last_temperature_c_ = 0;
  float slope_c_per_s_ = 0;
  float idle_integral_pct_ = 0;
  float target_c_ = 0;
  float filtered_power_pct_ = 0;
  bool has_filtered_power_ = false;
  bool was_flowing_ = false;
  bool was_preheating_ = false;
  bool uncompensated_purge_ = false;
  BrewHeating brew_;
  WindowPhase restarted_phase_ = WindowPhase::kNone;
  uint64_t recovery_until_ms_ = 0;
  std::array<CommandSample, kCommandHistoryCapacity> command_history_{};
  size_t command_history_count_ = 0;
  size_t command_history_next_ = 0;
};

}  // namespace core::thermal
