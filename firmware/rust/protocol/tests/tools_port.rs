//! Port des vecteurs de `archive/firmware/tools/test/` (test_crc, test_framing,
//! test_messages, test_protocol, et la partie table de codes de test_decoder)
//! et cas propres à l'enum `Message`.

use protocol::*;

fn frame(message_type: MessageType, dest: Dest, src: Node, payload: &[u8]) -> Frame {
    let id = encode_can_id(CanId {
        message_type,
        dest,
        src,
    });
    Frame::new(id, payload).unwrap()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn framed_hex(f: &Frame) -> String {
    let mut out = [0u8; MAX_FRAMED_SIZE];
    let n = encode_framed(f, &mut out).unwrap();
    hex(&out[..n])
}

#[test]
fn tools_crc_vectors() {
    assert_eq!(crc16_ccitt(b"123456789"), 0x29B1);
    assert_eq!(crc32_ieee(b"123456789"), 0xCBF4_3926);
}

#[test]
fn tools_golden_vectors_from_cpp() {
    // Mêmes deux trames que test_framing.py, produites par encode_framed() C++.
    let ping = frame(MessageType::Ping, Dest::Broadcast, Node::Screen, &[]);
    assert_eq!(framed_hex(&ping), "030101039dc800");

    let set = frame(
        MessageType::Set,
        Dest::Sensors,
        Node::Screen,
        &[0x03, 0x01, 42, 0xF4, 0x01],
    );
    assert_eq!(framed_hex(&set), "0231090503012af401756500");
}

#[test]
fn tools_pdu_and_stream_cases() {
    let pong = Frame::new(
        encode_can_id(CanId {
            message_type: MessageType::Pong,
            dest: Dest::Screen,
            src: Node::Sensors,
        }),
        &[0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17],
    )
    .unwrap();
    let mut pdu = [0u8; MAX_PDU_SIZE];
    let n = encode_pdu(&pong, &mut pdu).unwrap();
    assert_eq!(n, 3 + 8 + 2);
    assert_eq!(decode_pdu(&pdu[..n]), Ok(pong));

    // COBS : 13 octets dont un seul non nul.
    let mut data = [0u8; 13];
    data[1] = 0x05;
    let mut enc = [0u8; MAX_COBS_SIZE];
    let len = cobs_encode(&data, &mut enc).unwrap();
    assert!(!enc[..len].contains(&0));
    let mut dec = [0u8; 13];
    assert_eq!(cobs_decode(&enc[..len], &mut dec), Some(13));
    assert_eq!(dec, data);

    // Resynchronisation après du bruit, comme test_framing.py.
    let mut decoder = StreamDecoder::new();
    for b in [0x7A, 0x11, 0x00] {
        assert_eq!(decoder.push_byte(b), None);
    }
    assert_eq!(decoder.dropped_count, 1);
}

#[test]
fn tools_log_code_table_matches_generated_indices() {
    // Les numéros que test_decoder.py suppose pour la table générée.
    assert_eq!(LogCode::RuntimeLockoutTriggered as u8, 6);
    assert_eq!(LogCode::OtaPendingVerify as u8, 20);
    assert_eq!(LogCode::from_u8(40), Some(LogCode::BoilerAdcNotFound));
    assert_eq!(LogCode::from_u8(41), Some(LogCode::BoilerAdcI2cError));
    assert_eq!(
        LogCode::from_u8(42),
        Some(LogCode::BoilerAdcConversionTimeout)
    );
    assert_eq!(LogCode::from_u8(43), Some(LogCode::BoilerNtcInvalidReading));
    assert_eq!(LogCode::from_u8(44), Some(LogCode::BoilerAdcRecovered));
    assert_eq!(log_code_name(40), "BOILER_ADC_NOT_FOUND");
    assert_eq!(log_code_name(200), "UNKNOWN");
}

#[test]
fn log_table_is_consistent() {
    assert_eq!(LOG_CODES.len(), log_codes::LOG_CODE_COUNT);
    for (i, info) in LOG_CODES.iter().enumerate() {
        assert_eq!(info.code as usize, i);
        assert_eq!(LogCode::from_u8(i as u8), Some(info.code));
        assert_eq!(info.code.name(), info.name);
        assert_eq!(info.code.severity(), info.severity);
        assert!(!info.name.is_empty() && !info.description.is_empty());
        assert!(
            info.name
                .chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
        );
    }
    assert_eq!(LogCode::from_u8(LOG_CODES.len() as u8), None);
    assert_eq!(
        LogCode::RuntimeLockoutTriggered.severity(),
        LogSeverity::Error
    );
    assert_eq!(LogCode::Boot.severity(), LogSeverity::Info);
    assert_eq!(LogCode::DimmerMainsFreq.severity(), LogSeverity::Debug);
    assert_eq!(LogCode::LeaseExpired.severity(), LogSeverity::Warn);
    assert!(LogCode::OtaPendingVerify.description().contains("arg32"));
    for (i, name) in ["debug", "info", "warn", "error"].iter().enumerate() {
        let severity = LogSeverity::from_u8(i as u8).unwrap();
        assert_eq!(severity.name(), *name);
        assert_eq!(severity as usize, i);
    }
    assert_eq!(LogSeverity::from_u8(4), None);
    assert!(LogSeverity::Debug < LogSeverity::Error);
}

#[test]
fn message_set_encodes_like_the_firmware() {
    let m = Message::Set(messages::SetPayload {
        pump_pct: 42,
        ttl_ms: 500,
    });
    let f = m.encode();
    assert_eq!(f.id, 0x031); // SET, vers capteurs, depuis l'écran
    assert_eq!(f.len, 3);
    assert_eq!(f.payload(), &[42, 0xF4, 0x01]);
    assert_eq!(f.data[3..], [0; 5]);
    assert_eq!(Message::decode(&f), Ok(m));
}

#[test]
fn message_dlc_and_routes() {
    use messages::*;
    let identity = NodeIdentityPayload::new(Node::Sensors, Version::new(0, 4, 0), 7);
    let cases: [(Message, u8, u16); 8] = [
        (Message::Stop, 0, 0x011),
        (Message::Reset, 0, 0x051),
        (Message::PingLegacy, 0, 0x101),
        (Message::Ping(identity), 8, 0x10a),
        (Message::Pong(identity), 8, 0x12a),
        (
            Message::SetHeatingPower(SetHeatingPowerPayload {
                power_permille: 10,
                lease_ms: 1500,
                restart_window: true,
            }),
            5,
            0x0d1,
        ),
        (
            Message::FlashCtrl(FlashCtrlPayload::BlockStart {
                block_number: 1,
                block_crc16: 2,
            }),
            5,
            0x711,
        ),
        (
            Message::StatusHeating(StatusHeatingPayload::default()),
            6,
            0x46a,
        ),
    ];
    for (message, dlc, id) in cases {
        let f = message.encode();
        assert_eq!((f.len, f.id), (dlc, id), "{message:?}");
        assert_eq!(message.dlc(), dlc);
    }
    // Le LOG de l'écran se route explicitement.
    let log = Message::Log(LogPayload::default());
    assert_eq!(
        log.encode().id,
        encode_can_id(CanId {
            message_type: MessageType::Log,
            dest: Dest::Broadcast,
            src: Node::Sensors
        })
    );
    let from_screen = log.encode_routed(Node::Screen, Dest::Broadcast);
    assert_eq!(from_screen.can_id().unwrap().src, Node::Screen);
}

#[test]
fn message_decode_errors() {
    // Type inconnu (0x3F) : trame de présence, mais pas décodable.
    let unknown = Frame {
        id: 0x3F << 5 | 1 << 3 | 2,
        len: 8,
        data: [0xFF; 8],
    };
    assert_eq!(
        Message::decode(&unknown),
        Err(DecodeError::UnknownType(0x3F))
    );
    assert_eq!(unknown.can_id(), Err(CanIdError::UnknownType(0x3F)));
    // Identifiant au-delà de 11 bits, DLC au-delà de 8.
    assert_eq!(
        Message::decode(&Frame {
            id: 0x800,
            ..Frame::default()
        }),
        Err(DecodeError::InvalidId)
    );
    assert_eq!(
        Message::decode(&Frame {
            len: 9,
            ..frame(MessageType::Stop, Dest::Sensors, Node::Screen, &[])
        }),
        Err(DecodeError::BadLength)
    );
    // DLC trop court.
    let short = frame(MessageType::Set, Dest::Sensors, Node::Screen, &[1, 2]);
    assert_eq!(Message::decode(&short), Err(DecodeError::BadLength));
    // Valeur hors domaine.
    let bad = frame(
        MessageType::SetHeating,
        Dest::Sensors,
        Node::Screen,
        &[2, 0, 0],
    );
    assert_eq!(Message::decode(&bad), Err(DecodeError::InvalidValue));
    assert!(Frame::new(0, &[0; 9]).is_none());
}

#[test]
fn message_ping_formats() {
    // PING DLC 0 d'une image ancienne reste accepté ; DLC 1..7 ne l'est pas.
    let legacy = frame(MessageType::Ping, Dest::Broadcast, Node::Screen, &[]);
    assert_eq!(Message::decode(&legacy), Ok(Message::PingLegacy));
    let partial = frame(MessageType::Ping, Dest::Broadcast, Node::Screen, &[1, 0, 4]);
    assert_eq!(Message::decode(&partial), Err(DecodeError::BadLength));
    let pong_empty = frame(MessageType::Pong, Dest::Screen, Node::Sensors, &[]);
    assert_eq!(Message::decode(&pong_empty), Err(DecodeError::BadLength));
    // Un nœud inconnu reste lisible tel quel (c'est au récepteur de trancher).
    let odd = frame(
        MessageType::Pong,
        Dest::Screen,
        Node::Sensors,
        &[7, 0, 4, 0, 1, 0, 0, 0],
    );
    let Ok(Message::Pong(identity)) = Message::decode(&odd) else {
        panic!("PONG attendu");
    };
    assert_eq!(identity.node, 7);
    assert_eq!(identity.node(), None);
}

#[test]
fn message_stop_and_reset_ignore_payload() {
    for ty in [MessageType::Stop, MessageType::Reset] {
        for len in 0..=8 {
            let f = Frame::new(
                encode_can_id(CanId {
                    message_type: ty,
                    dest: Dest::Sensors,
                    src: Node::Screen,
                }),
                &[0xAA; 8][..len],
            )
            .unwrap();
            assert!(matches!(
                Message::decode(&f),
                Ok(Message::Stop | Message::Reset)
            ));
        }
    }
}

#[test]
fn message_flash_data_lengths() {
    for len in 1..=8usize {
        let bytes: Vec<u8> = (1..=len as u8).collect();
        let f = frame(MessageType::FlashData, Dest::Sensors, Node::Screen, &bytes);
        let Ok(Message::FlashData(d)) = Message::decode(&f) else {
            panic!("FLASH_DATA attendu");
        };
        assert_eq!(d.bytes(), &bytes[..]);
        assert_eq!(Message::FlashData(d).encode(), f);
    }
    let empty = frame(MessageType::FlashData, Dest::Sensors, Node::Screen, &[]);
    assert_eq!(Message::decode(&empty), Err(DecodeError::BadLength));
}

#[test]
fn message_confirm_ota_token() {
    let m = Message::ConfirmSensorsOta(Default::default());
    let f = m.encode();
    assert_eq!((f.id, f.len, f.data[0]), (0x0b1, 1, 63));
    assert_eq!(Message::decode(&f), Ok(m));
    // Python : CONFIRM_SENSORS_OTA décodé depuis un seul octet 0x3f.
    let python = Frame::new(0x0B1, &[0x3F]).unwrap();
    assert_eq!(Message::decode(&python), Ok(m));
}

#[test]
fn message_status_heating_python_cases() {
    // test_decoder.py : STATUS_HEATING en 4 octets, REQSTATUS vers STATUS_HEATING.
    let f = Frame::new(0x46A, &[0, 0, 0, 1]).unwrap();
    let Ok(Message::StatusHeating(p)) = Message::decode(&f) else {
        panic!("STATUS_HEATING attendu");
    };
    assert!(!p.heater_on && !p.power_capable && !p.fine_power_capable);
    let req = Frame::new(0x211, &[0x23, 0xFA, 0x00]).unwrap();
    let Ok(Message::ReqStatus(r)) = Message::decode(&req) else {
        panic!("REQSTATUS attendu");
    };
    assert_eq!(r.target(), Some(MessageType::StatusHeating));
    assert_eq!(r.period_ms, 250);
}

#[cfg(feature = "serde")]
#[test]
fn serde_derives_compile() {
    fn assert_serde<T: serde::Serialize + for<'de> serde::Deserialize<'de>>() {}
    assert_serde::<Message>();
    assert_serde::<Frame>();
    assert_serde::<scace::Frame>();
    assert_serde::<LogCode>();
}
