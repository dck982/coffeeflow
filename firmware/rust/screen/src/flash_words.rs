//! Fragments HTTP et derniers octets OTA : accès NOR par mots de quatre octets.
//! Les écritures préservent les autres octets, sans effacer leur secteur.
use embedded_storage::nor_flash::{NorFlash, ReadNorFlash};
pub fn read<F: ReadNorFlash>(
    flash: &mut F,
    mut address: u32,
    bytes: &mut [u8],
) -> Result<(), F::Error> {
    // Lire des mots entiers évite le tampon de secteur de 4 Ko de l'API
    // bytewise, tout en acceptant le dernier fragment de taille impaire.

    let mut remaining = bytes;
    while !remaining.is_empty() {
        if address.is_multiple_of(4) && remaining.len() >= 4 {
            let aligned = remaining.len() / 4 * 4;
            flash.read(address, &mut remaining[..aligned])?;
            address += aligned as u32;
            remaining = &mut remaining[aligned..];
        } else {
            let mut word = [0; 4];
            flash.read(address / 4 * 4, &mut word)?;
            let start = (address % 4) as usize;
            let count = remaining.len().min(4 - start);
            remaining[..count].copy_from_slice(&word[start..start + count]);
            address += count as u32;
            remaining = &mut remaining[count..];
        }
    }
    Ok(())
}
pub fn write<F: NorFlash>(flash: &mut F, mut address: u32, bytes: &[u8]) -> Result<(), F::Error> {
    // HTTP livre des fragments de longueur arbitraire. Préserver les autres
    // octets du mot évite tout effacement et autorise un dernier octet isolé.

    let mut remaining = bytes;
    while !remaining.is_empty() {
        if address.is_multiple_of(4) && remaining.len() >= 4 {
            let aligned = remaining.len() / 4 * 4;
            flash.write(address, &remaining[..aligned])?;
            address += aligned as u32;
            remaining = &remaining[aligned..];
        } else {
            let word_at = address / 4 * 4;
            let mut word = [0; 4];
            flash.read(word_at, &mut word)?;
            let start = (address % 4) as usize;
            let count = remaining.len().min(4 - start);
            word[start..start + count].copy_from_slice(&remaining[..count]);
            flash.write(word_at, &word)?;
            address += count as u32;
            remaining = &remaining[count..];
        }
    }
    Ok(())
}
