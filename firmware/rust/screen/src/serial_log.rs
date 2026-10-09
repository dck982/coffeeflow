//! Un journal entier occupe un seul élément de file et se termine par zéro.
//! Le décodeur COBS rejette le texte puis reçoit la trame CAN suivante entière.
use core::fmt::{self, Write};

pub fn record(level: &str, target: &str, args: fmt::Arguments<'_>) -> heapless::Vec<u8, 256> {
    let mut text = heapless::String::<252>::new();
    // Un message trop long reste tronqué, mais conserve toujours sa bordure.
    let _ = write!(text, "[{level}] {target}: {args}");
    let mut bytes = heapless::Vec::new();
    let _ = bytes.extend_from_slice(text.as_bytes());
    let _ = bytes.extend_from_slice(b"\r\n\0");
    bytes
}
