#include <cassert>
#include "heating_pwm.h"

int main() {
  HeatingPwm pwm;
  pwm.set_power(0, 10);  // 1,0 % = 50 ms / 5 s
  int on_ticks = 0;
  int longest_run = 0;
  int run = 0;
  for (int64_t t = 0; t < 100000000; t += 100000) {
    if (pwm.tick(t)) {
      ++on_ticks;
      longest_run = std::max(longest_run, ++run);
    } else run = 0;
  }
  assert(on_ticks == 10);
  assert(longest_run == 1);  // impulsions de 100 ms

  pwm.reset();
  pwm.set_power(0, 6);  // 0,6 %
  on_ticks = 0;
  for (int64_t t = 0; t < 100000000; t += 100000)
    if (pwm.tick(t)) ++on_ticks;
  assert(on_ticks == 6);

  pwm.reset();
  pwm.set_power(0, 800);
  assert(pwm.tick(0));
  pwm.set_power(2000000, 6);
  assert(!pwm.tick(2000000));  // baisse appliquée sans attendre 5 s

  // La hausse reçue après l'arrêt est retenue pour la fenêtre suivante.
  pwm.set_power(2100000, 1000);
  assert(!pwm.tick(2100000));
  assert(!pwm.tick(4900000));
  assert(pwm.tick(5000000));

  pwm.reset();
  pwm.set_power(0, 200);
  assert(pwm.tick(0));
  pwm.set_power(500000, 800);  // une hausse prolonge une impulsion en cours
  assert(pwm.tick(2000000));
  pwm.set_power(2100000, 100);
  assert(!pwm.tick(2100000));
  pwm.set_power(2200000, 1000);
  assert(!pwm.tick(2200000));
  assert(pwm.tick(5000000));

  pwm.reset();
  pwm.set_power(0, 1000);
  assert(pwm.tick(0));
  for (int i = 1; i < 50; ++i) {
    const int64_t t = static_cast<int64_t>(i) * 100000;
    pwm.set_power(t, i % 2 == 0 ? 1000 : 0);
    assert(!pwm.tick(t));  // aucun rallumage pendant la fenêtre
  }
  pwm.set_power(4950000, 1000);
  assert(!pwm.tick(4950000));
  assert(pwm.tick(5000000));  // dernière consigne non nulle retenue

  pwm.reset();
  pwm.set_power(0, 1000);
  assert(pwm.tick(0));
  pwm.set_power(1000000, 0);  // arrêt et expiration simulée du bail
  assert(!pwm.tick(1000000));
  pwm.set_power(2600000, 0);
  pwm.set_power(2700000, 1000);
  assert(!pwm.tick(2700000));
  assert(pwm.tick(5000000));

  pwm.reset();
  pwm.set_power(0, 1000);
  assert(pwm.tick(0));
  pwm.set_power(1000000, 0);
  assert(!pwm.tick(1000000));
  pwm.set_power(2000000, 10);
  assert(!pwm.tick(2000000));
  assert(pwm.tick(5000000));  // 1 % démarre dès la prochaine fenêtre
  assert(!pwm.tick(5100000));

  pwm.reset();
  pwm.set_power(0, 10);  // impulsion de 100 ms à faible puissance
  assert(pwm.tick(0));
  assert(!pwm.tick(100000));
  pwm.set_power(200000, 1000);
  assert(!pwm.tick(200000));
  assert(pwm.tick(5000000));

  pwm.reset();
  assert(!pwm.tick(0));
  pwm.set_power(0, 0);
  pwm.set_power(100000, 1000);
  assert(pwm.tick(100000));  // pas de délai au premier démarrage

  pwm.reset();
  pwm.set_power(0, 1000);
  for (int64_t t = 0; t <= 20000000; t += 100000)
    assert(pwm.tick(t));  // 100 % reste ON à la frontière des fenêtres
}
