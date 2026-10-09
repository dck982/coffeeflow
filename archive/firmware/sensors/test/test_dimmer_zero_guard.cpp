#include <cassert>
#include <cstdint>

#include "dimmer_zero_guard.h"

int main() {
  DimmerZeroGuard guard;
  uint8_t dimmer_level = 100;
  bool i2c_ok = false;
  int writes = 0;
  auto write_zero = [&] {
    ++writes;
    if (i2c_ok) dimmer_level = 0;
    return i2c_ok;
  };

  // L'arrêt immédiat a fermé la vanne, mais son écriture I2C a échoué.
  guard.tick(0, false, write_zero);
  assert(writes == 1 && dimmer_level == 100);
  guard.tick(99'999, false, write_zero);
  assert(writes == 1);

  // Une reprise du bus doit arrêter la pompe sans nouvelle commande CAN.
  i2c_ok = true;
  guard.tick(100'000, false, write_zero);
  assert(writes == 2 && dimmer_level == 0);

  // À l'arrêt, le zéro est réaffirmé périodiquement, mais jamais vanne ouverte.
  dimmer_level = 80;
  guard.tick(1'100'000, true, write_zero);
  assert(writes == 2 && dimmer_level == 80);
  guard.tick(1'100'000, false, write_zero);
  assert(writes == 3 && dimmer_level == 0);
}
