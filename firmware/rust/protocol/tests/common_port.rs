//! Port des cas de `archive/firmware/common/test/test_common.cpp`.

use protocol::framing::{decode_pdu, encode_pdu};
use protocol::messages::*;
use protocol::scace;
use protocol::version::heating_protocol_supported;
use protocol::*;

fn id(message_type: MessageType, dest: Dest, src: Node) -> CanId {
    CanId {
        message_type,
        dest,
        src,
    }
}

#[test]
fn can_id_roundtrip() {
    let original = id(MessageType::Set, Dest::Sensors, Node::Screen);
    let raw = encode_can_id(original);
    assert!(raw <= 0x7FF);
    assert_eq!(decode_can_id(raw), Ok(original));
}

#[test]
fn can_id_priority_ordering() {
    // STOP (0x00) gagne l'arbitrage face à FLASH_DATA (0x39) : l'identifiant
    // numériquement le plus petit passe en premier.
    let stop = encode_can_id(id(MessageType::Stop, Dest::Sensors, Node::Screen));
    let flash = encode_can_id(id(MessageType::FlashData, Dest::Sensors, Node::Screen));
    assert!(stop < flash);
}

#[test]
fn can_id_bit_layout() {
    // type << 5 | dest << 3 | src
    let raw = encode_can_id(id(MessageType::Pong, Dest::Screen, Node::Sensors));
    assert_eq!(raw, (0x09 << 5) | (1 << 3) | 2);
    assert_eq!(split_can_id(raw), (0x09, 1, 2));
    // Les bits au-delà du 11e sont ignorés par le décodage.
    assert_eq!(decode_can_id(raw | 0x7800), decode_can_id(raw));
}

#[test]
fn can_id_unknown_fields_are_reported() {
    assert_eq!(
        decode_can_id(0x3F << 5 | 1 << 3 | 1),
        Err(CanIdError::UnknownType(0x3F))
    );
    assert_eq!(decode_can_id(3 << 3 | 1), Err(CanIdError::UnknownDest(3)));
    assert_eq!(decode_can_id(0x08 << 5), Err(CanIdError::UnknownSrc(0)));
    assert_eq!(decode_can_id(0x08 << 5 | 3), Err(CanIdError::UnknownSrc(3)));
}

#[test]
fn known_message_type() {
    assert!(is_known_message_type(MessageType::Ping as u8));
    assert!(!is_known_message_type(0x3F)); // valeur haute, jamais assignée
    for t in 0..=0xFFu8 {
        assert_eq!(is_known_message_type(t), MessageType::from_u8(t).is_some());
        if let Some(m) = MessageType::from_u8(t) {
            assert_eq!(m as u8, t);
        }
    }
}

#[test]
fn heating_protocol_version() {
    assert!(!heating_protocol_supported(0, 2, 62));
    assert!(heating_protocol_supported(0, 2, 63));
    assert!(heating_protocol_supported(0, 2, 69));
    assert!(heating_protocol_supported(0, 3, 0));
    assert!(!heating_protocol_supported(1, 0, 0));
    assert!(FIRMWARE_VERSION.heating_protocol_supported());
}

#[test]
fn set_payload_roundtrip() {
    let original = SetPayload {
        pump_pct: 42,
        ttl_ms: 500,
    };
    let f = original.pack();
    assert_eq!(f[0], 42);
    assert_eq!(f[1], 0xF4);
    assert_eq!(f[2], 0x01);
    assert_eq!(SetPayload::unpack(&f), Ok(original));
}

#[test]
fn heating_protocol_is_independent_and_rejects_invalid_frames() {
    assert!(is_known_message_type(MessageType::SetHeating as u8));
    assert!(is_known_message_type(MessageType::SetHeatingPower as u8));
    assert!(is_known_message_type(MessageType::StatusHeating as u8));

    let command = SetHeatingPayload {
        on: true,
        duration_ms: 1000,
    };
    let request = command.pack();
    assert_eq!(&request[..3], &[1, 0xE8, 0x03]);
    assert_eq!(SetHeatingPayload::unpack(&request[..3]), Ok(command));
    assert!(SetHeatingPayload::unpack(&request[..2]).is_err());
    let mut invalid = request;
    invalid[0] = 2;
    assert!(SetHeatingPayload::unpack(&invalid[..3]).is_err());

    let power = SetHeatingPowerPayload {
        power_permille: 6,
        lease_ms: 1500,
        restart_window: false,
    };
    let power_frame = power.pack();
    let decoded = SetHeatingPowerPayload::unpack(&power_frame[..4]).unwrap();
    assert_eq!((decoded.power_permille, decoded.lease_ms), (6, 1500));
    let mut invalid = power_frame;
    invalid[0] = 0xE9;
    invalid[1] = 0x03;
    assert!(SetHeatingPowerPayload::unpack(&invalid[..4]).is_err());
    assert_eq!(power.dlc(), 4);
    assert!(!decoded.restart_window);

    let restart = SetHeatingPowerPayload {
        power_permille: 900,
        lease_ms: 1500,
        restart_window: true,
    };
    let restart_frame = restart.pack();
    assert_eq!(restart.dlc(), 5);
    assert_eq!(restart_frame[4], 1);
    let decoded = SetHeatingPowerPayload::unpack(&restart_frame[..5]).unwrap();
    assert_eq!(decoded.power_permille, 900);
    assert!(decoded.restart_window);
    assert!(
        !SetHeatingPowerPayload::unpack(&restart_frame[..4])
            .unwrap()
            .restart_window
    );
    let mut invalid = restart_frame;
    invalid[4] = 0x02; // bit inconnu
    assert!(SetHeatingPowerPayload::unpack(&invalid[..5]).is_err());
    assert!(SetHeatingPowerPayload::unpack(&restart_frame[..6]).is_err());
    // Format de l'écran 0.2.68 : pourcents entiers sur 3 octets.
    let legacy = SetHeatingPowerPayload::unpack(&[1, 0xDC, 0x05]).unwrap();
    assert_eq!(legacy.power_permille, 10);
    assert_eq!(legacy.lease_ms, 1500);
    assert!(!legacy.restart_window);
    assert!(SetHeatingPowerPayload::unpack(&[101, 0xDC, 0x05]).is_err());

    let status = StatusHeatingPayload {
        heater_on: true,
        lease_remaining_ms: 750,
        ..Default::default()
    };
    let echo = status.pack();
    let decoded = StatusHeatingPayload::unpack(&echo[..4]).unwrap();
    assert!(decoded.heater_on);
    assert_eq!(decoded.lease_remaining_ms, 750);
    assert!(!decoded.power_capable);

    let status = StatusHeatingPayload {
        power_capable: true,
        fine_power_capable: true,
        power_permille: 6,
        ..status
    };
    let power_echo = status.pack();
    let decoded = StatusHeatingPayload::unpack(&power_echo[..6]).unwrap();
    assert!(decoded.power_capable && decoded.fine_power_capable);
    assert_eq!(decoded.power_permille, 6);
    assert!(!decoded.window_restart_capable);

    let status = StatusHeatingPayload {
        window_restart_capable: true,
        ..status
    };
    let restart_echo = status.pack();
    let decoded = StatusHeatingPayload::unpack(&restart_echo[..6]).unwrap();
    assert!(decoded.window_restart_capable);
    assert_eq!(decoded.power_permille, 6);
    // Sans résolution 0,1 %, la capacité de nouvelle période est ignorée.
    let mut basic = restart_echo;
    basic[3] = 1 | 8;
    assert!(
        !StatusHeatingPayload::unpack(&basic[..4])
            .unwrap()
            .window_restart_capable
    );
    assert!(StatusHeatingPayload::unpack(&echo[..3]).is_err());
    let mut invalid = echo;
    invalid[3] = 0;
    assert!(StatusHeatingPayload::unpack(&invalid[..4]).is_err());
}

#[test]
fn pong_payload_roundtrip() {
    let original = NodeIdentityPayload::new(Node::Sensors, Version::new(1, 2, 3), 123_456_789);
    let f = original.pack();
    assert_eq!(NodeIdentityPayload::unpack(&f), Ok(original));
    assert_eq!(original.node(), Some(Node::Sensors));
}

#[test]
fn reqstatus_payload_roundtrip() {
    let original = ReqStatusPayload::new(MessageType::StatusFlow, 200);
    let f = original.pack();
    let decoded = ReqStatusPayload::unpack(&f).unwrap();
    assert_eq!(decoded, original);
    assert_eq!(decoded.target(), Some(MessageType::StatusFlow));
}

#[test]
fn status_pressure_payload_roundtrip() {
    let original = StatusPressurePayload {
        pressure_raw: 0x00AB_CDEF & 0xFF_FFFF,
        temperature_raw: 0x1234,
        timestamp_ms: 60000,
        flags: 0b01,
    };
    assert_eq!(
        StatusPressurePayload::unpack(&original.pack()),
        Ok(original)
    );
}

#[test]
fn status_flow_payload_roundtrip() {
    let original = StatusFlowPayload {
        pulse_count: 4_000_000_000,
        last_edge_ms: 1234,
        flags: 1,
    };
    assert_eq!(StatusFlowPayload::unpack(&original.pack()), Ok(original));
}

#[test]
fn status_actuators_payload_roundtrip() {
    let original = StatusActuatorsPayload {
        valve_open: true,
        pump_pct: 77,
        lease_remaining_ms: 480,
        continuous_on_ms: 59000,
        flags: 0b101,
    };
    assert_eq!(
        StatusActuatorsPayload::unpack(&original.pack()),
        Ok(original)
    );
}

#[test]
fn maintenance_valve_payload() {
    assert!(is_known_message_type(MessageType::MaintenanceValve as u8));
    let open = MaintenanceValvePayload {
        open: true,
        duration_ms: MAINTENANCE_VALVE_MAX_MS,
    };
    let f = open.pack();
    let decoded = MaintenanceValvePayload::unpack(&f[..3]).unwrap();
    assert!(decoded.open);
    assert_eq!(decoded.duration_ms, MAINTENANCE_VALVE_MAX_MS);
    assert!(MaintenanceValvePayload::unpack(&f[..2]).is_err());

    // Ouverture sans durée ou au-delà du plafond : refusée, pas bornée en silence.
    let zero = MaintenanceValvePayload {
        open: true,
        duration_ms: 0,
    }
    .pack();
    assert!(MaintenanceValvePayload::unpack(&zero[..3]).is_err());
    let too_long = MaintenanceValvePayload {
        open: true,
        duration_ms: MAINTENANCE_VALVE_MAX_MS + 1,
    }
    .pack();
    assert!(MaintenanceValvePayload::unpack(&too_long[..3]).is_err());
    let mut invalid = f;
    invalid[0] = 2;
    assert!(MaintenanceValvePayload::unpack(&invalid[..3]).is_err());

    let close = MaintenanceValvePayload {
        open: false,
        duration_ms: 1234,
    }
    .pack();
    let decoded = MaintenanceValvePayload::unpack(&close[..3]).unwrap();
    assert!(!decoded.open);
    assert_eq!(decoded.duration_ms, 0);
}

#[test]
fn log_payload_roundtrip() {
    let original = LogPayload {
        code: LogCode::RuntimeLockoutTriggered as u8,
        severity: LogSeverity::Error as u8,
        arg16: 0,
        arg32: 60000,
    };
    let decoded = LogPayload::unpack(&original.pack()).unwrap();
    assert_eq!(decoded, original);
    assert_eq!(log_code_name(decoded.code), "RUNTIME_LOCKOUT_TRIGGERED");
}

#[test]
fn flash_ctrl_payload_roundtrip() {
    let begin = FlashCtrlPayload::Begin {
        image_size: 1_048_576,
    };
    assert_eq!(FlashCtrlPayload::unpack(&begin.pack()), Ok(begin));

    let ack = FlashCtrlPayload::BlockAck {
        block_number: 42,
        block_crc16: 0xBEEF,
    };
    assert_eq!(FlashCtrlPayload::unpack(&ack.pack()), Ok(ack));

    let start = FlashCtrlPayload::BlockStart {
        block_number: 7,
        block_crc16: 0xA55A,
    };
    let f = start.pack();
    assert_eq!(FlashCtrlPayload::unpack(&f[..5]), Ok(start));
    assert!(FlashCtrlPayload::unpack(&f[..4]).is_err());

    let end = FlashCtrlPayload::End {
        image_crc32: 0xDEAD_BEEF,
    };
    assert_eq!(FlashCtrlPayload::unpack(&end.pack()), Ok(end));
    assert_eq!(FlashCtrlPayload::unpack(&[3]), Ok(FlashCtrlPayload::Abort));
    assert!(FlashCtrlPayload::unpack(&[5, 0, 0, 0, 0]).is_err());
    assert!(FlashCtrlPayload::unpack(&[]).is_err());
}

#[test]
fn framing_pdu_roundtrip() {
    let mut data = [0u8; 8];
    for (i, b) in data.iter_mut().enumerate() {
        *b = 0x10 + i as u8;
    }
    let original = Frame {
        id: encode_can_id(id(MessageType::Pong, Dest::Screen, Node::Sensors)),
        len: 8,
        data,
    };
    let mut pdu = [0u8; MAX_PDU_SIZE];
    let pdu_len = encode_pdu(&original, &mut pdu).unwrap();
    assert_eq!(pdu_len, 3 + 8 + 2);
    assert_eq!(decode_pdu(&pdu[..pdu_len]), Ok(original));
}

#[test]
fn framing_pdu_rejects_bad_crc() {
    let original = Frame {
        id: encode_can_id(id(MessageType::Ping, Dest::Broadcast, Node::Screen)),
        len: 0,
        data: [0; 8],
    };
    let mut pdu = [0u8; MAX_PDU_SIZE];
    let pdu_len = encode_pdu(&original, &mut pdu).unwrap();
    pdu[pdu_len - 1] ^= 0xFF;
    assert_eq!(decode_pdu(&pdu[..pdu_len]), Err(FramingError::Crc));
}

#[test]
fn framing_pdu_rejects_bad_shapes() {
    let ok = Frame {
        id: 0x7FF,
        len: 2,
        data: [1, 2, 0, 0, 0, 0, 0, 0],
    };
    let mut pdu = [0u8; MAX_PDU_SIZE];
    let n = encode_pdu(&ok, &mut pdu).unwrap();
    assert_eq!(decode_pdu(&pdu[..n - 1]), Err(FramingError::Length));
    assert_eq!(decode_pdu(&pdu[..4]), Err(FramingError::Length));
    assert_eq!(decode_pdu(&[]), Err(FramingError::Length));
    let mut too_long = [0u8; MAX_PDU_SIZE + 1];
    too_long[..n].copy_from_slice(&pdu[..n]);
    assert_eq!(decode_pdu(&too_long[..n + 1]), Err(FramingError::Length));
    // Identifiant au-delà de 11 bits, avec un CRC cohérent.
    let mut wide = [0u8; 5];
    wide[0] = 0x00;
    wide[1] = 0x08;
    let crc = crc16_ccitt(&wide[..3]);
    wide[3] = crc as u8;
    wide[4] = (crc >> 8) as u8;
    assert_eq!(decode_pdu(&wide), Err(FramingError::Id));
    // Et l'encodage refuse un gabarit hors norme.
    let big_id = Frame { id: 0x800, ..ok };
    assert_eq!(encode_pdu(&big_id, &mut pdu), Err(FramingError::Id));
    let big_len = Frame { len: 9, ..ok };
    assert_eq!(encode_pdu(&big_len, &mut pdu), Err(FramingError::Length));
    let mut out = [0u8; MAX_FRAMED_SIZE];
    assert_eq!(encode_framed(&big_id, &mut out), None);
    assert_eq!(encode_framed(&big_len, &mut out), None);
}

#[test]
fn cobs_roundtrip_with_zeros() {
    // Un bloc FLASH_DATA plein de zéros est le cas que COBS doit absorber.
    let mut data = [0u8; MAX_PDU_SIZE];
    data[1] = 0x05; // un seul octet non nul au milieu

    let mut encoded = [0u8; MAX_COBS_SIZE];
    let encoded_len = cobs_encode(&data, &mut encoded).unwrap();
    assert!(encoded[..encoded_len].iter().all(|&b| b != 0));

    let mut decoded = [0u8; MAX_PDU_SIZE];
    let decoded_len = cobs_decode(&encoded[..encoded_len], &mut decoded).unwrap();
    assert_eq!(&decoded[..decoded_len], &data);
}

#[test]
fn cobs_buffers_too_small_are_refused() {
    let mut tiny = [0u8; 2];
    assert_eq!(cobs_encode(&[1, 2, 3], &mut tiny), None);
    assert_eq!(cobs_encode(&[1], &mut []), None);
    assert_eq!(cobs_decode(&[4, 1, 2, 3], &mut tiny), None);
    assert_eq!(cobs_decode(&[0x02, 0x01, 0x00], &mut [0u8; 8]), None);
    assert_eq!(cobs_decode(&[0x05, 1, 2], &mut [0u8; 8]), None);
}

#[test]
fn encode_framed_roundtrip_via_stream_decoder() {
    let original = Frame {
        id: encode_can_id(id(MessageType::Set, Dest::Sensors, Node::Screen)),
        len: 5,
        data: [0x03, 0x01, 42, 0xF4, 0x01, 0, 0, 0],
    };
    let mut framed = [0u8; MAX_FRAMED_SIZE];
    let framed_len = encode_framed(&original, &mut framed).unwrap();
    assert_eq!(framed[framed_len - 1], 0x00);

    let mut decoder = StreamDecoder::new();
    let mut received = None;
    let mut count = 0;
    for &b in &framed[..framed_len] {
        if let Some(frame) = decoder.push_byte(b) {
            received = Some(frame);
            count += 1;
        }
    }
    assert_eq!(count, 1);
    assert_eq!(received, Some(original));
    assert_eq!(decoder.dropped_count, 0);
}

#[test]
fn stream_decoder_resyncs_after_garbage() {
    // Un octet perdu (ou un branchement en cours de trame) ne doit coûter que
    // la trame en cours, pas bloquer les suivantes.
    let ping = Frame {
        id: encode_can_id(id(MessageType::Ping, Dest::Broadcast, Node::Sensors)),
        len: 0,
        data: [0; 8],
    };
    let mut framed = [0u8; MAX_FRAMED_SIZE];
    let framed_len = encode_framed(&ping, &mut framed).unwrap();

    let mut decoder = StreamDecoder::new();
    let mut received = 0;
    for b in [0x7A, 0x11, 0x00] {
        // trame invalide, close par 0x00
        if decoder.push_byte(b).is_some() {
            received += 1;
        }
    }
    assert_eq!(received, 0);
    assert_eq!(decoder.dropped_count, 1);

    for &b in &framed[..framed_len] {
        if decoder.push_byte(b) == Some(ping) {
            received += 1;
        }
    }
    assert_eq!(received, 1);
}

#[test]
fn stream_decoder_drops_oversized_runs_and_ignores_double_zero() {
    let mut decoder = StreamDecoder::new();
    for _ in 0..MAX_COBS_SIZE {
        assert!(decoder.push_byte(0x11).is_none());
    }
    assert_eq!(decoder.dropped_count, 0);
    // Un octet de plus que le gabarit : trame abandonnée.
    assert!(decoder.push_byte(0x11).is_none());
    assert_eq!(decoder.dropped_count, 1);
    // Des 0x00 à vide ne comptent pour rien.
    assert!(decoder.push_byte(0).is_none());
    assert!(decoder.push_byte(0).is_none());
    assert_eq!(decoder.dropped_count, 1);
}

#[test]
fn crc16_known_vector() {
    // "123456789" -> 0x29B1 en CRC-16/CCITT-FALSE (poly 0x1021, init 0xFFFF).
    assert_eq!(crc16_ccitt(b"123456789"), 0x29B1);
}

#[test]
fn crc32_known_vector() {
    // "123456789" -> 0xCBF43926 en CRC-32 (poly 0xEDB88320, IEEE 802.3).
    assert_eq!(crc32_ieee(b"123456789"), 0xCBF4_3926);
}

#[test]
fn crc32_incremental_matches_one_shot() {
    let data: [u8; 300] = core::array::from_fn(|i| (i * 31 + 7) as u8);
    for split in [0, 1, 7, 8, 150, 299, 300] {
        let mut crc = Crc32Incremental::new();
        crc.update(&data[..split]);
        crc.update(&data[split..]);
        assert_eq!(crc.finish(), crc32_ieee(&data));
    }
    assert_eq!(Crc32Incremental::default().finish(), 0);
}

fn uuid_matches(text: &str, le: &[u8; 16]) -> bool {
    let nibbles: Vec<u8> = text
        .chars()
        .filter(|&c| c != '-')
        .map(|c| c.to_digit(16).unwrap() as u8)
        .collect();
    if nibbles.len() != 32 {
        return false;
    }
    let be: Vec<u8> = nibbles.chunks(2).map(|p| (p[0] << 4) | p[1]).collect();
    be.iter().zip(le.iter().rev()).all(|(a, b)| a == b)
}

#[test]
fn scace_uuids_match_text() {
    // Les tableaux petit-boutistes donnés à la pile BLE doivent rester l'image
    // exacte des UUID texte, que lit le script Mac.
    assert!(uuid_matches(scace::SERVICE_UUID, &scace::SERVICE_UUID_LE));
    assert!(uuid_matches(scace::FRAME_UUID, &scace::FRAME_UUID_LE));
    assert_eq!(scace::DEVICE_NAME, "SCACE");
}

#[test]
fn scace_frame_roundtrip() {
    let original = scace::Frame {
        seq: 0xfffe,
        ms: 0x1234,
        a0: 26416,
        a1: -3,
        centi_c: 2503,
        status: scace::Status::Ok,
    };
    let bytes = original.encode();
    assert_eq!(&bytes[0..2], &[0xfe, 0xff]);
    assert_eq!(&bytes[8..10], &[0xc7, 0x09]);
    assert_eq!(&bytes[10..12], &[1, 0]);
    assert_eq!(scace::Frame::decode(&bytes), Some(original));
}

#[test]
fn scace_frame_rejects_short_and_unknown_status() {
    let mut bytes = scace::Frame::default().encode();
    assert_eq!(scace::Frame::default().centi_c, scace::NO_TEMPERATURE);
    assert_eq!(scace::Frame::decode(&bytes[..scace::FRAME_SIZE - 1]), None);
    bytes[10] = 4;
    assert_eq!(scace::Frame::decode(&bytes), None);
}

#[test]
fn scace_frame_ignores_extra_bytes() {
    let original = scace::Frame {
        seq: 3,
        ..Default::default()
    };
    let mut longer = [0xAAu8; 20];
    longer[..scace::FRAME_SIZE].copy_from_slice(&original.encode());
    assert_eq!(scace::Frame::decode(&longer), Some(original));
}
