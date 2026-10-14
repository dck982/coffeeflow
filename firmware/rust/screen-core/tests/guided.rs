//! Infusion guidée de bout en bout sur le banc du cœur : traces synthétiques
//! de poids et de température, une par cas tranché.

#![allow(clippy::field_reassign_with_default)]

mod common;

use common::Rig;
use protocol::{LogCode, Message};
use screen_core::config::ConfigStatus;
use screen_core::core::{Action, ActionCommand, ActionSource, ActionStatus, BleCommand, Output};
use screen_core::guided::{Hint, Screen, View};
use screen_core::hf_capture::Status;
use screen_core::http::{handle_post_action, hf_capture_document};
use screen_core::json::{self, Json};
use screen_core::snapshot::CycleState;

/// Poids lu par la balance quand la tasse est retirée, après une tare avec
/// tasse.
const CUP_OFF: f32 = -120.0;
/// Tasse vide replacée sur une balance tarée à vide.
const CUP_BACK: f32 = 31.0;

struct Bench {
    rig: Rig,
    weight: f32,
    log: Vec<(i64, Output)>,
}

impl Bench {
    /// Machine prête à infuser, balance connectée à 0 g.
    fn new() -> Self {
        let mut rig = Rig::new();
        rig.ready();
        let clock = rig.clock();
        assert_eq!(
            rig.core.put_config(clock, &Rig::ready_config()).status,
            ConfigStatus::Ok
        );
        rig.advance(4000);
        rig.take_outputs();
        let clock = rig.clock();
        rig.core.update_scale_connection(clock, true);
        rig.core.update_scale_weight(clock, 0.0);
        Self {
            rig,
            weight: 0.0,
            log: Vec::new(),
        }
    }

    fn drain(&mut self) {
        let now = self.rig.now_us;
        for output in self.rig.take_outputs() {
            self.log.push((now, output));
        }
    }

    /// Avance de `ms`, la balance publiant `self.weight` à chaque pas de 50 ms.
    fn run(&mut self, ms: i64) {
        for _ in 0..ms / 50 {
            let clock = self.rig.clock();
            self.rig.core.update_scale_weight(clock, self.weight);
            self.rig.advance(50);
            self.drain();
        }
    }

    /// Fait passer `ms` par pas de 2 s en gardant capteurs et balance vivants.
    fn skip(&mut self, ms: i64) {
        for _ in 0..ms / 2000 {
            self.rig.now_us += 1_900_000;
            let clock = self.rig.clock();
            self.rig.core.update_scale_weight(clock, self.weight);
            self.rig.advance(100);
            self.drain();
        }
    }

    fn weigh(&mut self, weight: f32, ms: i64) {
        self.weight = weight;
        self.run(ms);
    }

    fn act(&mut self, action: Action) -> ActionStatus {
        let clock = self.rig.clock();
        let status = self
            .rig
            .core
            .perform_action(clock, &ActionCommand::new(action), ActionSource::Ui)
            .status;
        self.drain();
        status
    }

    fn view(&mut self) -> View {
        self.rig.snapshot().guided
    }

    fn tares(&self) -> usize {
        self.log
            .iter()
            .filter(|(_, o)| matches!(o, Output::Ble(BleCommand::Tare)))
            .count()
    }

    /// Instants (µs) des journaux CAN de ce code.
    fn logs(&self, code: LogCode) -> Vec<i64> {
        self.log
            .iter()
            .filter_map(|(t, o)| match o {
                Output::Can(frame) => match Message::decode(frame) {
                    Ok(Message::Log(log)) if log.code == code as u8 => Some(*t),
                    _ => None,
                },
                _ => None,
            })
            .collect()
    }

    /// Indice dans le journal de la première sortie qui vérifie `matches`.
    fn position(&self, matches: impl Fn(&Output) -> bool) -> usize {
        self.log.iter().position(|(_, o)| matches(o)).unwrap()
    }

    /// Premier rinçage fait : écran 2, tasse tarée avec 24 g d'eau.
    fn through_rinse1(&mut self) {
        self.weigh(CUP_BACK, 500);
        assert_eq!(self.act(Action::GuidedOpen), ActionStatus::Ok);
        assert_eq!(self.act(Action::GuidedNext), ActionStatus::Ok);
        self.weigh(0.0, 3000);
        self.weigh(24.0, 5000);
        assert_eq!(self.view().screen, Screen::Rinse2);
    }

    /// Les deux rinçages faits : écran 3, chaudière hors de la consigne.
    fn through_rinse2(&mut self) {
        self.through_rinse1();
        assert_eq!(self.act(Action::GuidedNext), ActionStatus::Ok);
        self.rig.boiler_user_c = 80.0;
        self.weigh(48.0, 7000);
        assert_eq!(self.view().screen, Screen::Wait);
    }

    /// Retrait de la tasse, tare à vide et retour de la tasse vide.
    fn swap_cup(&mut self) {
        self.weigh(CUP_OFF, 2000);
        self.weigh(0.0, 1000);
        self.weigh(CUP_BACK, 1000);
    }
}

fn session_json(bench: &Bench) -> Json {
    let doc = hf_capture_document(&bench.rig.core, None);
    assert_eq!(doc.status, "200 OK");
    json::parse(doc.body.as_bytes())
        .unwrap()
        .get("session")
        .cloned()
        .unwrap()
}

fn field(json: &Json, key: &str) -> f64 {
    json.get(key).and_then(Json::as_f64).unwrap()
}

// --- Écran 1 ------------------------------------------------------------------------------------

#[test]
fn next_needs_scale_cup_and_rinse_temperature() {
    let mut b = Bench::new();
    // Sans balance.
    let clock = b.rig.clock();
    b.rig.core.update_scale_connection(clock, false);
    assert_eq!(b.act(Action::GuidedOpen), ActionStatus::Ok);
    b.run(200);
    let v = b.view();
    assert!(v.active && v.screen == Screen::Rinse1);
    assert_eq!((v.hint, v.button_enabled), (Hint::ConnectScale, false));
    assert_eq!(b.act(Action::GuidedNext), ActionStatus::Unavailable);

    // Tasse à 25 g : pas posée.
    let clock = b.rig.clock();
    b.rig.core.update_scale_connection(clock, true);
    b.weigh(25.0, 300);
    let v = b.view();
    assert_eq!((v.hint, v.button_enabled), (Hint::PlaceCup, false));

    // Tasse à 31 g, chaudière sous consigne moins 5 degrés.
    b.rig.boiler_user_c = 84.0;
    b.weigh(CUP_BACK, 300);
    let v = b.view();
    assert_eq!((v.hint, v.button_enabled), (Hint::WaitRinseTemp, false));
    assert_eq!(v.rinse_min_c, 85.0);

    // À la température de rinçage (le guide n'attend pas la consigne).
    b.rig.boiler_user_c = 85.3;
    b.weigh(CUP_BACK, 300);
    let v = b.view();
    assert_eq!((v.hint, v.button_enabled), (Hint::PressNextRinse1, true));
    assert_eq!(b.tares(), 0);
}

#[test]
fn rinse1_tares_first_stops_at_five_seconds_and_waits_for_the_real_end() {
    let mut b = Bench::new();
    b.weigh(CUP_BACK, 500);
    assert_eq!(b.act(Action::GuidedOpen), ActionStatus::Ok);
    assert_eq!(b.act(Action::GuidedNext), ActionStatus::Ok);
    // La tare part avant la purge.
    let tare = b.position(|o| matches!(o, Output::Ble(BleCommand::Tare)));
    let started = b.logs(LogCode::PurgeStarted);
    assert_eq!(started.len(), 1);
    let purge_log = b.position(|o| match o {
        Output::Can(f) => matches!(Message::decode(f), Ok(Message::Log(l)) if l.code == LogCode::PurgeStarted as u8),
        _ => false,
    });
    assert!(tare < purge_log);
    let v = b.view();
    assert!(v.purging && v.button_enabled);

    // L'écran 2 n'arrive qu'après l'arrêt réel de la pompe.
    b.weight = 0.0;
    for _ in 0..140 {
        b.run(50);
        let s = b.rig.snapshot();
        if s.guided.screen == Screen::Rinse2 {
            assert_ne!(s.cycle_state, CycleState::Purge);
            assert!(!s.guided.purging);
            break;
        }
        assert!(s.guided.purging);
    }
    assert_eq!(b.view().screen, Screen::Rinse2);
    let stopped = b.logs(LogCode::PurgeStopped);
    assert_eq!(stopped.len(), 1);
    let elapsed_ms = (stopped[0] - started[0]) / 1000;
    assert!((5000..=5100).contains(&elapsed_ms), "{elapsed_ms}");
}

#[test]
fn purge_stopped_elsewhere_ends_the_rinse_early() {
    let mut b = Bench::new();
    b.weigh(CUP_BACK, 500);
    b.act(Action::GuidedOpen);
    b.act(Action::GuidedNext);
    b.weigh(0.0, 2000);
    // Un relâchement venu d'ailleurs : le guide suit l'arrêt réel.
    let clock = b.rig.clock();
    b.rig.core.perform_action(
        clock,
        &ActionCommand::new(Action::PurgeRelease),
        ActionSource::Http,
    );
    b.weigh(0.0, 200);
    assert_eq!(b.view().screen, Screen::Rinse2);
    let session = b.rig.core.snapshot(b.rig.clock());
    assert!(!session.guided.purging);
}

#[test]
fn screen_touch_release_spares_the_rinse_but_stops_the_held_purge() {
    let mut b = Bench::new();
    b.weigh(CUP_BACK, 500);
    b.act(Action::GuidedOpen);
    b.act(Action::GuidedNext);
    // Le doigt qui se lève après **suivant** ne coupe pas le rinçage.
    for _ in 0..3 {
        let clock = b.rig.clock();
        b.rig.core.release_held_purge(clock);
        b.weigh(0.0, 50);
    }
    assert_eq!(b.rig.snapshot().cycle_state, CycleState::Purge);
    b.weigh(0.0, 5500);
    assert_eq!(b.view().screen, Screen::Rinse2);
    let started = b.logs(LogCode::PurgeStarted);
    let stopped = b.logs(LogCode::PurgeStopped);
    assert_eq!(stopped.len(), 1);
    let elapsed_ms = (stopped[0] - started[0]) / 1000;
    assert!((5000..=5100).contains(&elapsed_ms), "{elapsed_ms}");

    // Le bouton purge maintenu, guide fermé, s'arrête au relâchement.
    assert_eq!(b.act(Action::GuidedQuit), ActionStatus::Ok);
    assert_eq!(b.act(Action::PurgePress), ActionStatus::Ok);
    b.weigh(0.0, 500);
    let clock = b.rig.clock();
    b.rig.core.release_held_purge(clock);
    assert_ne!(b.rig.snapshot().cycle_state, CycleState::Purge);
}

// --- Écran 2 ------------------------------------------------------------------------------------

#[test]
fn rinse2_removal_tares_once_after_stability_and_replacement_reenables_next() {
    let mut b = Bench::new();
    b.through_rinse1();
    assert_eq!(b.tares(), 1);
    let v = b.view();
    assert_eq!((v.hint, v.button_enabled), (Hint::InstallPortafilter, true));

    // Tasse retirée : pas de tare avant la stabilité d'une seconde.
    b.weigh(CUP_OFF, 600);
    let v = b.view();
    assert_eq!((v.hint, v.button_enabled), (Hint::ReplaceCup, false));
    assert_eq!(b.tares(), 1);
    b.weigh(CUP_OFF, 800);
    assert_eq!(b.tares(), 2);
    // La tare tarde : pas de seconde tare pendant l'attente.
    b.weigh(CUP_OFF, 1000);
    assert_eq!(b.tares(), 2);
    // Elle revient : la balance vide lit zéro.
    b.weigh(0.0, 3000);
    assert_eq!(b.tares(), 2);
    let v = b.view();
    assert_eq!((v.hint, v.button_enabled), (Hint::ReplaceCup, false));

    b.weigh(CUP_BACK, 300);
    let v = b.view();
    assert_eq!((v.hint, v.button_enabled), (Hint::InstallPortafilter, true));
    // Le second rinçage ne tare pas : la tasse garde l'eau du premier.
    assert_eq!(b.act(Action::GuidedNext), ActionStatus::Ok);
    assert_eq!(b.tares(), 2);
    assert!(b.view().purging);
}

#[test]
fn a_tare_that_never_lands_is_sent_again_after_the_wait() {
    let mut b = Bench::new();
    b.through_rinse1();
    b.weigh(CUP_OFF, 8000);
    assert!(b.tares() >= 3, "{}", b.tares());
}

#[test]
fn rinse2_waits_for_temperature_and_a_present_scale() {
    let mut b = Bench::new();
    b.through_rinse1();
    b.rig.boiler_user_c = 80.0;
    b.weigh(24.0, 400);
    let v = b.view();
    assert_eq!((v.hint, v.button_enabled), (Hint::WaitRinseTemp, false));
    b.rig.boiler_user_c = 90.0;
    b.weigh(24.0, 400);
    assert!(b.view().button_enabled);
    // Balance perdue : consigne de reconnexion, bouton inactif.
    b.run(2500);
    b.weight = 24.0;
    let clock = b.rig.clock();
    b.rig.core.update_scale_connection(clock, false);
    b.rig.advance(500);
    let v = b.view();
    assert_eq!((v.hint, v.button_enabled), (Hint::ConnectScale, false));
}

#[test]
fn refused_tare_counts_as_a_missing_scale_until_reconnection() {
    let mut b = Bench::new();
    b.weigh(CUP_BACK, 500);
    b.act(Action::GuidedOpen);
    b.rig.core.set_scale_writable(false);
    assert_eq!(b.act(Action::GuidedNext), ActionStatus::Unavailable);
    assert_eq!(b.view().screen, Screen::Rinse1);
    assert!(!b.view().purging);
    let v = b.view();
    assert_eq!((v.hint, v.button_enabled), (Hint::ConnectScale, false));
    // Même écrivable de nouveau, il faut une reconnexion.
    b.rig.core.set_scale_writable(true);
    b.run(300);
    assert!(!b.view().button_enabled);
    let clock = b.rig.clock();
    b.rig.core.update_scale_connection(clock, false);
    b.rig.advance(100);
    b.rig.core.update_scale_connection(b.rig.clock(), true);
    b.run(300);
    assert!(b.view().button_enabled);
}

#[test]
fn core_refusal_is_shown_then_expires() {
    let mut b = Bench::new();
    b.weigh(CUP_BACK, 500);
    b.act(Action::GuidedOpen);
    b.rig.dimmer_ready = false;
    b.run(300);
    assert_eq!(b.act(Action::GuidedNext), ActionStatus::DimmerNotReady);
    let v = b.view();
    assert_eq!(v.refusal, Some(ActionStatus::DimmerNotReady));
    assert_eq!(v.screen, Screen::Rinse1);
    assert!(!v.purging);
    b.run(5500);
    assert_eq!(b.view().refusal, None);
}

// --- Écran 3 ------------------------------------------------------------------------------------

#[test]
fn early_removals_are_counted_and_the_setpoint_time_is_kept() {
    let mut b = Bench::new();
    b.through_rinse2();
    b.rig.heater_on = true;
    let v = b.view();
    assert_eq!((v.hint, v.button_enabled), (Hint::WaitSetpoint, false));
    assert!(v.since_rinse_s <= 3);

    // Deux retraits avant la consigne.
    b.weigh(CUP_OFF, 2000);
    let v = b.view();
    assert_eq!((v.hint, v.button_enabled), (Hint::CupRemovedEarly, false));
    b.weigh(0.0, 1000);
    b.weigh(CUP_BACK, 500);
    assert_eq!(b.view().hint, Hint::WaitSetpoint);
    b.swap_cup();
    // Tasse replacée, pas encore à la consigne : démarrer reste inactif.
    let v = b.view();
    assert_eq!((v.hint, v.button_enabled), (Hint::WaitSetpoint, false));

    // La consigne est atteinte, mais la tasse n'a pas été retirée depuis.
    b.rig.boiler_user_c = 90.0;
    b.weigh(CUP_BACK, 12_000);
    let v = b.view();
    assert_eq!((v.hint, v.button_enabled), (Hint::Prepare, false));

    // La NTC sort de la plage : l'instant de la consigne est gardé.
    b.rig.boiler_user_c = 80.0;
    b.weigh(CUP_BACK, 3000);
    assert_eq!(b.view().hint, Hint::Prepare);

    // Retrait puis retour après la consigne, chaudière hors plage.
    b.swap_cup();
    let v = b.view();
    assert_eq!((v.hint, v.button_enabled), (Hint::WaitBrewTemp, false));
    b.rig.boiler_user_c = 90.0;
    b.weigh(CUP_BACK, 8000);
    let v = b.view();
    assert_eq!((v.hint, v.button_enabled), (Hint::PressStart, true));

    // Démarrer : tare, puis infusion.
    let tares = b.tares();
    assert_eq!(b.act(Action::GuidedNext), ActionStatus::Ok);
    assert_eq!(b.tares(), tares + 1);
    // L'infusion attend la tare : tant que la balance lit encore la tasse,
    // rien ne part.
    b.weigh(CUP_BACK, 500);
    assert!(b.view().active);
    assert!(!b.rig.snapshot().cycle_state.is_active());
    b.weigh(0.0, 300);
    let tare = b
        .log
        .iter()
        .rposition(|(_, o)| matches!(o, Output::Ble(BleCommand::Tare)))
        .unwrap();
    let brew = b.log.iter().rposition(|(_, o)| match o {
        Output::Can(f) => matches!(Message::decode(f), Ok(Message::Log(l)) if l.code == LogCode::BrewStarted as u8),
        _ => false,
    }).unwrap();
    assert!(tare < brew);
    assert!(!b.view().active);
    assert_ne!(b.rig.snapshot().cycle_state, CycleState::Idle);

    // Session attachée, instants négatifs et cohérents.
    let start = b.rig.core.hf_capture_info().started_at_us;
    let s = *b.rig.core.hf_capture_session().unwrap();
    assert_eq!(s.start_pressed_us, start);
    assert_eq!(s.early_removals, 2);
    let reached = s.setpoint_reached_us.unwrap();
    let removed = s.cup_removed_us.unwrap();
    let returned = s.cup_returned_us.unwrap();
    assert!(s.opened_us <= s.purges[0].start_us);
    assert!(s.purges[1].start_us < reached);
    assert!(reached < removed && removed < returned && returned < start);
    assert!(s.wait_ntc_min_c < 83.0, "{}", s.wait_ntc_min_c);
    assert!(s.heater_on_us > 20_000_000, "{}", s.heater_on_us);
    assert_eq!(s.purge_count, 2);
}

#[test]
fn cup_never_removed_since_the_setpoint_keeps_start_disabled() {
    let mut b = Bench::new();
    b.through_rinse2();
    b.rig.boiler_user_c = 90.0;
    // La tasse encore pleine des rinçages (48 g) ne passe pas pour vide.
    b.weigh(48.0, 14_000);
    let v = b.view();
    assert_eq!((v.hint, v.button_enabled), (Hint::Prepare, false));
    assert_eq!(b.act(Action::GuidedNext), ActionStatus::Unavailable);
    // Retirée mais pas replacée.
    b.weigh(CUP_OFF, 2500);
    let v = b.view();
    assert_eq!((v.hint, v.button_enabled), (Hint::Prepare, false));
    b.weigh(0.0, 1000);
    assert!(!b.view().button_enabled);
    b.weigh(CUP_BACK, 500);
    assert!(b.view().button_enabled);
}

#[test]
fn start_refused_by_the_core_keeps_the_guide_on_screen_3() {
    let mut b = Bench::new();
    b.through_rinse2();
    b.rig.boiler_user_c = 90.0;
    b.weigh(48.0, 12_000);
    b.swap_cup();
    b.weigh(CUP_BACK, 500);
    assert!(b.view().button_enabled);
    // Le dimmer décroche entre l'appui et le départ de l'infusion : le cœur
    // refuse, le guide reste là et montre la raison.
    assert_eq!(b.act(Action::GuidedNext), ActionStatus::Ok);
    b.rig.dimmer_ready = false;
    b.weigh(0.0, 2500);
    let v = b.view();
    assert!(v.active);
    assert_eq!(v.screen, Screen::Wait);
    assert_eq!(v.refusal, Some(ActionStatus::DimmerNotReady));
    assert!(b.rig.core.hf_capture_session().is_none());
}

#[test]
fn scale_loss_is_accumulated() {
    let mut b = Bench::new();
    b.through_rinse2();
    let clock = b.rig.clock();
    b.rig.core.update_scale_connection(clock, false);
    b.rig.advance(2000);
    let v = b.view();
    assert_eq!((v.hint, v.button_enabled), (Hint::ConnectScale, false));
    b.rig.core.update_scale_connection(b.rig.clock(), true);
    b.run(500);
    b.rig.boiler_user_c = 90.0;
    b.weigh(48.0, 12_000);
    b.swap_cup();
    b.weigh(CUP_BACK, 500);
    b.rig.boiler_user_c = 90.0;
    assert_eq!(b.act(Action::GuidedNext), ActionStatus::Ok);
    b.weigh(0.0, 300);
    let s = *b.rig.core.hf_capture_session().unwrap();
    assert!(
        (1_800_000..2_600_000).contains(&s.scale_lost_us),
        "{}",
        s.scale_lost_us
    );
}

// --- Abandons -------------------------------------------------------------------------------------

#[test]
fn quit_during_a_rinse_releases_the_pump() {
    let mut b = Bench::new();
    b.weigh(CUP_BACK, 500);
    b.act(Action::GuidedOpen);
    b.act(Action::GuidedNext);
    b.weigh(0.0, 2000);
    assert_eq!(b.rig.snapshot().cycle_state, CycleState::Purge);
    assert_eq!(b.act(Action::GuidedQuit), ActionStatus::Ok);
    assert_eq!(b.logs(LogCode::PurgeStopped).len(), 1);
    let s = b.rig.snapshot();
    assert_ne!(s.cycle_state, CycleState::Purge);
    assert!(!s.guided.active);
    b.run(1000);
    assert_eq!(b.rig.pump_pct, 0);
    // Quitter à l'accueil ne fait rien.
    assert_eq!(b.act(Action::GuidedQuit), ActionStatus::Ok);
}

#[test]
fn lockout_abandons_the_guide_and_releases_a_rinse() {
    let mut b = Bench::new();
    b.weigh(CUP_BACK, 500);
    b.act(Action::GuidedOpen);
    b.act(Action::GuidedNext);
    b.weigh(0.0, 1000);
    b.rig.lockout = true;
    b.weigh(0.0, 300);
    assert!(!b.view().active);
    assert_eq!(b.logs(LogCode::PurgeStopped).len(), 1);
}

#[test]
fn thirty_minutes_without_a_press_abandon_the_guide() {
    let mut b = Bench::new();
    b.weigh(CUP_BACK, 500);
    b.act(Action::GuidedOpen);
    b.skip(29 * 60 * 1000);
    assert!(b.view().active);
    // Un appui repousse l'abandon.
    b.rig.core.note_touch_press();
    b.skip(29 * 60 * 1000);
    assert!(b.view().active);
    b.skip(2 * 60 * 1000);
    assert!(!b.view().active);
}

#[test]
fn open_follows_the_purge_preconditions_but_not_the_temperature() {
    let mut cold = Bench::new();
    cold.rig.boiler_user_c = 40.0;
    cold.run(500);
    assert_eq!(cold.act(Action::GuidedOpen), ActionStatus::Ok);

    let mut locked = Bench::new();
    locked.rig.lockout = true;
    locked.run(500);
    assert_eq!(locked.act(Action::GuidedOpen), ActionStatus::Locked);

    let mut busy = Bench::new();
    busy.act(Action::PurgePress);
    busy.run(200);
    assert_eq!(busy.act(Action::GuidedOpen), ActionStatus::CycleActive);

    let mut dimmer = Bench::new();
    dimmer.rig.dimmer_ready = false;
    dimmer.run(500);
    assert_eq!(dimmer.act(Action::GuidedOpen), ActionStatus::DimmerNotReady);

    let mut offline = Bench::new();
    offline.rig.sensors_online = false;
    offline.run(4000);
    assert_eq!(offline.act(Action::GuidedOpen), ActionStatus::BusLost);
}

// --- HTTP ---------------------------------------------------------------------------------------------

#[test]
fn http_refuses_the_three_guided_actions() {
    let mut b = Bench::new();
    for action in [Action::GuidedOpen, Action::GuidedNext, Action::GuidedQuit] {
        let clock = b.rig.clock();
        let result =
            b.rig
                .core
                .perform_action(clock, &ActionCommand::new(action), ActionSource::Http);
        assert_eq!(result.status, ActionStatus::Unavailable, "{action:?}");
    }
    assert!(!b.view().active);
    for name in ["guided_open", "guided_next", "guided_quit"] {
        let clock = b.rig.clock();
        let body = format!("{{\"action\":\"{name}\"}}");
        let response = handle_post_action(&mut b.rig.core, clock, body.as_bytes());
        assert_eq!(response.status, "400 Bad Request", "{name}");
    }
    assert!(!b.view().active);
}

// --- Export ---------------------------------------------------------------------------------------------

#[test]
fn export_carries_the_session_for_a_guided_brew_only() {
    let mut b = Bench::new();
    b.rig.heater_on = false;
    b.through_rinse2();
    b.rig.boiler_user_c = 90.0;
    b.rig.pulses = 0;
    b.weigh(48.0, 12_000);
    b.swap_cup();
    b.weigh(CUP_BACK, 500);
    assert_eq!(b.act(Action::GuidedNext), ActionStatus::Ok);
    b.weigh(0.0, 2000);
    assert_eq!(b.act(Action::StopBrew), ActionStatus::Ok);
    b.weigh(0.0, 26_000);
    assert_eq!(b.rig.core.hf_capture_info().status, Status::Complete);

    let doc = hf_capture_document(&b.rig.core, None);
    let session_at = doc.body.find("\"session\":{").unwrap();
    assert!(session_at < doc.body.find("\"samples\":[").unwrap());
    let session = session_json(&b);
    assert_eq!(field(&session, "start_pressed_ms"), 0.0);
    assert!(field(&session, "opened_ms") < field(&session, "cup_removed_ms"));
    assert!(field(&session, "cup_removed_ms") < field(&session, "cup_returned_ms"));
    assert!(field(&session, "cup_returned_ms") < 0.0);
    assert_eq!(
        session.get("opened_at_unix_s").and_then(Json::as_f64),
        Some(0.0)
    );
    assert_eq!(field(&session, "setpoint_c"), 90.0);
    let Some(Json::Array(purges)) = session.get("purges") else {
        panic!("purges");
    };
    assert_eq!(purges.len(), 2);
    for (purge, water) in purges.iter().zip([24.0, 24.0]) {
        let duration = field(purge, "duration_ms");
        assert!((5000.0..=5100.0).contains(&duration), "{duration}");
        assert!((field(purge, "weight_g") - water).abs() < 0.01);
        assert!(field(purge, "ntc_min_c") <= field(purge, "ntc_start_c"));
    }
    assert!(field(&purges[0], "start_ms") < field(&purges[1], "start_ms"));
    let wait = session.get("wait").unwrap();
    assert_eq!(field(wait, "early_removals"), 0.0);
    assert!(field(wait, "setpoint_reached_ms") < field(&session, "cup_removed_ms"));

    // Une infusion directe porte `session: null`.
    assert_eq!(b.act(Action::DismissSummary), ActionStatus::Ok);
    b.rig.boiler_user_c = 90.0;
    b.weigh(0.0, 6000);
    assert_eq!(b.act(Action::StartBrew), ActionStatus::Ok);
    assert!(b.rig.core.hf_capture_session().is_none());
    b.weigh(0.0, 2000);
    b.act(Action::StopBrew);
    b.weigh(0.0, 26_000);
    let doc = hf_capture_document(&b.rig.core, None);
    assert!(
        doc.body.contains("\"session\":null,\"samples\":["),
        "{}",
        &doc.body[..300]
    );

    // Une purge n'a pas la clé.
    assert_eq!(b.act(Action::DismissSummary), ActionStatus::Ok);
    assert_eq!(b.act(Action::PurgePress), ActionStatus::Ok);
    b.weigh(0.0, 1000);
    assert_eq!(b.act(Action::PurgeRelease), ActionStatus::Ok);
    b.weigh(0.0, 7000);
    let doc = hf_capture_document(&b.rig.core, None);
    assert!(doc.body.contains("\"origin\":\"purge\""));
    assert!(!doc.body.contains("\"session\""));
}

#[test]
fn a_later_capture_erases_the_attached_session() {
    let mut b = Bench::new();
    b.through_rinse2();
    b.rig.boiler_user_c = 90.0;
    b.weigh(48.0, 12_000);
    b.swap_cup();
    b.weigh(CUP_BACK, 500);
    assert_eq!(b.act(Action::GuidedNext), ActionStatus::Ok);
    b.weigh(0.0, 300);
    assert!(b.rig.core.hf_capture_session().is_some());
    b.weigh(0.0, 1000);
    b.act(Action::StopBrew);
    b.weigh(0.0, 26_000);
    assert!(b.rig.core.hf_capture_session().is_some());
    b.act(Action::DismissSummary);
    b.act(Action::PurgePress);
    b.weigh(0.0, 500);
    assert!(b.rig.core.hf_capture_session().is_none());
}
