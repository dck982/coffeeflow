#pragma once

#include <algorithm>
#include <array>
#include <cmath>
#include <cstddef>
#include <cstdint>
#include <limits>

namespace core {

// Résumé d'une infusion, calculé échantillon par échantillon pendant la
// capture HF (docs/ecran-infusion.md). Il ne relit jamais le tampon : une
// capture tronquée garde un résumé complet. describe_hf_capture.py applique
// les mêmes définitions aux captures exportées.
enum class ShotSegment : uint8_t { kPreheat, kFilling, kPreinfusion, kRamp, kInfusion, kDrip, kCount };
inline constexpr size_t kShotSegmentCount = static_cast<size_t>(ShotSegment::kCount);

enum class ShotSampleMode : uint8_t { kPreheat, kFilling, kPreinfusion, kInfusion, kCooldown, kOther };

struct ShotSummary {
  bool active = false;
  // Début et fin de chaque segment, en secondes depuis le début de la
  // capture ; NaN pour un segment absent. Le segment en cours finit au
  // dernier échantillon.
  std::array<float, kShotSegmentCount> segment_start_s{};
  std::array<float, kShotSegmentCount> segment_end_s{};
  float elapsed_s = 0.0f;
  float pump_start_s = std::numeric_limits<float>::quiet_NaN();
  float pump_stop_s = std::numeric_limits<float>::quiet_NaN();
  // Infusion : de la consigne − 1 bar à l'arrêt de la pompe ; sans pression
  // atteinte, toute la phase d'infusion.
  float infusion_s = std::numeric_limits<float>::quiet_NaN();
  float infusion_cup_g_s = std::numeric_limits<float>::quiet_NaN();
  float infusion_ml_s = std::numeric_limits<float>::quiet_NaN();
  float cup_weight_g = std::numeric_limits<float>::quiet_NaN();
  float drip_gain_g = 0.0f;
  bool drip_done = false;
  float cup_rate_g_s = std::numeric_limits<float>::quiet_NaN();
  // Moyenne NTC pondérée par la tasse jusqu'à l'arrêt de la pompe.
  float cup_mean_c = std::numeric_limits<float>::quiet_NaN();
  float start_temperature_c = std::numeric_limits<float>::quiet_NaN();
  float min_temperature_c = std::numeric_limits<float>::quiet_NaN();
  float max_temperature_c = std::numeric_limits<float>::quiet_NaN();
  // Échelle de la frise, en secondes : arrêt prévu, puis réel, plus la réserve
  // des gouttes ; la fin des gouttes une fois celles-ci terminées.
  float axis_s = 0.0f;
  bool pump_running() const { return std::isfinite(pump_start_s) && !std::isfinite(pump_stop_s); }
  bool pump_stopped() const { return std::isfinite(pump_stop_s); }
  bool has(ShotSegment segment) const {
    return std::isfinite(segment_start_s[static_cast<size_t>(segment)]);
  }
  float duration_s(ShotSegment segment) const {
    const size_t i = static_cast<size_t>(segment);
    return has(segment) ? segment_end_s[i] - segment_start_s[i] : 0.0f;
  }
};

struct ShotSample {
  float t_s = 0.0f;
  ShotSampleMode mode = ShotSampleMode::kOther;
  uint8_t pump_pct = 0;
  bool pressure_valid = false;
  float pressure_bar = 0.0f;
  bool scale_present = false;
  float weight_g = 0.0f;
  float volume_ml = 0.0f;
  bool temperature_valid = false;
  float temperature_c = 0.0f;
};

class ShotSummaryBuilder {
 public:
  static constexpr float kPressureBelowTargetBar = 1.0f;
  // Débit en tasse sur 2 s, comme l'estimation de fin de la chauffe.
  static constexpr float kCupRateWindowS = 2.0f;
  // Arrêt prévu : 30 s de pompe après son démarrage. La machine est souvent
  // éteinte entre deux cafés ; le cycle précédent n'est donc pas retenu.
  static constexpr float kExpectedPumpRunS = 30.0f;
  static constexpr float kAxisGrowthAt = 0.9f;
  static constexpr float kAxisGrowthFactor = 1.25f;
  static constexpr float kDripReserveS = 4.0f;

  void reset(float target_pressure_bar, float preheat_s) {
    summary_ = ShotSummary{};
    summary_.active = true;
    constexpr float kNaN = std::numeric_limits<float>::quiet_NaN();
    summary_.segment_start_s.fill(kNaN);
    summary_.segment_end_s.fill(kNaN);
    pressure_threshold_bar_ = target_pressure_bar - kPressureBelowTargetBar;
    expected_stop_s_ = std::max(0.0f, preheat_s) + kExpectedPumpRunS;
    pressure_reached_ = false;
    pressure_t_s_ = kNaN;
    pressure_weight_g_ = pressure_volume_ml_ = kNaN;
    infusion_start_s_ = kNaN;
    infusion_weight_g_ = infusion_volume_ml_ = kNaN;
    last_weight_g_ = kNaN;
    last_volume_ml_ = 0.0f;
    stopped_flows_frozen_ = false;
    weight_origin_g_ = kNaN;
    weight_max_g_ = 0.0f;
    weighted_sum_ = weight_sum_ = 0.0f;
    last_temperature_c_ = kNaN;
    stop_weight_g_ = kNaN;
    window_count_ = window_next_ = 0;
    current_ = ShotSegment::kCount;
    summary_.axis_s = expected_stop_s_ + kDripReserveS;
  }

  void clear() { summary_ = ShotSummary{}; }

  void add(const ShotSample& sample) {
    if (!summary_.active) return;
    const float t = sample.t_s;
    summary_.elapsed_s = t;
    const bool pump_on = sample.pump_pct > 0;
    if (pump_on && !std::isfinite(summary_.pump_start_s)) {
      summary_.pump_start_s = t;
      expected_stop_s_ = t + kExpectedPumpRunS;
    }
    if (!pump_on && summary_.pump_running()) summary_.pump_stop_s = t;

    const float weight = relative_weight(sample);
    push_weight(t, weight);
    summary_.cup_rate_g_s = cup_rate(t);
    track_temperature(sample, weight);
    track_segments(sample, weight);
    track_drips(t, weight);
    update_infusion(t);
    update_axis(t);
  }

  const ShotSummary& summary() const { return summary_; }

 private:
  struct WeightPoint {
    float t_s = 0.0f;
    float weight_g = 0.0f;
  };
  // 2 s à 100 ms, plus de la marge pour une capture plus lente.
  static constexpr size_t kWindowCapacity = 32;

  float relative_weight(const ShotSample& sample) {
    if (!sample.scale_present || !std::isfinite(sample.weight_g))
      return std::numeric_limits<float>::quiet_NaN();
    if (!std::isfinite(weight_origin_g_)) weight_origin_g_ = sample.weight_g;
    return sample.weight_g - weight_origin_g_;
  }

  void push_weight(float t, float weight) {
    window_[window_next_] = {t, weight};
    window_next_ = (window_next_ + 1) % kWindowCapacity;
    if (window_count_ < kWindowCapacity) ++window_count_;
  }

  // Poids sur les 2 dernières secondes. Une mesure manquante ou négative dans
  // la fenêtre la rend inconnue : une tasse retirée n'est pas un débit.
  float cup_rate(float t) const {
    constexpr float kNaN = std::numeric_limits<float>::quiet_NaN();
    if (window_count_ < 2) return kNaN;
    const WeightPoint& last = window_[(window_next_ + kWindowCapacity - 1) % kWindowCapacity];
    if (!std::isfinite(last.weight_g) || last.weight_g < 0.0f) return kNaN;
    for (size_t i = 1; i < window_count_; ++i) {
      const WeightPoint& p = window_[(window_next_ + kWindowCapacity - 1 - i) % kWindowCapacity];
      if (!std::isfinite(p.weight_g) || p.weight_g < 0.0f) return kNaN;
      if (t - p.t_s >= kCupRateWindowS - 1e-3f) return (last.weight_g - p.weight_g) / (t - p.t_s);
    }
    return kNaN;
  }

  void track_temperature(const ShotSample& sample, float weight) {
    if (!sample.temperature_valid || !std::isfinite(sample.temperature_c)) return;
    const float temperature = sample.temperature_c;
    if (!std::isfinite(summary_.start_temperature_c)) summary_.start_temperature_c = temperature;
    if (summary_.pump_running()) {
      summary_.min_temperature_c = std::isfinite(summary_.min_temperature_c)
          ? std::min(summary_.min_temperature_c, temperature) : temperature;
      summary_.max_temperature_c = std::isfinite(summary_.max_temperature_c)
          ? std::max(summary_.max_temperature_c, temperature) : temperature;
      // Chaque gramme compte avec la NTC du même instant ; le poids suit son
      // maximum courant, les reculs de la balance ne retirent rien.
      if (std::isfinite(weight) && weight > weight_max_g_) {
        const float previous = std::isfinite(last_temperature_c_) ? last_temperature_c_ : temperature;
        weighted_sum_ += (weight - weight_max_g_) * (previous + temperature) * 0.5f;
        weight_sum_ += weight - weight_max_g_;
        weight_max_g_ = weight;
        if (weight_sum_ > 0.5f) summary_.cup_mean_c = weighted_sum_ / weight_sum_;
      }
    }
    last_temperature_c_ = temperature;
  }

  void open(ShotSegment segment, float t) {
    if (segment == current_) return;
    if (current_ != ShotSegment::kCount) summary_.segment_end_s[static_cast<size_t>(current_)] = t;
    current_ = segment;
    summary_.segment_start_s[static_cast<size_t>(segment)] = t;
  }

  void track_segments(const ShotSample& sample, float weight) {
    const float t = sample.t_s;
    switch (sample.mode) {
      case ShotSampleMode::kPreheat: open(ShotSegment::kPreheat, t); break;
      case ShotSampleMode::kFilling: open(ShotSegment::kFilling, t); break;
      case ShotSampleMode::kPreinfusion: open(ShotSegment::kPreinfusion, t); break;
      case ShotSampleMode::kInfusion:
        if (!std::isfinite(infusion_start_s_)) {
          infusion_start_s_ = t;
          infusion_weight_g_ = weight;
          infusion_volume_ml_ = sample.volume_ml;
        }
        if (!pressure_reached_ && sample.pressure_valid &&
            sample.pressure_bar >= pressure_threshold_bar_) {
          pressure_reached_ = true;
          pressure_t_s_ = t;
          pressure_weight_g_ = weight;
          pressure_volume_ml_ = sample.volume_ml;
        }
        open(pressure_reached_ ? ShotSegment::kInfusion : ShotSegment::kRamp, t);
        last_volume_ml_ = sample.volume_ml;
        if (std::isfinite(weight)) last_weight_g_ = weight;
        break;
      case ShotSampleMode::kCooldown:
        if (!summary_.drip_done) open(ShotSegment::kDrip, t);
        break;
      case ShotSampleMode::kOther: break;
    }
    if (current_ != ShotSegment::kCount && !summary_.drip_done)
      summary_.segment_end_s[static_cast<size_t>(current_)] = t;
    // Pression jamais atteinte : la montée devient l'infusion à l'arrêt.
    if (summary_.pump_stopped() && !pressure_reached_ && summary_.has(ShotSegment::kRamp) &&
        !summary_.has(ShotSegment::kInfusion)) {
      const size_t ramp = static_cast<size_t>(ShotSegment::kRamp);
      const size_t infusion = static_cast<size_t>(ShotSegment::kInfusion);
      summary_.segment_start_s[infusion] = summary_.segment_start_s[ramp];
      summary_.segment_end_s[infusion] = summary_.segment_end_s[ramp];
      summary_.segment_start_s[ramp] = summary_.segment_end_s[ramp] =
          std::numeric_limits<float>::quiet_NaN();
    }
  }

  // Gouttes : du stop de la pompe au premier retour à 0 du débit en tasse,
  // ou à une tasse retirée. Le poids est alors figé.
  void track_drips(float t, float weight) {
    if (summary_.drip_done) return;
    if (std::isfinite(weight) && weight >= 0.0f) summary_.cup_weight_g = weight;
    if (!summary_.pump_stopped()) return;
    if (!std::isfinite(stop_weight_g_)) stop_weight_g_ = summary_.cup_weight_g;
    if (std::isfinite(summary_.cup_weight_g) && std::isfinite(stop_weight_g_))
      summary_.drip_gain_g = std::max(0.0f, summary_.cup_weight_g - stop_weight_g_);
    const float rate = summary_.cup_rate_g_s;
    if (t > summary_.pump_stop_s && (!std::isfinite(rate) || rate <= 0.0f)) {
      summary_.drip_done = true;
      if (summary_.has(ShotSegment::kDrip)) summary_.segment_end_s[static_cast<size_t>(ShotSegment::kDrip)] = t;
    }
  }

  void update_infusion(float t) {
    const bool reached = pressure_reached_;
    const float start = reached ? pressure_t_s_ : infusion_start_s_;
    if (!std::isfinite(start)) return;
    // Sans pression atteinte, la durée n'est connue qu'à l'arrêt de la pompe.
    if (!reached && !summary_.pump_stopped()) return;
    const float end = summary_.pump_stopped() ? summary_.pump_stop_s : t;
    const float duration = std::max(0.0f, end - start);
    summary_.infusion_s = duration;
    if (duration < 1.0f) return;
    const float start_weight = reached ? pressure_weight_g_ : infusion_weight_g_;
    const float start_volume = reached ? pressure_volume_ml_ : infusion_volume_ml_;
    if (!summary_.pump_stopped()) {
      if (std::isfinite(start_weight) && std::isfinite(last_weight_g_))
        summary_.infusion_cup_g_s = (last_weight_g_ - start_weight) / duration;
      if (std::isfinite(start_volume)) summary_.infusion_ml_s = (last_volume_ml_ - start_volume) / duration;
    } else if (!stopped_flows_frozen_) {
      if (std::isfinite(start_weight) && std::isfinite(last_weight_g_))
        summary_.infusion_cup_g_s = (last_weight_g_ - start_weight) / duration;
      if (std::isfinite(start_volume)) summary_.infusion_ml_s = (last_volume_ml_ - start_volume) / duration;
      stopped_flows_frozen_ = true;
    }
  }

  // Arrêt prévu, agrandi de 25 % quand l'écoulé en atteint 90 % ; jamais
  // réduit tant que la pompe tourne, pour que la frise ne se recompresse pas.
  void update_axis(float t) {
    if (summary_.drip_done) {
      summary_.axis_s = summary_.segment_end_s[static_cast<size_t>(ShotSegment::kDrip)];
      if (!std::isfinite(summary_.axis_s)) summary_.axis_s = t;
      return;
    }
    if (summary_.pump_stopped()) {
      summary_.axis_s = std::max(summary_.pump_stop_s + kDripReserveS, t + 0.5f);
      return;
    }
    while (t > kAxisGrowthAt * expected_stop_s_) expected_stop_s_ *= kAxisGrowthFactor;
    summary_.axis_s = expected_stop_s_ + kDripReserveS;
  }

  ShotSummary summary_{};
  float pressure_threshold_bar_ = 8.0f;
  float expected_stop_s_ = kExpectedPumpRunS;
  bool pressure_reached_ = false;
  float pressure_t_s_ = 0.0f;
  float pressure_weight_g_ = 0.0f;
  float pressure_volume_ml_ = 0.0f;
  float infusion_start_s_ = 0.0f;
  float infusion_weight_g_ = 0.0f;
  float infusion_volume_ml_ = 0.0f;
  float last_weight_g_ = std::numeric_limits<float>::quiet_NaN();
  float last_volume_ml_ = 0.0f;
  bool stopped_flows_frozen_ = false;
  float weight_origin_g_ = 0.0f;
  float weight_max_g_ = 0.0f;
  float weighted_sum_ = 0.0f;
  float weight_sum_ = 0.0f;
  float last_temperature_c_ = 0.0f;
  float stop_weight_g_ = 0.0f;
  std::array<WeightPoint, kWindowCapacity> window_{};
  size_t window_count_ = 0;
  size_t window_next_ = 0;
  ShotSegment current_ = ShotSegment::kCount;
};

}  // namespace core
