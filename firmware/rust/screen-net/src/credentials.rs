//! Blob `wifi/credentials` partagé avec le firmware C++ : deux chaînes C,
//! respectivement de 33 et 64 octets. Aucun secret n'est journalisable.
use alloc::string::String;

/// Identifiants WPA personnels ; volontairement sans implémentation Debug.
#[derive(Clone, PartialEq, Eq)]
pub struct Credentials {
    pub ssid: String,
    pub password: String,
}

impl Credentials {
    /// Valide les limites en octets du pilote et rejette les NUL incorporés.
    pub fn new(ssid: String, password: String) -> Option<Self> {
        if ssid.is_empty()
            || ssid.len() > 32
            || !(8..=63).contains(&password.len())
            || ssid.contains('\0')
            || password.contains('\0')
        {
            return None;
        }
        Some(Self { ssid, password })
    }

    /// Document `application/x-www-form-urlencoded` envoyé par la page AP.
    pub fn from_form(body: &[u8]) -> Option<Self> {
        let body = core::str::from_utf8(body).ok()?;
        let mut ssid = None;
        let mut password = None;
        for part in body.split('&') {
            let (key, value) = part.split_once('=')?;
            match key {
                "ssid" if ssid.is_none() => ssid = Some(decode(value)?),
                "password" if password.is_none() => password = Some(decode(value)?),
                "ssid" | "password" => return None,
                _ => {}
            }
        }
        Self::new(ssid?, password?)
    }

    /// Écrit exactement la disposition du struct Credentials de l'ancien firmware.
    pub fn to_blob(&self) -> [u8; 97] {
        let mut blob = [0; 97];
        blob[..self.ssid.len()].copy_from_slice(self.ssid.as_bytes());
        blob[33..33 + self.password.len()].copy_from_slice(self.password.as_bytes());
        blob
    }

    /// Relit un blob IDF ; une longueur ou terminaison incorrecte déclenche l'AP.
    pub fn from_blob(blob: &[u8]) -> Option<Self> {
        if blob.len() != 97 {
            return None;
        }
        let ssid_end = blob[..33].iter().position(|b| *b == 0)?;
        let password_end = blob[33..].iter().position(|b| *b == 0)?;
        Self::new(
            core::str::from_utf8(&blob[..ssid_end]).ok()?.into(),
            core::str::from_utf8(&blob[33..33 + password_end])
                .ok()?
                .into(),
        )
    }
}

fn decode(input: &str) -> Option<String> {
    let mut out = alloc::vec::Vec::new();
    let mut bytes = input.bytes();
    while let Some(byte) = bytes.next() {
        out.push(match byte {
            b'+' => b' ',
            b'%' => (hex(bytes.next()?)? << 4) | hex(bytes.next()?)?,
            other => other,
        });
    }
    String::from_utf8(out).ok()
}
fn hex(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}
