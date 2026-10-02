#include <cassert>
#include <cmath>
#include "core/calibration_machine.h"
#include "core/thermal_control.h"

int main() {
  using core::calibration_machine::boiler_sensor_temperature_c;
  using core::calibration_machine::boiler_user_temperature_c;
  static_assert(core::calibration_machine::kBoilerNtcR0Ohm == 47000.0f);
  static_assert(core::calibration_machine::kBoilerNtcBetaK == 3930.0f);
  static_assert(boiler_sensor_temperature_c(90.0f) == 100.5f);
  static_assert(boiler_user_temperature_c(104.5f) == 94.0f);
  using Mode = core::thermal::Controller::Mode;
  core::thermal::Controller controller;
  auto out = controller.step(1000, 20, 90, true, true, Mode::kIdle);
  assert(out.power_permille == 1000 && !out.ready);
  out = controller.step(1250, 20, 90, true, true, Mode::kBrew);
  assert(out.power_permille == 450);  // sans débit mesuré : repli de l'infusion
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

  // Repos : pertes 0,45 W/K au-dessus de 24,5 °C (2,45 % à 90 °C), plus
  // 6 % par °C d'erreur prédite.
  core::thermal::Controller fine;
  assert(fine.step(1000, 89.925f, 90, true, true, Mode::kIdle).power_permille == 29);
  fine.reset();
  assert(fine.step(1000, 89.875f, 90, true, true, Mode::kIdle).power_permille == 32);
  core::thermal::Controller above;
  assert(above.step(1000, 90.5f, 90, true, true, Mode::kIdle).power_permille == 0);

  // Une température stable à la cible garde une commande proche des pertes.
  core::thermal::Controller hold;
  for (uint64_t now = 1000; now <= 60000; now += 250) {
    out = hold.step(now, 90, 90, true, true, Mode::kIdle);
    assert(out.power_permille >= 23 && out.power_permille <= 26);
  }

  // Chaleur commandée mais pas encore visible : une NTC arrivée à la
  // consigne ne relance pas la chauffe au-delà des pertes. Une fois le
  // retard de 23,5 s écoulé, la commande revient vers le maintien.
  core::thermal::Controller pending;
  for (uint64_t now = 1000; now <= 11000; now += 250)
    pending.step(now, 89.0f, 90, true, true, Mode::kIdle);
  out = pending.step(11250, 90.0f, 90, true, true, Mode::kIdle);
  assert(out.power_permille < 25);
  for (uint64_t now = 11500; now <= 45000; now += 250)
    out = pending.step(now, 90.0f, 90, true, true, Mode::kIdle);
  assert(out.power_permille >= 15 && out.power_permille <= 30);

  // Une erreur persistante ne doit pas faire augmenter la puissance pendant
  // que l'effet des commandes des 23,5 dernières secondes est encore attendu.
  core::thermal::Controller delayed;
  const uint16_t initial_power = delayed.step(1000, 89.0f, 90, true, true, Mode::kIdle).power_permille;
  for (uint64_t now = 1250; now <= 21000; now += 250)
    out = delayed.step(now, 89.0f, 90, true, true, Mode::kIdle);
  assert(out.power_permille < initial_power);
  assert(delayed.step(21250, 91.0f, 90, true, true, Mode::kIdle).power_permille == 0);

  // Infusion : la commande compense l'eau admise (3,5 % + 24,56 % par ml/s,
  // bornée à 90 %), sans filtre et sans dépendre de la NTC.
  core::thermal::Controller infusion;
  infusion.step(1000, 90.0f, 90, true, true, Mode::kIdle);
  assert(infusion.step(1250, 90.0f, 90, true, true, Mode::kBrew).power_permille == 450);
  core::thermal::Controller infusion_flow;
  assert(infusion_flow.step(1000, 90, 90, true, true, Mode::kBrew, 3.6f, true).power_permille == 900);
  assert(infusion_flow.step(1250, 90, 90, true, true, Mode::kBrew, 2, true).power_permille == 526);
  assert(infusion_flow.step(1500, 90, 90, true, true, Mode::kBrew, 1.4f, true).power_permille == 379);
  // Un pic de précharge ne retire pas l'appoint : seule la sécurité à
  // consigne + 4 °C coupe (trou de chauffe du 28 septembre à 13 h 01).
  core::thermal::Controller infusion_warm, infusion_hot, infusion_cold;
  assert(infusion_warm.step(1000, 93.9f, 90, true, true, Mode::kBrew, 2, true).power_permille == 526);
  assert(infusion_hot.step(1000, 94.1f, 90, true, true, Mode::kBrew, 2, true).power_permille == 0);
  assert(infusion_cold.step(1000, 78, 90, true, true, Mode::kBrew, 4, true).power_permille == 900);
  assert(infusion_cold.step(1250, 78, 90, true, true, Mode::kIdle).power_permille <= 350);

  // Fin prévue à 11 s ou moins : la chauffe est coupée jusqu'à l'arrêt de la
  // pompe, même si l'estimation remonte ou devient inconnue.
  core::thermal::Controller infusion_end;
  infusion_end.step(1000, 90, 90, true, true, Mode::kIdle);
  assert(infusion_end.step(1250, 90, 90, true, true, Mode::kBrew, 2, true, 20.0f).power_permille == 526);
  assert(infusion_end.step(1500, 90, 90, true, true, Mode::kBrew, 2, true, 11.0f).power_permille == 0);
  assert(infusion_end.step(1750, 90, 90, true, true, Mode::kBrew, 2, true, 14.0f).power_permille == 0);
  assert(infusion_end.step(2000, 90, 90, true, true, Mode::kBrew, 2, true).power_permille == 0);
  infusion_end.step(2250, 90, 90, true, true, Mode::kIdle);
  assert(infusion_end.step(2500, 90, 90, true, true, Mode::kBrew, 2, true).power_permille == 526);

  // La loi d'infusion n'hérite pas de l'état du régulateur de repos : une
  // intégrale accumulée au repos ne change pas la commande d'infusion.
  core::thermal::Controller idle_then_brew;
  for (uint64_t now = 1000; now <= 20000; now += 250)
    idle_then_brew.step(now, 87.0f, 90, true, true, Mode::kIdle);
  assert(idle_then_brew.step(20250, 87.0f, 90, true, true, Mode::kBrew, 2, true).power_permille == 526);

  // Temps restant avant l'arrêt : au temps, direct ; au poids, sur le débit
  // en tasse des 2 dernières secondes, une fois 3 g atteints.
  using core::thermal::BrewEndEstimator;
  assert(std::fabs(BrewEndEstimator::by_time(17000, 28.0f) - 11.0f) < 1e-4f);
  BrewEndEstimator end;
  float remaining = 0.0f;
  for (uint64_t now = 0; now <= 1750; now += 250) {
    remaining = end.by_weight(now, 1.5f * static_cast<float>(now) / 1000.0f, 21.0f);
    assert(std::isnan(remaining));
  }
  remaining = end.by_weight(2000, 3.0f, 21.0f);
  assert(std::fabs(remaining - 12.0f) < 1e-3f);
  remaining = end.by_weight(2250, 3.375f, 21.0f);
  assert(std::fabs(remaining - 11.75f) < 1e-3f);
  BrewEndEstimator stalled;
  for (uint64_t now = 0; now <= 2000; now += 250)
    remaining = stalled.by_weight(now, 5.0f + 0.2f * static_cast<float>(now) / 1000.0f, 21.0f);
  assert(std::isnan(remaining));

  // La précharge applique immédiatement sa puissance fixe sans débit. Elle
  // est coupée si la NTC est déjà à plus de 0,5 °C au-dessus de la cible.
  core::thermal::Controller preheat, preheat_hot;
  assert(preheat.step(1000, 90, 90, true, true, Mode::kThermalPreheat).power_permille == 900);
  assert(preheat_hot.step(1000, 90.6f, 90, true, true, Mode::kThermalPreheat).power_permille == 0);
  core::thermal::Controller preheat_after_flow;
  preheat_after_flow.step(1000, 90, 90, true, true, Mode::kBrew);
  preheat_after_flow.step(1250, 90, 90, true, true, Mode::kIdle);
  assert(preheat_after_flow.step(1500, 90, 90, true, true,
                                 Mode::kThermalPreheat).power_permille == 900);

  // Nouvelle période SSR : une seule fois au début de la précharge, puis une
  // fois au début de l'écoulement, sur la première commande non nulle.
  core::thermal::Controller window;
  assert(!window.step(1000, 89.9f, 90, true, true, Mode::kIdle).restart_window);
  auto restart = window.step(1250, 89.9f, 90, true, true, Mode::kThermalPreheat);
  assert(restart.power_permille == 900 && restart.restart_window);
  assert(!window.step(1500, 89.9f, 90, true, true, Mode::kThermalPreheat).restart_window);
  assert(!window.step(1750, 89.9f, 90, true, true, Mode::kThermalPreheat).restart_window);
  restart = window.step(2000, 89.9f, 90, true, true, Mode::kBrew);
  assert(restart.power_permille == 450 && restart.restart_window);
  assert(!window.step(2250, 89.9f, 90, true, true, Mode::kBrew, 3.5f, true).restart_window);
  assert(!window.step(2500, 89.9f, 90, true, true, Mode::kBrew, 1.5f, true).restart_window);
  assert(!window.step(2750, 90.0f, 90, true, true, Mode::kIdle).restart_window);
  // Cycle suivant : le flag revient.
  assert(window.step(3000, 89.9f, 90, true, true, Mode::kThermalPreheat).restart_window);

  // Précharge supprimée (NTC trop chaude) : pas de flag tant que la commande
  // est nulle ; il part avec la première commande non nulle de la phase.
  core::thermal::Controller window_hot;
  restart = window_hot.step(1000, 90.6f, 90, true, true, Mode::kThermalPreheat);
  assert(restart.power_permille == 0 && !restart.restart_window);
  assert(window_hot.step(1250, 90.4f, 90, true, true, Mode::kThermalPreheat).restart_window);
  // Sans précharge, le premier paquet de l'écoulement porte le flag.
  core::thermal::Controller window_no_preheat;
  window_no_preheat.step(1000, 89.9f, 90, true, true, Mode::kIdle);
  assert(window_no_preheat.step(1250, 89.9f, 90, true, true, Mode::kBrew).restart_window);
  // La purge n'en porte jamais.
  core::thermal::Controller window_purge;
  for (uint64_t now = 1000; now <= 3000; now += 250)
    assert(!window_purge.step(now, 89.0f, 90, true, true, Mode::kPurge, 3.5f, true).restart_window);

  // L'appoint de débit est linéaire entre 2 et 4 ml/s et se retire dès que
  // le débit baisse. Il reste limité au mode écoulement et aux mesures fraîches.
  core::thermal::Controller flow_low, flow_mid, flow_high, flow_stale;
  assert(flow_low.step(1000, 90, 90, true, true, Mode::kPurge, 2, true).power_permille == 180);
  assert(flow_mid.step(1000, 90, 90, true, true, Mode::kPurge, 3, true).power_permille == 230);
  assert(flow_high.step(1000, 90, 90, true, true, Mode::kPurge, 4, true).power_permille == 280);
  assert(flow_high.step(1250, 90, 90, true, true, Mode::kPurge, 2, true).power_permille == 180);
  assert(flow_stale.step(1000, 90, 90, true, true, Mode::kBrew, 4, false).power_permille == 450);
  assert(flow_stale.step(1250, 90, 90, true, true, Mode::kIdle, 4, true).power_permille <= 350);
  // Le plancher et l'appoint de débit de la purge décroissent ensemble : une
  // NTC légèrement au-dessus de la cible ne coupe plus la chauffe d'un coup.
  core::thermal::Controller purge_half, purge_one, purge_late, purge_no_flow;
  assert(purge_half.step(1000, 90.5f, 90, true, true, Mode::kPurge, 4, true).power_permille == 210);
  assert(purge_one.step(1000, 91.0f, 90, true, true, Mode::kPurge, 4, true).power_permille == 140);
  assert(purge_late.step(1000, 91.8f, 90, true, true, Mode::kPurge, 4, true).power_permille == 28);
  assert(purge_no_flow.step(1000, 91.0f, 90, true, true, Mode::kPurge, 4, false).power_permille == 90);
  core::thermal::Controller purge_rising;
  assert(purge_rising.step(1000, 90.0f, 90, true, true, Mode::kPurge, 4, true).power_permille == 280);
  assert(purge_rising.step(1250, 90.5f, 90, true, true, Mode::kPurge, 4, true).power_permille == 210);
  assert(purge_rising.step(1500, 92.0f, 90, true, true, Mode::kPurge, 4, true).power_permille == 0);
  core::thermal::Controller flow_capped, flow_too_hot, flow_at_limit;
  assert(flow_capped.step(1000, 85.1f, 90, true, true, Mode::kPurge, 4, true).power_permille == 450);
  assert(flow_at_limit.step(1000, 92.0f, 90, true, true, Mode::kPurge, 4, true).power_permille == 0);
  assert(flow_too_hot.step(1000, 92.1f, 90, true, true, Mode::kPurge, 4, true).power_permille == 0);

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
