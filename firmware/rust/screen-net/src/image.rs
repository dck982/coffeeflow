//! Vérification d'image ESP32-S3 depuis un lecteur borné, testable sur hôte.
use sha2::{Digest, Sha256};
/// Valide la structure, la somme XOR IDF et le SHA256 appendu ; les lectures
/// sont faites sur la flash de destination, donc aussi la relecture physique.
pub fn verify(
    size: u32,
    mut read: impl FnMut(u32, &mut [u8]) -> Result<(), &'static str>,
) -> Result<(), &'static str> {
    if size < 24 + 8 + 16 + 32 {
        return Err("invalid_firmware");
    }
    let mut head = [0; 24];
    read(0, &mut head)?;
    if head[0] != 0xe9
        || !(1..=16).contains(&head[1])
        || head[23] != 1
        || u16::from_le_bytes([head[12], head[13]]) != 9
    {
        return Err("invalid_firmware");
    }
    let mut at = 24u32;
    let mut checksum = 0xef;
    let mut bytes = [0; 1024];
    for _ in 0..head[1] {
        let mut segment = [0; 8];
        read(at, &mut segment)?;
        at += 8;
        let len = u32::from_le_bytes(segment[4..].try_into().unwrap());
        let end = at
            .checked_add(len)
            .filter(|end| *end < size - 32)
            .ok_or("invalid_firmware")?;
        while at < end {
            let count = ((end - at) as usize).min(bytes.len());
            read(at, &mut bytes[..count])?;
            for b in &bytes[..count] {
                checksum ^= b;
            }
            at += count as u32;
        }
    }
    let checksum_at = (at + 1).next_multiple_of(16) - 1;
    if checksum_at.checked_add(33) != Some(size) {
        return Err("invalid_firmware");
    }
    let mut got = [0];
    read(checksum_at, &mut got)?;
    if got[0] != checksum {
        return Err("invalid_firmware");
    }
    let mut hash = Sha256::new();
    let mut at = 0;
    while at < size - 32 {
        let count = ((size - 32 - at) as usize).min(bytes.len());
        read(at, &mut bytes[..count])?;
        hash.update(&bytes[..count]);
        at += count as u32;
    }
    let mut expected = [0; 32];
    read(size - 32, &mut expected)?;
    if hash.finalize()[..] != expected {
        return Err("invalid_firmware");
    }
    Ok(())
}
