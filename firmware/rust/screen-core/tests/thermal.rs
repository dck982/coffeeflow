//! Port de `archive/firmware/screen/test/test_thermal.cpp`.

#![allow(clippy::field_reassign_with_default)]

use screen_core::brew_heating::BrewHeating;
use screen_core::calibration::{
    BOILER_NTC_BETA_K, BOILER_NTC_R0_OHM, boiler_sensor_temperature_c, boiler_user_temperature_c,
};
use screen_core::thermal::{Controller, Flow, Mode, Output};

/// Pas de repos : mesure valide, chauffage autorisé, pas de débit.
fn step(c: &mut Controller, now: u64, temperature: f32, target: f32, mode: Mode) -> Output {
    c.step(now, temperature, target, true, true, mode, Flow::NONE)
}

/// Pas avec une mesure de débit.
fn flow(
    c: &mut Controller,
    now: u64,
    temperature: f32,
    target: f32,
    mode: Mode,
    ml_s: f32,
    valid: bool,
) -> Output {
    c.step(
        now,
        temperature,
        target,
        true,
        true,
        mode,
        Flow::new(ml_s, valid),
    )
}

fn permille(out: Output) -> u16 {
    out.power_permille
}

#[test]
fn calibration_constants() {
    assert_eq!(BOILER_NTC_R0_OHM, 47000.0);
    assert_eq!(BOILER_NTC_BETA_K, 3930.0);
    assert_eq!(boiler_sensor_temperature_c(90.0), 100.5);
    assert_eq!(boiler_user_temperature_c(104.5), 94.0);
}

#[test]
fn idle_basics_and_ready() {
    let mut c = Controller::new();
    let mut out = step(&mut c, 1000, 20.0, 90.0, Mode::Idle);
    assert!(out.power_permille == 1000 && !out.ready);
    out = step(&mut c, 1250, 20.0, 90.0, Mode::Brew);
    assert_eq!(out.power_permille, 450); // sans débit mesuré : repli de l'infusion
    out = c.step(1500, 20.0, 90.0, true, false, Mode::Idle, Flow::NONE);
    assert!(out.power_permille == 0 && !out.ready);
    out = step(&mut c, 1750, 86.0, 90.0, Mode::Idle);
    assert!(out.power_permille > 0 && out.power_permille < 1000);
    out = step(&mut c, 2000, 90.0, 90.0, Mode::Idle);
    assert!(!out.ready);
    let mut now = 2500;
    while now <= 5500 {
        out = step(&mut c, now, 90.0, 90.0, Mode::Idle);
        now += 500;
    }
    assert!(out.ready);
    out = step(&mut c, 5750, 90.6, 90.0, Mode::Idle);
    assert!(out.ready);
    out = step(&mut c, 6000, 91.1, 90.0, Mode::Idle);
    assert!(!out.ready);
    out = step(&mut c, 6250, 106.0, 90.0, Mode::Idle);
    assert!(out.power_permille == 0 && !out.ready);
    out = c.step(6500, 20.0, 90.0, false, true, Mode::Idle, Flow::NONE);
    assert!(out.power_permille == 0 && !out.ready);
}

#[test]
fn idle_losses_and_proportional() {
    // Repos : pertes 0,45 W/K au-dessus de 24,5 °C (2,45 % à 90 °C), plus
    // 6 % par °C d'erreur prédite.
    let mut fine = Controller::new();
    assert_eq!(
        permille(step(&mut fine, 1000, 89.925, 90.0, Mode::Idle)),
        29
    );
    fine.reset();
    assert_eq!(
        permille(step(&mut fine, 1000, 89.875, 90.0, Mode::Idle)),
        32
    );
    let mut above = Controller::new();
    assert_eq!(permille(step(&mut above, 1000, 90.5, 90.0, Mode::Idle)), 0);

    // Une température stable à la cible garde une commande proche des pertes.
    let mut hold = Controller::new();
    let mut now = 1000;
    while now <= 60000 {
        let out = step(&mut hold, now, 90.0, 90.0, Mode::Idle);
        assert!((23..=26).contains(&out.power_permille), "{now}");
        now += 250;
    }
}

#[test]
fn idle_delay_predictor() {
    // Chaleur commandée mais pas encore visible : une NTC arrivée à la
    // consigne ne relance pas la chauffe au-delà des pertes. Une fois le
    // retard de 23,5 s écoulé, la commande revient vers le maintien.
    let mut pending = Controller::new();
    let mut now = 1000;
    while now <= 11000 {
        step(&mut pending, now, 89.0, 90.0, Mode::Idle);
        now += 250;
    }
    let out = step(&mut pending, 11250, 90.0, 90.0, Mode::Idle);
    assert!(out.power_permille < 25);
    let mut out = out;
    now = 11500;
    while now <= 45000 {
        out = step(&mut pending, now, 90.0, 90.0, Mode::Idle);
        now += 250;
    }
    assert!((15..=30).contains(&out.power_permille));

    // Une erreur persistante ne doit pas faire augmenter la puissance pendant
    // que l'effet des commandes des 23,5 dernières secondes est encore attendu.
    let mut delayed = Controller::new();
    let initial_power = step(&mut delayed, 1000, 89.0, 90.0, Mode::Idle).power_permille;
    now = 1250;
    while now <= 21000 {
        out = step(&mut delayed, now, 89.0, 90.0, Mode::Idle);
        now += 250;
    }
    assert!(out.power_permille < initial_power);
    assert_eq!(
        permille(step(&mut delayed, 21250, 91.0, 90.0, Mode::Idle)),
        0
    );
}

#[test]
fn infusion_compensates_water_without_ntc() {
    // Infusion : la commande compense l'eau admise (3,5 % + 24,56 % par ml/s,
    // bornée à 90 %), sans filtre et sans dépendre de la NTC.
    let mut infusion = Controller::new();
    step(&mut infusion, 1000, 90.0, 90.0, Mode::Idle);
    assert_eq!(
        permille(step(&mut infusion, 1250, 90.0, 90.0, Mode::Brew)),
        450
    );
    let mut infusion_flow = Controller::new();
    assert_eq!(
        permille(flow(
            &mut infusion_flow,
            1000,
            90.0,
            90.0,
            Mode::Brew,
            3.6,
            true
        )),
        900
    );
    assert_eq!(
        permille(flow(
            &mut infusion_flow,
            1250,
            90.0,
            90.0,
            Mode::Brew,
            2.0,
            true
        )),
        526
    );
    assert_eq!(
        permille(flow(
            &mut infusion_flow,
            1500,
            90.0,
            90.0,
            Mode::Brew,
            1.4,
            true
        )),
        379
    );
    // Un pic de précharge ne retire pas l'appoint : seule la sécurité à
    // consigne + 4 °C coupe.
    let mut warm = Controller::new();
    let mut hot = Controller::new();
    let mut cold = Controller::new();
    assert_eq!(
        permille(flow(&mut warm, 1000, 93.9, 90.0, Mode::Brew, 2.0, true)),
        526
    );
    assert_eq!(
        permille(flow(&mut hot, 1000, 94.1, 90.0, Mode::Brew, 2.0, true)),
        0
    );
    assert_eq!(
        permille(flow(&mut cold, 1000, 78.0, 90.0, Mode::Brew, 4.0, true)),
        900
    );
    assert!(permille(step(&mut cold, 1250, 78.0, 90.0, Mode::Idle)) <= 350);

    // Plus de coupure de fin : l'appoint suit le débit jusqu'à l'arrêt de la
    // pompe.
    let mut end = Controller::new();
    step(&mut end, 1000, 90.0, 90.0, Mode::Idle);
    assert_eq!(
        permille(flow(&mut end, 1250, 90.0, 90.0, Mode::Brew, 2.0, true)),
        526
    );
    assert_eq!(
        permille(flow(&mut end, 1500, 90.0, 90.0, Mode::Infusion, 2.0, true)),
        526
    );

    // La loi d'infusion n'hérite pas de l'état du régulateur de repos : une
    // intégrale accumulée au repos ne change pas la commande d'infusion.
    let mut idle_then_brew = Controller::new();
    let mut now = 1000;
    while now <= 20000 {
        step(&mut idle_then_brew, now, 87.0, 90.0, Mode::Idle);
        now += 250;
    }
    assert_eq!(
        permille(flow(
            &mut idle_then_brew,
            20250,
            87.0,
            90.0,
            Mode::Brew,
            2.0,
            true
        )),
        526
    );
}

#[test]
fn preheat_debt_is_repaid_by_the_infusion() {
    // Remboursement de la précharge : 5,5 s à 90 % font 495 %·s de dette. Le
    // remplissage garde son appoint ; dès l'infusion, l'appoint de 52,6 % à
    // 2 ml/s est retenu pendant 38 pas de 250 ms (13,15 %·s chacun).
    let mut repay = Controller::new();
    let mut now = 1000;
    while now < 6500 {
        assert_eq!(
            permille(step(&mut repay, now, 90.0, 90.0, Mode::ThermalPreheat)),
            900
        );
        now += 250;
    }
    while now < 9250 {
        assert_eq!(
            permille(flow(&mut repay, now, 90.0, 90.0, Mode::Brew, 3.6, true)),
            900
        );
        now += 250;
    }
    let mut withheld = 0;
    while permille(flow(&mut repay, now, 90.0, 90.0, Mode::Infusion, 2.0, true)) == 0 {
        withheld += 1;
        now += 250;
    }
    assert_eq!(withheld, 38);
    assert_eq!(
        permille(flow(
            &mut repay,
            now + 250,
            90.0,
            90.0,
            Mode::Infusion,
            2.0,
            true
        )),
        526
    );

    // Sans précharge, ou après une précharge interrompue, rien n'est retenu.
    let mut no_debt = Controller::new();
    step(&mut no_debt, 1000, 90.0, 90.0, Mode::Idle);
    assert_eq!(
        permille(flow(
            &mut no_debt,
            1250,
            90.0,
            90.0,
            Mode::Infusion,
            2.0,
            true
        )),
        526
    );
    let mut aborted = Controller::new();
    step(&mut aborted, 1000, 90.0, 90.0, Mode::ThermalPreheat);
    step(&mut aborted, 1250, 90.0, 90.0, Mode::ThermalPreheat);
    step(&mut aborted, 1500, 90.0, 90.0, Mode::Idle);
    assert_eq!(
        permille(flow(
            &mut aborted,
            1750,
            90.0,
            90.0,
            Mode::Infusion,
            2.0,
            true
        )),
        526
    );
    // Une précharge supprimée (NTC à la cible + 0,5 °C) ne crée pas de dette.
    let mut hot_preheat = Controller::new();
    step(&mut hot_preheat, 1000, 90.6, 90.0, Mode::ThermalPreheat);
    step(&mut hot_preheat, 1250, 90.6, 90.0, Mode::ThermalPreheat);
    assert_eq!(
        permille(flow(
            &mut hot_preheat,
            1500,
            90.6,
            90.0,
            Mode::Infusion,
            2.0,
            true
        )),
        526
    );
}

#[test]
fn debt_counts_real_command_duration() {
    // La dette se compte sur la durée réelle de chaque commande ; la sécurité,
    // déjà à 0 %, ne rembourse rien.
    let mut law = BrewHeating::new();
    law.reset();
    law.preheat_pct(1000, 90.0, 90.0);
    law.preheat_pct(1500, 90.0, 90.0);
    assert_eq!(law.flow_pct(2000, 90.0, 90.0, 2.0, true, true), 0.0);
    assert!((law.debt_pct_s() - 90.0).abs() < 1e-3);
    assert_eq!(law.flow_pct(2250, 95.0, 90.0, 2.0, true, true), 0.0);
    assert!((law.debt_pct_s() - (90.0 - 0.25 * 52.615)).abs() < 1e-2);
    assert_eq!(law.flow_pct(2500, 90.0, 90.0, 2.0, true, true), 0.0);
    assert!((law.debt_pct_s() - (90.0 - 0.25 * 52.615)).abs() < 1e-2);
    // Un pas manqué ne compte que pour 1 s.
    let mut gap = BrewHeating::new();
    gap.reset();
    gap.preheat_pct(1000, 90.0, 90.0);
    gap.flow_pct(4000, 90.0, 90.0, 2.0, true, false);
    assert!((gap.debt_pct_s() - 90.0).abs() < 1e-3);
}

#[test]
fn preheat_power() {
    // La précharge applique immédiatement sa puissance fixe sans débit. Elle
    // est coupée si la NTC est déjà à plus de 0,5 °C au-dessus de la cible.
    let mut preheat = Controller::new();
    let mut preheat_hot = Controller::new();
    assert_eq!(
        permille(step(&mut preheat, 1000, 90.0, 90.0, Mode::ThermalPreheat)),
        900
    );
    assert_eq!(
        permille(step(
            &mut preheat_hot,
            1000,
            90.6,
            90.0,
            Mode::ThermalPreheat
        )),
        0
    );
    let mut after_flow = Controller::new();
    step(&mut after_flow, 1000, 90.0, 90.0, Mode::Brew);
    step(&mut after_flow, 1250, 90.0, 90.0, Mode::Idle);
    assert_eq!(
        permille(step(
            &mut after_flow,
            1500,
            90.0,
            90.0,
            Mode::ThermalPreheat
        )),
        900
    );
}

#[test]
fn restart_window_flag() {
    // Nouvelle période SSR : une seule fois au début de la précharge, puis une
    // fois au début de l'écoulement, sur la première commande non nulle.
    let mut window = Controller::new();
    assert!(!step(&mut window, 1000, 89.9, 90.0, Mode::Idle).restart_window);
    let restart = step(&mut window, 1250, 89.9, 90.0, Mode::ThermalPreheat);
    assert!(restart.power_permille == 900 && restart.restart_window);
    assert!(!step(&mut window, 1500, 89.9, 90.0, Mode::ThermalPreheat).restart_window);
    assert!(!step(&mut window, 1750, 89.9, 90.0, Mode::ThermalPreheat).restart_window);
    let restart = step(&mut window, 2000, 89.9, 90.0, Mode::Brew);
    assert!(restart.power_permille == 450 && restart.restart_window);
    assert!(!flow(&mut window, 2250, 89.9, 90.0, Mode::Brew, 3.5, true).restart_window);
    assert!(!flow(&mut window, 2500, 89.9, 90.0, Mode::Brew, 1.5, true).restart_window);
    assert!(!step(&mut window, 2750, 90.0, 90.0, Mode::Idle).restart_window);
    // Cycle suivant : le flag revient.
    assert!(step(&mut window, 3000, 89.9, 90.0, Mode::ThermalPreheat).restart_window);

    // Précharge supprimée (NTC trop chaude) : pas de flag tant que la commande
    // est nulle ; il part avec la première commande non nulle de la phase.
    let mut window_hot = Controller::new();
    let restart = step(&mut window_hot, 1000, 90.6, 90.0, Mode::ThermalPreheat);
    assert!(restart.power_permille == 0 && !restart.restart_window);
    assert!(step(&mut window_hot, 1250, 90.4, 90.0, Mode::ThermalPreheat).restart_window);
    // Sans précharge, le premier paquet de l'écoulement porte le flag.
    let mut no_preheat = Controller::new();
    step(&mut no_preheat, 1000, 89.9, 90.0, Mode::Idle);
    assert!(step(&mut no_preheat, 1250, 89.9, 90.0, Mode::Brew).restart_window);
    // La purge n'en porte jamais.
    let mut purge = Controller::new();
    let mut now = 1000;
    while now <= 3000 {
        assert!(!flow(&mut purge, now, 89.0, 90.0, Mode::Purge, 3.5, true).restart_window);
        now += 250;
    }
}

#[test]
fn purge_flow_bonus() {
    // L'appoint de débit est linéaire entre 2 et 4 ml/s et se retire dès que
    // le débit baisse. Il reste limité au mode écoulement et aux mesures
    // fraîches.
    let mut low = Controller::new();
    let mut mid = Controller::new();
    let mut high = Controller::new();
    let mut stale = Controller::new();
    assert_eq!(
        permille(flow(&mut low, 1000, 90.0, 90.0, Mode::Purge, 2.0, true)),
        180
    );
    assert_eq!(
        permille(flow(&mut mid, 1000, 90.0, 90.0, Mode::Purge, 3.0, true)),
        230
    );
    assert_eq!(
        permille(flow(&mut high, 1000, 90.0, 90.0, Mode::Purge, 4.0, true)),
        280
    );
    assert_eq!(
        permille(flow(&mut high, 1250, 90.0, 90.0, Mode::Purge, 2.0, true)),
        180
    );
    assert_eq!(
        permille(flow(&mut stale, 1000, 90.0, 90.0, Mode::Brew, 4.0, false)),
        450
    );
    assert!(permille(flow(&mut stale, 1250, 90.0, 90.0, Mode::Idle, 4.0, true)) <= 350);
    // Le plancher et l'appoint de débit de la purge décroissent ensemble : une
    // NTC légèrement au-dessus de la cible ne coupe plus la chauffe d'un coup.
    let mut half = Controller::new();
    let mut one = Controller::new();
    let mut late = Controller::new();
    let mut no_flow = Controller::new();
    assert_eq!(
        permille(flow(&mut half, 1000, 90.5, 90.0, Mode::Purge, 4.0, true)),
        210
    );
    assert_eq!(
        permille(flow(&mut one, 1000, 91.0, 90.0, Mode::Purge, 4.0, true)),
        140
    );
    assert_eq!(
        permille(flow(&mut late, 1000, 91.8, 90.0, Mode::Purge, 4.0, true)),
        28
    );
    assert_eq!(
        permille(flow(
            &mut no_flow,
            1000,
            91.0,
            90.0,
            Mode::Purge,
            4.0,
            false
        )),
        90
    );
    let mut rising = Controller::new();
    assert_eq!(
        permille(flow(&mut rising, 1000, 90.0, 90.0, Mode::Purge, 4.0, true)),
        280
    );
    assert_eq!(
        permille(flow(&mut rising, 1250, 90.5, 90.0, Mode::Purge, 4.0, true)),
        210
    );
    assert_eq!(
        permille(flow(&mut rising, 1500, 92.0, 90.0, Mode::Purge, 4.0, true)),
        0
    );
    let mut capped = Controller::new();
    let mut too_hot = Controller::new();
    let mut at_limit = Controller::new();
    assert_eq!(
        permille(flow(&mut capped, 1000, 85.1, 90.0, Mode::Purge, 4.0, true)),
        450
    );
    assert_eq!(
        permille(flow(
            &mut at_limit,
            1000,
            92.0,
            90.0,
            Mode::Purge,
            4.0,
            true
        )),
        0
    );
    assert_eq!(
        permille(flow(&mut too_hot, 1000, 92.1, 90.0, Mode::Purge, 4.0, true)),
        0
    );
}

#[test]
fn recovery_after_purge_is_bounded() {
    // La baisse mesurée après une purge n'est pas extrapolée sur 20 secondes :
    // l'appoint et la reprise restent bornés même si la NTC chute fortement.
    let mut purge = Controller::new();
    step(&mut purge, 1000, 89.7, 90.0, Mode::Idle);
    let mut now = 1250;
    while now <= 9000 {
        let out = step(&mut purge, now, 89.7, 90.0, Mode::Purge);
        assert!((180..=350).contains(&out.power_permille), "{now}");
        now += 250;
    }
    let out = step(&mut purge, 9250, 87.3, 90.0, Mode::Idle);
    assert!(out.power_permille <= 350);
    now = 9500;
    while now <= 29000 {
        let out = step(&mut purge, now, 82.0, 90.0, Mode::Idle);
        assert!(out.power_permille <= 350);
        now += 250;
    }
    let out = step(&mut purge, 29250, 94.0, 90.0, Mode::Idle);
    assert_eq!(out.power_permille, 0);

    // La chaleur en transit et la pente montante décrivent en partie la même
    // énergie : la reprise ne doit pas s'arrêter alors que la NTC est à 83 °C.
    let mut rise = Controller::new();
    now = 1000;
    while now <= 9000 {
        step(&mut rise, now, 90.0, 90.0, Mode::Purge);
        now += 250;
    }
    now = 9250;
    while now <= 20000 {
        step(&mut rise, now, 79.4, 90.0, Mode::Idle);
        now += 250;
    }
    let mut out = Output::default();
    now = 20250;
    while now <= 28000 {
        out = step(
            &mut rise,
            now,
            79.4 + (now - 20000) as f32 / 2000.0,
            90.0,
            Mode::Idle,
        );
        now += 250;
    }
    assert!(out.power_permille > 0 && out.power_permille <= 350);
}

#[test]
fn cooling_purge_far_from_target() {
    // Une purge lancée loin de la cible sert au refroidissement : même si la
    // NTC passe sous la consigne durant l'écoulement, elle ne relance pas le SSR.
    let mut cooling = Controller::new();
    step(&mut cooling, 1000, 75.0, 65.0, Mode::Idle);
    assert_eq!(
        permille(step(&mut cooling, 1250, 75.0, 65.0, Mode::Purge)),
        0
    );
    assert_eq!(
        permille(step(&mut cooling, 2250, 70.0, 65.0, Mode::Purge)),
        0
    );
    assert_eq!(
        permille(step(&mut cooling, 3250, 63.0, 65.0, Mode::Purge)),
        0
    );
    let out = step(&mut cooling, 3500, 63.0, 65.0, Mode::Idle);
    assert!(out.power_permille > 0 && out.power_permille <= 350);

    let mut near = Controller::new();
    step(&mut near, 1000, 69.9, 65.0, Mode::Idle);
    step(&mut near, 1250, 69.9, 65.0, Mode::Purge);
    assert!(permille(step(&mut near, 2250, 65.0, 65.0, Mode::Purge)) >= 180);

    let mut cold = Controller::new();
    assert_eq!(permille(step(&mut cold, 1000, 59.0, 65.0, Mode::Purge)), 0);
}
