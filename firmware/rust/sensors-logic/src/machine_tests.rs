//! Scénarios de la machine d'état avec un faux matériel.

use protocol::messages::{
    FlashCtrlPayload, FlashDataPayload, MaintenanceValvePayload, ReqStatusPayload,
    SetHeatingPayload, SetHeatingPowerPayload, SetPayload, StatusActuatorsPayload,
    StatusEnclosurePayload, StatusHeatingPayload, StatusPumpPayload,
};
use protocol::{
    Dest, FIRMWARE_VERSION, Frame, LogCode, LogSeverity, Message, MessageType, Node, crc16_ccitt,
    crc32_ieee, decode_can_id,
};

use crate::flash_rx::{FlashSink, HEAD_LEN};
use crate::flow::FlowCounter;
use crate::machine::*;
use crate::pump::{TachCounter, pump_duty};

const S: i64 = 1_000_000;
const MS: i64 = 1_000;

struct Mock {
    now: i64,
    valve: bool,
    heater: bool,
    /// Dernier rapport cyclique de la pompe et tous ceux posés.
    duty: u16,
    duty_writes: Vec<u16>,
    /// Vitesse que le faux tachymètre rend : un front à l'instant courant,
    /// ou aucun front si 0.
    tach_rpm: u16,
    pressure_reads: u32,
    enclosure_raw: Option<i16>,
    pressure_raw: Option<i16>,
    sent: Vec<Frame>,
    flow: FlowCounter,
    lockout_stored: Vec<bool>,
    mark_valid_ok: bool,
    marked_valid: u32,
    slot: Option<u32>,
    flash: Vec<u8>,
    erased: u32,
    commits: u32,
}

impl Mock {
    fn new() -> Self {
        Self {
            now: 10 * S,
            valve: false,
            heater: false,
            duty: 0,
            duty_writes: vec![],
            tach_rpm: 0,
            pressure_reads: 0,
            enclosure_raw: Some(0x1F0),
            pressure_raw: None,
            sent: vec![],
            flow: FlowCounter::new(),
            lockout_stored: vec![],
            mark_valid_ok: true,
            marked_valid: 0,
            slot: Some(64 * 1024),
            flash: vec![0xFF; 64 * 1024],
            erased: 0,
            commits: 0,
        }
    }
}

impl FlashSink for Mock {
    fn slot_size(&self) -> Option<u32> {
        self.slot
    }
    fn erase_slot(&mut self) -> bool {
        self.erased += 1;
        self.flash.fill(0xFF);
        true
    }
    fn write(&mut self, offset: u32, data: &[u8]) -> bool {
        self.flash[offset as usize..offset as usize + data.len()].copy_from_slice(data);
        true
    }
    fn commit(&mut self, _head: &[u8; HEAD_LEN], _size: u32) -> bool {
        self.commits += 1;
        true
    }
}

impl Hw for Mock {
    fn now_us(&self) -> i64 {
        self.now
    }
    fn set_valve(&mut self, open: bool) {
        self.valve = open;
    }
    fn set_heater(&mut self, on: bool) {
        self.heater = on;
    }
    fn set_pump_duty(&mut self, duty: u16) {
        self.duty = duty;
        self.duty_writes.push(duty);
    }
    fn send(&mut self, frame: Frame) {
        self.sent.push(frame);
    }
    fn flow(&self) -> FlowCounter {
        self.flow
    }
    fn pump_tach(&self) -> TachCounter {
        if self.tach_rpm == 0 {
            return TachCounter::new();
        }
        let period_us = 30_000_000 / u32::from(self.tach_rpm);
        TachCounter {
            pulse_count: 100,
            last_edge_us: self.now - 1000,
            period_us,
            prev_period_us: period_us,
        }
    }
    fn twai_counters(&mut self) -> Option<TwaiCounters> {
        Some(TwaiCounters {
            rx_errors: 3,
            tx_errors: 5,
            bus_errors: 7,
        })
    }
    fn enclosure_temperature_raw(&mut self) -> Option<i16> {
        self.enclosure_raw
    }
    fn pressure_raw(&mut self) -> Option<i16> {
        self.pressure_reads += 1;
        self.pressure_raw
    }
    fn store_lockout(&mut self, active: bool) {
        self.lockout_stored.push(active);
    }
    fn ota_mark_valid(&mut self) -> bool {
        self.marked_valid += 1;
        self.mark_valid_ok
    }
}

fn cfg() -> Config {
    Config {
        lockout_active: false,
        ota_pending: None,
        validate: true,
    }
}

fn machine() -> Machine<Mock> {
    Machine::new(Mock::new(), cfg())
}

/// Machine avec l'écran présent (une trame reçue à l'instant courant).
fn present() -> Machine<Mock> {
    let mut m = machine();
    send_stop(&mut m);
    m.hw.sent.clear();
    m
}

fn frame_of(message: Message, src: Node, dest: Dest) -> Frame {
    message.encode_routed(src, dest)
}

fn rx(m: &mut Machine<Mock>, message: Message) {
    m.on_frame(&frame_of(message, Node::Screen, Dest::Sensors));
}

fn send_stop(m: &mut Machine<Mock>) {
    rx(m, Message::Stop);
}

fn set(m: &mut Machine<Mock>, pump_pct: u8, ttl_ms: u16) {
    rx(m, Message::Set(SetPayload { pump_pct, ttl_ms }));
}

/// Avance l'horloge de `us` par pas de 100 ms, avec un tick de sécurité à
/// chaque pas. `keepalive` renvoie une trame d'écran à chaque pas.
fn advance(m: &mut Machine<Mock>, us: i64, keepalive: bool) {
    let steps = us / TICK_PERIOD_US;
    for _ in 0..steps {
        m.hw.now += TICK_PERIOD_US;
        if keepalive {
            send_stop_keepalive(m);
        }
        m.tick_safety();
    }
}

/// Trame d'écran neutre pour garder la présence, sans toucher aux sorties :
/// un PONG.
fn send_stop_keepalive(m: &mut Machine<Mock>) {
    let id = protocol::messages::NodeIdentityPayload::new(Node::Screen, FIRMWARE_VERSION, 0);
    rx(m, Message::Pong(id));
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
struct Log {
    code: u8,
    severity: u8,
    arg16: u16,
    arg32: u32,
}

fn decoded(frame: &Frame) -> Message {
    Message::decode(frame).unwrap()
}

fn logs(m: &Machine<Mock>) -> Vec<Log> {
    m.hw.sent
        .iter()
        .filter_map(|f| match decoded(f) {
            Message::Log(l) => Some(Log {
                code: l.code,
                severity: l.severity,
                arg16: l.arg16,
                arg32: l.arg32,
            }),
            _ => None,
        })
        .collect()
}

fn has_log(m: &Machine<Mock>, code: LogCode) -> bool {
    logs(m).iter().any(|l| l.code == code as u8)
}

fn log_args(m: &Machine<Mock>, code: LogCode) -> Vec<(u16, u32)> {
    logs(m)
        .iter()
        .filter(|l| l.code == code as u8)
        .map(|l| (l.arg16, l.arg32))
        .collect()
}

fn last_actuators(m: &Machine<Mock>) -> StatusActuatorsPayload {
    m.hw.sent
        .iter()
        .rev()
        .find_map(|f| match decoded(f) {
            Message::StatusActuators(p) => Some(p),
            _ => None,
        })
        .expect("aucun STATUS_ACTUATORS")
}

fn last_heating(m: &Machine<Mock>) -> StatusHeatingPayload {
    m.hw.sent
        .iter()
        .rev()
        .find_map(|f| match decoded(f) {
            Message::StatusHeating(p) => Some(p),
            _ => None,
        })
        .expect("aucun STATUS_HEATING")
}

fn heating_power(m: &mut Machine<Mock>, permille: u16, restart_window: bool) {
    rx(
        m,
        Message::SetHeatingPower(SetHeatingPowerPayload {
            power_permille: permille,
            lease_ms: HEATING_POWER_LEASE_MS,
            restart_window,
        }),
    );
}

// ------------------------------------------------------------------ démarrage

#[test]
fn start_announces_identity_boot_and_pending_verify() {
    let mut m = Machine::new(
        Mock::new(),
        Config {
            lockout_active: false,
            ota_pending: Some(1),
            validate: true,
        },
    );
    m.start();
    m.ready();
    let sent = &m.hw.sent;
    let first = decoded(&sent[0]);
    let Message::Ping(id) = first else {
        panic!("premier message: {first:?}")
    };
    assert_eq!(id.node(), Some(Node::Sensors));
    assert_eq!(id.version, FIRMWARE_VERSION);
    assert_eq!(id.uptime_s, 10);
    let route = decode_can_id(sent[0].id).unwrap();
    assert_eq!((route.src, route.dest), (Node::Sensors, Dest::Screen));
    let codes: Vec<u8> = logs(&m).iter().map(|l| l.code).collect();
    assert_eq!(
        codes,
        [
            LogCode::Boot as u8,
            LogCode::OtaPendingVerify as u8,
            LogCode::Ready as u8
        ]
    );
    assert_eq!(log_args(&m, LogCode::OtaPendingVerify), [(0, 1)]);
}

#[test]
fn lockout_is_cleared_only_by_a_cold_start() {
    assert!(!lockout_after_boot(true, true, true));
    assert!(!lockout_after_boot(false, false, true));
    assert!(lockout_after_boot(false, true, true));
    assert!(!lockout_after_boot(false, true, false));
}

// ------------------------------------------------------------- bail et SET

#[test]
fn set_opens_valve_and_pump_then_the_lease_cuts_both() {
    let mut m = present();
    set(&mut m, 80, 300);
    assert!(m.hw.valve);
    assert_eq!(m.hw.duty, pump_duty(80));
    assert!(m.hw.duty > 0);
    let st = last_actuators(&m);
    assert!(st.valve_open);
    assert_eq!(st.pump_pct, 80);
    assert_eq!(st.lease_remaining_ms, 300);
    assert_eq!(
        st.flags & StatusActuatorsPayload::FLAG_MAINTENANCE_VALVE_CAPABLE,
        0x20
    );
    assert_eq!(st.flags & StatusActuatorsPayload::FLAG_PUMP_PRESENT, 0x04);
    assert_eq!(st.flags & StatusActuatorsPayload::FLAG_PUMP_READY, 0x02);
    assert_eq!(st.flags & StatusActuatorsPayload::FLAG_PUMP_FAULT, 0);

    advance(&mut m, 200 * MS, true);
    assert!(m.hw.valve);
    advance(&mut m, 100 * MS, true);
    assert!(!m.hw.valve);
    assert_eq!(m.hw.duty, 0);
    assert_eq!(m.hw.duty_writes.last(), Some(&0));
    assert!(has_log(&m, LogCode::LeaseExpired));
    let st = last_actuators(&m);
    assert!(!st.valve_open && st.pump_pct == 0 && st.lease_remaining_ms == 0);
}

#[test]
fn set_with_zero_ttl_uses_the_500_ms_default_and_renewals_extend() {
    let mut m = present();
    m.hw.tach_rpm = 2250;
    set(&mut m, 50, 0);
    assert_eq!(last_actuators(&m).lease_remaining_ms, 500);
    advance(&mut m, 400 * MS, true);
    set(&mut m, 50, 0);
    advance(&mut m, 400 * MS, true);
    assert!(m.hw.valve);
    advance(&mut m, 200 * MS, true);
    assert!(!m.hw.valve);
}

#[test]
fn set_above_100_is_ignored_and_set_zero_closes_the_valve() {
    let mut m = present();
    set(&mut m, 101, 500);
    assert!(!m.hw.valve);
    assert!(m.hw.sent.is_empty());
    set(&mut m, 40, 500);
    set(&mut m, 0, 500);
    assert!(!m.hw.valve);
    assert_eq!(m.hw.duty, 0);
    assert_eq!(m.hw.duty_writes.last(), Some(&0));
}

#[test]
fn stop_cuts_everything_and_reports() {
    let mut m = present();
    set(&mut m, 60, 500);
    heating_power(&mut m, 500, false);
    assert!(m.hw.heater);
    send_stop(&mut m);
    assert!(!m.hw.valve && !m.hw.heater);
    let st = last_actuators(&m);
    assert!(!st.valve_open && st.pump_pct == 0);
    let heat = last_heating(&m);
    assert!(!heat.heater_on && heat.power_permille == 0 && heat.lease_remaining_ms == 0);
}

// ---------------------------------------------------------------- filtrage

#[test]
fn frames_not_from_the_screen_or_not_for_us_are_ignored_and_keep_no_presence() {
    let mut m = machine();
    m.on_frame(&frame_of(Message::Stop, Node::Sensors, Dest::Sensors));
    m.on_frame(&frame_of(Message::Stop, Node::Screen, Dest::Screen));
    // Type inconnu : 0x3F n'existe pas.
    m.on_frame(&Frame {
        id: (0x3F << 5) | (2 << 3) | 1,
        len: 0,
        data: [0; 8],
    });
    assert!(m.hw.sent.is_empty());
    // Aucune présence retenue : le premier tick sonde le silence aussitôt.
    m.tick_safety();
    assert!(
        m.hw.sent
            .iter()
            .any(|f| matches!(decoded(f), Message::Ping(_)))
    );
}

#[test]
fn broadcast_from_the_screen_counts() {
    let mut m = machine();
    m.on_frame(&frame_of(Message::Stop, Node::Screen, Dest::Broadcast));
    rx(
        &mut m,
        Message::SetHeating(SetHeatingPayload {
            on: true,
            duration_ms: 1000,
        }),
    );
    assert!(m.hw.heater);
}

#[test]
fn a_malformed_payload_still_counts_as_presence() {
    let mut m = machine();
    // SET trop court (DLC 2).
    let frame = Frame::new(
        protocol::encode_can_id(protocol::CanId {
            message_type: MessageType::Set,
            dest: Dest::Sensors,
            src: Node::Screen,
        }),
        &[10, 0],
    )
    .unwrap();
    m.on_frame(&frame);
    assert!(!m.hw.valve);
    rx(
        &mut m,
        Message::SetHeating(SetHeatingPayload {
            on: true,
            duration_ms: 1000,
        }),
    );
    assert!(m.hw.heater);
}

#[test]
fn ping_of_any_length_gets_a_pong_and_pong_marks_the_roundtrip() {
    let mut m = machine();
    let id = protocol::encode_can_id(protocol::CanId {
        message_type: MessageType::Ping,
        dest: Dest::Sensors,
        src: Node::Screen,
    });
    m.on_frame(&Frame::new(id, &[]).unwrap());
    m.on_frame(&Frame::new(id, &[1, 2, 3]).unwrap());
    let pongs: Vec<_> =
        m.hw.sent
            .iter()
            .filter(|f| matches!(decoded(f), Message::Pong(_)))
            .collect();
    assert_eq!(pongs.len(), 2);
    let route = decode_can_id(pongs[0].id).unwrap();
    assert_eq!(
        (route.src, route.dest, pongs[0].len),
        (Node::Sensors, Dest::Screen, 8)
    );
}

#[test]
fn reset_logs_and_asks_for_a_reboot_but_never_lifts_the_lockout() {
    let mut m = Machine::new(
        Mock::new(),
        Config {
            lockout_active: true,
            ota_pending: None,
            validate: true,
        },
    );
    rx(&mut m, Message::Reset);
    assert!(has_log(&m, LogCode::RebootRequested));
    assert_eq!(m.take_action(), Action::Reboot);
    assert_eq!(m.take_action(), Action::None);
    assert!(m.lockout_active());
    assert!(m.hw.lockout_stored.is_empty());
}

// ----------------------------------------------------------------- présence

#[test]
fn silence_probes_three_times_then_declares_presence_lost() {
    let mut m = machine();
    // Au démarrage, le silence est déjà de 1,5 s : la sonde part au 1er tick.
    m.tick_safety();
    let pings = |m: &Machine<Mock>| {
        m.hw.sent
            .iter()
            .filter(|f| matches!(decoded(f), Message::Ping(_)))
            .count()
    };
    assert_eq!(pings(&m), 1);
    let ping_route = decode_can_id(m.hw.sent[0].id).unwrap();
    assert_eq!(ping_route.dest, Dest::Broadcast);

    // État de repos : « perdu » dès le départ, la perte ne se déclare pas.
    advance(&mut m, 2 * S, false);
    assert_eq!(pings(&m), 3);
    assert!(!has_log(&m, LogCode::PresenceLost));
}

#[test]
fn presence_lost_after_silence_cuts_outputs_and_logs_once() {
    let mut m = present();
    m.hw.tach_rpm = 3150;
    set(&mut m, 70, 5000);
    heating_power(&mut m, 500, false);
    m.hw.sent.clear();
    // Silence : 1,5 s avant la sonde, puis 1,5 s avant la perte.
    advance(&mut m, 1500 * MS, false);
    assert!(m.hw.valve);
    advance(&mut m, 1500 * MS, false);
    assert!(!m.hw.valve && !m.hw.heater);
    assert_eq!(log_args(&m, LogCode::PresenceLost).len(), 1);
    assert!(!last_heating(&m).heater_on);
    let pings =
        m.hw.sent
            .iter()
            .filter(|f| matches!(decoded(f), Message::Ping(_)))
            .count();
    assert_eq!(pings, 3);
    advance(&mut m, 5 * S, false);
    assert_eq!(log_args(&m, LogCode::PresenceLost).len(), 1);

    // Le retour d'une trame d'écran rend la main à la chauffe.
    send_stop(&mut m);
    heating_power(&mut m, 500, false);
    assert!(m.hw.heater);
}

#[test]
fn any_screen_frame_resets_the_silence() {
    let mut m = present();
    set(&mut m, 70, 60_000);
    for _ in 0..40 {
        advance(&mut m, 1000 * MS, false);
        send_stop_keepalive(&mut m);
        set(&mut m, 70, 60_000);
    }
    assert!(!has_log(&m, LogCode::PresenceLost));
}

// ---------------------------------------------------------------- verrou 60 s

#[test]
fn running_more_than_60_s_locks_out_and_refuses_commands() {
    let mut m = present();
    for _ in 0..59 {
        set(&mut m, 100, 2000);
        advance(&mut m, S, true);
    }
    assert!(!m.lockout_active());
    for _ in 0..2 {
        set(&mut m, 100, 2000);
        advance(&mut m, S, true);
    }
    assert!(m.lockout_active());
    assert_eq!(m.hw.lockout_stored, [true]);
    assert!(!m.hw.valve);
    let args = log_args(&m, LogCode::RuntimeLockoutTriggered);
    assert_eq!(args.len(), 1);
    assert!(args[0].1 > 60_000 && args[0].1 <= 61_100, "{args:?}");
    assert_eq!(
        last_actuators(&m).flags & StatusActuatorsPayload::FLAG_LOCKOUT,
        1
    );

    m.hw.sent.clear();
    set(&mut m, 50, 500);
    assert!(!m.hw.valve);
    assert!(has_log(&m, LogCode::CommandRefusedLocked));
    heating_power(&mut m, 500, false);
    assert!(!m.hw.heater);
    rx(
        &mut m,
        Message::MaintenanceValve(MaintenanceValvePayload {
            open: true,
            duration_ms: 1000,
        }),
    );
    assert!(!m.hw.valve);
}

#[test]
fn a_rearm_gap_under_2_s_does_not_reset_the_run_counter() {
    let mut m = present();
    for _ in 0..59 {
        set(&mut m, 100, 2000);
        advance(&mut m, S, true);
    }
    // Une pause d'une seconde : le compteur ne repart pas.
    send_stop(&mut m);
    advance(&mut m, 1000 * MS, true);
    for _ in 0..3 {
        set(&mut m, 100, 2000);
        advance(&mut m, S, true);
    }
    assert!(m.lockout_active());
}

#[test]
fn a_gap_of_2_s_or_more_rearms_the_counter() {
    let mut m = present();
    for _ in 0..59 {
        set(&mut m, 100, 2000);
        advance(&mut m, S, true);
    }
    send_stop(&mut m);
    advance(&mut m, 2200 * MS, true);
    for _ in 0..30 {
        set(&mut m, 100, 2000);
        advance(&mut m, S, true);
    }
    assert!(!m.lockout_active());
}

#[test]
fn lockout_read_from_rtc_is_enforced_from_the_first_command() {
    let mut m = Machine::new(
        Mock::new(),
        Config {
            lockout_active: true,
            ota_pending: None,
            validate: true,
        },
    );
    send_stop(&mut m);
    set(&mut m, 50, 500);
    assert!(!m.hw.valve);
    assert!(has_log(&m, LogCode::CommandRefusedLocked));
    // STOP et la lecture restent possibles.
    assert_eq!(last_actuators(&m).flags & 1, 1);
}

// --------------------------------------------------------------- maintenance

fn open_maintenance(m: &mut Machine<Mock>, ms: u16) {
    rx(
        m,
        Message::MaintenanceValve(MaintenanceValvePayload {
            open: true,
            duration_ms: ms,
        }),
    );
}

#[test]
fn maintenance_valve_opens_valve_only_with_heating_off() {
    let mut m = present();
    heating_power(&mut m, 700, false);
    assert!(m.hw.heater);
    m.hw.sent.clear();
    open_maintenance(&mut m, 10_000);
    assert!(m.hw.valve && !m.hw.heater);
    assert_eq!(m.hw.duty, 0);
    assert_eq!(m.hw.duty_writes.last(), Some(&0));
    let st = last_actuators(&m);
    assert!(st.valve_open && st.pump_pct == 0);
    assert_eq!(
        st.flags & StatusActuatorsPayload::FLAG_MAINTENANCE_VALVE_OPEN,
        0x10
    );
    assert_eq!(st.lease_remaining_ms, 10_000);
    assert_eq!(log_args(&m, LogCode::MaintenanceValveOpened), [(0, 10_000)]);
    assert!(!last_heating(&m).heater_on);
    // Pendant la maintenance, la chauffe est refusée.
    heating_power(&mut m, 700, false);
    assert!(!m.hw.heater);
}

#[test]
fn maintenance_valve_ends_by_duration_and_stays_out_of_the_60_s_cap() {
    let mut m = present();
    for _ in 0..4 {
        open_maintenance(&mut m, 30_000);
        advance(&mut m, 30_100 * MS, true);
        assert!(!m.hw.valve);
    }
    assert!(!m.lockout_active());
    assert_eq!(log_args(&m, LogCode::MaintenanceValveClosed), [(1, 0); 4]);
}

#[test]
fn maintenance_valve_close_command_and_safety_reasons() {
    let mut m = present();
    open_maintenance(&mut m, 20_000);
    rx(
        &mut m,
        Message::MaintenanceValve(MaintenanceValvePayload {
            open: false,
            duration_ms: 0,
        }),
    );
    assert!(!m.hw.valve);
    assert_eq!(log_args(&m, LogCode::MaintenanceValveClosed), [(0, 0)]);

    m.hw.sent.clear();
    open_maintenance(&mut m, 20_000);
    send_stop(&mut m);
    assert!(!m.hw.valve);
    assert_eq!(log_args(&m, LogCode::MaintenanceValveClosed), [(3, 0)]);
}

#[test]
fn a_set_takes_over_from_maintenance_and_maintenance_is_refused_during_a_brew() {
    let mut m = present();
    open_maintenance(&mut m, 20_000);
    set(&mut m, 50, 500);
    assert_eq!(log_args(&m, LogCode::MaintenanceValveClosed), [(2, 0)]);
    assert!(m.hw.valve && last_actuators(&m).pump_pct == 50);

    // Une infusion en cours refuse la maintenance.
    m.hw.sent.clear();
    open_maintenance(&mut m, 5000);
    assert!(!has_log(&m, LogCode::MaintenanceValveOpened));
    assert_eq!(last_actuators(&m).pump_pct, 50);
}

#[test]
fn maintenance_valve_is_refused_during_a_flash() {
    let mut m = present();
    rx(
        &mut m,
        Message::FlashCtrl(FlashCtrlPayload::Begin { image_size: 1000 }),
    );
    open_maintenance(&mut m, 5000);
    assert!(!m.hw.valve);
}

#[test]
fn reopening_maintenance_restarts_from_the_received_duration() {
    let mut m = present();
    open_maintenance(&mut m, 10_000);
    advance(&mut m, 5 * S, true);
    open_maintenance(&mut m, 10_000);
    assert_eq!(last_actuators(&m).lease_remaining_ms, 10_000);
    advance(&mut m, 9 * S, true);
    assert!(m.hw.valve);
}

#[test]
fn the_pump_stays_at_zero_during_maintenance() {
    let mut m = present();
    open_maintenance(&mut m, 10_000);
    advance(&mut m, 3 * S, true);
    assert!(m.hw.valve);
    assert!(m.hw.duty_writes.iter().all(|&d| d == 0));
}

// ------------------------------------------------------------------- chauffe

#[test]
fn diagnostic_heating_pulse_runs_for_its_duration_only() {
    let mut m = present();
    rx(
        &mut m,
        Message::SetHeating(SetHeatingPayload {
            on: true,
            duration_ms: 1000,
        }),
    );
    assert!(m.hw.heater);
    let heat = last_heating(&m);
    assert!(heat.heater_on);
    assert_eq!(heat.lease_remaining_ms, 1000);
    assert!(heat.power_capable && heat.fine_power_capable && heat.window_restart_capable);
    advance(&mut m, 900 * MS, true);
    assert!(m.hw.heater);
    advance(&mut m, 200 * MS, true);
    assert!(!m.hw.heater);
    assert!(!last_heating(&m).heater_on);
}

#[test]
fn diagnostic_heating_limits_and_off_command() {
    let mut m = present();
    rx(
        &mut m,
        Message::SetHeating(SetHeatingPayload {
            on: true,
            duration_ms: 0,
        }),
    );
    assert!(!m.hw.heater);
    rx(
        &mut m,
        Message::SetHeating(SetHeatingPayload {
            on: true,
            duration_ms: 30_001,
        }),
    );
    assert!(!m.hw.heater);
    rx(
        &mut m,
        Message::SetHeating(SetHeatingPayload {
            on: true,
            duration_ms: 30_000,
        }),
    );
    assert!(m.hw.heater);
    // Une seconde impulsion pendant la première est ignorée (pas de cumul).
    advance(&mut m, 10 * S, true);
    rx(
        &mut m,
        Message::SetHeating(SetHeatingPayload {
            on: true,
            duration_ms: 30_000,
        }),
    );
    advance(&mut m, 20 * S + 200 * MS, true);
    assert!(!m.hw.heater);
    rx(
        &mut m,
        Message::SetHeating(SetHeatingPayload {
            on: true,
            duration_ms: 5000,
        }),
    );
    rx(
        &mut m,
        Message::SetHeating(SetHeatingPayload {
            on: false,
            duration_ms: 0,
        }),
    );
    assert!(!m.hw.heater);
}

#[test]
fn heating_is_refused_during_a_flash_and_under_lockout() {
    let mut m = present();
    rx(
        &mut m,
        Message::FlashCtrl(FlashCtrlPayload::Begin { image_size: 1000 }),
    );
    rx(
        &mut m,
        Message::SetHeating(SetHeatingPayload {
            on: true,
            duration_ms: 1000,
        }),
    );
    heating_power(&mut m, 500, false);
    assert!(!m.hw.heater);
}

#[test]
fn power_heating_needs_the_1500_ms_lease_and_modulates() {
    let mut m = present();
    rx(
        &mut m,
        Message::SetHeatingPower(SetHeatingPowerPayload {
            power_permille: 500,
            lease_ms: 1000,
            restart_window: false,
        }),
    );
    assert!(!m.hw.heater);
    assert!(m.hw.sent.is_empty());

    heating_power(&mut m, 500, false);
    assert!(m.hw.heater);
    let heat = last_heating(&m);
    assert_eq!(heat.power_permille, 500);
    assert_eq!(heat.lease_remaining_ms, 1500);

    // 50 % : 500 ms allumé puis éteint dans la fenêtre de 1 s.
    let mut on_ticks = 0;
    for _ in 0..10 {
        m.hw.now += TICK_PERIOD_US;
        send_stop_keepalive(&mut m);
        heating_power(&mut m, 500, false);
        m.tick_safety();
        if m.hw.heater {
            on_ticks += 1;
        }
    }
    assert!((4..=6).contains(&on_ticks), "{on_ticks}");
}

#[test]
fn power_heating_lease_expiry_stops_it() {
    let mut m = present();
    heating_power(&mut m, 1000, false);
    assert!(m.hw.heater);
    advance(&mut m, 1400 * MS, true);
    assert!(m.hw.heater);
    advance(&mut m, 200 * MS, true);
    assert!(!m.hw.heater);
    let heat = last_heating(&m);
    assert!(!heat.heater_on && heat.power_permille == 0);
}

#[test]
fn power_zero_stops_the_modulation_and_extends_the_lease() {
    let mut m = present();
    heating_power(&mut m, 1000, false);
    heating_power(&mut m, 0, false);
    assert!(!m.hw.heater);
    let heat = last_heating(&m);
    assert_eq!(heat.power_permille, 0);
    // Une consigne nulle sans modulation en cours coupe tout.
    send_stop(&mut m);
    heating_power(&mut m, 0, false);
    assert!(!m.hw.heater);
}

#[test]
fn restart_window_reopens_a_window_immediately() {
    let mut m = present();
    heating_power(&mut m, 30, false);
    // 3 % : crédit initial insuffisant, la sortie reste éteinte.
    assert!(!m.hw.heater);
    advance(&mut m, 300 * MS, true);
    heating_power(&mut m, 900, true);
    assert!(m.hw.heater);

    let mut m = present();
    heating_power(&mut m, 30, false);
    advance(&mut m, 300 * MS, true);
    heating_power(&mut m, 900, false);
    assert!(!m.hw.heater);
}

#[test]
fn diagnostic_pulse_blocks_power_mode_and_the_reverse() {
    let mut m = present();
    rx(
        &mut m,
        Message::SetHeating(SetHeatingPayload {
            on: true,
            duration_ms: 5000,
        }),
    );
    heating_power(&mut m, 500, false);
    assert_eq!(last_heating(&m).power_permille, 0);

    let mut m = present();
    heating_power(&mut m, 500, false);
    rx(
        &mut m,
        Message::SetHeating(SetHeatingPayload {
            on: true,
            duration_ms: 5000,
        }),
    );
    assert_eq!(last_heating(&m).power_permille, 500);
    assert_eq!(last_heating(&m).lease_remaining_ms, 1500);
}

#[test]
fn heater_output_polarity_is_owned_by_the_hardware_layer() {
    // La logique ne parle que de « SSR actif » : jamais d'un niveau de
    // broche. Le niveau BAS = chauffe est l'affaire de `Hw::set_heater`.
    let mut m = present();
    heating_power(&mut m, 1000, false);
    assert!(m.hw.heater);
    send_stop(&mut m);
    assert!(!m.hw.heater);
}

// --------------------------------------------------------------------- pompe

fn last_pump(m: &Machine<Mock>) -> StatusPumpPayload {
    m.hw.sent
        .iter()
        .rev()
        .find_map(|f| match decoded(f) {
            Message::StatusPump(p) => Some(p),
            _ => None,
        })
        .expect("aucun STATUS_PUMP")
}

/// Pompe en marche à 50 % avec un tachymètre qui répond, après 1 s.
fn running_pump() -> Machine<Mock> {
    let mut m = present();
    m.hw.tach_rpm = 2250;
    m.hw.pressure_raw = Some(6000); // ≈ 2 bar : pas de marche à sec
    set(&mut m, 50, 20_000);
    advance(&mut m, S, true);
    assert_eq!(m.hw.duty, pump_duty(50));
    m
}

#[test]
fn the_duty_follows_the_set_percentage_and_logs_each_cycle_start() {
    let mut m = present();
    set(&mut m, 40, 5000);
    set(&mut m, 60, 5000);
    assert_eq!(m.hw.duty, pump_duty(60));
    // Un seul LOG de démarrage : le deuxième SET ne part pas de 0.
    assert_eq!(
        log_args(&m, LogCode::PumpStart),
        [(40, u32::from(pump_duty(40)))]
    );
    set(&mut m, 0, 5000);
    assert_eq!(m.hw.duty, 0);
    set(&mut m, 30, 5000);
    assert_eq!(log_args(&m, LogCode::PumpStart).len(), 2);
}

#[test]
fn the_pump_duty_is_zero_when_idle_and_full_scale_is_never_commanded() {
    let mut m = present();
    advance(&mut m, 3 * S, true);
    assert!(m.hw.duty_writes.iter().all(|&d| d == 0));
    set(&mut m, 100, 500);
    assert!(m.hw.duty > 0 && m.hw.duty < PUMP_DUTY_FULL);
}

#[test]
fn every_cut_path_sets_the_duty_to_zero() {
    // Perte de présence.
    let mut m = running_pump();
    advance(&mut m, 4 * S, false);
    assert!(has_log(&m, LogCode::PresenceLost));
    assert_eq!(m.hw.duty, 0);
    // STOP.
    let mut m = running_pump();
    send_stop(&mut m);
    assert_eq!(m.hw.duty, 0);
    // Bail expiré.
    let mut m = present();
    m.hw.tach_rpm = 2250;
    set(&mut m, 50, 300);
    advance(&mut m, 300 * MS, true);
    assert_eq!(m.hw.duty, 0);
    // Verrou de 60 s.
    let mut m = present();
    m.hw.tach_rpm = 2250;
    set(&mut m, 50, 500);
    for _ in 0..620 {
        advance(&mut m, 100 * MS, true);
        set(&mut m, 50, 500);
        if m.lockout_active() {
            break;
        }
    }
    assert!(m.lockout_active());
    assert_eq!(m.hw.duty, 0);
    // Vanne de maintenance : pompe à 0.
    let mut m = present();
    open_maintenance(&mut m, 10_000);
    assert_eq!(m.hw.duty, 0);
}

#[test]
fn a_silent_tachometer_cuts_the_pump_closes_the_valve_and_latches_the_fault() {
    let mut m = present();
    m.hw.tach_rpm = 0;
    set(&mut m, 50, 20_000);
    // 500 ms de consigne ne suffisent pas.
    advance(&mut m, 500 * MS, true);
    assert_eq!(m.hw.duty, pump_duty(50));
    assert!(!has_log(&m, LogCode::PumpFault));
    assert!(m.hw.valve);
    advance(&mut m, 200 * MS, true);
    assert_eq!(m.hw.duty, 0);
    // La pompe ne repartira pas de ce cycle : la vanne se ferme avec elle.
    assert!(!m.hw.valve);
    assert!(!last_actuators(&m).valve_open);
    assert_eq!(
        log_args(&m, LogCode::PumpFault),
        [(50, u32::from(pump_duty(50)))]
    );
    let st = last_actuators(&m);
    assert_eq!(st.flags & StatusActuatorsPayload::FLAG_PUMP_READY, 0);
    assert_eq!(st.flags & StatusActuatorsPayload::FLAG_PUMP_FAULT, 0x08);
    assert_eq!(st.flags & StatusActuatorsPayload::FLAG_PUMP_PRESENT, 0x04);

    // Verrouillé : les SET suivants ne relancent pas la pompe ni ne
    // rouvrent la vanne.
    set(&mut m, 50, 20_000);
    advance(&mut m, S, true);
    assert_eq!(m.hw.duty, 0);
    assert!(!m.hw.valve);
    assert_eq!(log_args(&m, LogCode::PumpFault).len(), 1);

    // Le cycle fini, la pompe est de nouveau prête mais le défaut reste.
    set(&mut m, 0, 500);
    let st = last_actuators(&m);
    assert_eq!(st.flags & StatusActuatorsPayload::FLAG_PUMP_READY, 0x02);
    assert_eq!(st.flags & StatusActuatorsPayload::FLAG_PUMP_FAULT, 0x08);

    // Le prochain SET parti de 0 lève le défaut et relance la pompe.
    m.hw.tach_rpm = 2250;
    set(&mut m, 50, 20_000);
    assert_eq!(m.hw.duty, pump_duty(50));
    assert!(m.hw.valve);
    let st = last_actuators(&m);
    assert!(st.valve_open);
    assert_eq!(st.flags & StatusActuatorsPayload::FLAG_PUMP_FAULT, 0);
    assert_eq!(st.flags & StatusActuatorsPayload::FLAG_PUMP_READY, 0x02);
    advance(&mut m, 2 * S, true);
    assert_eq!(m.hw.duty, pump_duty(50));
}

#[test]
fn a_restart_within_50_ms_of_the_cut_waits_for_the_pump_to_rearm() {
    let mut m = present();
    m.hw.tach_rpm = 0;
    set(&mut m, 50, 20_000);
    advance(&mut m, 600 * MS, true);
    assert_eq!(m.hw.duty, 0);
    // Au même instant que la coupure : SET 0 puis SET 30.
    set(&mut m, 0, 20_000);
    m.hw.tach_rpm = 1350;
    set(&mut m, 30, 20_000);
    assert_eq!(m.hw.duty, 0);
    m.hw.now += 50 * MS;
    m.tick_safety();
    assert_eq!(m.hw.duty, pump_duty(30));
    let st = last_actuators(&m);
    assert_eq!(st.flags & StatusActuatorsPayload::FLAG_PUMP_FAULT, 0);
}

#[test]
fn a_live_tachometer_or_a_tiny_setpoint_raises_no_fault() {
    let mut m = running_pump();
    advance(&mut m, 5 * S, true);
    assert!(!has_log(&m, LogCode::PumpFault));
    assert_eq!(m.hw.duty, pump_duty(50));
    // 6 % = 270 tr/min, sous 0,3 V : la pompe peut rester muette.
    let mut m = present();
    m.hw.tach_rpm = 0;
    set(&mut m, 6, 20_000);
    advance(&mut m, 3 * S, true);
    assert!(!has_log(&m, LogCode::PumpFault));
    assert_eq!(m.hw.duty, pump_duty(6));
}

fn dry_machine() -> Machine<Mock> {
    let mut m = present();
    m.hw.tach_rpm = 2250;
    m.hw.pressure_raw = Some(3200); // ≈ 0 bar
    set(&mut m, 50, 30_000);
    m
}

#[test]
fn running_dry_cuts_the_pump_after_two_seconds() {
    let mut m = dry_machine();
    advance(&mut m, 1900 * MS, true);
    assert_eq!(m.hw.duty, pump_duty(50));
    advance(&mut m, 200 * MS, true);
    assert_eq!(m.hw.duty, 0);
    assert_eq!(log_args(&m, LogCode::PumpDry), [(2250, 3200)]);
    let st = last_actuators(&m);
    assert_eq!(st.flags & StatusActuatorsPayload::FLAG_PUMP_FAULT, 0x08);
    assert_eq!(st.flags & StatusActuatorsPayload::FLAG_PUMP_READY, 0);
    assert!(!m.hw.valve);
    // Verrouillée comme le défaut de tachymètre.
    set(&mut m, 50, 30_000);
    advance(&mut m, S, true);
    assert_eq!(m.hw.duty, 0);
    assert!(!m.hw.valve);
    set(&mut m, 0, 500);
    set(&mut m, 50, 30_000);
    assert_eq!(m.hw.duty, pump_duty(50));
    assert!(m.hw.valve);
}

#[test]
fn pressure_a_digmesa_edge_or_a_slow_pump_prevent_the_dry_rule() {
    // Pression à 1 bar ou plus.
    let mut m = dry_machine();
    m.hw.pressure_raw = Some(DRY_PRESSURE_MAX_CODE);
    advance(&mut m, 5 * S, true);
    assert!(!has_log(&m, LogCode::PumpDry));
    // Une impulsion Digmesa toutes les 500 ms.
    let mut m = dry_machine();
    for i in 0..10 {
        m.hw.flow.on_edge(m.hw.now + 1000 + i);
        advance(&mut m, 500 * MS, true);
        m.hw.flow.last_edge_us = m.hw.now;
    }
    assert!(!has_log(&m, LogCode::PumpDry));
    // Sous 1 000 tr/min.
    let mut m = dry_machine();
    m.hw.tach_rpm = 900;
    advance(&mut m, 5 * S, true);
    assert!(!has_log(&m, LogCode::PumpDry));
}

#[test]
fn a_digmesa_edge_restarts_the_dry_window() {
    let mut m = dry_machine();
    advance(&mut m, 1500 * MS, true);
    m.hw.flow.last_edge_us = m.hw.now;
    advance(&mut m, 1500 * MS, true);
    assert!(!has_log(&m, LogCode::PumpDry));
    advance(&mut m, 700 * MS, true);
    assert!(has_log(&m, LogCode::PumpDry));
}

#[test]
fn a_missing_or_implausible_pressure_counts_as_below_one_bar() {
    for raw in [None, Some(100), Some(25_000)] {
        let mut m = dry_machine();
        m.hw.pressure_raw = raw;
        advance(&mut m, 2200 * MS, true);
        assert!(has_log(&m, LogCode::PumpDry), "{raw:?}");
    }
}

#[test]
fn the_dry_rule_reuses_a_fresh_published_pressure() {
    let mut m = dry_machine();
    rx(
        &mut m,
        Message::ReqStatus(ReqStatusPayload::new(MessageType::StatusPressure, 100)),
    );
    m.tick_pressure();
    let reads = m.hw.pressure_reads;
    m.hw.now += 100 * MS;
    m.tick_safety();
    assert_eq!(m.hw.pressure_reads, reads);
    // Au-delà de 300 ms, le capteur est relu.
    m.hw.now += 400 * MS;
    m.tick_safety();
    assert!(m.hw.pressure_reads > reads);
}

#[test]
fn the_old_dimmer_command_is_ignored_with_one_warning() {
    let mut m = running_pump();
    m.hw.sent.clear();
    let id = protocol::encode_can_id(protocol::CanId {
        message_type: MessageType::DimmerCommand,
        dest: Dest::Sensors,
        src: Node::Screen,
    });
    m.on_frame(&Frame::new(id, &[2]).unwrap());
    m.on_frame(&Frame::new(id, &[1]).unwrap());
    assert!(m.hw.valve);
    assert_eq!(m.hw.duty, pump_duty(50));
    assert_eq!(
        log_args(&m, LogCode::LegacyCommandIgnored),
        [(MessageType::DimmerCommand as u16, 0)]
    );
    let severity = logs(&m)[0].severity;
    assert_eq!(severity, LogSeverity::Warn as u8);
}

#[test]
fn status_pump_is_published_with_a_50_ms_floor_and_carries_the_tachometer() {
    let mut m = present();
    rx(
        &mut m,
        Message::ReqStatus(ReqStatusPayload::new(MessageType::StatusPump, 10)),
    );
    m.tick_safety();
    let st = last_pump(&m);
    assert_eq!((st.tach_pulses, st.rpm, st.flags), (0, 0, 0));

    m.hw.tach_rpm = 3000;
    m.hw.sent.clear();
    advance(&mut m, 1000 * MS, true);
    let count =
        m.hw.sent
            .iter()
            .filter(|f| decoded(f).message_type() == MessageType::StatusPump)
            .count();
    assert_eq!(count, 10);
    let st = last_pump(&m);
    assert_eq!((st.tach_pulses, st.rpm), (100, 3000));
    assert_eq!(st.flags, StatusPumpPayload::FLAG_TACH_SEEN);

    rx(
        &mut m,
        Message::ReqStatus(ReqStatusPayload::new(MessageType::StatusPump, 0)),
    );
    m.hw.sent.clear();
    advance(&mut m, 500 * MS, true);
    assert!(
        m.hw.sent
            .iter()
            .all(|f| decoded(f).message_type() != MessageType::StatusPump)
    );
}

#[test]
fn status_pump_flags_report_the_latched_faults() {
    let mut m = dry_machine();
    rx(
        &mut m,
        Message::ReqStatus(ReqStatusPayload::new(MessageType::StatusPump, 100)),
    );
    advance(&mut m, 2200 * MS, true);
    assert_eq!(
        last_pump(&m).flags,
        StatusPumpPayload::FLAG_TACH_SEEN | StatusPumpPayload::FLAG_DRY
    );
    let mut m = present();
    rx(
        &mut m,
        Message::ReqStatus(ReqStatusPayload::new(MessageType::StatusPump, 100)),
    );
    set(&mut m, 50, 20_000);
    advance(&mut m, 800 * MS, true);
    assert_eq!(last_pump(&m).flags, StatusPumpPayload::FLAG_FAULT);
}

// ------------------------------------------------------ publications / REQSTATUS

#[test]
fn reqstatus_sets_periods_and_publishes_at_once() {
    let mut m = present();
    rx(
        &mut m,
        Message::ReqStatus(ReqStatusPayload::new(MessageType::StatusActuators, 200)),
    );
    rx(
        &mut m,
        Message::ReqStatus(ReqStatusPayload::new(MessageType::StatusHeating, 300)),
    );
    rx(
        &mut m,
        Message::ReqStatus(ReqStatusPayload::new(MessageType::StatusFlow, 100)),
    );
    m.hw.flow.on_edge(12 * S);
    m.hw.sent.clear();
    m.tick_safety();
    let kinds: Vec<MessageType> =
        m.hw.sent
            .iter()
            .map(|f| decoded(f).message_type())
            .collect();
    assert!(kinds.contains(&MessageType::StatusFlow));
    assert!(kinds.contains(&MessageType::StatusActuators));
    assert!(kinds.contains(&MessageType::StatusHeating));
    let flow =
        m.hw.sent
            .iter()
            .find_map(|f| match decoded(f) {
                Message::StatusFlow(p) => Some(p),
                _ => None,
            })
            .unwrap();
    assert_eq!(
        (flow.pulse_count, flow.last_edge_ms, flow.flags),
        (1, 12_000, 1)
    );

    m.hw.sent.clear();
    advance(&mut m, 1000 * MS, true);
    fn count(m: &Machine<Mock>, kind: MessageType) -> usize {
        m.hw.sent
            .iter()
            .filter(|f| decoded(f).message_type() == kind)
            .count()
    }
    assert_eq!(count(&m, MessageType::StatusFlow), 10);
    assert_eq!(count(&m, MessageType::StatusActuators), 5);
    assert_eq!(count(&m, MessageType::StatusHeating), 3);

    rx(
        &mut m,
        Message::ReqStatus(ReqStatusPayload::new(MessageType::StatusFlow, 0)),
    );
    m.hw.sent.clear();
    advance(&mut m, 500 * MS, true);
    assert_eq!(count(&m, MessageType::StatusFlow), 0);
}

#[test]
fn pressure_is_published_without_a_sensor_with_a_100_ms_floor() {
    let mut m = present();
    rx(
        &mut m,
        Message::ReqStatus(ReqStatusPayload::new(MessageType::StatusPressure, 10)),
    );
    m.hw.sent.clear();
    m.tick_pressure();
    assert_eq!(m.hw.sent.len(), 1);
    let Message::StatusPressure(p) = decoded(&m.hw.sent[0]) else {
        panic!()
    };
    assert_eq!(p.flags, 0);
    assert_eq!(p.timestamp_ms, 10_000);
    assert_eq!(m.hw.sent[0].len, 8);
    m.hw.now += 90 * MS;
    m.tick_pressure();
    assert_eq!(m.hw.sent.len(), 1);
    m.hw.now += 10 * MS;
    m.tick_pressure();
    assert_eq!(m.hw.sent.len(), 2);
    rx(
        &mut m,
        Message::ReqStatus(ReqStatusPayload::new(MessageType::StatusPressure, 0)),
    );
    m.hw.now += S;
    m.tick_pressure();
    assert_eq!(m.hw.sent.len(), 2);
}

#[test]
fn pressure_is_valid_only_inside_the_probe_window() {
    let mut m = present();
    rx(
        &mut m,
        Message::ReqStatus(ReqStatusPayload::new(MessageType::StatusPressure, 100)),
    );
    let publish = |m: &mut Machine<Mock>, raw: Option<i16>| {
        m.hw.pressure_raw = raw;
        m.hw.now += 100 * MS;
        m.hw.sent.clear();
        m.tick_pressure();
        let Message::StatusPressure(p) = decoded(&m.hw.sent[0]) else {
            panic!()
        };
        (p.pressure_raw, p.flags)
    };
    // Sonde au repos (0 bar), puis à 9 bar.
    assert_eq!(publish(&mut m, Some(3167)), (3167, 1));
    assert_eq!(publish(&mut m, Some(15_167)), (15_167, 1));
    // Vide d'une chaudière refroidie (−0,47 bar), puis bornes de la fenêtre.
    assert_eq!(publish(&mut m, Some(2534)), (2534, 1));
    assert_eq!(publish(&mut m, Some(2230)), (2230, 1));
    assert_eq!(publish(&mut m, Some(20_800)), (20_800, 1));
    // Entrée à la masse, saturée, négative ou ADS1115 muet : invalide.
    assert_eq!(publish(&mut m, Some(2229)), (0, 0));
    assert_eq!(publish(&mut m, Some(20_801)), (0, 0));
    assert_eq!(publish(&mut m, Some(-5)), (0, 0));
    assert_eq!(publish(&mut m, None), (0, 0));
}

#[test]
fn enclosure_is_published_with_a_250_ms_floor() {
    fn enclosures(m: &Machine<Mock>) -> Vec<StatusEnclosurePayload> {
        m.hw.sent
            .iter()
            .filter_map(|f| match decoded(f) {
                Message::StatusEnclosure(p) => Some(p),
                _ => None,
            })
            .collect()
    }
    let mut m = present();
    rx(
        &mut m,
        Message::ReqStatus(ReqStatusPayload::new(MessageType::StatusEnclosure, 100)),
    );
    m.hw.sent.clear();
    m.tick_safety();
    let first = enclosures(&m);
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].temperature_c(), 31.0);
    assert_eq!(first[0].flags, StatusEnclosurePayload::FLAG_VALID);
    let frame =
        m.hw.sent
            .iter()
            .find(|f| decoded(f).message_type() == MessageType::StatusEnclosure);
    assert_eq!(frame.unwrap().len, 5);

    m.hw.sent.clear();
    m.hw.enclosure_raw = None;
    advance(&mut m, 1000 * MS, true);
    // Ticks de 100 ms : 250 ms se publient toutes les 300 ms.
    let next = enclosures(&m);
    assert_eq!(next.len(), 3);
    assert!(next.iter().all(|p| p.flags == 0));

    rx(
        &mut m,
        Message::ReqStatus(ReqStatusPayload::new(MessageType::StatusEnclosure, 0)),
    );
    m.hw.sent.clear();
    advance(&mut m, 1000 * MS, true);
    assert!(enclosures(&m).is_empty());
}

#[test]
fn twai_counters_are_logged_every_5_s() {
    let mut m = present();
    m.hw.sent.clear();
    advance(&mut m, 12 * S, true);
    // Le premier journal arrive quand le temps depuis le démarrage dépasse 5 s.
    let args = log_args(&m, LogCode::TwaiErrorCounters);
    assert_eq!(args.len(), 3);
    assert_eq!(args[0], ((3 << 8) | 5, 7));
}

#[test]
fn status_frames_have_the_dlc_of_the_cpp_senders() {
    let mut m = present();
    set(&mut m, 10, 500);
    heating_power(&mut m, 100, false);
    rx(
        &mut m,
        Message::ReqStatus(ReqStatusPayload::new(MessageType::StatusFlow, 100)),
    );
    m.tick_safety();
    for f in &m.hw.sent {
        let want = match decoded(f).message_type() {
            MessageType::StatusActuators | MessageType::StatusFlow => 7,
            MessageType::StatusHeating => 6,
            MessageType::StatusPressure
            | MessageType::Log
            | MessageType::Pong
            | MessageType::Ping => 8,
            other => panic!("{other:?}"),
        };
        assert_eq!(f.len, want);
        let id = decode_can_id(f.id).unwrap();
        assert_eq!(id.src, Node::Sensors);
    }
}

// --------------------------------------------------------- validation de l'OTA

fn pending(validate: bool) -> Machine<Mock> {
    let mut m = Machine::new(
        Mock::new(),
        Config {
            lockout_active: false,
            ota_pending: Some(1),
            validate,
        },
    );
    send_stop(&mut m);
    // Aller-retour PING/PONG et publications demandées.
    send_stop_keepalive(&mut m);
    rx(
        &mut m,
        Message::ReqStatus(ReqStatusPayload::new(MessageType::StatusActuators, 500)),
    );
    rx(
        &mut m,
        Message::ReqStatus(ReqStatusPayload::new(MessageType::StatusHeating, 500)),
    );
    m.hw.sent.clear();
    m
}

fn confirm(m: &mut Machine<Mock>, dlc: usize, token: u8) {
    let id = protocol::encode_can_id(protocol::CanId {
        message_type: MessageType::ConfirmSensorsOta,
        dest: Dest::Sensors,
        src: Node::Screen,
    });
    let data = [token, 0];
    m.on_frame(&Frame::new(id, &data[..dlc]).unwrap());
}

#[test]
fn confirm_validates_a_pending_image() {
    let mut m = pending(true);
    confirm(&mut m, 1, 63);
    m.tick_safety();
    assert_eq!(m.hw.marked_valid, 1);
    assert!(has_log(&m, LogCode::OtaValidated));
    assert_eq!(m.take_action(), Action::None);
    // Une fois validée, plus de retour en arrière.
    advance(&mut m, 40 * S, true);
    assert_eq!(m.take_action(), Action::None);
    assert_eq!(m.hw.marked_valid, 1);
}

#[test]
fn confirm_needs_dlc_1_and_token_63() {
    let mut m = pending(true);
    confirm(&mut m, 2, 63);
    confirm(&mut m, 1, 62);
    m.tick_safety();
    assert_eq!(m.hw.marked_valid, 0);
}

#[test]
fn confirm_needs_the_roundtrip_the_periods_and_rest() {
    // Pas d'aller-retour PING/PONG.
    let mut m = Machine::new(
        Mock::new(),
        Config {
            lockout_active: false,
            ota_pending: Some(0),
            validate: true,
        },
    );
    send_stop(&mut m);
    rx(
        &mut m,
        Message::ReqStatus(ReqStatusPayload::new(MessageType::StatusActuators, 500)),
    );
    rx(
        &mut m,
        Message::ReqStatus(ReqStatusPayload::new(MessageType::StatusHeating, 500)),
    );
    confirm(&mut m, 1, 63);
    m.tick_safety();
    assert_eq!(m.hw.marked_valid, 0);

    // Pas de période de publication demandée.
    let mut m = Machine::new(
        Mock::new(),
        Config {
            lockout_active: false,
            ota_pending: Some(1),
            validate: true,
        },
    );
    send_stop_keepalive(&mut m);
    confirm(&mut m, 1, 63);
    m.tick_safety();
    assert_eq!(m.hw.marked_valid, 0);

    // Pompe en marche.
    let mut m = pending(true);
    set(&mut m, 30, 5000);
    confirm(&mut m, 1, 63);
    m.tick_safety();
    assert_eq!(m.hw.marked_valid, 0);

    // Impulsion de chauffe diagnostique : refusée ; chauffe régulée : permise.
    let mut m = pending(true);
    rx(
        &mut m,
        Message::SetHeating(SetHeatingPayload {
            on: true,
            duration_ms: 5000,
        }),
    );
    confirm(&mut m, 1, 63);
    m.tick_safety();
    assert_eq!(m.hw.marked_valid, 0);

    let mut m = pending(true);
    heating_power(&mut m, 500, false);
    confirm(&mut m, 1, 63);
    m.tick_safety();
    assert_eq!(m.hw.marked_valid, 1);
}

#[test]
fn an_image_not_confirmed_within_30_s_rolls_back() {
    let mut m = pending(true);
    advance(&mut m, 29 * S, true);
    assert_eq!(m.take_action(), Action::None);
    assert!(!has_log(&m, LogCode::OtaRollback));
    advance(&mut m, 2 * S, true);
    assert!(has_log(&m, LogCode::OtaRollback));
    assert_eq!(m.take_action(), Action::RollbackReboot);
}

#[test]
fn a_failed_mark_valid_keeps_waiting_and_rolls_back_at_30_s() {
    let mut m = pending(true);
    m.hw.mark_valid_ok = false;
    confirm(&mut m, 1, 63);
    advance(&mut m, 31 * S, true);
    assert!(!has_log(&m, LogCode::OtaValidated));
    assert_eq!(m.take_action(), Action::RollbackReboot);
}

#[test]
fn a_validated_image_never_rolls_back() {
    let mut m = machine();
    send_stop(&mut m);
    advance(&mut m, 60 * S, true);
    assert_eq!(m.take_action(), Action::None);
    assert!(!has_log(&m, LogCode::OtaRollback));
}

#[test]
fn no_validate_ignores_the_confirmation_and_reboots_after_30_s() {
    let mut m = pending(false);
    confirm(&mut m, 1, 63);
    advance(&mut m, 29 * S, true);
    assert_eq!(m.hw.marked_valid, 0);
    assert_eq!(m.take_action(), Action::None);
    advance(&mut m, 2 * S, true);
    assert_eq!(m.hw.marked_valid, 0);
    assert!(!has_log(&m, LogCode::OtaValidated));
    assert_eq!(m.take_action(), Action::RollbackReboot);
}

// ----------------------------------------------------------------- réception FLASH

fn esp_image(len: usize) -> Vec<u8> {
    let mut img: Vec<u8> = (0..len).map(|i| (i * 13 + 5) as u8).collect();
    img[0] = 0xE9;
    img[1] = 4;
    img[12] = 9;
    img[13] = 0;
    img
}

fn flash_ctrl(m: &mut Machine<Mock>, ctrl: FlashCtrlPayload) {
    rx(m, Message::FlashCtrl(ctrl));
}

fn flash_data(m: &mut Machine<Mock>, bytes: &[u8]) {
    for chunk in bytes.chunks(8) {
        rx(m, Message::FlashData(FlashDataPayload::new(chunk).unwrap()));
    }
}

fn block_acks(m: &Machine<Mock>) -> Vec<(u16, u16)> {
    m.hw.sent
        .iter()
        .filter_map(|f| match decoded(f) {
            Message::FlashCtrl(FlashCtrlPayload::BlockAck {
                block_number,
                block_crc16,
            }) => {
                let id = decode_can_id(f.id).unwrap();
                assert_eq!((id.src, id.dest, f.len), (Node::Sensors, Dest::Screen, 8));
                Some((block_number, block_crc16))
            }
            _ => None,
        })
        .collect()
}

#[test]
fn flash_begin_cuts_the_actuators_erases_and_acks_block_0() {
    let mut m = present();
    set(&mut m, 50, 5000);
    heating_power(&mut m, 500, false);
    m.hw.sent.clear();
    flash_ctrl(&mut m, FlashCtrlPayload::Begin { image_size: 5000 });
    assert!(!m.hw.valve && !m.hw.heater);
    assert_eq!(m.hw.erased, 1);
    assert_eq!(block_acks(&m), [(0, 0)]);
    assert_eq!(log_args(&m, LogCode::FlashBegin), [(0, 5000)]);
    // Aucune commande d'actionneur tant que le transfert dure.
    set(&mut m, 50, 5000);
    assert!(!m.hw.valve);
}

#[test]
fn flash_begin_too_big_or_zero_fails_without_erasing() {
    let mut m = present();
    flash_ctrl(
        &mut m,
        FlashCtrlPayload::Begin {
            image_size: 64 * 1024 + 1,
        },
    );
    flash_ctrl(&mut m, FlashCtrlPayload::Begin { image_size: 0 });
    assert_eq!(m.hw.erased, 0);
    assert_eq!(
        log_args(&m, LogCode::FlashFailed),
        [(0, 64 * 1024 + 1), (0, 0)]
    );
    assert!(block_acks(&m).is_empty());
    set(&mut m, 50, 500);
    assert!(m.hw.valve);
}

#[test]
fn a_complete_transfer_acks_every_block_then_commits_and_reboots() {
    let img = esp_image(2048 * 2 + 100);
    let mut m = present();
    flash_ctrl(
        &mut m,
        FlashCtrlPayload::Begin {
            image_size: img.len() as u32,
        },
    );
    m.hw.sent.clear();

    let mut expected = vec![];
    for (n, block) in img.chunks(2048).enumerate() {
        let crc = crc16_ccitt(block);
        flash_ctrl(
            &mut m,
            FlashCtrlPayload::BlockStart {
                block_number: n as u16 + 1,
                block_crc16: crc,
            },
        );
        flash_data(&mut m, block);
        expected.push((n as u16 + 1, crc));
    }
    assert_eq!(block_acks(&m), expected);
    assert_eq!(&m.hw.flash[..img.len()], &img[..]);
    let progress = log_args(&m, LogCode::FlashProgress);
    assert_eq!(progress, [(0, 2048), (0, 4096), (0, 4196)]);

    m.hw.sent.clear();
    flash_ctrl(
        &mut m,
        FlashCtrlPayload::End {
            image_crc32: crc32_ieee(&img),
        },
    );
    assert_eq!(m.hw.commits, 1);
    assert!(has_log(&m, LogCode::FlashDone));
    let end =
        m.hw.sent
            .iter()
            .find(|f| matches!(decoded(f), Message::FlashCtrl(FlashCtrlPayload::End { .. })))
            .expect("accusé END");
    let id = decode_can_id(end.id).unwrap();
    assert_eq!((id.src, id.dest, end.len), (Node::Sensors, Dest::Screen, 8));
    assert_eq!(m.take_action(), Action::Reboot);
}

#[test]
fn a_lost_ack_is_replayed_without_rewriting_the_block() {
    let img = esp_image(4096);
    let mut m = present();
    flash_ctrl(&mut m, FlashCtrlPayload::Begin { image_size: 4096 });
    let crc1 = crc16_ccitt(&img[..2048]);
    flash_ctrl(
        &mut m,
        FlashCtrlPayload::BlockStart {
            block_number: 1,
            block_crc16: crc1,
        },
    );
    flash_data(&mut m, &img[..2048]);
    m.hw.sent.clear();

    flash_ctrl(
        &mut m,
        FlashCtrlPayload::BlockStart {
            block_number: 1,
            block_crc16: crc1,
        },
    );
    assert_eq!(block_acks(&m), [(1, crc1)]);
    let progress_before = log_args(&m, LogCode::FlashProgress).len();
    flash_data(&mut m, &img[..2048]);
    assert_eq!(log_args(&m, LogCode::FlashProgress).len(), progress_before);
    assert_eq!(block_acks(&m), [(1, crc1)]);

    let crc2 = crc16_ccitt(&img[2048..]);
    flash_ctrl(
        &mut m,
        FlashCtrlPayload::BlockStart {
            block_number: 2,
            block_crc16: crc2,
        },
    );
    flash_data(&mut m, &img[2048..]);
    flash_ctrl(
        &mut m,
        FlashCtrlPayload::End {
            image_crc32: crc32_ieee(&img),
        },
    );
    assert_eq!(m.hw.commits, 1);
}

#[test]
fn a_bad_global_crc_at_end_fails_and_leaves_the_boot_slot_alone() {
    let img = esp_image(3000);
    let mut m = present();
    flash_ctrl(&mut m, FlashCtrlPayload::Begin { image_size: 3000 });
    for (n, block) in img.chunks(2048).enumerate() {
        flash_ctrl(
            &mut m,
            FlashCtrlPayload::BlockStart {
                block_number: n as u16 + 1,
                block_crc16: crc16_ccitt(block),
            },
        );
        flash_data(&mut m, block);
    }
    flash_ctrl(&mut m, FlashCtrlPayload::End { image_crc32: 1 });
    assert_eq!(m.hw.commits, 0);
    assert_eq!(log_args(&m, LogCode::FlashFailed), [(0, 3000)]);
    assert_eq!(m.take_action(), Action::None);
    // Le transfert est fini : les actionneurs répondent de nouveau.
    set(&mut m, 50, 500);
    assert!(m.hw.valve);
}

#[test]
fn abort_ends_the_transfer_with_a_warning_and_never_commits() {
    let mut m = present();
    flash_ctrl(&mut m, FlashCtrlPayload::Begin { image_size: 3000 });
    flash_ctrl(&mut m, FlashCtrlPayload::Abort);
    let l = logs(&m);
    let failed = l
        .iter()
        .find(|l| l.code == LogCode::FlashFailed as u8)
        .unwrap();
    assert_eq!(failed.severity, LogSeverity::Warn as u8);
    assert_eq!(m.hw.commits, 0);
    // Un second ABORT sans transfert ne dit rien.
    m.hw.sent.clear();
    flash_ctrl(&mut m, FlashCtrlPayload::Abort);
    assert!(m.hw.sent.is_empty());
}

#[test]
fn end_without_a_transfer_fails() {
    let mut m = present();
    flash_ctrl(&mut m, FlashCtrlPayload::End { image_crc32: 0 });
    assert!(has_log(&m, LogCode::FlashFailed));
    assert_eq!(m.take_action(), Action::None);
}

#[test]
fn presence_loss_during_a_flash_does_not_abort_it() {
    // Comportement du C++ archivé : seule la sécurité des actionneurs joue.
    let mut m = present();
    flash_ctrl(&mut m, FlashCtrlPayload::Begin { image_size: 3000 });
    advance(&mut m, 4 * S, false);
    assert!(has_log(&m, LogCode::PresenceLost));
    set(&mut m, 50, 500);
    assert!(!m.hw.valve);
}

#[test]
fn pump_pct_over_100_never_reaches_the_pump() {
    let mut m = present();
    set(&mut m, 255, 500);
    assert!(m.hw.duty_writes.iter().all(|&d| d == 0));
}
