//! Port de `archive/firmware/screen/test/test_shot_summary.cpp`.

#![allow(clippy::field_reassign_with_default)]

use screen_core::shot_summary::{ShotSample, ShotSampleMode, ShotSegment, ShotSummaryBuilder};

fn near(a: f32, b: f32, tolerance: f32) -> bool {
    (a - b).abs() <= tolerance
}

// Précharge 5 s, remplissage 3 s, pause 4 s, montée de 4 s jusqu'à 8 bar,
// infusion à 1 g/s et 1,5 ml/s jusqu'à 40 s, gouttes de 0,5 g en 2 s.
fn sample(t: f32, scale: bool, pressure_max: f32, removed_at: f32) -> ShotSample {
    let mode = if t < 5.0 {
        ShotSampleMode::Preheat
    } else if t < 8.0 {
        ShotSampleMode::Filling
    } else if t < 12.0 {
        ShotSampleMode::Preinfusion
    } else if t < 40.0 {
        ShotSampleMode::Infusion
    } else {
        ShotSampleMode::Cooldown
    };
    let mut s = ShotSample {
        t_s: t,
        mode,
        pump_pct: if (5.0..40.0).contains(&t) { 60 } else { 0 },
        pressure_valid: true,
        pressure_bar: if t < 12.0 {
            0.3
        } else {
            pressure_max.min(0.3 + (t - 12.0) * 2.0)
        },
        scale_present: scale,
        weight_g: 10.0
            + (t.min(40.0) - 16.0).max(0.0)
            + if t > 40.0 {
                (t - 40.0).min(2.0) * 0.25
            } else {
                0.0
            },
        volume_ml: (t.min(40.0) - 5.0).max(0.0) * 1.5,
        temperature_valid: true,
        temperature_c: if t < 16.0 {
            90.0 - (t - 5.0).max(0.0) * 0.1
        } else {
            88.9 + (t - 16.0) * 0.1
        },
    };
    if t >= removed_at {
        s.weight_g = -100.0;
    }
    s
}

fn run(until: f32, scale: bool, pressure_max: f32, removed_at: f32) -> ShotSummaryBuilder {
    let mut builder = ShotSummaryBuilder::new();
    builder.reset(9.0, 5.0);
    let mut i = 0;
    while i as f32 * 0.1 <= until + 1e-4 {
        builder.add(&sample(i as f32 * 0.1, scale, pressure_max, removed_at));
        i += 1;
    }
    builder
}

fn run_default(until: f32) -> ShotSummaryBuilder {
    run(until, true, 9.0, f32::INFINITY)
}

#[test]
fn complete_shot() {
    // Infusion complète : segments, infusion à consigne − 1 bar, débits, gouttes.
    let builder = run_default(60.0);
    let s = builder.summary();
    assert!(s.active);
    assert!(near(s.duration_s(ShotSegment::Preheat), 5.0, 0.11));
    assert!(near(s.duration_s(ShotSegment::Filling), 3.0, 0.11));
    assert!(near(s.duration_s(ShotSegment::Preinfusion), 4.0, 0.11));
    // 8 bar atteints à 12 + 7,7 / 2 = 15,85 s : premier échantillon à 15,9 s.
    assert!(near(
        s.segment_start_s[ShotSegment::Infusion as usize],
        15.9,
        0.01
    ));
    assert!(near(s.duration_s(ShotSegment::Ramp), 3.9, 0.11));
    assert!(near(s.pump_start_s, 5.0, 0.01) && near(s.pump_stop_s, 40.0, 0.01));
    assert!(near(s.infusion_s, 24.1, 0.01));
    assert!(near(s.infusion_cup_g_s, 23.9 / 24.1, 0.01));
    assert!(near(s.infusion_ml_s, 1.5, 0.02));
    // Poids relatif au premier échantillon (10 g de tare restante).
    assert!(near(s.cup_weight_g, 24.5, 0.06));
    assert!(near(s.drip_gain_g, 0.5, 0.06) && s.drip_done);
    assert!(near(s.start_temperature_c, 90.0, 1e-3) && near(s.min_temperature_c, 88.9, 0.02));
    assert!(near(s.max_temperature_c, 91.3, 0.02));
    // Moyenne pondérée par la tasse : NTC de 88,9 à 91,3 °C pendant l'écoulement.
    assert!(near(s.cup_mean_c, 90.1, 0.05));
    assert!(near(
        s.axis_s,
        s.segment_end_s[ShotSegment::Drip as usize],
        1e-3
    ));
}

#[test]
fn axis_scale() {
    // Échelle : 30 s de pompe prévus + 4 s de réserve, +25 % à 90 % de l'arrêt
    // prévu, puis l'arrêt réel.
    assert!(near(
        run_default(3.0).summary().axis_s,
        5.0 + 30.0 + 4.0,
        1e-3
    ));
    assert!(near(run_default(20.0).summary().axis_s, 35.0 + 4.0, 1e-3));
    assert!(near(
        run_default(32.0).summary().axis_s,
        35.0 * 1.25 + 4.0,
        1e-3
    ));
    let stopped = run_default(41.0);
    let stopped = stopped.summary();
    assert!(!stopped.drip_done && near(stopped.axis_s, 44.0, 1e-3));
}

#[test]
fn pressure_never_reached() {
    // Pression jamais atteinte : la montée devient l'infusion à l'arrêt.
    let running = run(30.0, true, 6.0, f32::INFINITY);
    let running = running.summary();
    assert!(running.has(ShotSegment::Ramp) && running.infusion_s.is_nan());
    let stopped = run(45.0, true, 6.0, f32::INFINITY);
    let s = stopped.summary();
    assert!(!s.has(ShotSegment::Ramp) && s.has(ShotSegment::Infusion));
    assert!(near(s.infusion_s, 28.0, 0.01));
}

#[test]
fn without_scale() {
    // Sans balance : poids et débit en tasse inconnus, débit amont connu.
    let builder = run(60.0, false, 9.0, f32::INFINITY);
    let s = builder.summary();
    assert!(s.cup_weight_g.is_nan() && s.infusion_cup_g_s.is_nan());
    assert!(s.cup_mean_c.is_nan() && near(s.infusion_ml_s, 1.5, 0.02));
    assert!(s.drip_done);
}

#[test]
fn cup_removed_during_drips() {
    // Tasse retirée pendant les gouttes : poids figé, gouttes terminées.
    let builder = run(60.0, true, 9.0, 41.0);
    let s = builder.summary();
    assert!(s.drip_done && near(s.cup_weight_g, 24.25, 0.06));
}

#[test]
fn purge_clears_the_summary() {
    // Une purge efface le résumé.
    let mut builder = run_default(10.0);
    builder.clear();
    assert!(!builder.summary().active);
}
