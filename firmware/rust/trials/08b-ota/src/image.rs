//! Image d'app au format d'IDF : en-tête de 24 octets, segments, octet de
//! somme de contrôle, puis SHA-256 de tout ce qui précède. Coffeeflow (IDF)
//! et `espflash save-image` produisent ce format.

/// Le premier segment commence après l'en-tête (24 octets) et son propre
/// en-tête (8 octets) ; le descripteur d'app (`esp_app_desc_t`) l'ouvre.
const APP_DESC_AT: usize = 32;
const APP_DESC_MAGIC: u32 = 0xABCD_5432;
/// Assez pour l'en-tête et les champs du descripteur lus ici.
pub const HEAD_BYTES: usize = APP_DESC_AT + 144;
pub const SHA_BYTES: usize = 32;

const MAGIC: u8 = 0xE9;
const CHIP_ID_ESP32S3: u16 = 9;

/// Ce que dit le début de l'image.
pub struct Head {
    pub version: heapless::String<32>,
    pub project: heapless::String<32>,
}

/// Refuse ce qui n'est pas une image ESP32-S3 avec SHA-256 en fin. Le
/// descripteur d'app est facultatif : l'image est acceptée sans.
pub fn check_head(head: &[u8; HEAD_BYTES]) -> Result<Head, &'static str> {
    if head[0] != MAGIC {
        return Err("magic");
    }
    if u16::from_le_bytes([head[12], head[13]]) != CHIP_ID_ESP32S3 {
        return Err("puce");
    }
    if head[23] != 1 {
        return Err("sans_sha256");
    }
    let mut out = Head { version: heapless::String::new(), project: heapless::String::new() };
    let magic = u32::from_le_bytes(head[APP_DESC_AT..APP_DESC_AT + 4].try_into().unwrap());
    if magic == APP_DESC_MAGIC {
        out.version = c_string(&head[APP_DESC_AT + 16..APP_DESC_AT + 48]);
        out.project = c_string(&head[APP_DESC_AT + 48..APP_DESC_AT + 80]);
    }
    Ok(out)
}

fn c_string(bytes: &[u8]) -> heapless::String<32> {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    let mut s = heapless::String::new();
    for &b in &bytes[..end] {
        let _ = s.push(if b.is_ascii_graphic() || b == b' ' { b as char } else { '?' });
    }
    s
}
