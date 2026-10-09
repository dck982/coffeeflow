#[path = "../../screen/src/serial_log.rs"]
mod serial_log;

#[test]
fn text_logs_do_not_damage_following_can_frames_even_when_truncated() {
    let frame = protocol::Frame::new(0x104, &[0, 1, 2, 0, 4]).unwrap();
    let mut framed = [0; protocol::MAX_FRAMED_SIZE];
    let n = protocol::encode_framed(&frame, &mut framed).unwrap();
    let long = "x".repeat(1024);
    for message in ["boot done", long.as_str()] {
        let line = serial_log::record("INFO", "can", format_args!("{message}"));
        assert!(line.len() <= 256 && line.ends_with(b"\r\n\0"));
        let mut decoder = protocol::StreamDecoder::new();
        let received: Vec<_> = line
            .iter()
            .chain(&framed[..n])
            .filter_map(|b| decoder.push_byte(*b))
            .collect();
        assert_eq!(received, vec![frame]);
    }
}
