//! CRC16 et CRC32 du protocole. Calcul bit à bit : pas de table en flash, et
//! la vitesse suffit pour les blocs de 2 Ko de l'OTA.

/// CRC16-CCITT (poly 0x1021, init 0xFFFF), utilisé pour l'acquittement de
/// bloc du flash (`FLASH_CTRL BLOCK_ACK`, tous les 2 Ko) et pour le PDU série.
pub fn crc16_ccitt(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for &byte in data {
        crc ^= u16::from(byte) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 {
                (crc << 1) ^ 0x1021
            } else {
                crc << 1
            };
        }
    }
    crc
}

/// CRC32 (poly 0xEDB88320, init 0xFFFFFFFF, xorout 0xFFFFFFFF : variante
/// zlib / IEEE 802.3), utilisé pour la vérification globale de l'image
/// (`FLASH_CTRL END`).
pub fn crc32_ieee(data: &[u8]) -> u32 {
    let mut crc = Crc32Incremental::new();
    crc.update(data);
    crc.finish()
}

/// Même CRC32 que [`crc32_ieee`], calculé au fil de l'eau : le récepteur du
/// flash vérifie ainsi l'image sans la garder entière en RAM.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Crc32Incremental {
    crc: u32,
}

impl Crc32Incremental {
    pub const fn new() -> Self {
        Self { crc: 0xFFFF_FFFF }
    }

    pub fn update(&mut self, data: &[u8]) {
        let mut crc = self.crc;
        for &byte in data {
            crc ^= u32::from(byte);
            for _ in 0..8 {
                let mask = 0u32.wrapping_sub(crc & 1);
                crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
            }
        }
        self.crc = crc;
    }

    /// CRC des octets reçus jusqu'ici ; le calcul peut continuer ensuite.
    pub const fn finish(&self) -> u32 {
        self.crc ^ 0xFFFF_FFFF
    }
}

impl Default for Crc32Incremental {
    fn default() -> Self {
        Self::new()
    }
}
