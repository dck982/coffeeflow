//! Flash partagée entre NVS, OTA locale et staging sensors.
//! Le propriétaire vit sur le cœur 1 ; aucun alias ni verrou inter-cœurs.
//! esp-storage sans section critique conserve les interruptions radio.
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

    pub fn storage_mut(&mut self) -> &mut FlashStorage<'static> {
        &mut self.inner
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
    Bounds,
    Layout,
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

/// Zone de la table de partitions. Les accès restent bornés à cette zone.
#[derive(Clone, Copy, Debug)]
pub struct Region {
    pub offset: u32,
    pub size: u32,
}

impl Region {
    pub fn read(self, flash: &mut Flash, at: u32, bytes: &mut [u8]) -> Result<(), Error> {
        if at
            .checked_add(bytes.len() as u32)
            .is_none_or(|end| end > self.size)
        {
            return Err(Error::Bounds);
        }
        crate::flash_words::read(flash, self.offset + at, bytes)
    }
    pub fn write(self, flash: &mut Flash, at: u32, bytes: &[u8]) -> Result<(), Error> {
        if at
            .checked_add(bytes.len() as u32)
            .is_none_or(|end| end > self.size)
        {
            return Err(Error::Bounds);
        }
        crate::flash_words::write(flash, self.offset + at, bytes)
    }
    pub fn erase(self, flash: &mut Flash) -> Result<(), Error> {
        flash.erase(self.offset, self.offset + self.size)
    }
}

/// Zones utiles, sans adresse codée en dur : une table modifiée reste sûre.
#[derive(Clone, Copy, Debug)]
pub struct Layout {
    pub nvs: Region,
    pub otadata: Region,
    pub slots: [Region; 2],
    pub staging: Region,
    pub booted: Option<u32>,
}

impl Layout {
    pub fn target(self) -> Region {
        self.slots[self.booted.map_or(0, |slot| 1 - slot) as usize]
    }
}

pub fn layout(flash: &mut Flash) -> Result<Layout, Error> {
    let mut table = [0; partitions::PARTITION_TABLE_MAX_LEN];
    let pt = partitions::read_partition_table(&mut flash.inner, &mut table)
        .map_err(|_| Error::Layout)?;
    let find = |kind| -> Result<Region, Error> {
        let p = pt
            .find_partition(kind)
            .map_err(|_| Error::Layout)?
            .ok_or(Error::Layout)?;
        Ok(Region {
            offset: p.offset(),
            size: p.len(),
        })
    };
    let booted = match pt
        .booted_partition()
        .ok()
        .flatten()
        .map(|p| p.partition_type())
    {
        Some(PartitionType::App(AppPartitionSubType::Ota0)) => Some(0),
        Some(PartitionType::App(AppPartitionSubType::Ota1)) => Some(1),
        _ => None,
    };
    let staging = pt
        .iter()
        .find(|p| p.label_as_str() == "ota_staging")
        .map(|p| Region {
            offset: p.offset(),
            size: p.len(),
        })
        .ok_or(Error::Layout)?;
    Ok(Layout {
        nvs: find(PartitionType::Data(DataPartitionSubType::Nvs))?,
        otadata: find(PartitionType::Data(DataPartitionSubType::Ota))?,
        slots: [
            find(PartitionType::App(AppPartitionSubType::Ota0))?,
            find(PartitionType::App(AppPartitionSubType::Ota1))?,
        ],
        staging,
        booted,
    })
}
