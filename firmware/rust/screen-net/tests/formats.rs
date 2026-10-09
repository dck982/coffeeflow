use screen_net::{credentials::Credentials, http::{Request, Error}, sntp, websocket};

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
        "Content-Length: -1\r\n", "Content-Length: +2\r\n",
        "Content-Length: 999999999999999999999999999\r\n",
        "Authorization: Bearer one\r\nAuthorization: Bearer two\r\n",
        " malformed: header\r\n",
    ] {
        let text = format!("POST /config HTTP/1.1\r\n{bad}\r\n");
        assert_eq!(Request::parse(text.as_bytes()).unwrap_err(), Error::Invalid);
    }
    assert_eq!(Request::parse(b"POST /config HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n").unwrap_err(), Error::UnsupportedTransfer);
    assert_eq!(Request::parse(&vec![b'x'; 4096]).unwrap_err(), Error::TooLarge);
}

#[test]
fn websocket_handshake_matches_rfc6455_and_frames_can_pdus() {
    let request = Request::parse(b"GET /ws HTTP/1.1\r\nUpgrade: WebSocket\r\nConnection: keep-alive, Upgrade\r\nSec-WebSocket-Version: 13\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\r\n").unwrap();
    assert!(request.websocket_upgrade);
    assert_eq!(websocket::accept_key(request.websocket_key.unwrap()).unwrap(), "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=");
    assert!(websocket::accept_key("bad").is_none());
    for (len, expected) in [(8, vec![0x82, 8]), (126, vec![0x82, 126, 0, 126]), (65536, vec![0x82, 127, 0, 0, 0, 0, 0, 1, 0, 0])] {
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
