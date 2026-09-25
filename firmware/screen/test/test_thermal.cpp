#include <cassert>
#include "core/thermal_control.h"

int main() {
  using Mode = core::thermal::Controller::Mode;
  core::thermal::Controller controller;
  auto out = controller.step(1000, 20, 90, true, true, Mode::kIdle);
  assert(out.power_permille == 1000 && !out.ready);
  out = controller.step(1250, 20, 90, true, true, Mode::kBrew);
  assert(out.power_permille == 350);  // l'appoint infusion reste borné
  out = controller.step(1500, 20, 90, true, false, Mode::kIdle);
  assert(out.power_permille == 0 && !out.ready);
  out = controller.step(1750, 86, 90, true, true, Mode::kIdle);
  assert(out.power_permille > 0 && out.power_permille < 1000);
  out = controller.step(2000, 90, 90, true, true, Mode::kIdle);
  assert(!out.ready);
  for (uint64_t now = 2500; now <= 5500; now += 500)
    out = controller.step(now, 90, 90, true, true, Mode::kIdle);
  assert(out.ready);
  out = controller.step(5750, 90.6f, 90, true, true, Mode::kIdle);
  assert(out.ready);
  out = controller.step(6000, 91.1f, 90, true, true, Mode::kIdle);
  assert(!out.ready);
  out = controller.step(6250, 106, 90, true, true, Mode::kIdle);
  assert(out.power_permille == 0 && !out.ready);
  out = controller.step(6500, 20, 90, false, true, Mode::kIdle);
  assert(out.power_permille == 0 && !out.ready);

  core::thermal::Controller fine;
  assert(fine.step(1000, 89.925f, 90, true, true, Mode::kIdle).power_permille == 41);
  fine.reset();
  assert(fine.step(1000, 89.875f, 90, true, true, Mode::kIdle).power_permille == 45);

  // Une température stable à la cible garde une impulsion de maintien.
  core::thermal::Controller hold;
  for (uint64_t now = 1000; now <= 30000; now += 250)
    assert(hold.step(now, 90, 90, true, true, Mode::kIdle).power_permille == 35);

  // La descente déclenche la chauffe avant le passage sous la consigne.
  core::thermal::Controller cooling;
  cooling.step(1000, 90.6f, 90, true, true, Mode::kIdle);
  out = cooling.step(2000, 90.5f, 90, true, true, Mode::kIdle);
  assert(out.power_permille > 0);

  // Pendant une montée rapide, la chaleur résiduelle reste prioritaire :
  // le terme de maintien ne force pas le chauffage.
  core::thermal::Controller rising;
  rising.step(1000, 89.5f, 90, true, true, Mode::kIdle);
  rising.step(2000, 89.7f, 90, true, true, Mode::kIdle);
  out = rising.step(3000, 89.9f, 90, true, true, Mode::kIdle);
  assert(out.power_permille == 0);

  // Une erreur persistante ne doit pas faire augmenter la puissance pendant
  // que l'effet des commandes des 22 dernières secondes est encore attendu.
  core::thermal::Controller delayed;
  const uint16_t initial_power = delayed.step(1000, 89.0f, 90, true, true, Mode::kIdle).power_permille;
  for (uint64_t now = 1250; now <= 21000; now += 250)
    out = delayed.step(now, 89.0f, 90, true, true, Mode::kIdle);
  assert(out.power_permille < initial_power);
  assert(delayed.step(21250, 91.0f, 90, true, true, Mode::kIdle).power_permille == 0);

  // Le mode infusion conserve son appoint sans attendre le filtre de sortie.
  core::thermal::Controller infusion;
  infusion.step(1000, 90.0f, 90, true, true, Mode::kIdle);
  assert(infusion.step(1250, 90.0f, 90, true, true, Mode::kBrew).power_permille >= 180);

  // La baisse mesurée après une purge n'est pas extrapolée sur 20 secondes :
  // l'appoint et la reprise restent bornés même si la NTC chute fortement.
  core::thermal::Controller purge;
  purge.step(1000, 89.7f, 90, true, true, Mode::kIdle);
  for (uint64_t now = 1250; now <= 9000; now += 250) {
    out = purge.step(now, 89.7f, 90, true, true, Mode::kPurge);
    assert(out.power_permille >= 180 && out.power_permille <= 350);
  }
  out = purge.step(9250, 87.3f, 90, true, true, Mode::kIdle);
  assert(out.power_permille <= 350);
  for (uint64_t now = 9500; now <= 29000; now += 250) {
    out = purge.step(now, 82.0f, 90, true, true, Mode::kIdle);
    assert(out.power_permille <= 350);
  }
  out = purge.step(29250, 94.0f, 90, true, true, Mode::kIdle);
  assert(out.power_permille == 0);

  // La chaleur en transit et la pente montante décrivent en partie la même
  // énergie : la reprise ne doit pas s'arrêter alors que la NTC est à 83 °C.
  core::thermal::Controller recovery_rise;
  for (uint64_t now = 1000; now <= 9000; now += 250)
    recovery_rise.step(now, 90.0f, 90, true, true, Mode::kPurge);
  for (uint64_t now = 9250; now <= 20000; now += 250)
    recovery_rise.step(now, 79.4f, 90, true, true, Mode::kIdle);
  for (uint64_t now = 20250; now <= 28000; now += 250)
    out = recovery_rise.step(now, 79.4f + (now - 20000) / 2000.0f,
                             90, true, true, Mode::kIdle);
  assert(out.power_permille > 0 && out.power_permille <= 350);

  // Une purge lancée loin de la cible sert au refroidissement : même si la
  // NTC passe sous la consigne durant l'écoulement, elle ne relance pas le SSR.
  core::thermal::Controller cooling_purge;
  cooling_purge.step(1000, 75.0f, 65, true, true, Mode::kIdle);
  assert(cooling_purge.step(1250, 75.0f, 65, true, true, Mode::kPurge).power_permille == 0);
  assert(cooling_purge.step(2250, 70.0f, 65, true, true, Mode::kPurge).power_permille == 0);
  assert(cooling_purge.step(3250, 63.0f, 65, true, true, Mode::kPurge).power_permille == 0);
  out = cooling_purge.step(3500, 63.0f, 65, true, true, Mode::kIdle);
  assert(out.power_permille > 0 && out.power_permille <= 350);

  core::thermal::Controller near_purge;
  near_purge.step(1000, 69.9f, 65, true, true, Mode::kIdle);
  near_purge.step(1250, 69.9f, 65, true, true, Mode::kPurge);
  assert(near_purge.step(2250, 65.0f, 65, true, true, Mode::kPurge).power_permille >= 180);

  core::thermal::Controller cold_purge;
  assert(cold_purge.step(1000, 59.0f, 65, true, true, Mode::kPurge).power_permille == 0);
}
