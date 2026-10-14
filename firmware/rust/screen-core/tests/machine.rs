//! Port de `archive/firmware/screen/test/test_machine.cpp`, bloc par bloc.

#![allow(clippy::field_reassign_with_default)]

use screen_core::config::{PreinfusionMode, RampdownMode};
use screen_core::machine::{Config, Input, Machine, State, StopReason};

fn config() -> Config {
    Config {
        target_weight_g: 36.0,
        target_time_s: 28,
        target_pressure_bar: 9.0,
        brew_preheat_time_s: 0.0,
        filling_time_s: 1,
        filling_pressure_bar: 1.0,
        filling_pump_pct: 100,
        preinfusion_mode: PreinfusionMode::TIME,
        preinfusion_time_s: 4,
        preinfusion_pump_pct: 35,
        rampdown_mode: RampdownMode::None,
        rampdown_lead_time_s: 3.0,
        rampdown_lead_weight_g: 4.0,
        rampdown_pressure_drop_bar: 1.0,
        brew_pump_pct: 100,
        purge_pump_pct: 100,
        purge_max_s: 20,
    }
}

fn input(weight_g: f32, scale_present: bool, pressure_bar: f32) -> Input {
    Input {
        weight_g,
        scale_present,
        pressure_bar,
        pressure_valid: false,
    }
}

fn valid(weight_g: f32, scale_present: bool, pressure_bar: f32) -> Input {
    Input {
        pressure_valid: true,
        ..input(weight_g, scale_present, pressure_bar)
    }
}

#[test]
fn thermal_preheat_then_filling_then_preinfusion() {
    let mut machine = Machine::new();
    let i = input(0.0, false, 0.0);
    let mut c = config();
    c.brew_preheat_time_s = 2.5;
    assert!(machine.start(1000, &c, &i));
    assert_eq!(machine.state(), State::ThermalPreheat);
    assert_eq!(machine.tick(3499, &i).dimmer, 0);
    assert_eq!(machine.elapsed_ms(3499), 0);
    assert_eq!(machine.tick(3500, &i).dimmer, 100);
    assert_eq!(machine.state(), State::Filling);
    assert_eq!(machine.elapsed_ms(3500), 0);
    assert_eq!(machine.tick(4499, &i).dimmer, 100);
    assert_eq!(machine.tick(4500, &i).dimmer, 35);
}

#[test]
fn time_only_cycle_with_ramp_from_minimum() {
    let mut machine = Machine::new();
    let i = input(0.0, false, 0.0);
    assert!(machine.start(1000, &config(), &i));
    assert_eq!(machine.state(), State::Filling);
    assert_eq!(machine.tick(1999, &i).dimmer, 100);
    assert_eq!(machine.tick(2000, &i).dimmer, 35);
    assert_eq!(machine.state(), State::Preinfusion);
    assert_eq!(machine.tick(5999, &i).dimmer, 35);
    // rampe depuis MINIMUM_BREW_PUMP_PCT, pas depuis la pause
    assert_eq!(machine.tick(6000, &i).dimmer, 50);
    assert_eq!(machine.tick(6249, &i).dimmer, 50);
    // 10 pas de 5 % en 2,5 s
    assert_eq!(machine.tick(6250, &i).dimmer, 55);
    assert_eq!(machine.state(), State::Brew);
    assert_eq!(machine.tick(28999, &i).dimmer, 100);
    assert_eq!(machine.tick(29000, &i).dimmer, 0);
    assert_eq!(machine.stop_reason(), StopReason::TargetTime);
}

#[test]
fn no_preinfusion_goes_straight_to_brew() {
    let mut machine = Machine::new();
    let mut c = config();
    c.preinfusion_mode = PreinfusionMode::NONE;
    let i = input(0.0, false, 0.0);
    assert!(machine.start(1000, &c, &i));
    assert_eq!(machine.state(), State::Filling);
    assert_eq!(machine.tick(2000, &i).dimmer, 100);
    assert_eq!(machine.state(), State::Brew);
}

#[test]
fn pressure_regulation() {
    let mut machine = Machine::new();
    let mut c = config();
    c.preinfusion_mode = PreinfusionMode::NONE;
    let mut i = valid(0.0, false, 6.9);
    assert!(machine.start(1000, &c, &i));
    assert_eq!(machine.tick(2000, &i).dimmer, 100);
    assert_eq!(machine.tick(2199, &i).dimmer, 100); // pas avant 200 ms
    assert_eq!(machine.tick(2200, &i).dimmer, 100); // activation à cible - 3 bar, sans saut
    i.pressure_bar = 7.0;
    assert_eq!(machine.tick(2400, &i).dimmer, 100);
    i.pressure_bar = 8.0;
    assert_eq!(machine.tick(2600, &i).dimmer, 85); // terme proportionnel dominant
    i.pressure_bar = 8.5;
    assert_eq!(machine.tick(2800, &i).dimmer, 78);
    i.pressure_bar = 9.0;
    assert_eq!(machine.tick(3000, &i).dimmer, 71);
    i.pressure_bar = 9.5;
    assert_eq!(machine.tick(3200, &i).dimmer, 63);
    i.pressure_valid = false;
    assert_eq!(machine.tick(3400, &i).dimmer, 63);
    i.pressure_valid = true;
    i.pressure_bar = 12.0;
    let mut now = 3600;
    while now <= 5600 {
        assert_eq!(machine.tick(now, &i).dimmer, 50);
        now += 200;
    }
    i.pressure_bar = 9.0;
    assert_eq!(machine.tick(5800, &i).dimmer, 70); // pas de windup à la borne basse
}

#[test]
fn intermittent_pressure_integrates_real_elapsed_time() {
    // Après une indisponibilité durable, l'intégrale reprend sur le vrai temps
    // écoulé, borné à 400 ms, plutôt que de perdre les corrections sautées.
    let mut machine = Machine::new();
    let mut c = config();
    c.preinfusion_mode = PreinfusionMode::NONE;
    let mut i = valid(0.0, false, 7.0);
    assert!(machine.start(1000, &c, &i));
    assert_eq!(machine.tick(2000, &i).dimmer, 100);
    assert_eq!(machine.tick(2200, &i).dimmer, 100); // activation
    i.pressure_bar = 10.0;
    assert_eq!(machine.tick(2400, &i).dimmer, 54);
    i.pressure_valid = false;
    assert_eq!(machine.tick(2600, &i).dimmer, 54);
    assert_eq!(machine.tick(2800, &i).dimmer, 54);
    assert_eq!(machine.tick(3000, &i).dimmer, 54);
    i.pressure_valid = true;
    assert_eq!(machine.tick(3200, &i).dimmer, 53);
}

#[test]
fn safe_direct_config_never_below_minimum() {
    let mut machine = Machine::new();
    let mut c = config();
    c.preinfusion_mode = PreinfusionMode::NONE;
    c.brew_pump_pct = 20;
    let i = valid(0.0, false, 10.0);
    assert!(machine.start(1000, &c, &i));
    assert_eq!(machine.tick(2000, &i).dimmer, 50);
    assert_eq!(machine.tick(2200, &i).dimmer, 50);
}

#[test]
fn rampdown_after_regulation() {
    let mut machine = Machine::new();
    let mut c = config();
    c.preinfusion_mode = PreinfusionMode::NONE;
    c.rampdown_mode = RampdownMode::Time;
    c.rampdown_lead_time_s = 3.0;
    let i = valid(0.0, false, 10.0);
    assert!(machine.start(1000, &c, &i));
    assert_eq!(machine.tick(2000, &i).dimmer, 100);
    assert_eq!(machine.tick(2200, &i).dimmer, 85);
    assert_eq!(machine.tick(2400, &i).dimmer, 84);
    assert_eq!(machine.tick(26000, &i).dimmer, 84);
    assert_eq!(machine.state(), State::Rampdown);
}

#[test]
fn fast_preinfusion_keeps_its_start_level() {
    let mut machine = Machine::new();
    let mut c = config();
    c.preinfusion_pump_pct = 80;
    let i = input(0.0, false, 0.0);
    assert!(machine.start(1000, &c, &i));
    assert_eq!(machine.tick(2000, &i).dimmer, 80);
    // au-dessus du minimum : départ inchangé
    assert_eq!(machine.tick(6000, &i).dimmer, 80);
    assert_eq!(machine.state(), State::Brew);
}

#[test]
fn first_drop_ends_weight_preinfusion() {
    let mut c = config();
    c.preinfusion_mode = PreinfusionMode::WEIGHT;
    let mut machine = Machine::new();
    let mut i = input(10.0, true, 0.0);
    assert!(machine.start(1000, &c, &i));
    assert_eq!(machine.tick(2000, &i).dimmer, 35);
    i.weight_g = 10.09;
    assert_eq!(machine.tick(2100, &i).dimmer, 35);
    i.weight_g = 10.1;
    // début de rampe après la première goutte
    assert_eq!(machine.tick(2200, &i).dimmer, 50);
    assert_eq!(machine.state(), State::Brew);

    // Sans balance, la pré-infusion par poids est ignorée.
    let mut missing_scale = Machine::new();
    let i = input(0.0, false, 0.0);
    assert!(missing_scale.start(1000, &c, &i));
    assert_eq!(missing_scale.tick(2000, &i).dimmer, 100);
    assert_eq!(missing_scale.state(), State::Brew);

    // Temps + poids : la pression ne termine pas la pré-infusion.
    c.preinfusion_mode = PreinfusionMode::TIME | PreinfusionMode::WEIGHT;
    let mut combined = Machine::new();
    let mut i = input(10.0, true, 0.0);
    assert!(combined.start(1000, &c, &i));
    assert_eq!(combined.tick(2000, &i).dimmer, 35);
    i.pressure_bar = 9.0;
    assert_eq!(combined.tick(2100, &i).dimmer, 35);
    assert_eq!(combined.state(), State::Preinfusion);
    i.weight_g = 10.1;
    assert_eq!(combined.tick(2200, &i).dimmer, 50);
    assert_eq!(combined.state(), State::Brew);
}

#[test]
fn weight_goal_stops_and_rampdown_by_weight() {
    let c = config();
    let mut weighted = Machine::new();
    let mut i = input(10.0, true, 0.0);
    assert!(weighted.start(1000, &c, &i));
    assert!(weighted.weight_goal());
    assert_eq!(weighted.tick(2000, &i).dimmer, 35);
    i.weight_g = 46.0;
    assert_eq!(weighted.tick(2100, &i).dimmer, 0);
    assert_eq!(weighted.stop_reason(), StopReason::TargetWeight);

    let mut c = config();
    let mut ramp_weight = Machine::new();
    c.rampdown_mode = RampdownMode::Weight;
    c.preinfusion_time_s = 0;
    let mut i = input(10.0, true, 0.0);
    assert!(ramp_weight.start(1000, &c, &i));
    assert_eq!(ramp_weight.tick(2000, &i).dimmer, 100);
    i.weight_g = 42.0;
    assert_eq!(ramp_weight.tick(2100, &i).dimmer, 100);
    assert_eq!(ramp_weight.state(), State::Rampdown);
    i.weight_g = 46.0;
    assert_eq!(ramp_weight.tick(2200, &i).dimmer, 0);
    assert_eq!(ramp_weight.stop_reason(), StopReason::TargetWeight);
}

#[test]
fn scale_lost_stops_the_cycle() {
    let c = config();
    let mut lost = Machine::new();
    let mut i = input(10.0, true, 0.0);
    assert!(lost.start(1000, &c, &i));
    assert_eq!(lost.tick(2000, &i).dimmer, 35);
    i.scale_present = false;
    assert_eq!(lost.tick(2100, &i).dimmer, 0);
    assert_eq!(lost.stop_reason(), StopReason::ScaleLost);
}

#[test]
fn filling_ends_on_absolute_pressure() {
    let mut c = config();
    c.filling_time_s = 10;
    let mut filling_pressure = Machine::new();
    // pression résiduelle au repos
    let mut i = valid(0.0, false, 1.44);
    assert!(filling_pressure.start(1000, &c, &i));
    assert_eq!(filling_pressure.tick(1500, &i).dimmer, 100); // garde : ignorée
    i.pressure_bar = 0.26;
    assert_eq!(filling_pressure.tick(2000, &i).dimmer, 100);
    i.pressure_bar = 1.25;
    assert_eq!(filling_pressure.tick(2300, &i).dimmer, 100); // paquet isolé au-dessus
    i.pressure_bar = 0.40;
    assert_eq!(filling_pressure.tick(2400, &i).dimmer, 100); // retombé : confirmation remise à zéro
    i.pressure_bar = 0.99;
    assert_eq!(filling_pressure.tick(2500, &i).dimmer, 100); // sous le seuil
    i.pressure_bar = 1.0;
    assert_eq!(filling_pressure.tick(2600, &i).dimmer, 100);
    assert_eq!(filling_pressure.tick(2749, &i).dimmer, 100);
    assert_eq!(filling_pressure.tick(2750, &i).dimmer, 35); // 150 ms à 1 bar ou plus
    assert_eq!(filling_pressure.state(), State::Preinfusion);

    let mut lost_measure = Machine::new();
    let mut i = valid(0.0, false, 0.26);
    assert!(lost_measure.start(1000, &c, &i));
    assert_eq!(lost_measure.tick(2000, &i).dimmer, 100);
    i.pressure_bar = 1.1;
    assert_eq!(lost_measure.tick(2100, &i).dimmer, 100);
    i.pressure_valid = false;
    assert_eq!(lost_measure.tick(2200, &i).dimmer, 100); // mesure perdue : pas de sortie
    i.pressure_valid = true;
    assert_eq!(lost_measure.tick(2300, &i).dimmer, 100);
    assert_eq!(lost_measure.tick(2449, &i).dimmer, 100);
    assert_eq!(lost_measure.tick(2450, &i).dimmer, 35);
    assert_eq!(lost_measure.state(), State::Preinfusion);

    // Pression jamais atteinte : la durée sert de secours.
    let mut timeout = Machine::new();
    let i = valid(0.0, false, 0.40);
    assert!(timeout.start(1000, &c, &i));
    assert_eq!(timeout.tick(10999, &i).dimmer, 100);
    assert_eq!(timeout.tick(11000, &i).dimmer, 35);
    assert_eq!(timeout.state(), State::Preinfusion);
}

#[test]
fn purge() {
    let c = config();
    let i = valid(0.0, false, 0.40);
    let mut purge = Machine::new();
    assert!(purge.purge_press(1000, &c, 20_000));
    assert_eq!(purge.tick(2000, &i).dimmer, 100);
    assert!(purge.purge_release(2000));
    assert_eq!(purge.tick(2001, &i).dimmer, 0);
    assert_eq!(purge.stop_reason(), StopReason::PurgeReleased);
    assert_eq!(purge.elapsed_ms(2001), 1000);
    assert_eq!(purge.elapsed_ms(10000), 1000);
    assert!(purge.purge_press(3000, &c, 20_000));
    assert_eq!(purge.tick(22999, &i).dimmer, 100);
    assert_eq!(purge.tick(23000, &i).dimmer, 0);
    assert_eq!(purge.stop_reason(), StopReason::PurgeTimeout);
}

#[test]
fn purge_duration_is_set_by_the_press() {
    let c = config();
    let i = valid(0.0, false, 0.40);
    let mut purge = Machine::new();
    assert!(purge.purge_press(1000, &c, 5000));
    assert_eq!(purge.tick(5999, &i).dimmer, 100);
    assert_eq!(purge.tick(6000, &i).dimmer, 0);
    assert_eq!(purge.stop_reason(), StopReason::PurgeTimeout);
    assert_eq!(purge.elapsed_ms(6000), 5000);
}

#[test]
fn purge_after_weighted_brew_has_no_weight_goal() {
    let c = config();
    let mut machine = Machine::new();
    let mut i = input(0.0, true, 0.0);
    assert!(machine.start(1000, &c, &i));
    i.weight_g = 36.0;
    machine.tick(2000, &i);
    assert_eq!(machine.stop_reason(), StopReason::TargetWeight);
    assert!(machine.weight_goal());
    // balance éteinte avant de rincer la douche
    assert!(machine.purge_press(3000, &c, 20_000));
    assert!(!machine.weight_goal());
}

#[test]
fn stop_and_dismiss() {
    let c = config();
    let i = input(0.0, false, 0.0);
    let mut machine = Machine::new();
    assert!(!machine.stop(1000));
    assert!(machine.start(1000, &c, &i));
    assert!(!machine.start(1100, &c, &i));
    assert!(machine.stop(1200));
    assert_eq!(machine.stop_reason(), StopReason::Manual);
    assert_eq!(machine.state(), State::Finished);
    assert!(machine.dismiss());
    assert_eq!(machine.state(), State::Idle);
    assert!(!machine.dismiss());
}
