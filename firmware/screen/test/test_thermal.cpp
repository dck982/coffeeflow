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
  assert(fine.step(1000, 89.925f, 90, true, true, false).power_permille == 41);
  fine.reset();
  assert(fine.step(1000, 89.875f, 90, true, true, false).power_permille == 45);

  // Une température stable à la cible garde une impulsion de maintien.
  core::thermal::Controller hold;
  for (uint64_t now = 1000; now <= 30000; now += 250)
    assert(hold.step(now, 90, 90, true, true, false).power_permille == 35);

  // La descente déclenche la chauffe avant le passage sous la consigne.
  core::thermal::Controller cooling;
  cooling.step(1000, 90.6f, 90, true, true, false);
  out = cooling.step(2000, 90.5f, 90, true, true, false);
  assert(out.power_permille > 0);

  // Pendant une montée rapide, la chaleur résiduelle reste prioritaire :
  // le terme de maintien ne force pas le chauffage.
  core::thermal::Controller rising;
  rising.step(1000, 89.5f, 90, true, true, false);
  rising.step(2000, 89.7f, 90, true, true, false);
  out = rising.step(3000, 89.9f, 90, true, true, false);
  assert(out.power_permille == 0);

  // Une erreur persistante ne doit pas faire augmenter la puissance pendant
  // que l'effet des commandes des 22 dernières secondes est encore attendu.
  core::thermal::Controller delayed;
  const uint16_t initial_power = delayed.step(1000, 89.0f, 90, true, true, false).power_permille;
  for (uint64_t now = 1250; now <= 21000; now += 250)
    out = delayed.step(now, 89.0f, 90, true, true, false);
  assert(out.power_permille < initial_power);
  assert(delayed.step(21250, 91.0f, 90, true, true, false).power_permille == 0);

  // Le mode infusion conserve son appoint sans attendre le filtre de sortie.
  core::thermal::Controller infusion;
  infusion.step(1000, 90.0f, 90, true, true, false);
  assert(infusion.step(1250, 90.0f, 90, true, true, true).power_permille >= 180);
}
