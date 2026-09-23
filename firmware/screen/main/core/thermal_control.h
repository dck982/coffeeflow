#pragma once

#include <algorithm>
#include <cmath>
#include <cstdint>

namespace core::thermal {

// Réglage initial prudent. Les gains et l'avance doivent être recalibrés sur
// une capture de chauffe/refroidissement de la machine réelle.
class Controller {
 public:
  struct Output { uint16_t power_permille; bool ready; };

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
      last_temperature_c_ = temperature_c;
      last_ms_ = now_ms;
    }
    const float dt_s = std::min(static_cast<float>(now_ms - last_ms_) / 1000.0f, 1.0f);
    if (dt_s > 0) {
      const float measured_slope = (temperature_c - last_temperature_c_) / dt_s;
      const float alpha = dt_s / (5.0f + dt_s);
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

    // La pente positive anticipe la chaleur encore en route vers la sonde.
    const float predicted_c = temperature_c + std::max(0.0f, slope_c_per_s_) * 15.0f;
    const float predicted_error = target_c - predicted_c;
    if (std::fabs(error) < 8.0f && dt_s > 0) {
      integral_pct_ = std::clamp(integral_pct_ + 0.18f * error * dt_s, 0.0f, 35.0f);
    }
    if (error < -0.5f) integral_pct_ = 0;
    float power = error > 15.0f && predicted_error > 10.0f
                      ? 80.0f
                      : 8.0f * predicted_error + integral_pct_;
    if (brewing && error > -0.5f) power += 15.0f;
    return {static_cast<uint16_t>(std::lround(std::clamp(power, 0.0f, 80.0f) * 10.0f)), ready};
  }

  void reset() {
    last_ms_ = 0;
    ready_since_ms_ = 0;
    integral_pct_ = 0;
    slope_c_per_s_ = 0;
  }

 private:
  uint64_t last_ms_ = 0;
  uint64_t ready_since_ms_ = 0;
  float last_temperature_c_ = 0;
  float slope_c_per_s_ = 0;
  float integral_pct_ = 0;
  float target_c_ = 0;
};

}  // namespace core::thermal
