#pragma once

#include <cstdint>

// Le dimmer conserve son dernier niveau si l'écriture I2C d'arrêt échoue.
// Tant que la vanne est fermée, réaffirmer zéro même après un succès : cela
// couvre aussi un redémarrage du contrôleur ou une écriture perdue.
class DimmerZeroGuard {
 public:
  static constexpr int64_t kRetryAfterErrorUs = 100 * 1000;
  static constexpr int64_t kRefreshUs = 1000 * 1000;

  template <typename WriteZero>
  void tick(int64_t now_us, bool valve_open, WriteZero write_zero) {
    if (valve_open || now_us < next_attempt_us_) return;
    next_attempt_us_ = now_us + (write_zero() ? kRefreshUs : kRetryAfterErrorUs);
  }

 private:
  int64_t next_attempt_us_ = 0;
};
