//! Régressions de changement de politique pendant une session radio.
#[path = "../../screen/src/ble/control.rs"]
mod control;
use screen_core::ble_policy::{Inputs, Scan};

#[test]
fn live_probe_scan_stops_when_brewing_starts_or_provisioning_begins() {
    let mut input = Inputs {
        probe_wanted: true,
        ..Inputs::default()
    };
    assert!(control::scan_continues(
        Scan::ProbeWindow,
        &input,
        true,
        100,
        1
    ));
    input.cycle_active = true;
    assert!(!control::scan_continues(
        Scan::ProbeWindow,
        &input,
        true,
        101,
        1
    ));
    input.cycle_active = false;
    assert!(!control::scan_continues(
        Scan::ProbeWindow,
        &input,
        false,
        101,
        1
    ));
    assert!(!control::scan_continues(
        Scan::ProbeWindow,
        &input,
        true,
        1_000_001,
        1
    ));
}

#[test]
fn live_scale_scan_yields_to_connection_and_switches_to_sparse_probe_policy() {
    let mut input = Inputs {
        scale_wanted: true,
        probe_wanted: true,
        cycle_active: true,
        connecting: false,
    };
    assert!(control::scan_continues(
        Scan::Continuous,
        &input,
        true,
        20_000_000,
        1
    ));
    input.connecting = true;
    assert!(!control::scan_continues(
        Scan::Continuous,
        &input,
        true,
        20_000_001,
        1
    ));
    input.connecting = false;
    input.scale_wanted = false;
    assert!(!control::scan_continues(
        Scan::Continuous,
        &input,
        true,
        20_000_002,
        1
    ));
    input.cycle_active = false;
    assert!(control::scan_continues(
        Scan::ProbeWindow,
        &input,
        true,
        20_000_100,
        20_000_010
    ));
    input.probe_wanted = false;
    assert!(!control::scan_continues(
        Scan::ProbeWindow,
        &input,
        true,
        20_000_101,
        20_000_010
    ));
}

#[test]
fn initiating_connection_or_provisioning_prevents_scan_start() {
    let mut input = Inputs {
        scale_wanted: true,
        ..Inputs::default()
    };
    assert!(control::scan_can_start(&input, true));
    input.connecting = true;
    assert!(!control::scan_can_start(&input, true));
    input.connecting = false;
    assert!(!control::scan_can_start(&input, false));
}
#[test]
fn notification_stream_does_not_postpone_heartbeat_deadline() {
    let mut scale = screen_core::acaia::Session::new();
    let deadline = 2_500_000;
    for now in (10_000..deadline).step_by(10_000) {
        assert!(!control::heartbeat_poll_due(now, deadline));
        assert!(scale.heartbeat(now as i64, true).is_empty());
    }
    assert!(control::heartbeat_poll_due(deadline, deadline));
    assert_eq!(scale.heartbeat(deadline as i64, true).len(), 3);
}
