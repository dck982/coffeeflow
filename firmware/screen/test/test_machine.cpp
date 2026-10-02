#include <cassert>
#include <cstdio>

#include "core/machine.h"

using core::machine::Config;
using core::machine::Input;
using core::machine::Machine;
using core::machine::PreinfusionMode;
using core::machine::RampdownMode;
using core::machine::State;
using core::machine::StopReason;

Config config() {
  return {36, 28, 9.0f, 0.0f, 1, .1f, 100, PreinfusionMode::kTime, 4, 35, RampdownMode::kNone,
          3, 4, 1, 100, 100, 20};
}

int main() {
  Machine thermal_preheat;
  Input input{0, false, 0};
  Config c = config();
  c.brew_preheat_time_s = 2.5f;
  assert(thermal_preheat.start(1000, c, input));
  assert(thermal_preheat.state() == State::kThermalPreheat);
  assert(thermal_preheat.tick(3499, input).dimmer == 0);
  assert(thermal_preheat.elapsed_ms(3499) == 0);
  assert(thermal_preheat.tick(3500, input).dimmer == 100);
  assert(thermal_preheat.state() == State::kFilling);
  assert(thermal_preheat.elapsed_ms(3500) == 0);
  assert(thermal_preheat.tick(4499, input).dimmer == 100);
  assert(thermal_preheat.tick(4500, input).dimmer == 35);

  Machine machine;
  input = {0, false, 0};
  c = config();
  assert(machine.start(1000, c, input));
  assert(machine.state() == State::kFilling);
  assert(machine.tick(1999, input).dimmer == 100);
  assert(machine.tick(2000, input).dimmer == 35);
  assert(machine.state() == State::kPreinfusion);
  assert(machine.tick(5999, input).dimmer == 35);
  assert(machine.tick(6000, input).dimmer == 50);  // rampe depuis kMinimumBrewPumpPct, pas depuis la pause
  assert(machine.tick(6249, input).dimmer == 50);
  assert(machine.tick(6250, input).dimmer == 55);  // 10 pas de 5 % en 2,5 s
  assert(machine.state() == State::kBrew);
  assert(machine.tick(28999, input).dimmer == 100);
  assert(machine.tick(29000, input).dimmer == 0);
  assert(machine.stop_reason() == StopReason::kTargetTime);

  Machine no_preinfusion;
  c.preinfusion_mode = PreinfusionMode::kNone;
  input = {0, false, 0};
  assert(no_preinfusion.start(1000, c, input));
  assert(no_preinfusion.state() == State::kFilling);
  assert(no_preinfusion.tick(2000, input).dimmer == 100);
  assert(no_preinfusion.state() == State::kBrew);

  Machine regulated;
  c = config();
  c.preinfusion_mode = PreinfusionMode::kNone;
  input = {0, false, 6.9f, true};
  assert(regulated.start(1000, c, input));
  assert(regulated.tick(2000, input).dimmer == 100);
  assert(regulated.tick(2199, input).dimmer == 100);  // pas avant 200 ms
  assert(regulated.tick(2200, input).dimmer == 100);  // activation à cible - 3 bar, sans saut
  input.pressure_bar = 7.0f;
  assert(regulated.tick(2400, input).dimmer == 100);
  input.pressure_bar = 8.0f;
  assert(regulated.tick(2600, input).dimmer == 85);   // terme proportionnel dominant
  input.pressure_bar = 8.5f;
  assert(regulated.tick(2800, input).dimmer == 78);
  input.pressure_bar = 9.0f;
  assert(regulated.tick(3000, input).dimmer == 71);
  input.pressure_bar = 9.5f;
  assert(regulated.tick(3200, input).dimmer == 63);
  input.pressure_valid = false;
  assert(regulated.tick(3400, input).dimmer == 63);
  input.pressure_valid = true;
  input.pressure_bar = 12.0f;
  for (uint64_t now = 3600; now <= 5600; now += 200) {
    assert(regulated.tick(now, input).dimmer == 50);
  }
  input.pressure_bar = 9.0f;
  assert(regulated.tick(5800, input).dimmer == 70);  // pas de windup à la borne basse

  // Après une indisponibilité durable, l'intégrale reprend sur le vrai temps
  // écoulé, borné à 400 ms, plutôt que de perdre les corrections sautées.
  Machine intermittent_pressure;
  c = config();
  c.preinfusion_mode = PreinfusionMode::kNone;
  input = {0, false, 7.0f, true};
  assert(intermittent_pressure.start(1000, c, input));
  assert(intermittent_pressure.tick(2000, input).dimmer == 100);
  assert(intermittent_pressure.tick(2200, input).dimmer == 100);  // activation
  input.pressure_bar = 10.0f;
  assert(intermittent_pressure.tick(2400, input).dimmer == 54);
  input.pressure_valid = false;
  assert(intermittent_pressure.tick(2600, input).dimmer == 54);
  assert(intermittent_pressure.tick(2800, input).dimmer == 54);
  assert(intermittent_pressure.tick(3000, input).dimmer == 54);
  input.pressure_valid = true;
  assert(intermittent_pressure.tick(3200, input).dimmer == 53);

  Machine safe_direct_config;
  c = config();
  c.preinfusion_mode = PreinfusionMode::kNone;
  c.brew_pump_pct = 20;
  input = {0, false, 10.0f, true};
  assert(safe_direct_config.start(1000, c, input));
  assert(safe_direct_config.tick(2000, input).dimmer == 50);
  assert(safe_direct_config.tick(2200, input).dimmer == 50);

  Machine ramp_after_regulation;
  c = config();
  c.preinfusion_mode = PreinfusionMode::kNone;
  c.rampdown_mode = RampdownMode::kTime;
  c.rampdown_lead_time_s = 3;
  input = {0, false, 10.0f, true};
  assert(ramp_after_regulation.start(1000, c, input));
  assert(ramp_after_regulation.tick(2000, input).dimmer == 100);
  assert(ramp_after_regulation.tick(2200, input).dimmer == 85);
  assert(ramp_after_regulation.tick(2400, input).dimmer == 84);
  assert(ramp_after_regulation.tick(26000, input).dimmer == 84);
  assert(ramp_after_regulation.state() == State::kRampdown);

  Machine fast_preinfusion;
  c = config();
  c.preinfusion_pump_pct = 80;
  input = {0, false, 0};
  assert(fast_preinfusion.start(1000, c, input));
  assert(fast_preinfusion.tick(2000, input).dimmer == 80);
  assert(fast_preinfusion.tick(6000, input).dimmer == 80);  // au-dessus du minimum : départ inchangé
  assert(fast_preinfusion.state() == State::kBrew);

  Machine first_drop;
  c = config();
  c.preinfusion_mode = PreinfusionMode::kWeight;
  input = {10, true, 0};
  assert(first_drop.start(1000, c, input));
  assert(first_drop.tick(2000, input).dimmer == 35);
  input.weight_g = 10.09f;
  assert(first_drop.tick(2100, input).dimmer == 35);
  input.weight_g = 10.1f;
  assert(first_drop.tick(2200, input).dimmer == 50);  // début de rampe après la première goutte
  assert(first_drop.state() == State::kBrew);

  Machine missing_scale;
  input = {0, false, 0};
  assert(missing_scale.start(1000, c, input));
  assert(missing_scale.tick(2000, input).dimmer == 100);
  assert(missing_scale.state() == State::kBrew);

  Machine combined;
  c.preinfusion_mode = PreinfusionMode::kTime | PreinfusionMode::kWeight;
  input = {10, true, 0};
  assert(combined.start(1000, c, input));
  assert(combined.tick(2000, input).dimmer == 35);
  input.pressure_bar = 9.0f;
  assert(combined.tick(2100, input).dimmer == 35);  // la pression ne termine plus la pré-infusion
  assert(combined.state() == State::kPreinfusion);
  input.weight_g = 10.1f;
  assert(combined.tick(2200, input).dimmer == 50);  // la première goutte, si
  assert(combined.state() == State::kBrew);

  c = config();
  Machine weighted;
  input = {10, true, 0};
  assert(weighted.start(1000, c, input));
  assert(weighted.weight_goal());
  assert(weighted.tick(2000, input).dimmer == 35);
  input.weight_g = 46;
  assert(weighted.tick(2100, input).dimmer == 0);
  assert(weighted.stop_reason() == StopReason::kTargetWeight);

  Machine ramp_weight;
  c.rampdown_mode = RampdownMode::kWeight;
  c.preinfusion_time_s = 0;
  input = {10, true, 0};
  assert(ramp_weight.start(1000, c, input));
  assert(ramp_weight.tick(2000, input).dimmer == 100);
  input.weight_g = 42;
  assert(ramp_weight.tick(2100, input).dimmer == 100);
  assert(ramp_weight.state() == State::kRampdown);
  input.weight_g = 46;
  assert(ramp_weight.tick(2200, input).dimmer == 0);
  assert(ramp_weight.stop_reason() == StopReason::kTargetWeight);
  c = config();

  Machine lost;
  input = {10, true, 0};
  assert(lost.start(1000, c, input));
  assert(lost.tick(2000, input).dimmer == 35);
  input.scale_present = false;
  assert(lost.tick(2100, input).dimmer == 0);
  assert(lost.stop_reason() == StopReason::kScaleLost);

  Machine filling_pressure;
  c = config();
  c.filling_time_s = 6;
  input = {0, false, .48f, true};  // pression résiduelle au repos
  assert(filling_pressure.start(1000, c, input));
  input.pressure_bar = .29f;
  assert(filling_pressure.tick(2000, input).dimmer == 100);  // fin de garde, plancher 0,29
  input.pressure_bar = .26f;
  assert(filling_pressure.tick(2300, input).dimmer == 100);  // plancher 0,26
  input.pressure_bar = .37f;
  assert(filling_pressure.tick(2400, input).dimmer == 100);  // paquet isolé au-dessus
  input.pressure_bar = .30f;
  assert(filling_pressure.tick(2500, input).dimmer == 100);  // retombé : confirmation remise à zéro
  input.pressure_bar = .35f;
  assert(filling_pressure.tick(2600, input).dimmer == 100);  // montée de 0,09 bar : sous le seuil
  input.pressure_bar = .37f;
  assert(filling_pressure.tick(2700, input).dimmer == 100);
  assert(filling_pressure.tick(2800, input).dimmer == 100);
  assert(filling_pressure.tick(2849, input).dimmer == 100);
  assert(filling_pressure.tick(2850, input).dimmer == 35);  // 150 ms au-dessus de 0,26 + 0,10
  assert(filling_pressure.state() == State::kPreinfusion);

  Machine filling_pressure_lost;
  input = {0, false, .26f, true};
  assert(filling_pressure_lost.start(1000, c, input));
  assert(filling_pressure_lost.tick(2000, input).dimmer == 100);
  input.pressure_bar = .40f;
  assert(filling_pressure_lost.tick(2100, input).dimmer == 100);
  input.pressure_valid = false;
  assert(filling_pressure_lost.tick(2200, input).dimmer == 100);  // mesure perdue : pas de sortie
  input.pressure_valid = true;
  assert(filling_pressure_lost.tick(2300, input).dimmer == 100);
  assert(filling_pressure_lost.tick(2449, input).dimmer == 100);
  assert(filling_pressure_lost.tick(2450, input).dimmer == 35);
  assert(filling_pressure_lost.state() == State::kPreinfusion);

  Machine purge;
  assert(purge.purge_press(1000, c));
  assert(purge.tick(2000, input).dimmer == 100);
  assert(purge.purge_release(2000));
  assert(purge.tick(2001, input).dimmer == 0);
  assert(purge.stop_reason() == StopReason::kPurgeReleased);
  assert(purge.elapsed_ms(2001) == 1000);
  assert(purge.elapsed_ms(10000) == 1000);
  assert(purge.purge_press(3000, c));
  assert(purge.tick(23000, input).dimmer == 0);
  assert(purge.stop_reason() == StopReason::kPurgeTimeout);

  std::puts("machine tests passed");
}
