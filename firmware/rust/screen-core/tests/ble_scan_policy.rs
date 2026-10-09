//! Port de `archive/firmware/screen/test/test_ble_scan_policy.cpp`, plus la
//! règle des 50 % de temps radio.

#![allow(clippy::field_reassign_with_default)]

use screen_core::ble_policy::{
    Inputs, PROBE_SCAN_PERIOD_US, SCAN_INTERVAL_MS, SCAN_WINDOW_MS, Scan, decide, probe_window_due,
};

#[test]
fn nothing_wanted_stops_scan() {
    assert_eq!(decide(&Inputs::default()), Scan::Off);
}

#[test]
fn scale_keeps_continuous_scan() {
    let mut input = Inputs {
        scale_wanted: true,
        ..Inputs::default()
    };
    assert_eq!(decide(&input), Scan::Continuous);
    input.probe_wanted = true;
    assert_eq!(decide(&input), Scan::Continuous);
    input.cycle_active = true;
    assert_eq!(decide(&input), Scan::Continuous);
}

#[test]
fn probe_alone_uses_windows() {
    let input = Inputs {
        probe_wanted: true,
        ..Inputs::default()
    };
    assert_eq!(decide(&input), Scan::ProbeWindow);
}

#[test]
fn probe_alone_never_scans_during_cycle() {
    let input = Inputs {
        probe_wanted: true,
        cycle_active: true,
        ..Inputs::default()
    };
    assert_eq!(decide(&input), Scan::Off);
}

#[test]
fn connection_attempt_stops_scan() {
    let input = Inputs {
        scale_wanted: true,
        probe_wanted: true,
        connecting: true,
        ..Inputs::default()
    };
    assert_eq!(decide(&input), Scan::Off);
}

#[test]
fn probe_window_period() {
    const START: i64 = 5_000_000;
    assert!(probe_window_due(START, 0));
    assert!(!probe_window_due(START + PROBE_SCAN_PERIOD_US - 1, START));
    assert!(probe_window_due(START + PROBE_SCAN_PERIOD_US, START));
}

#[test]
fn scan_uses_at_most_half_of_the_radio_time() {
    assert_eq!((SCAN_INTERVAL_MS, SCAN_WINDOW_MS), (60, 30));
    for scan in [Scan::Continuous, Scan::ProbeWindow] {
        let params = scan.params().unwrap();
        assert!(u32::from(params.window_ms) * 2 <= u32::from(params.interval_ms));
    }
    assert!(Scan::Off.params().is_none());
    assert_eq!(Scan::Continuous.params().unwrap().duration_us, None);
    assert_eq!(
        Scan::ProbeWindow.params().unwrap().duration_us,
        Some(1_000_000)
    );
}
