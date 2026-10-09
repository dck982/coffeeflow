//! Vecteurs produits par le C++ archivé (`vectors/golden.cpp`) : octets
//! encodés, PDU cadrés, CRC, COBS, flux avec bruit, et comportement de
//! `unpack()` sur des entrées quelconques.

use protocol::messages::*;
use protocol::*;

const ENCODED: &str = include_str!("../vectors/encoded.txt");
const DIFFERENTIAL: &str = include_str!("../vectors/differential.txt");

fn unhex(s: &str) -> Vec<u8> {
    if s == "-" {
        return Vec::new();
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

fn lines<'a>(text: &'a str, kind: &'a str) -> impl Iterator<Item = Vec<&'a str>> {
    text.lines()
        .filter(move |l| l.split(' ').next() == Some(kind))
        .map(|l| l.split(' ').skip(1).collect())
}

fn identity(node: Node, major: u8, minor: u8, patch: u8, uptime_s: u32) -> NodeIdentityPayload {
    NodeIdentityPayload::new(node, Version::new(major, minor, patch), uptime_s)
}

/// Le message, l'émetteur et le destinataire de chaque trame de référence.
#[rustfmt::skip]
fn expected(name: &str) -> (Message, Node, Dest) {
    use Dest::{Broadcast as B, Screen as Sc, Sensors as S};
    use Node::{Screen, Sensors};
    let status_heating = |p: StatusHeatingPayload| Message::StatusHeating(p);
    match name {
        "stop" => (Message::Stop, Screen, S),
        "reset" => (Message::Reset, Screen, S),
        "set_42_500" => (Message::Set(SetPayload { pump_pct: 42, ttl_ms: 500 }), Screen, S),
        "set_100_default" => (Message::Set(SetPayload { pump_pct: 100, ttl_ms: 0 }), Screen, S),
        "set_0_65535" => (Message::Set(SetPayload { pump_pct: 0, ttl_ms: 65535 }), Screen, S),
        "dimmer_reset" => (Message::DimmerCommand(DimmerCommandPayload { command: DimmerCommand::Reset }), Screen, S),
        "dimmer_recalibrate" => (Message::DimmerCommand(DimmerCommandPayload { command: DimmerCommand::Recalibrate }), Screen, S),
        "set_heating_on" => (Message::SetHeating(SetHeatingPayload { on: true, duration_ms: 1000 }), Screen, S),
        "set_heating_off" => (Message::SetHeating(SetHeatingPayload { on: false, duration_ms: 0 }), Screen, S),
        "confirm_ota" => (Message::ConfirmSensorsOta(ConfirmSensorsOtaPayload { protocol_patch: 63 }), Screen, S),
        "set_heating_power_dlc4" => (Message::SetHeatingPower(SetHeatingPowerPayload { power_permille: 600, lease_ms: 1500, restart_window: false }), Screen, S),
        "set_heating_power_dlc5" => (Message::SetHeatingPower(SetHeatingPowerPayload { power_permille: 900, lease_ms: 1500, restart_window: true }), Screen, S),
        "set_heating_power_zero" => (Message::SetHeatingPower(SetHeatingPowerPayload { power_permille: 0, lease_ms: 1500, restart_window: false }), Screen, S),
        "set_heating_power_max" => (Message::SetHeatingPower(SetHeatingPowerPayload { power_permille: 1000, lease_ms: 65535, restart_window: true }), Screen, S),
        "maintenance_open_max" => (Message::MaintenanceValve(MaintenanceValvePayload { open: true, duration_ms: 30000 }), Screen, S),
        "maintenance_open_1" => (Message::MaintenanceValve(MaintenanceValvePayload { open: true, duration_ms: 1 }), Screen, S),
        "maintenance_close" => (Message::MaintenanceValve(MaintenanceValvePayload { open: false, duration_ms: 0 }), Screen, S),
        "ping_legacy" => (Message::PingLegacy, Screen, B),
        "ping_screen" => (Message::Ping(identity(Screen, 0, 4, 0, 3600)), Screen, S),
        "ping_sensors_broadcast" => (Message::Ping(identity(Sensors, 0, 3, 48, 1)), Sensors, B),
        "pong_sensors" => (Message::Pong(identity(Sensors, 1, 2, 3, 123_456_789)), Sensors, Sc),
        "pong_uptime_max" => (Message::Pong(identity(Sensors, 255, 255, 255, u32::MAX)), Sensors, Sc),
        "pong_screen" => (Message::Pong(identity(Screen, 0, 4, 0, 42)), Screen, S),
        "reqstatus_flow" => (Message::ReqStatus(ReqStatusPayload::new(MessageType::StatusFlow, 200)), Screen, S),
        "reqstatus_heating" => (Message::ReqStatus(ReqStatusPayload::new(MessageType::StatusHeating, 250)), Screen, S),
        "reqstatus_stop" => (Message::ReqStatus(ReqStatusPayload::new(MessageType::StatusPressure, 0)), Screen, S),
        "status_pressure" => (Message::StatusPressure(StatusPressurePayload { pressure_raw: 0xABCDEF, temperature_raw: 0x1234, timestamp_ms: 60000, flags: 1 }), Sensors, Sc),
        "status_pressure_timeout" => (Message::StatusPressure(StatusPressurePayload { pressure_raw: 0x800000, temperature_raw: 0xFFFF, timestamp_ms: 0, flags: 3 }), Sensors, Sc),
        "status_flow" => (Message::StatusFlow(StatusFlowPayload { pulse_count: 4_000_000_000, last_edge_ms: 1234, flags: 1 }), Sensors, Sc),
        "status_flow_zero" => (Message::StatusFlow(StatusFlowPayload { pulse_count: 0, last_edge_ms: 0, flags: 1 }), Sensors, Sc),
        "status_actuators" => (Message::StatusActuators(StatusActuatorsPayload { valve_open: true, pump_pct: 77, lease_remaining_ms: 480, continuous_on_ms: 59000, flags: 0x25 }), Sensors, Sc),
        "status_actuators_idle" => (Message::StatusActuators(StatusActuatorsPayload { flags: 0x30, ..Default::default() }), Sensors, Sc),
        "status_heating_full" => (status_heating(StatusHeatingPayload { heater_on: true, lease_remaining_ms: 350, power_capable: true, fine_power_capable: true, window_restart_capable: true, power_permille: 937 }), Sensors, Sc),
        "status_heating_off" => (status_heating(StatusHeatingPayload::default()), Sensors, Sc),
        "status_heating_max" => (status_heating(StatusHeatingPayload { power_capable: true, fine_power_capable: true, power_permille: 1000, ..Default::default() }), Sensors, Sc),
        // DLC 5 et 4 : le décodage perd la résolution fine, puis la puissance.
        "status_heating_coarse_dlc5" => (status_heating(StatusHeatingPayload { heater_on: true, lease_remaining_ms: 1000, power_capable: true, power_permille: 60, ..Default::default() }), Sensors, Sc),
        "status_heating_basic_dlc4" => (status_heating(StatusHeatingPayload::default()), Sensors, Sc),
        "log_lockout" => (Message::Log(LogPayload { code: 6, severity: 3, arg16: 0, arg32: 60000 }), Sensors, B),
        "log_boiler_ntc" => (Message::Log(LogPayload { code: 43, severity: 2, arg16: 0x48, arg32: (26485 << 16) | 1050 }), Screen, B),
        "log_args_max" => (Message::Log(LogPayload { code: 255, severity: 255, arg16: 0xFFFF, arg32: u32::MAX }), Sensors, B),
        "flash_begin" => (Message::FlashCtrl(FlashCtrlPayload::Begin { image_size: 1_048_576 }), Screen, S),
        "flash_block_ack" => (Message::FlashCtrl(FlashCtrlPayload::BlockAck { block_number: 42, block_crc16: 0xBEEF }), Sensors, Sc),
        "flash_block_start" => (Message::FlashCtrl(FlashCtrlPayload::BlockStart { block_number: 7, block_crc16: 0xA55A }), Screen, S),
        "flash_end" => (Message::FlashCtrl(FlashCtrlPayload::End { image_crc32: 0xDEAD_BEEF }), Screen, S),
        "flash_end_ack" => (Message::FlashCtrl(FlashCtrlPayload::End { image_crc32: 0 }), Sensors, Sc),
        "flash_abort" => (Message::FlashCtrl(FlashCtrlPayload::Abort), Screen, S),
        "flash_data_8" => (Message::FlashData(FlashDataPayload::new(&[0xDE, 0xAD, 0xBE, 0xEF, 0, 1, 2, 0xFF]).unwrap()), Screen, S),
        "flash_data_3" => (Message::FlashData(FlashDataPayload::new(&[0, 0, 7]).unwrap()), Screen, S),
        "flash_data_zeros" => (Message::FlashData(FlashDataPayload::new(&[0; 8]).unwrap()), Screen, S),
        other => panic!("trame de référence inconnue: {other}"),
    }
}

#[test]
fn encoded_frames_match_cpp() {
    let mut seen = 0;
    for l in lines(ENCODED, "frame") {
        let (name, id, dlc, data, framed) = (l[0], l[1], l[2], unhex(l[3]), unhex(l[4]));
        let id = u16::from_str_radix(id, 16).unwrap();
        let dlc: u8 = dlc.parse().unwrap();
        let (message, src, dest) = expected(name);

        let on_bus = Frame {
            id,
            len: dlc,
            data: data.clone().try_into().unwrap(),
        };

        // Octets sur le bus : identifiant, DLC et 8 octets de données. Les
        // deux STATUS_HEATING courts sont des formats reçus d'anciennes
        // images, que les firmwares n'émettent plus (toujours DLC 6).
        let decode_only = name.starts_with("status_heating_") && name.contains("_dlc");
        if !decode_only {
            assert_eq!(message.encode_routed(src, dest), on_bus, "{name}: encodage");
        }

        // Le cadrage série de la même trame.
        let mut out = [0u8; MAX_FRAMED_SIZE];
        let n = encode_framed(&on_bus, &mut out).unwrap();
        assert_eq!(out[..n], framed[..], "{name}: trame cadrée");

        // Et le sens inverse : le décodage rend le message d'origine.
        assert_eq!(Message::decode(&on_bus), Ok(message), "{name}: décodage");
        let mut decoder = StreamDecoder::new();
        let received: Vec<Frame> = framed
            .iter()
            .filter_map(|&b| decoder.push_byte(b))
            .collect();
        assert_eq!(received, [on_bus], "{name}: flux");
        seen += 1;
    }
    assert!(seen >= 40);
}

#[test]
fn default_routes_match_the_firmware_senders() {
    // Quand l'acheminement par défaut s'applique, il donne l'identifiant C++.
    for l in lines(ENCODED, "frame") {
        let (message, src, dest) = expected(l[0]);
        let id = u16::from_str_radix(l[1], 16).unwrap();
        let routed = message.encode_routed(src, dest);
        assert_eq!(routed.id, id);
        let default = message.encode();
        let by_default = default.can_id().unwrap();
        // Les LOG de l'écran, les accusés END des capteurs et les PING vers
        // le broadcast sont les seuls écarts voulus.
        let special = matches!(
            l[0],
            "log_boiler_ntc" | "flash_end_ack" | "ping_sensors_broadcast"
        );
        assert_eq!(default.id == id, !special, "{}: {by_default:?}", l[0]);
    }
}

#[test]
fn scace_frames_match_cpp() {
    let mut seen = 0;
    for l in lines(ENCODED, "scace") {
        let bytes = unhex(l[1]);
        let decoded = scace::Frame::decode(&bytes).unwrap();
        assert_eq!(decoded.encode()[..], bytes[..], "{}", l[0]);
        seen += 1;
    }
    assert_eq!(seen, 4);
}

#[test]
fn crc_vectors_match_cpp() {
    let mut seen = 0;
    for l in lines(ENCODED, "crc") {
        let input = unhex(l[0]);
        assert_eq!(crc16_ccitt(&input), u16::from_str_radix(l[1], 16).unwrap());
        assert_eq!(crc32_ieee(&input), u32::from_str_radix(l[2], 16).unwrap());
        seen += 1;
    }
    assert_eq!(seen, 5);
}

#[test]
fn cobs_vectors_match_cpp() {
    let mut seen = 0;
    for l in lines(ENCODED, "cobs") {
        let input = unhex(l[0]);
        let expected = unhex(l[1]);
        let mut out = vec![0u8; input.len() + input.len() / 254 + 2];
        let n = cobs_encode(&input, &mut out).unwrap();
        assert_eq!(out[..n], expected[..]);
        // L'inverse retrouve l'entrée.
        let mut back = vec![0u8; input.len() + 1];
        assert_eq!(cobs_decode(&expected, &mut back), Some(input.len()));
        assert_eq!(back[..input.len()], input[..]);
        seen += 1;
    }
    assert!(seen >= 30);
    for l in lines(ENCODED, "uncobs") {
        let input = unhex(l[0]);
        let want_len: usize = l[1].parse().unwrap();
        let want = unhex(l[2]);
        let mut out = vec![0u8; input.len() + 1];
        // Le C++ renvoie 0 pour un refus : `None` et `Some(0)` lui correspondent.
        let n = cobs_decode(&input, &mut out).unwrap_or(0);
        assert_eq!(n, want_len, "{}", l[0]);
        assert_eq!(out[..n], want[..], "{}", l[0]);
    }
}

#[test]
fn stream_vectors_match_cpp() {
    let mut seen = 0;
    for l in lines(ENCODED, "stream") {
        let input = unhex(l[0]);
        let dropped: u32 = l[1].parse().unwrap();
        let mut decoder = StreamDecoder::new();
        let got: Vec<String> = input
            .iter()
            .filter_map(|&b| decoder.push_byte(b))
            .map(|f| {
                let data: String = f.payload().iter().map(|b| format!("{b:02x}")).collect();
                format!(
                    "{:03x}:{}:{}",
                    f.id,
                    f.len,
                    if data.is_empty() { "-" } else { &data }
                )
            })
            .collect();
        assert_eq!(decoder.dropped_count, dropped, "{}", l[0]);
        assert_eq!(got, l[2..], "{}", l[0]);
        seen += 1;
    }
    assert_eq!(seen, 40);
}

#[test]
fn unpack_matches_cpp_on_arbitrary_input() {
    let (mut accepted, mut rejected) = (0, 0);
    for l in DIFFERENTIAL.lines().filter(|l| !l.starts_with('#')) {
        let f: Vec<&str> = l.split(' ').collect();
        let type_byte = u8::from_str_radix(f[0], 16).unwrap();
        let len: usize = f[1].parse().unwrap();
        let input = unhex(f[2]);
        assert_eq!(input.len(), len);
        // PING DLC 0 est le format ancien : le C++ le traite hors de unpack().
        if type_byte == 0x08 && len == 0 {
            continue;
        }
        let id = encode_can_id(CanId {
            message_type: MessageType::from_u8(type_byte).unwrap(),
            dest: Dest::Sensors,
            src: Node::Screen,
        });
        let frame = Frame::new(id, &input).unwrap();
        match (Message::decode(&frame), f[3]) {
            (Err(_), "REJECT") => rejected += 1,
            (Ok(message), repacked) if repacked != "REJECT" => {
                // `pack()` du C++ après `unpack()` : mêmes 8 octets.
                let again = message.encode();
                assert_eq!(again.data[..], unhex(repacked)[..], "{l}");
                accepted += 1;
            }
            (got, want) => panic!("{l}: Rust donne {got:?}, C++ donne {want}"),
        }
    }
    assert!(accepted > 1000 && rejected > 1000, "{accepted} {rejected}");
}
