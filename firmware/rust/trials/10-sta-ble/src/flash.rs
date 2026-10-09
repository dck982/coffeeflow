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
use esp_bootloader_esp_idf::partitions::{self, DataPartitionSubType, PartitionType};
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

/// (début, taille) de la partition `nvs`, d'après la table de partitions.
#[allow(
    clippy::large_stack_frames,
    reason = "table de partitions de 3 Ko sur la pile du cœur 1 (16 Ko), lue une fois au démarrage"
)]
pub fn nvs_region(storage: &mut FlashStorage<'_>) -> (u32, u32) {
    let mut table = [0u8; partitions::PARTITION_TABLE_MAX_LEN];
    let pt = partitions::read_partition_table(storage, &mut table).unwrap();
    let nvs = pt
        .find_partition(PartitionType::Data(DataPartitionSubType::Nvs))
        .unwrap()
        .unwrap();
    (nvs.offset(), nvs.len())
}
