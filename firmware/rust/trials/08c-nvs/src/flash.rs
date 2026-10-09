//! La flash vue par esp-nvs, avec un verrou d'écriture.
//!
//! `Nvs::new` peut écrire dès l'ouverture : fin d'une défragmentation
//! interrompue, entrées abîmées ou trous marqués effacés, doublons effacés,
//! anciens blobs migrés. Sur le vrai NVS, la lecture passe donc par une
//! flash qui refuse toute écriture et compte les refus : si esp-nvs voulait
//! écrire, on le sait, et rien n'a changé.

use core::sync::atomic::{AtomicU32, Ordering::Relaxed};

use embedded_storage::nor_flash::{
    ErrorType, NorFlash, NorFlashError, NorFlashErrorKind, ReadNorFlash,
};
use esp_bootloader_esp_idf::partitions::{
    self, AppPartitionSubType, DataPartitionSubType, PartitionType,
};
use esp_nvs::platform::{Crc, software_crc32};
use esp_storage::{FlashStorage, FlashStorageError};

/// Écritures et effacements faits, et refusés, depuis le démarrage.
pub static WRITES: AtomicU32 = AtomicU32::new(0);
pub static REFUSED: AtomicU32 = AtomicU32::new(0);

pub struct Flash {
    inner: FlashStorage<'static>,
    writable: bool,
}

impl Flash {
    pub fn new(inner: FlashStorage<'static>, writable: bool) -> Self {
        Self { inner, writable }
    }

    pub fn into_inner(self) -> FlashStorage<'static> {
        self.inner
    }

    fn allow(&self) -> Result<(), Error> {
        if self.writable {
            WRITES.fetch_add(1, Relaxed);
            Ok(())
        } else {
            REFUSED.fetch_add(1, Relaxed);
            Err(Error::ReadOnly)
        }
    }
}

#[derive(Debug)]
pub enum Error {
    Storage(#[allow(dead_code, reason = "lu par Debug")] FlashStorageError),
    ReadOnly,
}

impl NorFlashError for Error {
    fn kind(&self) -> NorFlashErrorKind {
        NorFlashErrorKind::Other
    }
}

impl ErrorType for Flash {
    type Error = Error;
}

impl ReadNorFlash for Flash {
    const READ_SIZE: usize = <FlashStorage as ReadNorFlash>::READ_SIZE;

    fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Error> {
        ReadNorFlash::read(&mut self.inner, offset, bytes).map_err(Error::Storage)
    }

    fn capacity(&self) -> usize {
        ReadNorFlash::capacity(&self.inner)
    }
}

impl NorFlash for Flash {
    const WRITE_SIZE: usize = <FlashStorage as NorFlash>::WRITE_SIZE;
    const ERASE_SIZE: usize = <FlashStorage as NorFlash>::ERASE_SIZE;

    fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Error> {
        self.allow()?;
        NorFlash::write(&mut self.inner, offset, bytes).map_err(Error::Storage)
    }

    fn erase(&mut self, from: u32, to: u32) -> Result<(), Error> {
        self.allow()?;
        NorFlash::erase(&mut self.inner, from, to).map_err(Error::Storage)
    }
}

/// Le CRC32 d'IDF (`esp_rom_crc32_le`), en logiciel.
impl Crc for Flash {
    fn crc32(init: u32, data: &[u8]) -> u32 {
        software_crc32(init, data)
    }
}

/// CRC32 d'une zone, lue par secteurs : son empreinte avant et après.
pub fn region_crc(storage: &mut FlashStorage<'_>, start: u32, len: u32) -> u32 {
    let mut sector = [0u32; 1024];
    let bytes = unsafe { &mut *(&mut sector as *mut [u32; 1024]).cast::<[u8; 4096]>() };
    let mut crc = 0;
    for at in (start..start + len).step_by(4096) {
        storage.read_nor(at, bytes).unwrap();
        crc = software_crc32(crc, bytes);
    }
    crc
}

/// Ce que la table de partitions dit du NVS et du slot démarré.
pub struct Layout {
    /// (début, taille) de la partition `nvs`.
    pub nvs: (u32, u32),
    /// (début, taille) du slot démarré.
    pub booted: (u32, u32),
    pub booted_name: heapless::String<16>,
}

pub fn layout(storage: &mut FlashStorage<'_>) -> Layout {
    let mut table = [0u8; partitions::PARTITION_TABLE_MAX_LEN];
    let pt = partitions::read_partition_table(storage, &mut table).unwrap();
    let nvs = pt
        .find_partition(PartitionType::Data(DataPartitionSubType::Nvs))
        .unwrap()
        .unwrap();
    let booted = pt.booted_partition().unwrap().unwrap();
    assert!(
        matches!(
            booted.partition_type(),
            PartitionType::App(AppPartitionSubType::Ota0 | AppPartitionSubType::Ota1)
        ),
        "démarré hors d'un slot OTA"
    );
    let mut booted_name = heapless::String::new();
    let _ = booted_name.push_str(booted.label_as_str());
    Layout {
        nvs: (nvs.offset(), nvs.len()),
        booted: (booted.offset(), booted.len()),
        booted_name,
    }
}

#[cfg(feature = "c-factice")]
/// Fin de l'image démarrée, d'après son en-tête : segments, somme de
/// contrôle alignée sur 16, puis SHA-256. Une borne haute suffit.
pub fn image_end(storage: &mut FlashStorage<'_>, slot: u32) -> u32 {
    let mut head = [0u8; 24];
    storage.read_nor(slot, &mut head).unwrap();
    assert_eq!(head[0], 0xE9, "pas d'image au début du slot démarré");
    let mut at = slot + 24;
    for _ in 0..head[1] {
        let mut seg = [0u8; 8];
        storage.read_nor(at, &mut seg).unwrap();
        at += 8 + u32::from_le_bytes(seg[4..8].try_into().unwrap());
    }
    at.next_multiple_of(16) + 16 + 32
}
