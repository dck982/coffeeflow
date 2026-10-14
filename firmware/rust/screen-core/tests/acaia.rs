//! Octets Acaia et réassemblage des notifications, sans pile BLE.
use screen_core::acaia::*;

// Réponse de poids : longueur inclut le type, deux octets de checksum ignorés
// en réception, comme dans le firmware archivé.
fn notification(raw: u32, exponent: u8, sign: u8) -> Vec<u8> {
    let mut bytes = vec![0xEF, 0xDD, 12, 7, 5];
    bytes.extend(raw.to_le_bytes());
    bytes.extend([exponent, sign, 0]);
    bytes
}

#[test]
fn command_bytes_and_checksums_are_identical_to_cpp() {
    assert_eq!(tare_frame(), [0xEF, 0xDD, 0x04, 0, 0, 0]);
    let frames = subscribe_frames();
    assert_eq!(
        frames[0],
        [
            0xEF, 0xDD, 0x0B, 0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x30,
            0x31, 0x32, 0x33, 0x34, 0x9A, 0x6D
        ]
    );
    assert_eq!(frames[1], [0xEF, 0xDD, 0, 2, 0, 2, 0]);
    assert_eq!(
        frames[2],
        [0xEF, 0xDD, 12, 9, 0, 1, 1, 2, 2, 5, 3, 4, 21, 6]
    );
    assert_eq!(encode_frame(1, &[255; 59], false).unwrap().len(), 64);
    assert!(encode_frame(1, &[255; 60], false).is_none());
    assert_eq!(encode_frame(1, &[255; 58], true).unwrap().len(), 64);
    assert!(encode_frame(1, &[255; 59], true).is_none());
}

#[test]
fn fragmented_and_coalesced_frames_resynchronize_and_decode_sign() {
    let mut session = Session::new();
    session.set_active(true);
    let bytes = notification(12345, 2, 0);
    assert!(session.on_notification(&[0, 1, 0xEF], 1_000_000).is_empty());
    assert!(session.on_notification(&bytes[..5], 1_000_000).is_empty());
    assert_eq!(
        session.on_notification(&bytes[5..], 1_000_000),
        [RxEvent::Weight(123.45)]
    );
    let mut combined = notification(200, 1, 2);
    combined.extend(notification(10, 0, 0));
    assert_eq!(
        session.on_notification(&combined, 1_000_001),
        [RxEvent::Weight(-20.0), RxEvent::Weight(10.0)]
    );
    assert_eq!(
        session.on_notification(&[0xEF, 0xDD, 0x20], 1_000_002),
        [RxEvent::Disconnected]
    );
    assert!(session.on_notification(&bytes[5..], 1_000_003).is_empty());
}

#[test]
fn idle_publish_and_heartbeat_cadence_use_injected_time() {
    let mut session = Session::new();
    let bytes = notification(1, 4, 0);
    assert!(session.on_notification(&bytes, 999_999).is_empty());
    assert_eq!(
        session.on_notification(&bytes, 1_000_000),
        [RxEvent::Weight(0.0001)]
    );
    assert!(session.on_notification(&bytes, 1_999_999).is_empty());
    assert_eq!(
        session.on_notification(&bytes, 2_000_000),
        [RxEvent::Weight(0.0001)]
    );
    assert!(session.heartbeat(2_499_999, true).is_empty());
    assert_eq!(session.heartbeat(2_500_000, true), subscribe_frames());
    assert!(session.heartbeat(5_000_000, false).is_empty());
    assert!(session.heartbeat(5_000_001, true).is_empty());
    assert_eq!(session.heartbeat(7_500_000, true), subscribe_frames());
}

#[test]
fn malformed_frames_reset_buffer_and_oversized_notification_is_ignored() {
    let mut session = Session::new();
    session.set_active(true);
    for invalid in [[0xEF, 0xDD, 12, 0, 0, 0], [0xEF, 0xDD, 12, 124, 0, 0]] {
        assert!(session.on_notification(&invalid, 1_000_000).is_empty());
        assert_eq!(
            session.on_notification(&notification(100, 0, 0), 1_000_000),
            [RxEvent::Weight(100.0)]
        );
    }
    assert!(session.on_notification(&[0; 129], 1_000_000).is_empty());
    let mut unknown = vec![0xEF, 0xDD, 12, 1, 99, 0];
    unknown.extend(notification(100, 0, 0));
    assert_eq!(
        session.on_notification(&unknown, 1_000_000),
        [RxEvent::Weight(100.0)]
    );
}

#[test]
fn advertisements_match_services_or_case_insensitive_names() {
    assert!(is_acaia_advertisement(&Advertisement {
        uuids128_le: &[MODERN_SERVICE_UUID_LE],
        ..Advertisement::default()
    }));
    assert!(is_acaia_advertisement(&Advertisement {
        uuids16: &[LEGACY_SERVICE_UUID16],
        ..Advertisement::default()
    }));
    assert!(is_acaia_advertisement(&Advertisement {
        name: Some(b"acaial-123"),
        ..Advertisement::default()
    }));
    assert!(is_acaia_advertisement(&Advertisement {
        name: Some(b"my Lunar"),
        ..Advertisement::default()
    }));
    assert!(!is_acaia_advertisement(&Advertisement {
        name: Some(b"other scale"),
        ..Advertisement::default()
    }));
}
