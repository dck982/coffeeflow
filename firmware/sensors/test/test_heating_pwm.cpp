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
  pwm.reset();
  assert(!pwm.tick(0));
}
