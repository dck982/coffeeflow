//! Port de `archive/firmware/screen/test/test_machine.cpp`, bloc par bloc.

#![allow(clippy::field_reassign_with_default)]

use screen_core::config::PreinfusionMode;
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
        stop_lead_g: 4.0,
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
        pressure_sample_ms: 0,
    }
}

fn valid(weight_g: f32, scale_present: bool, pressure_bar: f32) -> Input {
    Input {
        pressure_valid: true,
        ..input(weight_g, scale_present, pressure_bar)
    }
}

/// Pas de la machine avec une mesure de pression reçue à cet instant.
fn tick_sampled(machine: &mut Machine, now_ms: u64, i: &mut Input) -> u8 {
    i.pressure_sample_ms = now_ms;
    machine.tick(now_ms, i).pump_pct
}

#[test]
fn thermal_preheat_then_filling_then_preinfusion() {
    let mut machine = Machine::new();
    let i = input(0.0, false, 0.0);
    let mut c = config();
    c.brew_preheat_time_s = 2.5;
    assert!(machine.start(1000, &c, &i));
    assert_eq!(machine.state(), State::ThermalPreheat);
    assert_eq!(machine.tick(3499, &i).pump_pct, 0);
    assert_eq!(machine.elapsed_ms(3499), 0);
    assert_eq!(machine.tick(3500, &i).pump_pct, 100);
    assert_eq!(machine.state(), State::Filling);
    assert_eq!(machine.elapsed_ms(3500), 0);
    assert_eq!(machine.tick(4499, &i).pump_pct, 100);
    assert_eq!(machine.tick(4500, &i).pump_pct, 35);
}

#[test]
fn time_only_cycle_with_ramp_from_preinfusion() {
    let mut machine = Machine::new();
    let i = input(0.0, false, 0.0);
    assert!(machine.start(1000, &config(), &i));
    assert_eq!(machine.state(), State::Filling);
    assert_eq!(machine.tick(1999, &i).pump_pct, 100);
    assert_eq!(machine.tick(2000, &i).pump_pct, 35);
    assert_eq!(machine.state(), State::Preinfusion);
    assert_eq!(machine.tick(5999, &i).pump_pct, 35);
    // rampe depuis le niveau de la pré-infusion, au-dessus du plancher
    assert_eq!(machine.tick(6000, &i).pump_pct, 35);
    assert_eq!(machine.tick(6191, &i).pump_pct, 35);
    // 13 pas de 5 % en 2,5 s
    assert_eq!(machine.tick(6192, &i).pump_pct, 40);
    assert_eq!(machine.state(), State::Brew);
    assert_eq!(machine.tick(28999, &i).pump_pct, 100);
    assert_eq!(machine.tick(29000, &i).pump_pct, 0);
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
    assert_eq!(machine.tick(2000, &i).pump_pct, 100);
    assert_eq!(machine.state(), State::Brew);
}

#[test]
fn pressure_regulation() {
    let mut machine = Machine::new();
    let mut c = config();
    c.preinfusion_mode = PreinfusionMode::NONE;
    let mut i = valid(0.0, false, 6.9);
    assert!(machine.start(1000, &c, &i));
    // activation aux 2/3 de la cible, sans saut ni pente connue
    assert_eq!(tick_sampled(&mut machine, 2000, &mut i), 100);
    // sans nouvelle mesure, pas de nouvelle commande
    assert_eq!(machine.tick(2050, &i).pump_pct, 100);
    i.pressure_bar = 7.0;
    assert_eq!(tick_sampled(&mut machine, 2100, &mut i), 99); // P dominant
    i.pressure_bar = 8.0;
    // 5,5 bar/s sur deux intervalles : le terme dérivé coupe avant la cible
    assert_eq!(tick_sampled(&mut machine, 2200, &mut i), 68);
    i.pressure_bar = 9.0;
    assert_eq!(tick_sampled(&mut machine, 2300, &mut i), 39); // 10 bar/s
    assert_eq!(tick_sampled(&mut machine, 2400, &mut i), 54); // 5 bar/s
    i.pressure_bar = 9.5;
    assert_eq!(tick_sampled(&mut machine, 2500, &mut i), 54);
    assert_eq!(tick_sampled(&mut machine, 2600, &mut i), 54);
    // pente nulle : sommet, le terme dérivé s'éteint
    assert_eq!(tick_sampled(&mut machine, 2700, &mut i), 61);
    i.pressure_valid = false;
    assert_eq!(tick_sampled(&mut machine, 2800, &mut i), 61);
    i.pressure_valid = true;
    // Bien au-dessus de la cible : le PI plafonne à MINIMUM_BREW_PUMP_PCT.
    i.pressure_bar = 20.0;
    let mut now = 3000;
    while now <= 5000 {
        assert_eq!(tick_sampled(&mut machine, now, &mut i), 10);
        now += 100;
    }
    i.pressure_bar = 9.0;
    tick_sampled(&mut machine, 5100, &mut i);
    tick_sampled(&mut machine, 5200, &mut i);
    assert_eq!(tick_sampled(&mut machine, 5300, &mut i), 69); // pas de windup à la borne basse
}

#[test]
fn derivative_cuts_a_fast_rise_before_the_target() {
    // Montée du 15/11 sur la pompe à engrenage : ≈ 6,7 bar/s de 6 à 9 bar.
    // Sans anticipation, la commande resterait à 100 % jusqu'à 6,9 bar.
    let mut machine = Machine::new();
    let mut c = config();
    c.preinfusion_mode = PreinfusionMode::NONE;
    let mut i = valid(0.0, false, 4.7);
    assert!(machine.start(1000, &c, &i));
    assert_eq!(tick_sampled(&mut machine, 2000, &mut i), 100);
    i.pressure_bar = 5.4;
    assert_eq!(tick_sampled(&mut machine, 2100, &mut i), 100); // sous 6 bar
    i.pressure_bar = 6.2;
    // activation : P + I reprend 100 %, le terme dérivé (7,5 bar/s) en
    // retire 22,5
    assert_eq!(tick_sampled(&mut machine, 2200, &mut i), 78);
    i.pressure_bar = 6.9;
    assert_eq!(tick_sampled(&mut machine, 2300, &mut i), 68);
}

#[test]
fn derivative_stops_at_the_first_pressure_peak() {
    // Sur le plateau, le terme dérivé entretenait une oscillation de 0,8 s :
    // passé le premier sommet, seul le PI règle, même si la pression remonte
    // vite.
    let mut machine = Machine::new();
    let mut c = config();
    c.preinfusion_mode = PreinfusionMode::NONE;
    let mut i = valid(0.0, false, 8.0);
    assert!(machine.start(1000, &c, &i));
    assert_eq!(tick_sampled(&mut machine, 2000, &mut i), 100); // activation
    i.pressure_bar = 9.0;
    tick_sampled(&mut machine, 2100, &mut i);
    i.pressure_bar = 8.8;
    // 4 bar/s sur deux intervalles : le terme dérivé agit encore
    // 15 × 0,2 + 85,06 − 3 × 4 = 76,06
    assert_eq!(tick_sampled(&mut machine, 2200, &mut i), 76);
    i.pressure_bar = 8.6;
    // −2 bar/s : sommet passé ; 15 × 0,4 + 85,18 = 91,18
    assert_eq!(tick_sampled(&mut machine, 2300, &mut i), 91);
    i.pressure_bar = 9.0;
    tick_sampled(&mut machine, 2400, &mut i);
    i.pressure_bar = 9.6;
    // +5 bar/s, sans effet (il retirerait 15 %) : −15 × 0,6 + 85,00 = 76
    assert_eq!(tick_sampled(&mut machine, 2500, &mut i), 76);
}

#[test]
fn pressure_regulation_waits_for_two_thirds_of_a_low_target() {
    let mut machine = Machine::new();
    let mut c = config();
    c.preinfusion_mode = PreinfusionMode::NONE;
    c.target_pressure_bar = 6.0;
    // Mesures espacées de 300 ms : la pente s'étale au-delà de 400 ms et
    // le terme dérivé reste nul.
    let mut i = valid(0.0, false, 3.5);
    assert!(machine.start(1000, &c, &i));
    assert_eq!(tick_sampled(&mut machine, 2000, &mut i), 100);
    i.pressure_bar = 3.8;
    assert_eq!(tick_sampled(&mut machine, 2300, &mut i), 100); // 3 bar passés, sous 4 bar : pas de PI
    i.pressure_bar = 4.3;
    assert_eq!(tick_sampled(&mut machine, 2600, &mut i), 100); // activation sans saut
    i.pressure_bar = 7.0;
    assert_eq!(tick_sampled(&mut machine, 2900, &mut i), 59);
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
    assert_eq!(tick_sampled(&mut machine, 2000, &mut i), 100); // activation
    i.pressure_bar = 10.0;
    let after_jump = tick_sampled(&mut machine, 2100, &mut i);
    i.pressure_valid = false;
    assert_eq!(tick_sampled(&mut machine, 2300, &mut i), after_jump);
    assert_eq!(tick_sampled(&mut machine, 2600, &mut i), after_jump);
    assert_eq!(tick_sampled(&mut machine, 3000, &mut i), after_jump);
    i.pressure_valid = true;
    // la pente est trop étalée pour compter ; seule l'intégrale avance, de
    // −3 %/(bar·s) × 1 bar × 0,4 s
    assert_eq!(tick_sampled(&mut machine, 3100, &mut i), 54);
}

#[test]
fn ramp_never_starts_below_the_minimum() {
    let mut machine = Machine::new();
    let mut c = config();
    c.preinfusion_pump_pct = 5;
    let i = input(0.0, false, 0.0);
    assert!(machine.start(1000, &c, &i));
    assert_eq!(machine.tick(2000, &i).pump_pct, 5);
    // la rampe part du plancher, pas du niveau de pause : 18 pas de 5 %
    assert_eq!(machine.tick(6000, &i).pump_pct, 10);
    assert_eq!(machine.tick(6137, &i).pump_pct, 10);
    assert_eq!(machine.tick(6138, &i).pump_pct, 15);
}

#[test]
fn safe_direct_config_never_below_minimum() {
    let mut machine = Machine::new();
    let mut c = config();
    c.preinfusion_mode = PreinfusionMode::NONE;
    c.brew_pump_pct = 5;
    let i = valid(0.0, false, 10.0);
    assert!(machine.start(1000, &c, &i));
    assert_eq!(machine.tick(2000, &i).pump_pct, 10);
    assert_eq!(machine.tick(2200, &i).pump_pct, 10);
}

#[test]
fn fast_preinfusion_keeps_its_start_level() {
    let mut machine = Machine::new();
    let mut c = config();
    c.preinfusion_pump_pct = 80;
    let i = input(0.0, false, 0.0);
    assert!(machine.start(1000, &c, &i));
    assert_eq!(machine.tick(2000, &i).pump_pct, 80);
    // au-dessus du minimum : départ inchangé
    assert_eq!(machine.tick(6000, &i).pump_pct, 80);
    assert_eq!(machine.state(), State::Brew);
}

#[test]
fn first_drop_ends_weight_preinfusion() {
    let mut c = config();
    c.preinfusion_mode = PreinfusionMode::WEIGHT;
    let mut machine = Machine::new();
    let mut i = input(10.0, true, 0.0);
    assert!(machine.start(1000, &c, &i));
    assert_eq!(machine.tick(2000, &i).pump_pct, 35);
    i.weight_g = 10.09;
    assert_eq!(machine.tick(2100, &i).pump_pct, 35);
    i.weight_g = 10.1;
    // début de rampe après la première goutte
    assert_eq!(machine.tick(2200, &i).pump_pct, 35);
    assert_eq!(machine.state(), State::Brew);

    // Sans balance, la pré-infusion par poids est ignorée.
    let mut missing_scale = Machine::new();
    let i = input(0.0, false, 0.0);
    assert!(missing_scale.start(1000, &c, &i));
    assert_eq!(missing_scale.tick(2000, &i).pump_pct, 100);
    assert_eq!(missing_scale.state(), State::Brew);

    // Temps + poids : la pression ne termine pas la pré-infusion.
    c.preinfusion_mode = PreinfusionMode::TIME | PreinfusionMode::WEIGHT;
    let mut combined = Machine::new();
    let mut i = input(10.0, true, 0.0);
    assert!(combined.start(1000, &c, &i));
    assert_eq!(combined.tick(2000, &i).pump_pct, 35);
    i.pressure_bar = 9.0;
    assert_eq!(combined.tick(2100, &i).pump_pct, 35);
    assert_eq!(combined.state(), State::Preinfusion);
    i.weight_g = 10.1;
    assert_eq!(combined.tick(2200, &i).pump_pct, 35);
    assert_eq!(combined.state(), State::Brew);
}

#[test]
fn weight_goal_stops_ahead_of_the_target_by_the_stop_lead() {
    let c = config();
    let mut weighted = Machine::new();
    let mut i = input(10.0, true, 0.0);
    assert!(weighted.start(1000, &c, &i));
    assert!(weighted.weight_goal());
    assert_eq!(weighted.tick(2000, &i).pump_pct, 35);
    i.weight_g = 46.0;
    assert_eq!(weighted.tick(2100, &i).pump_pct, 0);
    assert_eq!(weighted.stop_reason(), StopReason::TargetWeight);

    assert_eq!(weighted.stop_weight_g(), 32.0);

    // Sans avance, la pompe s'arrête à la cible.
    let mut c = config();
    c.stop_lead_g = 0.0;
    c.preinfusion_time_s = 0;
    let mut exact = Machine::new();
    let mut i = input(10.0, true, 0.0);
    assert!(exact.start(1000, &c, &i));
    assert_eq!(exact.tick(2000, &i).pump_pct, 100);
    i.weight_g = 42.0;
    assert_eq!(exact.tick(2100, &i).pump_pct, 100);
    assert_eq!(exact.state(), State::Brew);
    i.weight_g = 46.0;
    assert_eq!(exact.tick(2200, &i).pump_pct, 0);
    assert_eq!(exact.stop_reason(), StopReason::TargetWeight);
}

#[test]
fn scale_lost_stops_the_cycle() {
    let c = config();
    let mut lost = Machine::new();
    let mut i = input(10.0, true, 0.0);
    assert!(lost.start(1000, &c, &i));
    assert_eq!(lost.tick(2000, &i).pump_pct, 35);
    i.scale_present = false;
    assert_eq!(lost.tick(2100, &i).pump_pct, 0);
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
    assert_eq!(filling_pressure.tick(1500, &i).pump_pct, 100); // garde : ignorée
    i.pressure_bar = 0.26;
    assert_eq!(filling_pressure.tick(2000, &i).pump_pct, 100);
    i.pressure_bar = 1.25;
    assert_eq!(filling_pressure.tick(2300, &i).pump_pct, 100); // paquet isolé au-dessus
    i.pressure_bar = 0.40;
    assert_eq!(filling_pressure.tick(2400, &i).pump_pct, 100); // retombé : confirmation remise à zéro
    i.pressure_bar = 0.99;
    assert_eq!(filling_pressure.tick(2500, &i).pump_pct, 100); // sous le seuil
    i.pressure_bar = 1.0;
    assert_eq!(filling_pressure.tick(2600, &i).pump_pct, 100);
    assert_eq!(filling_pressure.tick(2749, &i).pump_pct, 100);
    assert_eq!(filling_pressure.tick(2750, &i).pump_pct, 35); // 150 ms à 1 bar ou plus
    assert_eq!(filling_pressure.state(), State::Preinfusion);

    let mut lost_measure = Machine::new();
    let mut i = valid(0.0, false, 0.26);
    assert!(lost_measure.start(1000, &c, &i));
    assert_eq!(lost_measure.tick(2000, &i).pump_pct, 100);
    i.pressure_bar = 1.1;
    assert_eq!(lost_measure.tick(2100, &i).pump_pct, 100);
    i.pressure_valid = false;
    assert_eq!(lost_measure.tick(2200, &i).pump_pct, 100); // mesure perdue : pas de sortie
    i.pressure_valid = true;
    assert_eq!(lost_measure.tick(2300, &i).pump_pct, 100);
    assert_eq!(lost_measure.tick(2449, &i).pump_pct, 100);
    assert_eq!(lost_measure.tick(2450, &i).pump_pct, 35);
    assert_eq!(lost_measure.state(), State::Preinfusion);

    // Pression jamais atteinte : la durée sert de secours.
    let mut timeout = Machine::new();
    let i = valid(0.0, false, 0.40);
    assert!(timeout.start(1000, &c, &i));
    assert_eq!(timeout.tick(10999, &i).pump_pct, 100);
    assert_eq!(timeout.tick(11000, &i).pump_pct, 35);
    assert_eq!(timeout.state(), State::Preinfusion);
}

#[test]
fn purge() {
    let c = config();
    let i = valid(0.0, false, 0.40);
    let mut purge = Machine::new();
    assert!(purge.purge_press(1000, &c, 20_000));
    assert_eq!(purge.tick(2000, &i).pump_pct, 100);
    assert!(purge.purge_release(2000));
    assert_eq!(purge.tick(2001, &i).pump_pct, 0);
    assert_eq!(purge.stop_reason(), StopReason::PurgeReleased);
    assert_eq!(purge.elapsed_ms(2001), 1000);
    assert_eq!(purge.elapsed_ms(10000), 1000);
    assert!(purge.purge_press(3000, &c, 20_000));
    assert_eq!(purge.tick(22999, &i).pump_pct, 100);
    assert_eq!(purge.tick(23000, &i).pump_pct, 0);
    assert_eq!(purge.stop_reason(), StopReason::PurgeTimeout);
}

#[test]
fn purge_duration_is_set_by_the_press() {
    let c = config();
    let i = valid(0.0, false, 0.40);
    let mut purge = Machine::new();
    assert!(purge.purge_press(1000, &c, 5000));
    assert_eq!(purge.tick(5999, &i).pump_pct, 100);
    assert_eq!(purge.tick(6000, &i).pump_pct, 0);
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
