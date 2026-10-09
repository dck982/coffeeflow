use screen_net::{
    credentials::Credentials,
    http::{Error, Request},
    sntp, websocket,
};

#[test]
fn credentials_use_cpp_blob_and_decode_the_browser_form() {
    let c = Credentials::from_form(b"ssid=Caf%C3%A9+David&password=a%26b%3Dc%25de").unwrap();
    assert_eq!(c.ssid, "Café David");
    assert_eq!(c.password, "a&b=c%de");
    let blob = c.to_blob();
    assert_eq!(blob.len(), 97);
    assert_eq!(&blob[33..41], b"a&b=c%de");
    assert!(Credentials::from_blob(&blob).unwrap() == c);
    assert!(Credentials::from_form(b"ssid=a&password=short").is_none());
    assert!(Credentials::from_form(b"ssid=a%00b&password=12345678").is_none());
    assert!(Credentials::from_form(b"ssid=a&ssid=b&password=12345678").is_none());
    assert!(Credentials::from_form(b"ssid=%XX&password=12345678").is_none());
    assert!(Credentials::from_blob(&[b'a'; 97]).is_none());
}

#[test]
fn http_all_fragment_boundaries_and_first_body_bytes() {
    let head = b"POST /firmware?target=sensors HTTP/1.1\r\ncontent-length: 1048576\r\nAuthorization: Bearer token\r\n\r\n";
    for i in 0..head.len() {
        assert_eq!(Request::parse(&head[..i]).unwrap_err(), Error::Incomplete);
    }
    let mut combined = head.to_vec();
    combined.extend_from_slice(&[0xe9, 0, 0xff]);
    let request = Request::parse(&combined).unwrap();
    assert_eq!(request.path, "/firmware");
    assert_eq!(request.query, Some("target=sensors"));
    assert_eq!(request.content_length, 1048576);
    assert_eq!(request.authorization, Some("Bearer token"));
    assert_eq!(&combined[request.header_bytes..], &[0xe9, 0, 0xff]);
}

#[test]
fn http_ambiguous_or_unbounded_framing_is_rejected() {
    for bad in [
        "Content-Length: 3\r\nContent-Length: 3\r\n",
        "Content-Length: -1\r\n",
        "Content-Length: +2\r\n",
        "Content-Length: 999999999999999999999999999\r\n",
        "Authorization: Bearer one\r\nAuthorization: Bearer two\r\n",
        " malformed: header\r\n",
    ] {
        let text = format!("POST /config HTTP/1.1\r\n{bad}\r\n");
        assert_eq!(Request::parse(text.as_bytes()).unwrap_err(), Error::Invalid);
    }
    assert_eq!(
        Request::parse(b"POST /config HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n").unwrap_err(),
        Error::UnsupportedTransfer
    );
    assert_eq!(
        Request::parse(&vec![b'x'; 4096]).unwrap_err(),
        Error::TooLarge
    );
}

#[test]
fn websocket_handshake_matches_rfc6455_and_frames_can_pdus() {
    let request = Request::parse(b"GET /ws HTTP/1.1\r\nUpgrade: WebSocket\r\nConnection: keep-alive, Upgrade\r\nSec-WebSocket-Version: 13\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\r\n").unwrap();
    assert!(request.websocket_upgrade);
    assert_eq!(
        websocket::accept_key(request.websocket_key.unwrap()).unwrap(),
        "s3pPLMBiTxaQ9kYGzzhZRbK+xOo="
    );
    assert!(websocket::accept_key("bad").is_none());
    for (len, expected) in [
        (8, vec![0x82, 8]),
        (126, vec![0x82, 126, 0, 126]),
        (65536, vec![0x82, 127, 0, 0, 0, 0, 0, 1, 0, 0]),
    ] {
        let (header, bytes) = websocket::binary_header(len);
        assert_eq!(&header[..bytes], expected);
    }
}

#[test]
fn sntp_checks_origin_sync_stratum_and_the_2036_rollover() {
    let cookie = 0x1122334455667788u64.to_be_bytes();
    let request = sntp::request(cookie);
    assert_eq!(request[0], 0x23);
    assert_eq!(&request[40..], &cookie);
    let mut response = [0; 48];
    response[0] = 0x24;
    response[1] = 2;
    response[24..32].copy_from_slice(&cookie);
    response[40..44].copy_from_slice(&3_999_000_000u32.to_be_bytes());
    assert_eq!(sntp::response(&response, cookie), Some(1_790_011_200));
    assert!(sntp::response(&response, [0; 8]).is_none());
    response[0] |= 0xc0;
    assert!(sntp::response(&response, cookie).is_none());
    response[0] = 0x24;
    response[1] = 0;
    assert!(sntp::response(&response, cookie).is_none());
    response[1] = 2;
    response[40..44].copy_from_slice(&100u32.to_be_bytes());
    assert_eq!(sntp::response(&response, cookie), Some(2_085_978_596));
}

#[test]
fn ota_image_validation_checks_structure_chip_checksum_and_readback_digest() {
    use sha2::{Digest, Sha256};
    let mut image = vec![0u8; 48];
    image[0] = 0xe9;
    image[1] = 1;
    image[12] = 9;
    image[23] = 1;
    image[28..32].copy_from_slice(&8u32.to_le_bytes());
    image[32..40].copy_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);
    image[47] = 0xef ^ image[32..40].iter().fold(0, |a, b| a ^ b);
    image.extend_from_slice(&Sha256::digest(&image));
    let validate = |image: &[u8]| {
        screen_net::image::verify(image.len() as u32, |at, bytes| {
            let data = image
                .get(at as usize..at as usize + bytes.len())
                .ok_or("bounds")?;
            bytes.copy_from_slice(data);
            Ok(())
        })
    };
    assert!(validate(&image).is_ok());
    for at in [0, 1, 12, 23, 28, 32, 47, 79] {
        let mut corrupt = image.clone();
        corrupt[at] ^= 1;
        assert!(validate(&corrupt).is_err(), "{at}");
    }
    assert!(validate(&image[..79]).is_err());
    let mut huge = image.clone();
    huge[28..32].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(validate(&huge).is_err());
}

#[test]
fn websocket_clients_are_masked_bounded_and_ping_payload_is_preserved() {
    use screen_net::websocket::{ClientError, ClientFrame};
    let bytes = [0x89, 0x83, 1, 2, 3, 4, b'a' ^ 1, b'b' ^ 2, b'c' ^ 3];
    for i in 0..bytes.len() {
        assert_eq!(
            ClientFrame::parse(&bytes[..i]).err(),
            Some(ClientError::Incomplete)
        );
    }
    let frame = ClientFrame::parse(&bytes).unwrap();
    assert_eq!(frame.opcode, 9);
    assert_eq!(&frame.payload[..frame.length], b"abc");
    assert_eq!(frame.consumed, 9);
    assert_eq!(
        ClientFrame::parse(&[0x82, 3]).err(),
        Some(ClientError::Invalid)
    );
    assert_eq!(
        ClientFrame::parse(&[0x82, 0x80 | 14]).err(),
        Some(ClientError::Invalid)
    );
    assert_eq!(
        ClientFrame::parse(&[0x89, 0x80 | 126]).err(),
        Some(ClientError::Invalid)
    );
}
