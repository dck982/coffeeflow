//! WebSocket en miroir CAN : handshake RFC6455 et trames binaires émises.
//! Aucun paquet reçu du client n'est converti en commande CAN.
use alloc::string::String;
use base64::{Engine, engine::general_purpose::STANDARD};
use sha1::{Digest, Sha1};

/// Accepte uniquement une clé de handshake qui encode seize octets.
pub fn accept_key(key: &str) -> Option<String> {
    if STANDARD.decode(key).ok()?.len() != 16 { return None; }
    let mut hash = Sha1::new();
    hash.update(key.as_bytes());
    hash.update(b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11");
    Some(STANDARD.encode(hash.finalize()))
}

/// En-tête d'une trame serveur binaire complète, sans masque.
pub fn binary_header(len: usize) -> ([u8; 10], usize) {
    let mut out = [0; 10];
    out[0] = 0x82;
    if len < 126 { out[1] = len as u8; (out, 2) }
    else if len <= u16::MAX as usize { out[1] = 126; out[2..4].copy_from_slice(&(len as u16).to_be_bytes()); (out, 4) }
    else { out[1] = 127; out[2..10].copy_from_slice(&(len as u64).to_be_bytes()); (out, 10) }
}
