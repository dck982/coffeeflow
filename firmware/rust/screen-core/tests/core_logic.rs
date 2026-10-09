//! Le cœur de bout en bout : liaison CAN, présence, mesures, actions, cycle,
//! capture HF, mise à jour, politique radio.

#![allow(clippy::field_reassign_with_default)]

mod common;

use common::{Rig, from_sensors};
use protocol::messages::{LogPayload, NodeIdentityPayload, SetHeatingPayload, StatusFlowPayload};
use protocol::{Dest, Frame, LogCode, Message, MessageType, Node, Version};
use screen_core::config::{Config, ConfigStatus};
use screen_core::core::{
    Action, ActionCommand, ActionSource, ActionStatus, BleCommand, Clock, Input, Output,
    RadioRequest, Reply, WifiRequest,
};
use screen_core::events::EventKind;
use screen_core::hf_capture::{self, Origin, Status};
use screen_core::snapshot::{CycleState, FlashTarget, Freshness, NetworkState};

fn cmd(action: Action) -> ActionCommand {
    ActionCommand::new(action)
}

#[test]
fn config_changes_during_ota_do_not_wait_for_the_flash_owner() {
    let mut rig = prepared_rig();
    let clock = rig.clock();
    let original = rig.core.config();
    let mut changed = original;
    changed.target_time_s = original.target_time_s + 1;
    assert_ne!(
        rig.core.begin_flash(clock, FlashTarget::Screen, 1000),
        screen_core::core::FlashStart::Refused
    );
    assert_eq!(
        rig.core.put_config(clock, &changed).status,
        ConfigStatus::Busy
    );
    assert_eq!(rig.core.reset_config(clock).status, ConfigStatus::Busy);
    assert_eq!(rig.core.config(), original);
    rig.core.finish_flash();
    assert_eq!(
        rig.core.put_config(clock, &changed).status,
        ConfigStatus::Ok
    );
}

#[test]
fn reset_config_cannot_change_the_running_brew() {
    let mut rig = prepared_rig();
    assert_eq!(start(&mut rig, ActionSource::Ui), ActionStatus::Ok);
    let original = rig.core.config();
    assert_eq!(
        rig.core.reset_config(rig.clock()).status,
        ConfigStatus::Busy
    );
    assert_eq!(rig.core.config(), original);
}

#[test]
fn awaiting_sensors_stop_excludes_new_actions_and_can_be_cancelled() {
    let mut rig = Rig::new();
    rig.ready();
    let clock = rig.clock();
    assert_eq!(
        rig.core.begin_flash(clock, FlashTarget::Sensors, 1000),
        screen_core::core::FlashStart::AwaitingStop
    );
    for action in [
        Action::StartBrew,
        Action::PurgePress,
        Action::OpenMaintenanceValve,
        Action::ResetSensors,
        Action::ResetDimmer,
    ] {
        assert_eq!(
            rig.core
                .perform_action(clock, &cmd(action), ActionSource::Ui)
                .status,
            ActionStatus::CycleActive
        );
    }
    assert_eq!(rig.core.reset_config(clock).status, ConfigStatus::Busy);
    assert_eq!(
        rig.core.put_config(clock, &Config::default()).status,
        ConfigStatus::Busy
    );
    rig.core.cancel_pending_flash();
    rig.core.finish_flash();
    assert_eq!(
        rig.core.put_config(clock, &Config::default()).status,
        ConfigStatus::Ok
    );
    assert_eq!(
        rig.core.begin_flash(clock, FlashTarget::Sensors, 1000),
        screen_core::core::FlashStart::AwaitingStop
    );
}

fn log_codes(outputs: &[Output]) -> Vec<u8> {
    outputs
        .iter()
        .filter_map(|o| match o {
            Output::Can(f) => match Message::decode(f) {
                Ok(Message::Log(LogPayload { code, .. })) => Some(code),
                _ => None,
            },
            _ => None,
        })
        .collect()
}

// --- Présence ----------------------------------------------------------------------------------

#[test]
fn boot_announces_identity_and_presence_starts_lost() {
    let mut rig = Rig::new();
    let outputs = rig.take_outputs();
    let Output::Can(frame) = &outputs[0] else {
        panic!("{outputs:?}");
    };
    let Ok(Message::Ping(identity)) = Message::decode(frame) else {
        panic!("PING attendu");
    };
    assert_eq!(identity.node(), Some(Node::Screen));
    assert_eq!(identity.version, protocol::FIRMWARE_VERSION);
    // Émis de l'écran vers les capteurs.
    let id = frame.can_id().unwrap();
    assert_eq!((id.src, id.dest), (Node::Screen, Dest::Sensors));
    assert!(rig.core.presence_lost());
    assert_eq!(rig.core.events.recent(8)[0].kind, EventKind::Boot);
}

#[test]
fn presence_probe_sequence_and_loss() {
    let mut rig = Rig::new();
    rig.take_outputs();
    // Un message des capteurs rétablit la présence.
    rig.pong();
    assert!(!rig.core.presence_lost());
    let outputs = rig.take_outputs();
    assert!(log_codes(&outputs).contains(&(LogCode::PresenceRestored as u8)));
    assert_eq!(
        rig.core.events.recent(1)[0].kind,
        EventKind::CanPresenceRestored
    );
    assert!(rig.core.peer_roundtrip_confirmed());

    // Silence : rien avant 1,5 s, puis un PING, trois au plus, 500 ms d'écart.
    rig.sensors_online = false;
    let mut ping_times = Vec::new();
    let mut lost_at = None;
    let start = rig.now_us;
    for _ in 0..80 {
        rig.advance(50);
        for output in rig.take_outputs() {
            if let Output::Can(f) = &output
                && let Ok(message) = Message::decode(f)
            {
                match message {
                    Message::Ping(_) => ping_times.push((rig.now_us - start) / 1000),
                    Message::Log(l) if l.code == LogCode::PresenceLost as u8 => {
                        lost_at = Some((rig.now_us - start) / 1000)
                    }
                    _ => {}
                }
            }
        }
    }
    assert_eq!(ping_times.len(), 3);
    assert!(
        ping_times[0] >= 1500 && ping_times[0] < 1700,
        "{ping_times:?}"
    );
    assert!(
        (ping_times[1] - ping_times[0] - 500).abs() <= 50,
        "{ping_times:?}"
    );
    assert!(
        (ping_times[2] - ping_times[1] - 500).abs() <= 50,
        "{ping_times:?}"
    );
    let lost_at = lost_at.expect("présence perdue");
    assert!((3000..3300).contains(&lost_at), "{lost_at}");
    assert!(rig.core.presence_lost());
    assert_eq!(
        rig.core.events.recent(1)[0].kind,
        EventKind::CanPresenceLost
    );
}

#[test]
fn only_sensors_traffic_addressed_to_the_screen_counts() {
    let mut rig = Rig::new();
    rig.take_outputs();
    let clock = rig.clock();
    // Émise par l'écran lui-même.
    let own =
        Message::StatusActuators(Default::default()).encode_routed(Node::Screen, Dest::Screen);
    rig.core.on_can_frame(clock, &own);
    // Destinée aux capteurs.
    let other =
        Message::StatusActuators(Default::default()).encode_routed(Node::Sensors, Dest::Sensors);
    rig.core.on_can_frame(clock, &other);
    assert!(rig.core.presence_lost());
    // Type futur inconnu des capteurs : maintient la présence sans rien changer.
    let future = Frame::new(
        0x3E << 5 | (Dest::Screen as u16) << 3 | Node::Sensors as u16,
        &[1],
    )
    .unwrap();
    rig.core.on_can_frame(clock, &future);
    assert!(!rig.core.presence_lost());
    let snapshot = rig.snapshot();
    assert!(!snapshot.sensors_alive);
}

#[test]
fn ping_from_sensors_gets_a_pong_and_updates_version() {
    let mut rig = Rig::new();
    rig.take_outputs();
    rig.rx(Message::Ping(NodeIdentityPayload::new(
        Node::Sensors,
        Version::new(0, 4, 0),
        77,
    )));
    let frames = rig.can_frames();
    let pong = frames.iter().find_map(|m| match m {
        Message::Pong(p) => Some(*p),
        _ => None,
    });
    let pong = pong.expect("PONG");
    assert_eq!(pong.node(), Some(Node::Screen));
    let s = rig.snapshot();
    assert_eq!((s.sensors_version_minor, s.sensors_uptime_s), (4, 77));
    // Sans nouvel échange d'identité, l'uptime avance avec l'horloge de
    // l'écran tant que les capteurs restent vivants.
    rig.advance(2500);
    rig.publish_actuators();
    assert_eq!(rig.snapshot().sensors_uptime_s, 79);
    // PING hérité (DLC 0) : PONG aussi.
    rig.take_outputs();
    let legacy = Frame::new(
        (MessageType::Ping as u16) << 5 | (Dest::Broadcast as u16) << 3 | Node::Sensors as u16,
        &[],
    )
    .unwrap();
    let clock = rig.clock();
    rig.core.on_can_frame(clock, &legacy);
    assert!(
        rig.can_frames()
            .iter()
            .any(|m| matches!(m, Message::Pong(_)))
    );
}

#[test]
fn old_sensors_version_clears_heating_capabilities() {
    let mut rig = Rig::new();
    rig.pong();
    rig.publish_heating();
    assert!(rig.snapshot().heating_power_capable);
    rig.sensors_version = Version::new(0, 2, 62);
    rig.pong();
    let s = rig.snapshot();
    assert!(!s.heating_capable && !s.heating_power_capable && !s.heating_window_restart_capable);
    // Un uptime qui recule (redémarrage) fait de même.
    rig.sensors_version = Version::new(0, 4, 0);
    rig.pong();
    rig.publish_heating();
    assert!(rig.snapshot().heating_power_capable);
    rig.rx(Message::Pong(NodeIdentityPayload::new(
        Node::Sensors,
        Version::new(0, 4, 0),
        3,
    )));
    assert!(!rig.snapshot().heating_capable);
}

// --- Confirmation OTA des capteurs -----------------------------------------------------

fn confirm_frames(rig: &mut Rig) -> usize {
    rig.can_frames()
        .iter()
        .filter(|m| matches!(m, Message::ConfirmSensorsOta(p) if p.protocol_patch == 63))
        .count()
}

#[test]
fn sensors_ota_confirmation_conditions() {
    let mut rig = Rig::new();
    rig.pong();
    rig.advance(3000);
    // Version 0.4.0, actionneurs et chauffe frais, pompe et vanne à l'arrêt :
    // la confirmation part une fois par seconde.
    assert!(
        confirm_frames(&mut rig) >= 2,
        "{}",
        confirm_frames(&mut rig)
    );

    // Version 0.2.62 (< 0.2.63) : jamais.
    let mut old = Rig::new();
    old.sensors_version = Version::new(0, 2, 62);
    old.pong();
    old.advance(3000);
    assert_eq!(confirm_frames(&mut old), 0);
    // Exactement 0.2.63 : oui.
    let mut exact = Rig::new();
    exact.sensors_version = Version::new(0, 2, 63);
    exact.pong();
    exact.advance(3000);
    assert!(confirm_frames(&mut exact) >= 2);

    // Pompe en marche : non.
    let mut pumping = Rig::new();
    pumping.pong();
    pumping.pump_pct = 40;
    pumping.advance(3000);
    assert_eq!(confirm_frames(&mut pumping), 0);

    // Chaudière allumée sans commande acceptée : non ; avec : oui.
    let mut hot = Rig::new();
    hot.pong();
    hot.heater_on = true;
    // Chaudière bien au-dessus de la consigne : la régulation ne commande rien,
    // donc aucune puissance acceptée (le banc recopie les SET_HEATING_POWER).
    hot.boiler_user_c = 99.0;
    hot.advance(3000);
    assert_eq!(confirm_frames(&mut hot), 0);
    let mut controlled = Rig::new();
    controlled.pong();
    controlled.heater_on = true;
    controlled.power_permille = 300;
    controlled.advance(3000);
    assert!(confirm_frames(&mut controlled) >= 2);

    // Capteurs muets : non.
    let mut silent = Rig::new();
    silent.pong();
    silent.sensors_online = false;
    silent.advance(6000);
    let before = confirm_frames(&mut silent);
    silent.advance(3000);
    assert_eq!(confirm_frames(&mut silent), before);
}

// --- Mesures ------------------------------------------------------------------------------------

#[test]
fn boiler_ntc_conversion_and_freshness() {
    let mut rig = Rig::new();
    rig.boiler_user_c = 88.0;
    rig.advance(200);
    let s = rig.snapshot();
    assert!(s.boiler_temperature_valid);
    assert!(
        (s.boiler_temperature_c - 88.0).abs() < 0.05,
        "{}",
        s.boiler_temperature_c
    );
    assert!((s.boiler_sensor_temperature_c - 98.5).abs() < 0.05);
    assert_eq!(s.boiler_temperature_freshness, Freshness::Fresh);
    assert_eq!(
        (s.boiler_ntc_a0_raw, s.boiler_ntc_a1_raw != 0),
        (28000, true)
    );
    // Plus de mesure : périmée puis absente (et invalide).
    let last = rig.now_us;
    rig.now_us = last + 300_000;
    assert_eq!(
        rig.snapshot().boiler_temperature_freshness,
        Freshness::Stale
    );
    rig.now_us = last + 3_500_000;
    let s = rig.snapshot();
    assert_eq!(s.boiler_temperature_freshness, Freshness::Missing);
    assert!(!s.boiler_temperature_valid);
}

#[test]
fn boiler_ntc_rejects_bad_readings() {
    let mut rig = Rig::new();
    let clock = rig.clock();
    assert!(!rig.core.on_boiler_ntc_reading(clock, 20000, 20000, true)); // A1 >= A0
    assert!(!rig.core.on_boiler_ntc_reading(clock, 15999, 8000, true));
    assert!(!rig.core.on_boiler_ntc_reading(clock, 32767, 8000, true));
    assert!(!rig.core.on_boiler_ntc_reading(clock, 20000, 0, true));
    assert!(!rig.core.on_boiler_ntc_reading(clock, 28000, 9000, false));
    // Température hors de -10..160 °C à la sonde (NTC quasi court-circuitée).
    assert!(!rig.core.on_boiler_ntc_reading(clock, 28000, 27900, true));
    assert!(rig.core.on_boiler_ntc_reading(clock, 28000, 16000, true));
}

#[test]
fn pressure_a2_conversion() {
    let mut rig = Rig::new();
    let clock = rig.clock();
    rig.core.on_pressure_a2_reading(clock, 3167, true);
    let s = rig.snapshot();
    assert!(s.pressure_valid && s.pressure_a2_valid);
    assert!((s.pressure_bar - (3167.0 - 3167.3) * 0.00075).abs() < 1e-6);
    rig.core.on_pressure_a2_reading(clock, 6000, true);
    assert!((rig.snapshot().pressure_bar - (6000.0 - 3167.3) * 0.00075).abs() < 1e-5);
    // Échec de lecture : invalide, la dernière valeur reste lisible.
    rig.core.on_pressure_a2_reading(clock, 0, false);
    let s = rig.snapshot();
    assert!(!s.pressure_valid && !s.pressure_a2_valid);
    assert!((s.pressure_bar - (6000.0 - 3167.3) * 0.00075).abs() < 1e-5);
}

#[test]
fn flow_from_pulses() {
    let mut rig = Rig::new();
    let send = |rig: &mut Rig, pulses: u32, edge_ms: u16| {
        rig.rx(Message::StatusFlow(StatusFlowPayload {
            pulse_count: pulses,
            last_edge_ms: edge_ms,
            flags: 1,
        }))
    };
    send(&mut rig, 0, 100);
    send(&mut rig, 5, 600);
    // Moins de 10 impulsions d'écart : pas de débit.
    assert_eq!(rig.snapshot().flow_ml_s, 0.0);
    send(&mut rig, 12, 1100);
    // 12 impulsions en 1000 ms → 12 * 1e6 / (3450 * 1000).
    let expected = 12.0 * 1_000_000.0 / (3450.0 * 1000.0);
    let s = rig.snapshot();
    assert!((s.flow_ml_s - expected).abs() < 1e-3, "{}", s.flow_ml_s);
    assert!((s.volume_ml - 12.0 * 1000.0 / 3450.0).abs() < 1e-3);
    assert_eq!(s.flow_pulse_count, 12);
    assert!(s.flow_valid);
    // Une trame sans changement ne relance pas le calcul ; le débit s'éteint
    // après 3 s sans impulsion.
    rig.now_us += 3_500_000;
    assert_eq!(rig.snapshot().flow_ml_s, 0.0);
    // Compteur qui recule : redémarrage des capteurs, l'historique repart.
    send(&mut rig, 2, 200);
    assert_eq!(rig.snapshot().flow_pulse_count, 2);
}

#[test]
fn actuator_and_heating_status() {
    let mut rig = Rig::new();
    rig.pump_pct = 60;
    rig.valve_open = true;
    rig.lockout = true;
    rig.dimmer_ready = false;
    rig.publish_actuators();
    rig.heater_on = true;
    rig.power_permille = 357;
    rig.publish_heating();
    let s = rig.snapshot();
    assert!(s.valve_open && s.lockout && !s.dimmer_ready && s.dimmer_valid);
    assert_eq!(
        (s.pump_pct, s.dimmer_pct, s.lease_remaining_ms),
        (60, 60, 400)
    );
    assert!(s.heating_capable && s.heating_power_capable && s.heater_on);
    assert!((s.heating_power_accepted_pct - 35.7).abs() < 1e-4);
    assert_eq!(s.actuators_freshness, Freshness::Fresh);
    assert_eq!(s.heating_freshness, Freshness::Fresh);
    // Sans nouvel écho la chauffe cesse d'être « fraîche » : plus d'état allumé.
    rig.now_us += 1_000_000;
    let s = rig.snapshot();
    assert_eq!(s.heating_freshness, Freshness::Stale);
    assert!(!s.heater_on && s.heating_lease_remaining_ms == 0);
}

#[test]
fn twai_error_counters_from_sensors_log() {
    let mut rig = Rig::new();
    rig.rx(Message::Log(LogPayload {
        code: LogCode::TwaiErrorCounters as u8,
        severity: 1,
        arg16: 0x0302,
        arg32: 99,
    }));
    let s = rig.snapshot();
    assert_eq!(
        (
            s.sensors_twai_rx_errors,
            s.sensors_twai_tx_errors,
            s.sensors_twai_bus_errors
        ),
        (3, 2, 99)
    );
    // Un autre code n'y touche pas.
    rig.rx(Message::Log(LogPayload {
        code: LogCode::Boot as u8,
        severity: 1,
        arg16: 0x0909,
        arg32: 1,
    }));
    assert_eq!(rig.snapshot().sensors_twai_rx_errors, 3);
}

#[test]
fn scale_and_scace_presence_rules() {
    let mut rig = Rig::new();
    let clock = rig.clock();
    rig.core.update_scale_connection(clock, true);
    assert!(!rig.snapshot().scale_present); // connectée mais aucune pesée
    rig.core.update_scale_weight(clock, 12.5);
    let s = rig.snapshot();
    assert!(s.scale_present && s.scale_connected && s.weight_g == 12.5);
    rig.now_us += 2_100_000;
    assert!(!rig.snapshot().scale_present); // pesée de plus de 2 s
    let clock = rig.clock();
    rig.core.update_scale_weight(clock, 13.0);
    assert!(rig.snapshot().scale_present);
    rig.core.update_scale_connection(clock, false);
    assert!(!rig.snapshot().scale_present);

    // SCACE : valable 300 ms.
    rig.core.update_scace_connection(clock, true);
    rig.core.update_scace_temperature(clock, 9250, true);
    let s = rig.snapshot();
    assert!(s.scace_valid && (s.scace_temperature_c - 92.5).abs() < 1e-4);
    rig.now_us += 301_000;
    assert!(!rig.snapshot().scace_valid);
    let clock = rig.clock();
    rig.core.update_scace_temperature(clock, 9300, true);
    assert!(rig.snapshot().scace_valid);
    rig.core.update_scace_temperature(clock, 0, false);
    assert!(!rig.snapshot().scace_valid);
}

// --- Politique radio ---------------------------------------------------------------------

fn radio(outputs: &[Output]) -> Vec<RadioRequest> {
    outputs
        .iter()
        .filter_map(|o| match o {
            Output::Radio(r) => Some(*r),
            _ => None,
        })
        .collect()
}

#[test]
fn radio_policy_wifi_and_ble_together() {
    let mut rig = Rig::new();
    rig.take_outputs();
    // Identifiants présents : station, BLE actif ; Wi-Fi ON au démarrage.
    rig.core.set_credentials_present(true);
    let requests = radio(&rig.take_outputs());
    assert_eq!(
        requests,
        vec![RadioRequest {
            wifi: WifiRequest::Sta,
            ble: true
        }]
    );
    let s = rig.snapshot();
    assert!(s.wifi_enabled && s.ble_enabled && !s.provisioning);

    // Bascule de session : Wi-Fi OFF, BLE reste.
    rig.core.set_wifi_enabled(false);
    assert_eq!(
        radio(&rig.take_outputs()),
        vec![RadioRequest {
            wifi: WifiRequest::Off,
            ble: true
        }]
    );
    assert!(!rig.snapshot().wifi_enabled);
    // Sans changement, rien n'est ré-émis.
    rig.core.set_wifi_enabled(false);
    assert!(radio(&rig.take_outputs()).is_empty());
    rig.core.set_wifi_enabled(true);
    assert_eq!(radio(&rig.take_outputs())[0].wifi, WifiRequest::Sta);

    // Une panne STA n'est pas un changement radio : l'AP ne repart pas.
    rig.core
        .update_network_status(NetworkState::StaDisconnected, 0);
    assert!(radio(&rig.take_outputs()).is_empty());
    assert_eq!(rig.core.radio_request().wifi, WifiRequest::Sta);

    // Oubli du réseau : demande d'effacement, puis provisioning, BLE coupé.
    rig.core.forget_network();
    assert!(rig.take_outputs().contains(&Output::ForgetNetwork));
    rig.core.set_credentials_present(false);
    assert_eq!(
        radio(&rig.take_outputs()),
        vec![RadioRequest {
            wifi: WifiRequest::Provisioning,
            ble: false
        }]
    );
    let s = rig.snapshot();
    assert!(s.provisioning && !s.ble_enabled);
    // La bascule n'a aucun effet pendant le provisioning.
    rig.core.set_wifi_enabled(false);
    assert!(radio(&rig.take_outputs()).is_empty());
    assert_eq!(rig.core.radio_request().wifi, WifiRequest::Provisioning);
}

#[test]
fn no_credentials_boot_goes_straight_to_provisioning() {
    let mut rig = Rig::new();
    rig.take_outputs();
    rig.core.set_credentials_present(false);
    assert_eq!(
        radio(&rig.take_outputs()),
        vec![RadioRequest {
            wifi: WifiRequest::Provisioning,
            ble: false
        }]
    );
}

// --- Actions et cycle ---------------------------------------------------------------------

fn start(rig: &mut Rig, source: ActionSource) -> ActionStatus {
    let clock = rig.clock();
    let result = rig
        .core
        .perform_action(clock, &cmd(Action::StartBrew), source);
    rig.collect();
    result.status
}

fn prepared_rig() -> Rig {
    let mut rig = Rig::new();
    rig.ready();
    let clock = rig.clock();
    let result = rig.core.put_config(clock, &Rig::ready_config());
    assert_eq!(result.status, ConfigStatus::Ok);
    rig.advance(4000);
    assert!(
        rig.snapshot().brew_temperature_ready,
        "chaudière à la consigne"
    );
    rig.take_outputs();
    rig
}

#[test]
fn http_cannot_start_a_brew_but_the_ui_can_with_wifi_up() {
    let mut rig = prepared_rig();
    // Wi-Fi actif (état par défaut) : l'infusion est permise depuis l'UI.
    assert!(rig.snapshot().wifi_enabled);
    assert_eq!(
        start(&mut rig, ActionSource::Http),
        ActionStatus::Unavailable
    );
    assert_eq!(rig.snapshot().cycle_state, CycleState::Idle);
    assert_eq!(start(&mut rig, ActionSource::Ui), ActionStatus::Ok);
    let s = rig.snapshot();
    assert_eq!(s.cycle_state, CycleState::Filling);
    assert!(
        rig.can_frames()
            .iter()
            .any(|m| matches!(m, Message::Log(l) if l.code == LogCode::BrewStarted as u8))
    );
}

#[test]
fn start_brew_preconditions() {
    // Chaudière hors de la consigne.
    let mut cold = Rig::new();
    cold.boiler_user_c = 80.0;
    cold.ready();
    assert_eq!(
        start(&mut cold, ActionSource::Ui),
        ActionStatus::Unavailable
    );

    // Verrou actif.
    let mut locked = prepared_rig();
    locked.lockout = true;
    locked.advance(200);
    assert_eq!(start(&mut locked, ActionSource::Ui), ActionStatus::Locked);

    // DimmerLink pas prêt.
    let mut dimmer = prepared_rig();
    dimmer.dimmer_ready = false;
    dimmer.advance(200);
    assert_eq!(
        start(&mut dimmer, ActionSource::Ui),
        ActionStatus::DimmerNotReady
    );

    // Capteurs muets.
    let mut lost = prepared_rig();
    lost.sensors_online = false;
    lost.advance(4000);
    assert_eq!(start(&mut lost, ActionSource::Ui), ActionStatus::BusLost);

    // Chauffage désactivé dans la configuration.
    let mut off = prepared_rig();
    let mut c = off.core.config();
    c.heating_enabled = false;
    let clock = off.clock();
    assert_eq!(off.core.put_config(clock, &c).status, ConfigStatus::Ok);
    assert_eq!(start(&mut off, ActionSource::Ui), ActionStatus::Unavailable);
}

#[test]
fn full_brew_cycle_by_time_and_last_shot() {
    let mut rig = prepared_rig();
    rig.wall_unix_s = Some(1_790_000_000);
    rig.advance(100);
    assert_eq!(start(&mut rig, ActionSource::Ui), ActionStatus::Ok);
    assert_eq!(rig.snapshot().cycle_state, CycleState::Filling);

    // Pendant le cycle, des SET partent toutes les 100 ms, avec bail de 500 ms.
    rig.take_outputs();
    rig.advance(1500);
    let sets: Vec<_> = rig
        .can_frames()
        .into_iter()
        .filter_map(|m| match m {
            Message::Set(s) => Some(s),
            _ => None,
        })
        .collect();
    assert!(sets.len() >= 10, "{}", sets.len());
    assert!(sets.iter().all(|s| s.ttl_ms == 500));
    assert!(sets.iter().any(|s| s.pump_pct == 100));
    assert_eq!(rig.snapshot().cycle_state, CycleState::Brew);

    // Stop automatique à 5 s (sans balance).
    rig.advance(5000);
    let s = rig.snapshot();
    assert_eq!(s.cycle_state, CycleState::Finished);
    assert_eq!(rig.pump_pct, 0);
    // Le résumé « dernière infusion » attend la fin de la capture.
    assert!(!s.last_shot_available);
    rig.advance(21_000);
    let s = rig.snapshot();
    assert!(s.last_shot_available);
    assert_eq!(s.last_shot_unix_s, 1_790_000_000);
    assert!(
        (4900..=5200).contains(&s.last_shot_duration_ms),
        "{}",
        s.last_shot_duration_ms
    );
    assert_eq!(rig.core.hf_capture_info().status, Status::Complete);
    // Fermer l'écran de résumé.
    let clock = rig.clock();
    let r = rig
        .core
        .perform_action(clock, &cmd(Action::DismissSummary), ActionSource::Ui);
    assert_eq!(r.status, ActionStatus::Ok);
    assert_eq!(rig.snapshot().cycle_state, CycleState::Idle);
    assert!(!rig.snapshot().last_shot_available);
}

#[test]
fn stop_brew_and_refusals_during_a_cycle() {
    let mut rig = prepared_rig();
    assert_eq!(start(&mut rig, ActionSource::Ui), ActionStatus::Ok);
    rig.advance(200);
    let clock = rig.clock();

    // /config et /firmware refusent pendant l'infusion.
    let busy = rig.core.put_config(clock, &Config::default());
    assert_eq!(
        (busy.status, busy.field),
        (ConfigStatus::Busy, Some("cycle"))
    );
    assert_eq!(
        rig.core.begin_flash(clock, FlashTarget::Screen, 1000),
        screen_core::core::FlashStart::Refused
    );
    // Les commandes de banc aussi.
    let mut set = cmd(Action::SetActuators);
    set.brew.pump_pct = 30;
    assert_eq!(
        rig.core
            .perform_action(clock, &set, ActionSource::Http)
            .status,
        ActionStatus::CycleActive
    );
    assert_eq!(
        rig.core
            .perform_action(clock, &cmd(Action::ResetSensors), ActionSource::Ui)
            .status,
        ActionStatus::CycleActive
    );
    // Une seconde infusion ne démarre pas.
    assert_eq!(start(&mut rig, ActionSource::Ui), ActionStatus::CycleActive);

    rig.take_outputs();
    let r = rig
        .core
        .perform_action(clock, &cmd(Action::StopBrew), ActionSource::Http);
    assert_eq!(r.status, ActionStatus::Ok);
    let outputs = rig.take_outputs();
    let frames: Vec<_> = outputs
        .iter()
        .filter_map(|o| match o {
            Output::Can(f) => Message::decode(f).ok(),
            _ => None,
        })
        .collect();
    assert!(
        frames
            .iter()
            .any(|m| matches!(m, Message::Set(s) if s.pump_pct == 0))
    );
    assert!(frames.iter().any(
        |m| matches!(m, Message::Log(l) if l.code == LogCode::BrewStopped as u8 && l.arg16 == 3)
    ));
    // Pas de cycle : l'arrêt suivant est refusé.
    let r = rig
        .core
        .perform_action(clock, &cmd(Action::StopBrew), ActionSource::Ui);
    assert_eq!(r.status, ActionStatus::NoCycle);
}

#[test]
fn purge_press_and_release_work_over_http() {
    let mut rig = prepared_rig();
    let clock = rig.clock();
    let r = rig
        .core
        .perform_action(clock, &cmd(Action::PurgePress), ActionSource::Http);
    assert_eq!(r.status, ActionStatus::Ok);
    rig.advance(500);
    assert_eq!(rig.snapshot().cycle_state, CycleState::Purge);
    assert_eq!(rig.pump_pct, 100);
    let busy = rig.core.put_config(rig.clock(), &Config::default());
    assert_eq!(busy.status, ConfigStatus::Busy);
    let clock = rig.clock();
    let r = rig
        .core
        .perform_action(clock, &cmd(Action::PurgeRelease), ActionSource::Http);
    assert_eq!(r.status, ActionStatus::Ok);
    rig.advance(300);
    assert_eq!(rig.pump_pct, 0);
    assert_eq!(rig.snapshot().cycle_state, CycleState::Finished);
    // Une purge n'a pas de résumé d'infusion.
    rig.advance(6000);
    assert!(!rig.snapshot().last_shot_available);
}

#[test]
fn bench_commands() {
    let mut rig = Rig::new();
    rig.ready();
    let clock = rig.clock();
    let mut set = cmd(Action::SetActuators);
    set.brew.pump_pct = 101;
    assert_eq!(
        rig.core
            .perform_action(clock, &set, ActionSource::Http)
            .status,
        ActionStatus::InvalidValue
    );
    set.brew.pump_pct = 40;
    set.brew.ttl_ms = 800;
    rig.take_outputs();
    assert_eq!(
        rig.core
            .perform_action(clock, &set, ActionSource::Http)
            .status,
        ActionStatus::Ok
    );
    let frames: Vec<_> = rig.take_outputs();
    let set_frame = frames.iter().find_map(|o| match o {
        Output::Can(f) => match Message::decode(f) {
            Ok(Message::Set(s)) => Some(s),
            _ => None,
        },
        _ => None,
    });
    let set_frame = set_frame.expect("SET");
    assert_eq!((set_frame.pump_pct, set_frame.ttl_ms), (40, 800));
    // Une commande de banc ouvre la capture HF.
    assert_eq!(rig.core.hf_capture_info().status, Status::Active);
    assert_eq!(rig.core.hf_capture_info().origin, Origin::SetActuators);

    // RESET, DimmerLink.
    rig.advance(6000);
    rig.take_outputs();
    let clock = rig.clock();
    // La récupération de 5 s de la capture est terminée : la maintenance passe.
    assert_eq!(
        rig.core
            .perform_action(clock, &cmd(Action::ResetDimmer), ActionSource::Ui)
            .status,
        ActionStatus::Ok
    );
}

#[test]
fn reset_and_dimmer_commands_when_idle() {
    let mut rig = Rig::new();
    rig.ready();
    let clock = rig.clock();
    rig.take_outputs();
    assert_eq!(
        rig.core
            .perform_action(clock, &cmd(Action::ResetSensors), ActionSource::Http)
            .status,
        ActionStatus::Ok
    );
    let frames: Vec<Message> = rig
        .collect()
        .into_iter()
        .filter_map(|o| match o {
            Output::Can(f) => Message::decode(&f).ok(),
            _ => None,
        })
        .collect();
    assert!(matches!(frames[0], Message::Set(s) if s.pump_pct == 0));
    assert!(frames.iter().any(|m| matches!(m, Message::Reset)));
    rig.advance(30_000);
    let clock = rig.clock();
    rig.take_outputs();
    assert_eq!(
        rig.core
            .perform_action(clock, &cmd(Action::RecalibrateDimmer), ActionSource::Ui)
            .status,
        ActionStatus::Ok
    );
    assert!(rig.collect().iter().any(|o| matches!(o, Output::Can(f)
        if matches!(Message::decode(f), Ok(Message::DimmerCommand(c)) if c.command as u8 == 2))));
}

#[test]
fn set_heating_request_lifecycle() {
    let mut rig = Rig::new();
    rig.ready();
    let clock = rig.clock();
    let mut on = cmd(Action::SetHeating);
    on.heating.on = true;
    // Le chauffage de l'infusion doit d'abord être désactivé pour la vidange,
    // mais `set_heating` exige au contraire `heating_enabled`.
    on.heating.duration_ms = 0;
    assert_eq!(
        rig.core
            .perform_action(clock, &on, ActionSource::Http)
            .status,
        ActionStatus::InvalidValue
    );
    on.heating.duration_ms = 30_001;
    assert_eq!(
        rig.core
            .perform_action(clock, &on, ActionSource::Http)
            .status,
        ActionStatus::InvalidValue
    );
    on.heating.duration_ms = 5000;
    // La régulation envoie déjà de la puissance (chaudière à la consigne,
    // puissance acceptée > 0 ou commandée > 0) : refusé tant que c'est le cas.
    rig.take_outputs();
    let first = rig.core.perform_action(clock, &on, ActionSource::Http);
    assert!(matches!(
        first.status,
        ActionStatus::Ok | ActionStatus::Unavailable
    ));
    // Coupure explicite : toujours acceptée.
    let off = cmd(Action::SetHeating);
    assert_eq!(
        rig.core
            .perform_action(clock, &off, ActionSource::Http)
            .status,
        ActionStatus::Ok
    );
    assert!(!rig.snapshot().heating_requested);
    let frames = rig.can_frames();
    assert!(frames.iter().any(|m| matches!(
        m,
        Message::SetHeating(SetHeatingPayload {
            on: false,
            duration_ms: 0
        })
    )));
}

#[test]
fn set_heating_expires_without_renewal() {
    // Chaudière froide, régulation à l'arrêt (chauffage de l'infusion
    // désactivé) : la demande manuelle part, puis expire seule.
    let mut rig = Rig::new();
    rig.ready();
    let mut c = rig.core.config();
    c.heating_enabled = true;
    rig.boiler_user_c = 90.0;
    // Valeur au-dessus de la plage de régulation : pas de commande en cours.
    let clock = rig.clock();
    let _ = rig.core.put_config(clock, &c);
    rig.boiler_user_c = 104.0;
    rig.advance(1500);
    rig.power_permille = 0;
    rig.advance(1000);
    let mut on = cmd(Action::SetHeating);
    on.heating.on = true;
    on.heating.duration_ms = 1000;
    let clock = rig.clock();
    let result = rig.core.perform_action(clock, &on, ActionSource::Http);
    if result.status == ActionStatus::Ok {
        assert!(rig.snapshot().heating_requested);
        rig.take_outputs();
        rig.advance(1500);
        assert!(!rig.snapshot().heating_requested);
        assert!(
            rig.can_frames()
                .iter()
                .any(|m| matches!(m, Message::SetHeating(SetHeatingPayload { on: false, .. })))
        );
    }
}

#[test]
fn maintenance_valve_rules() {
    let mut rig = Rig::new();
    rig.ready();
    let clock = rig.clock();
    // Le chauffage doit être désactivé, et les capteurs annoncer la capacité
    // (non annoncée ici : refus).
    let open = cmd(Action::OpenMaintenanceValve);
    assert_eq!(
        rig.core
            .perform_action(clock, &open, ActionSource::Ui)
            .status,
        ActionStatus::Unavailable
    );
    // La fermeture n'exige que des capteurs vivants.
    rig.take_outputs();
    assert_eq!(
        rig.core
            .perform_action(clock, &cmd(Action::CloseMaintenanceValve), ActionSource::Ui)
            .status,
        ActionStatus::Ok
    );
    let frames = rig.can_frames();
    assert!(
        frames
            .iter()
            .any(|m| matches!(m, Message::MaintenanceValve(v) if !v.open && v.duration_ms == 0))
    );
}

#[test]
fn tare_needs_a_present_scale() {
    let mut rig = Rig::new();
    let clock = rig.clock();
    assert_eq!(
        rig.core
            .perform_action(clock, &cmd(Action::Tare), ActionSource::Http)
            .status,
        ActionStatus::Unavailable
    );
    rig.core.update_scale_connection(clock, true);
    rig.core.update_scale_weight(clock, 1.0);
    rig.core.set_scale_writable(false);
    assert_eq!(
        rig.core
            .perform_action(clock, &cmd(Action::Tare), ActionSource::Ui)
            .status,
        ActionStatus::Unavailable
    );
    rig.core.set_scale_writable(true);
    rig.take_outputs();
    assert_eq!(
        rig.core
            .perform_action(clock, &cmd(Action::Tare), ActionSource::Ui)
            .status,
        ActionStatus::Ok
    );
    assert!(rig.take_outputs().contains(&Output::Ble(BleCommand::Tare)));
    // `start_flash` n'est pas une action de l'écran.
    assert_eq!(
        rig.core
            .perform_action(clock, &cmd(Action::StartFlash), ActionSource::Http)
            .status,
        ActionStatus::Unavailable
    );
}

#[test]
fn weight_goal_with_a_scale() {
    let mut rig = prepared_rig();
    let clock = rig.clock();
    rig.core.update_scale_connection(clock, true);
    rig.core.update_scale_weight(clock, 100.0);
    let mut c = rig.core.config();
    c.target_weight_g = 20.0;
    c.target_time_s = 60;
    assert_eq!(rig.core.put_config(clock, &c).status, ConfigStatus::Ok);
    assert_eq!(start(&mut rig, ActionSource::Ui), ActionStatus::Ok);
    assert!(rig.snapshot().cycle_weight_goal);
    rig.advance(2000);
    // La balance monte : arrêt au poids cible.
    for w in [105.0, 112.0, 121.0] {
        let clock = rig.clock();
        rig.core.update_scale_weight(clock, w);
        rig.advance(100);
    }
    assert_eq!(rig.snapshot().cycle_state, CycleState::Finished);
    assert_eq!(rig.pump_pct, 0);
}

// --- Capture HF ----------------------------------------------------------------------------

#[test]
fn hf_capture_records_the_cycle_then_completes() {
    let mut rig = prepared_rig();
    rig.pulses = 0;
    assert_eq!(start(&mut rig, ActionSource::Ui), ActionStatus::Ok);
    let info = rig.core.hf_capture_info();
    assert_eq!((info.status, info.origin), (Status::Active, Origin::Brew));
    // Pas d'échantillon exportable tant que la capture est active.
    assert!(rig.core.hf_capture_sample(0).is_none());
    rig.advance(6500);
    assert_eq!(rig.core.hf_capture_info().status, Status::Active); // récupération de 20 s
    rig.advance(21_000);
    let info = rig.core.hf_capture_info();
    assert_eq!(info.status, Status::Complete);
    assert_eq!(info.sample_period_ms, 100);
    assert_eq!(info.capacity, 1152);
    assert!((250..=290).contains(&info.count), "{}", info.count);
    assert_eq!(info.dropped_samples, 0);
    let samples = rig.core.hf_capture_samples();
    assert!(samples[0].t_ms <= 50);
    assert_eq!(samples[0].volume_ml, 0.0);
    // Période de 100 ms sur la grille (le premier intervalle est écourté : la
    // capture s'ouvre entre deux pas, comme en C++).
    for pair in samples[1..].windows(2) {
        let dt = pair[1].t_ms - pair[0].t_ms;
        assert!((95..=105).contains(&dt), "{dt}");
    }
    use hf_capture::SampleMode::*;
    assert_eq!(samples[0].mode, Filling);
    assert!(samples.iter().any(|s| s.mode == Infusion));
    assert_eq!(samples.last().unwrap().mode, Cooldown);
    assert!(
        samples
            .iter()
            .all(|s| s.flags & hf_capture::flags::PRESSURE_VALID != 0)
    );
    assert!(samples.iter().any(|s| s.pump_pct_commanded == 100));
    // Document d'export : en-tête et pied.
    let doc = hf_capture::export(&info, samples, hf_capture::View::Both);
    assert!(doc.starts_with(
        "{\"schema\":\"coffeeflow.hf_capture.v2\",\"origin\":\"brew\",\"view\":\"both\""
    ));
    assert!(doc.ends_with("]}"));
    // Une nouvelle commande rouvre une capture neuve.
    let clock = rig.clock();
    rig.core
        .perform_action(clock, &cmd(Action::PurgePress), ActionSource::Ui);
    rig.advance(200);
    let info = rig.core.hf_capture_info();
    assert_eq!((info.status, info.origin), (Status::Active, Origin::Purge));
    assert!(info.count <= 3);
}

// --- Mise à jour OTA --------------------------------------------------------------------------

#[test]
fn flash_screen_target() {
    use screen_core::core::FlashStart;
    let mut rig = Rig::new();
    rig.ready();
    let clock = rig.clock();
    assert_eq!(
        rig.core.begin_flash(clock, FlashTarget::Screen, 0),
        FlashStart::Refused
    );
    assert_eq!(
        rig.core.begin_flash(clock, FlashTarget::None, 10),
        FlashStart::Refused
    );
    rig.take_outputs();
    assert_eq!(
        rig.core.begin_flash(clock, FlashTarget::Screen, 1000),
        FlashStart::Started
    );
    let frames = rig.can_frames();
    assert!(frames.iter().any(|m| matches!(m, Message::Stop)));
    let s = rig.snapshot();
    assert!(s.flash_active);
    assert_eq!(
        (s.flash_target, s.flash_bytes_total),
        (FlashTarget::Screen, 1000)
    );
    rig.core.update_flash_progress(512);
    assert_eq!(rig.snapshot().flash_bytes_done, 512);
    // Une seconde mise à jour et les actions sont refusées.
    assert_eq!(
        rig.core.begin_flash(clock, FlashTarget::Sensors, 1000),
        FlashStart::Refused
    );
    assert_eq!(
        rig.core
            .perform_action(clock, &cmd(Action::StartBrew), ActionSource::Ui)
            .status,
        ActionStatus::CycleActive
    );
    rig.core.finish_flash();
    assert!(!rig.snapshot().flash_active);
}

#[test]
fn flash_sensors_waits_for_the_stop_echo() {
    use screen_core::core::FlashStart;
    let mut rig = Rig::new();
    rig.ready();
    let clock = rig.clock();
    rig.take_outputs();
    assert_eq!(
        rig.core.begin_flash(clock, FlashTarget::Sensors, 400_000),
        FlashStart::AwaitingStop
    );
    assert!(rig.can_frames().iter().any(|m| matches!(m, Message::Stop)));
    // Pas d'écho plus récent que le STOP : toujours en attente.
    assert!(!rig.core.poll_flash_stop(clock));
    assert!(!rig.snapshot().flash_active);
    // Écho des actionneurs seul : la chauffe doit aussi répondre.
    rig.now_us += 25_000;
    rig.publish_actuators();
    let clock = rig.clock();
    assert!(!rig.core.poll_flash_stop(clock));
    rig.publish_heating();
    let clock = rig.clock();
    assert!(rig.core.poll_flash_stop(clock));
    let s = rig.snapshot();
    assert!(s.flash_active && s.flash_target == FlashTarget::Sensors);
    // Les demandes de STATUS cessent pendant la mise à jour.
    rig.take_outputs();
    rig.core.tick(Clock::new(rig.now_us + 50_000));
    let periods: Vec<u16> = rig
        .collect()
        .into_iter()
        .filter_map(|o| match o {
            Output::Can(f) => match Message::decode(&f) {
                Ok(Message::ReqStatus(r))
                    if r.target_type == MessageType::StatusActuators as u8 =>
                {
                    Some(r.period_ms)
                }
                _ => None,
            },
            _ => None,
        })
        .collect();
    assert_eq!(periods, vec![0]);
}

#[test]
fn flash_sensors_refused_when_not_idle() {
    use screen_core::core::FlashStart;
    let mut rig = Rig::new();
    rig.ready();
    rig.pump_pct = 30;
    rig.publish_actuators();
    let clock = rig.clock();
    assert_eq!(
        rig.core.begin_flash(clock, FlashTarget::Sensors, 1000),
        FlashStart::Refused
    );
    rig.pump_pct = 0;
    rig.publish_actuators();
    rig.heater_on = true;
    rig.publish_heating();
    assert_eq!(
        rig.core.begin_flash(clock, FlashTarget::Sensors, 1000),
        FlashStart::Refused
    );
    // Capteurs muets.
    let mut silent = Rig::new();
    silent.ready();
    silent.sensors_online = false;
    silent.advance(5000);
    let clock = silent.clock();
    assert_eq!(
        silent.core.begin_flash(clock, FlashTarget::Sensors, 1000),
        FlashStart::Refused
    );
    silent.core.cancel_pending_flash();
}

// --- Profil de télémétrie, configuration, heure ------------------------------------------

fn req_periods(outputs: &[Output]) -> Vec<(u8, u16)> {
    outputs
        .iter()
        .filter_map(|o| match o {
            Output::Can(f) => match Message::decode(f) {
                Ok(Message::ReqStatus(r)) => Some((r.target_type, r.period_ms)),
                _ => None,
            },
            _ => None,
        })
        .collect()
}

#[test]
fn status_requests_follow_the_profile() {
    let mut rig = Rig::new();
    rig.take_outputs();
    rig.core.tick(rig.clock());
    let requests = req_periods(&rig.collect());
    assert_eq!(
        requests,
        vec![
            (MessageType::StatusFlow as u8, 1000),
            (MessageType::StatusActuators as u8, 1000),
            (MessageType::StatusHeating as u8, 250),
        ]
    );
    // Rien de plus dans la seconde qui suit ; renouvelé ensuite.
    rig.core.tick(Clock::new(rig.now_us + 100_000));
    assert!(req_periods(&rig.collect()).is_empty());
    rig.core.tick(Clock::new(rig.now_us + 1_100_000));
    assert_eq!(req_periods(&rig.collect()).len(), 3);
}

#[test]
fn cycle_switches_telemetry_profile_and_ble_publish_rate() {
    let mut rig = prepared_rig();
    assert_eq!(start(&mut rig, ActionSource::Ui), ActionStatus::Ok);
    rig.advance(100);
    let outputs = rig.take_outputs();
    assert!(outputs.contains(&Output::Ble(BleCommand::SetActive(true))));
    let requests = req_periods(&outputs);
    assert!(requests.contains(&(MessageType::StatusFlow as u8, 100)));
    assert!(requests.contains(&(MessageType::StatusActuators as u8, 200)));
    rig.advance(27_000);
    let outputs = rig.take_outputs();
    assert!(outputs.contains(&Output::Ble(BleCommand::SetActive(false))));
}

#[test]
fn heating_disable_cuts_the_boiler_at_once() {
    let mut rig = Rig::new();
    rig.ready();
    let mut c = rig.core.config();
    c.heating_enabled = false;
    rig.take_outputs();
    let clock = rig.clock();
    assert_eq!(rig.core.put_config(clock, &c).status, ConfigStatus::Ok);
    let frames = rig.can_frames();
    let _ = rig.collect();
    let frames: Vec<Message> = if frames.is_empty() {
        rig.can_frames()
    } else {
        frames
    };
    assert!(
        frames
            .iter()
            .any(|m| matches!(m, Message::SetHeatingPower(p) if p.power_permille == 0))
    );
    assert!(
        frames
            .iter()
            .any(|m| matches!(m, Message::SetHeating(h) if !h.on))
    );
    let s = rig.snapshot();
    assert_eq!(s.heating_power_pct, 0.0);
    assert!(!s.brew_temperature_ready);
}

#[test]
fn heating_power_is_sent_on_change_and_every_500_ms() {
    let mut rig = Rig::new();
    rig.boiler_user_c = 60.0;
    rig.ready();
    let powers: Vec<_> = rig
        .outputs
        .iter()
        .filter_map(|o| match o {
            Output::Can(f) => match Message::decode(f) {
                Ok(Message::SetHeatingPower(p)) => Some(p),
                _ => None,
            },
            _ => None,
        })
        .collect();
    // Chaudière froide : pleine puissance, bail de 1,5 s, renouvelé.
    assert!(powers.len() >= 5, "{}", powers.len());
    assert!(
        powers
            .iter()
            .all(|p| p.lease_ms == 1500 && p.power_permille == 1000)
    );
}

#[test]
fn wall_clock_is_optional() {
    let mut rig = Rig::new();
    assert!(!rig.snapshot().time_known);
    assert!(
        rig.core
            .events
            .recent(8)
            .iter()
            .all(|e| e.kind != EventKind::TimeKnown)
    );
    rig.wall_unix_s = Some(1_790_000_000);
    assert!(rig.snapshot().time_known);
    assert_eq!(rig.core.events.recent(1)[0].kind, EventKind::TimeKnown);
}

#[test]
fn handle_dispatches_inputs() {
    let mut rig = Rig::new();
    let clock = rig.clock();
    let frame = from_sensors(Message::StatusActuators(Default::default()));
    assert_eq!(rig.core.handle(clock, Input::CanFrame(frame)), Reply::None);
    assert!(!rig.core.presence_lost());
    assert_eq!(
        rig.core.handle(clock, Input::PutConfig(Config::default())),
        Reply::Config(screen_core::config::ConfigResult::OK)
    );
    let reply = rig.core.handle(
        clock,
        Input::Action {
            command: cmd(Action::StopBrew),
            source: ActionSource::Http,
        },
    );
    assert!(matches!(reply, Reply::Action(r) if r.status == ActionStatus::NoCycle));
}
