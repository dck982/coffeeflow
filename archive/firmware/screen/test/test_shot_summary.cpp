#include <cassert>
#include <cmath>
#include <limits>

#include "core/shot_summary.h"

using core::ShotSample;
using core::ShotSampleMode;
using core::ShotSegment;
using core::ShotSummaryBuilder;

namespace {
bool near(float a, float b, float tolerance = 1e-3f) { return std::fabs(a - b) <= tolerance; }

// Précharge 5 s, remplissage 3 s, pause 4 s, montée de 4 s jusqu'à 8 bar,
// infusion à 1 g/s et 1,5 ml/s jusqu'à 40 s, gouttes de 0,5 g en 2 s.
ShotSample sample(float t, bool scale = true, float pressure_max = 9.0f,
                  float removed_at = std::numeric_limits<float>::infinity()) {
  ShotSample s;
  s.t_s = t;
  s.mode = t < 5.0f ? ShotSampleMode::kPreheat
           : t < 8.0f ? ShotSampleMode::kFilling
           : t < 12.0f ? ShotSampleMode::kPreinfusion
           : t < 40.0f ? ShotSampleMode::kInfusion
                       : ShotSampleMode::kCooldown;
  s.pump_pct = t >= 5.0f && t < 40.0f ? 60 : 0;
  s.pressure_valid = true;
  s.pressure_bar = t < 12.0f ? 0.3f : std::min(pressure_max, 0.3f + (t - 12.0f) * 2.0f);
  s.scale_present = scale;
  s.weight_g = 10.0f + std::max(0.0f, std::min(t, 40.0f) - 16.0f) +
               (t > 40.0f ? std::min(t - 40.0f, 2.0f) * 0.25f : 0.0f);
  if (t >= removed_at) s.weight_g = -100.0f;
  s.volume_ml = std::max(0.0f, std::min(t, 40.0f) - 5.0f) * 1.5f;
  s.temperature_valid = true;
  s.temperature_c = t < 16.0f ? 90.0f - std::max(0.0f, t - 5.0f) * 0.1f : 88.9f + (t - 16.0f) * 0.1f;
  return s;
}

ShotSummaryBuilder run(float until, bool scale = true, float pressure_max = 9.0f,
                       float removed_at = std::numeric_limits<float>::infinity()) {
  ShotSummaryBuilder builder;
  builder.reset(9.0f, 5.0f);
  for (int i = 0; i * 0.1f <= until + 1e-4f; ++i)
    builder.add(sample(i * 0.1f, scale, pressure_max, removed_at));
  return builder;
}
}  // namespace

int main() {
  // Infusion complète : segments, infusion à consigne − 1 bar, débits, gouttes.
  {
    const auto builder = run(60.0f);
    const auto& s = builder.summary();
    assert(s.active);
    assert(near(s.duration_s(ShotSegment::kPreheat), 5.0f, 0.11f));
    assert(near(s.duration_s(ShotSegment::kFilling), 3.0f, 0.11f));
    assert(near(s.duration_s(ShotSegment::kPreinfusion), 4.0f, 0.11f));
    // 8 bar atteints à 12 + 7,7 / 2 = 15,85 s : premier échantillon à 15,9 s.
    assert(near(s.segment_start_s[static_cast<size_t>(ShotSegment::kInfusion)], 15.9f, 0.01f));
    assert(near(s.duration_s(ShotSegment::kRamp), 3.9f, 0.11f));
    assert(near(s.pump_start_s, 5.0f, 0.01f) && near(s.pump_stop_s, 40.0f, 0.01f));
    assert(near(s.infusion_s, 24.1f, 0.01f));
    assert(near(s.infusion_cup_g_s, 23.9f / 24.1f, 0.01f));
    assert(near(s.infusion_ml_s, 1.5f, 0.02f));
    // Poids relatif au premier échantillon (10 g de tare restante).
    assert(near(s.cup_weight_g, 24.5f, 0.06f));
    assert(near(s.drip_gain_g, 0.5f, 0.06f) && s.drip_done);
    assert(near(s.start_temperature_c, 90.0f) && near(s.min_temperature_c, 88.9f, 0.02f));
    assert(near(s.max_temperature_c, 91.3f, 0.02f));
    // Moyenne pondérée par la tasse : NTC de 88,9 à 91,3 °C pendant l'écoulement.
    assert(near(s.cup_mean_c, 90.1f, 0.05f));
    assert(near(s.axis_s, s.segment_end_s[static_cast<size_t>(ShotSegment::kDrip)]));
  }
  // Échelle : 30 s de pompe prévus + 4 s de réserve, +25 % à 90 % de l'arrêt
  // prévu, puis l'arrêt réel.
  {
    assert(near(run(3.0f).summary().axis_s, 5.0f + 30.0f + 4.0f));
    assert(near(run(20.0f).summary().axis_s, 35.0f + 4.0f));
    assert(near(run(32.0f).summary().axis_s, 35.0f * 1.25f + 4.0f));
    const auto& stopped = run(41.0f).summary();
    assert(!stopped.drip_done && near(stopped.axis_s, 44.0f));
  }
  // Pression jamais atteinte : la montée devient l'infusion à l'arrêt.
  {
    const auto& running = run(30.0f, true, 6.0f).summary();
    assert(running.has(ShotSegment::kRamp) && std::isnan(running.infusion_s));
    const auto& s = run(45.0f, true, 6.0f).summary();
    assert(!s.has(ShotSegment::kRamp) && s.has(ShotSegment::kInfusion));
    assert(near(s.infusion_s, 28.0f, 0.01f));
  }
  // Sans balance : poids et débit en tasse inconnus, débit amont connu.
  {
    const auto& s = run(60.0f, false).summary();
    assert(std::isnan(s.cup_weight_g) && std::isnan(s.infusion_cup_g_s));
    assert(std::isnan(s.cup_mean_c) && near(s.infusion_ml_s, 1.5f, 0.02f));
    assert(s.drip_done);
  }
  // Tasse retirée pendant les gouttes : poids figé, gouttes terminées.
  {
    const auto& s = run(60.0f, true, 9.0f, 41.0f).summary();
    assert(s.drip_done && near(s.cup_weight_g, 24.25f, 0.06f));
  }
  // Une purge efface le résumé.
  {
    auto builder = run(10.0f);
    builder.clear();
    assert(!builder.summary().active);
  }
  return 0;
}
