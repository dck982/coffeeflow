#include <cassert>
#include "heating_pwm.h"

int main() {
  HeatingPwm pwm;
  pwm.set_power(0, 10);  // 1,0 % = 10 ms / 1 s, accumulés en impulsions de 100 ms
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
  pwm.set_power(400000, 6);
  assert(!pwm.tick(400000));  // baisse appliquée sans attendre la fin de la fenêtre

  // La hausse reçue après l'arrêt est retenue pour la fenêtre suivante.
  pwm.set_power(500000, 1000);
  assert(!pwm.tick(500000));
  assert(!pwm.tick(900000));
  assert(pwm.tick(1000000));

  pwm.reset();
  pwm.set_power(0, 200);
  assert(pwm.tick(0));
  pwm.set_power(100000, 800);  // une hausse prolonge une impulsion en cours
  assert(pwm.tick(400000));
  pwm.set_power(500000, 100);
  assert(!pwm.tick(500000));
  pwm.set_power(600000, 1000);
  assert(!pwm.tick(600000));
  assert(pwm.tick(1000000));

  pwm.reset();
  pwm.set_power(0, 1000);
  assert(pwm.tick(0));
  for (int i = 1; i < 10; ++i) {
    const int64_t t = static_cast<int64_t>(i) * 100000;
    pwm.set_power(t, i % 2 == 0 ? 1000 : 0);
    assert(!pwm.tick(t));  // aucun rallumage pendant la fenêtre
  }
  pwm.set_power(950000, 1000);
  assert(!pwm.tick(950000));
  assert(pwm.tick(1000000));  // dernière consigne non nulle retenue

  pwm.reset();
  pwm.set_power(0, 1000);
  assert(pwm.tick(0));
  pwm.set_power(200000, 0);  // arrêt et expiration simulée du bail
  assert(!pwm.tick(200000));
  pwm.set_power(500000, 0);
  pwm.set_power(600000, 1000);
  assert(!pwm.tick(600000));
  assert(pwm.tick(1000000));

  pwm.reset();
  pwm.set_power(0, 1000);
  assert(pwm.tick(0));
  pwm.set_power(200000, 0);
  assert(!pwm.tick(200000));
  pwm.set_power(400000, 10);
  assert(!pwm.tick(400000));
  for (int64_t t = 1000000; t < 5000000; t += 1000000)
    assert(!pwm.tick(t));
  assert(pwm.tick(5000000));  // le crédit de 1 % atteint le quantum de 100 ms
  assert(!pwm.tick(5100000));

  pwm.reset();
  pwm.set_power(0, 10);  // le crédit initial ne suffit pas encore pour 100 ms
  assert(!pwm.tick(0));
  pwm.set_power(200000, 1000);
  assert(!pwm.tick(200000));
  assert(pwm.tick(1000000));

  pwm.reset();
  assert(!pwm.tick(0));
  pwm.set_power(0, 0);
  pwm.set_power(100000, 1000);
  assert(pwm.tick(100000));  // pas de délai au premier démarrage

  pwm.reset();
  pwm.set_power(0, 1000);
  for (int64_t t = 0; t <= 20000000; t += 100000)
    assert(pwm.tick(t));  // 100 % reste ON à la frontière des fenêtres

  // Nouvelle période à la demande : la hausse reçue après l'impulsion de repos
  // part aussitôt, sans attendre la fin de la période en cours.
  pwm.reset();
  pwm.set_power(0, 30);  // repos, 3 %
  on_ticks = 0;
  for (int64_t t = 0; t < 10000000; t += 100000)
    if (pwm.tick(t)) ++on_ticks;
  assert(on_ticks == 3);
  pwm.tick(10000000);  // période en cours depuis 10 s
  assert(!pwm.tick(10300000));
  pwm.restart(10300000, 900);
  on_ticks = 0;
  for (int64_t t = 10300000; t < 11300000; t += 100000)
    if (pwm.tick(t)) ++on_ticks;
  assert(on_ticks == 9);  // 900 ms dans la nouvelle période, dès 10,3 s
  assert(!pwm.tick(11200000));
  assert(pwm.tick(11300000));  // la période suivante part de 11,3 s

  // Comparaison : la même hausse sans nouvelle période attend jusqu'à 11 s.
  pwm.reset();
  pwm.set_power(0, 30);
  for (int64_t t = 0; t <= 10000000; t += 100000) pwm.tick(t);
  pwm.tick(10300000);
  pwm.set_power(10300000, 900);
  assert(!pwm.tick(10300000));
  assert(!pwm.tick(10900000));
  assert(pwm.tick(11000000));

  // Une nouvelle période pendant une impulsion la prolonge sans coupure.
  pwm.reset();
  pwm.set_power(0, 200);
  assert(pwm.tick(0));
  pwm.restart(100000, 900);
  for (int64_t t = 100000; t < 1000000; t += 100000)
    assert(pwm.tick(t));
  assert(!pwm.tick(1000000));

  // Reçue deux fois de suite, elle ne cumule pas les périodes.
  pwm.reset();
  pwm.set_power(0, 30);
  pwm.tick(0);
  pwm.restart(300000, 500);
  pwm.restart(350000, 500);
  on_ticks = 0;
  for (int64_t t = 350000; t < 1350000; t += 50000)
    if (pwm.tick(t)) ++on_ticks;
  assert(on_ticks == 10);  // 500 ms, pas 1 s

  // Avec une consigne nulle, elle se comporte comme `set_power`.
  pwm.reset();
  pwm.set_power(0, 1000);
  assert(pwm.tick(0));
  pwm.restart(200000, 0);
  assert(!pwm.tick(200000));
  pwm.restart(300000, 1000);
  assert(pwm.tick(300000));  // une consigne non nulle rouvre une période
}
