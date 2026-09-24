#pragma once

#include <algorithm>
#include <cstdint>

// Répartit les faibles consignes sur plusieurs fenêtres de 5 s. La sortie
// minimale est une impulsion de 100 ms, soit 2 % d'une fenêtre ; l'accumulateur
// conserve le crédit des fenêtres sans impulsion (1 % = 100 ms / 10 s).
class HeatingPwm {
 public:
  static constexpr int64_t kPeriodUs = 5 * 1000 * 1000;
  static constexpr int64_t kQuantumUs = 100 * 1000;

  void reset() { *this = HeatingPwm{}; }
  bool active() const { return active_; }

  void set_power(int64_t now_us, uint16_t permille) {
    if (permille > 1000) { reset(); return; }
    if (!active_) {
      if (permille == 0) return;
      active_ = true;
      window_start_us_ = now_us;
      credit_us_ = kQuantumUs / 2;
      power_permille_ = permille;
      allocate_window();
      return;
    }
    const uint16_t previous_permille = power_permille_;
    power_permille_ = permille;
    if (permille == 0) credit_us_ = kQuantumUs / 2;
    if (advance_window(now_us)) {
      if (permille == 0) window_closed_ = true;
      return;
    }
    if (permille == 0) {
      window_on_us_ = 0;
      window_closed_ = true;
      output_on_ = false;
      return;
    }
    // Une fois la sortie éteinte, aucune consigne ne peut la rallumer avant
    // la prochaine fenêtre. La nouvelle consigne reste celle de cette fenêtre.
    if (window_closed_ || (evaluated_ && !output_on_)) {
      window_closed_ = true;
      return;
    }
    if (permille == previous_permille) return;
    const int64_t requested_us = kPeriodUs * permille / 1000;
    if (requested_us > window_on_us_) {
      window_on_us_ = requested_us / kQuantumUs * kQuantumUs;
    } else {
      // Une baisse importante coupe la fenêtre courante au plus tard après
      // le quantum minimal ; le crédit futur suit déjà la nouvelle consigne.
      window_on_us_ = std::min(window_on_us_,
                               std::max(kQuantumUs, requested_us / kQuantumUs * kQuantumUs));
    }
    if (now_us - window_start_us_ >= window_on_us_) {
      window_closed_ = true;
      output_on_ = false;
    }
  }

  bool tick(int64_t now_us) {
    if (!active_) return false;
    advance_window(now_us);
    const bool on = !window_closed_ && now_us >= window_start_us_ &&
                    now_us - window_start_us_ < window_on_us_;
    if (output_on_ && !on) window_closed_ = true;
    output_on_ = on;
    evaluated_ = true;
    return on;
  }

 private:
  bool advance_window(int64_t now_us) {
    if (now_us - window_start_us_ < kPeriodUs) return false;
    window_start_us_ = now_us;
    window_closed_ = false;
    output_on_ = false;
    evaluated_ = false;
    allocate_window();
    return true;
  }

  void allocate_window() {
    credit_us_ += kPeriodUs * power_permille_ / 1000;
    window_on_us_ = std::min(kPeriodUs, credit_us_ / kQuantumUs * kQuantumUs);
    credit_us_ -= window_on_us_;
  }

  bool active_ = false;
  uint16_t power_permille_ = 0;
  int64_t window_start_us_ = 0;
  int64_t window_on_us_ = 0;
  int64_t credit_us_ = 0;
  bool window_closed_ = false;
  bool output_on_ = false;
  bool evaluated_ = false;
};
