#include <cassert>
#include "core/thermal_control.h"

int main() {
  core::thermal::Controller controller;
  auto out = controller.step(1000, 20, 90, true, true, false);
  assert(out.power_permille == 1000 && !out.ready);
  out = controller.step(1250, 20, 90, true, true, true);
  assert(out.power_permille == 1000);  // la compensation infusion reste bornée
  out = controller.step(1500, 20, 90, true, false, false);
  assert(out.power_permille == 0 && !out.ready);
  out = controller.step(1750, 86, 90, true, true, false);
  assert(out.power_permille > 0 && out.power_permille < 1000);
  out = controller.step(2000, 90, 90, true, true, false);
  assert(!out.ready);
  for (uint64_t now = 2500; now <= 5500; now += 500)
    out = controller.step(now, 90, 90, true, true, false);
  assert(out.ready);
  out = controller.step(5750, 90.6f, 90, true, true, false);
  assert(!out.ready);
  out = controller.step(6000, 106, 90, true, true, false);
  assert(out.power_permille == 0 && !out.ready);
  out = controller.step(6250, 20, 90, false, true, false);
  assert(out.power_permille == 0 && !out.ready);

  core::thermal::Controller fine;
  assert(fine.step(1000, 89.925f, 90, true, true, false).power_permille == 6);
  fine.reset();
  assert(fine.step(1000, 89.875f, 90, true, true, false).power_permille == 10);
}
