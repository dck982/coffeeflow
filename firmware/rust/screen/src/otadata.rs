//! Partition `otadata`, lue et écrite selon les règles d'IDF v6.1
//! (`bootloader_common_loader.c`, `esp_ota_ops.c`), pour deux slots OTA.
//!
//! On n'utilise pas `Ota` d'esp-bootloader-esp-idf : il prend la séquence la
//! plus haute sans regarder l'état, là où IDF écarte les copies `INVALID` et
//! `ABORTED`. Après un retour en arrière, la copie la plus haute est celle de
//! l'image refusée. La valider avec `Ota` relancerait cette image.
//!
//! Deux copies de 32 octets, au début de chacun des deux secteurs de la
//! partition : séquence, étiquette (inutilisée), état à l'octet 24, CRC32 de
//! la séquence seule. Le slot désigné par une copie est
//! `(séquence − 1) mod 2`.

use core::fmt;

use esp_storage::{FlashStorage, FlashStorageError};

const SECTOR: u32 = FlashStorage::SECTOR_SIZE;
const ENTRY_BYTES: usize = 32;
const STATE_AT: usize = 24;
const CRC_AT: usize = 28;
/// `ota_0` et `ota_1` : la table de partitions de l’écran n'en a pas
/// d'autre (plus `factory`, jamais désignée par une séquence).
const SLOTS: u32 = 2;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum State {
    New,
    PendingVerify,
    Valid,
    Invalid,
    Aborted,
    /// Copie effacée, ou état jamais écrit.
    Undefined,
    Unknown(u32),
}

impl State {
    fn from_raw(raw: u32) -> Self {
        match raw {
            0 => Self::New,
            1 => Self::PendingVerify,
            2 => Self::Valid,
            3 => Self::Invalid,
            4 => Self::Aborted,
            u32::MAX => Self::Undefined,
            other => Self::Unknown(other),
        }
    }

    fn raw(self) -> u32 {
        match self {
            Self::New => 0,
            Self::PendingVerify => 1,
            Self::Valid => 2,
            Self::Invalid => 3,
            Self::Aborted => 4,
            Self::Undefined => u32::MAX,
            Self::Unknown(raw) => raw,
        }
    }
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::New => f.write_str("NEW"),
            Self::PendingVerify => f.write_str("PENDING_VERIFY"),
            Self::Valid => f.write_str("VALID"),
            Self::Invalid => f.write_str("INVALID"),
            Self::Aborted => f.write_str("ABORTED"),
            Self::Undefined => f.write_str("UNDEFINED"),
            Self::Unknown(raw) => write!(f, "?{raw:#x}"),
        }
    }
}

/// Une copie, octet pour octet. Alignée sur 4 : esp-storage écrit alors
/// directement, sans tampon de 4 Ko sur la pile.
#[derive(Clone, Copy)]
#[repr(C, align(4))]
pub struct Entry([u8; ENTRY_BYTES]);

impl Entry {
    fn word(&self, at: usize) -> u32 {
        u32::from_le_bytes(self.0[at..at + 4].try_into().unwrap())
    }

    fn set_word(&mut self, at: usize, value: u32) {
        self.0[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }

    pub fn seq(&self) -> u32 {
        self.word(0)
    }

    pub fn state(&self) -> State {
        State::from_raw(self.word(STATE_AT))
    }

    pub fn crc_ok(&self) -> bool {
        self.word(CRC_AT) == seq_crc(self.seq())
    }

    /// `bootloader_common_ota_select_valid` : une séquence écrite, un CRC
    /// juste, et un état ni `INVALID` ni `ABORTED`.
    pub fn selectable(&self) -> bool {
        self.seq() != u32::MAX
            && !matches!(self.state(), State::Invalid | State::Aborted)
            && self.crc_ok()
    }

    /// Slot OTA désigné par la séquence (0 pour `ota_0`).
    pub fn slot(&self) -> u32 {
        self.seq().wrapping_sub(1) % SLOTS
    }
}

/// CRC32 de la séquence, comme `esp_rom_crc32_le(UINT32_MAX, &seq, 4)`
/// (le `zlib.crc32(données, 0xffffffff)` de Python).
pub fn seq_crc(seq: u32) -> u32 {
    let mut crc = !u32::MAX;
    for byte in seq.to_le_bytes() {
        crc ^= byte as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

/// Valeur relevée dans esp-bootloader-esp-idf (séquence 1) et vérifiée avec
/// zlib : le calcul doit la retrouver avant toute écriture.
const SEQ_1_CRC: u32 = 0x4743_989A;

#[derive(Debug)]
pub enum Error {
    #[allow(dead_code, reason = "lue par Debug, dans les journaux")]
    Flash(FlashStorageError),
    /// Aucune copie sélectionnable : IDF démarrerait `factory`.
    NoActive,
    /// La copie relue après écriture n'est pas celle écrite.
    Readback,
    /// Le calcul du CRC ne retrouve pas la valeur de référence.
    Crc,
}

impl From<FlashStorageError> for Error {
    fn from(err: FlashStorageError) -> Self {
        Self::Flash(err)
    }
}

/// Les deux copies, telles que lues dans la partition à `offset`.
pub struct Otadata {
    offset: u32,
    pub entries: [Entry; 2],
}

impl Otadata {
    pub fn read(storage: &mut FlashStorage<'_>, offset: u32) -> Result<Self, Error> {
        let mut entries = [Entry([0; ENTRY_BYTES]); 2];
        for (i, entry) in entries.iter_mut().enumerate() {
            storage.read_nor(offset + i as u32 * SECTOR, &mut entry.0)?;
        }
        Ok(Self { offset, entries })
    }

    /// `bootloader_common_get_active_otadata` : la copie sélectionnable de
    /// séquence la plus haute, la copie 0 à égalité.
    pub fn active(&self) -> Option<usize> {
        match self.entries.map(|e| e.selectable()) {
            [true, true] => Some(if self.entries[1].seq() > self.entries[0].seq() {
                1
            } else {
                0
            }),
            [true, false] => Some(0),
            [false, true] => Some(1),
            [false, false] => None,
        }
    }

    /// `esp_ota_set_boot_partition`, rollback actif : le prochain démarrage
    /// prend `slot`, en état `NEW`. Si la copie active désigne déjà ce slot,
    /// seul son état change ; sinon, l'autre copie reçoit la plus petite
    /// séquence qui dépasse l'active et désigne `slot`.
    pub fn set_boot(&mut self, storage: &mut FlashStorage<'_>, slot: u32) -> Result<(), Error> {
        let (index, seq) = match self.active() {
            Some(a) if self.entries[a].slot() == slot => (a, self.entries[a].seq()),
            Some(a) => (1 - a, next_seq(self.entries[a].seq(), slot)),
            None => (0, slot + 1),
        };
        self.rewrite(storage, index, seq, State::New)
    }

    /// `esp_ota_mark_app_valid_cancel_rollback` : la copie active passe en
    /// `VALID`, si elle ne l'est pas déjà.
    pub fn mark_valid(&mut self, storage: &mut FlashStorage<'_>) -> Result<(), Error> {
        let a = self.active().ok_or(Error::NoActive)?;
        if self.entries[a].state() == State::Valid {
            return Ok(());
        }
        self.rewrite(storage, a, self.entries[a].seq(), State::Valid)
    }

    /// `esp_ota_mark_app_invalid_rollback_and_reboot`, moitié otadata : la
    /// copie active passe en `INVALID`. Le bootloader écarte alors cette copie
    /// et démarre l'image précédente, y compris quand il ne fait pas lui-même
    /// le passage `NEW` → `ABORTED` d'un retour en arrière.
    pub fn mark_invalid(&mut self, storage: &mut FlashStorage<'_>) -> Result<(), Error> {
        let a = self.active().ok_or(Error::NoActive)?;
        self.rewrite(storage, a, self.entries[a].seq(), State::Invalid)
    }

    /// `rewrite_ota_seq` : la copie garde son étiquette, reçoit séquence,
    /// état et CRC, puis son secteur est effacé et réécrit. On relit pour
    /// contrôler.
    fn rewrite(
        &mut self,
        storage: &mut FlashStorage<'_>,
        index: usize,
        seq: u32,
        state: State,
    ) -> Result<(), Error> {
        if seq_crc(1) != SEQ_1_CRC {
            return Err(Error::Crc);
        }
        let mut entry = self.entries[index];
        entry.set_word(0, seq);
        entry.set_word(STATE_AT, state.raw());
        entry.set_word(CRC_AT, seq_crc(seq));
        let at = self.offset + index as u32 * SECTOR;
        storage.erase(at, at + SECTOR)?;
        storage.write_nor(at, &entry.0)?;
        let mut back = Entry([0; ENTRY_BYTES]);
        storage.read_nor(at, &mut back.0)?;
        self.entries[index] = back;
        if back.0 != entry.0 {
            return Err(Error::Readback);
        }
        Ok(())
    }
}

/// `compute_ota_seq_for_target_slot` : la plus petite séquence au moins
/// égale à `current` qui désigne `slot`.
fn next_seq(current: u32, slot: u32) -> u32 {
    let base = (slot + 1) % SLOTS;
    let mut i = 0;
    while current > base + i * SLOTS {
        i += 1;
    }
    base + i * SLOTS
}

/// Une ligne : chaque copie (séquence, état, CRC), la copie active et le
/// slot qu'elle désigne.
impl fmt::Display for Otadata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, e) in self.entries.iter().enumerate() {
            if e.seq() == u32::MAX {
                write!(f, "copie{i} vide ")?;
            } else {
                write!(
                    f,
                    "copie{i} seq={:#x} {} crc={} ",
                    e.seq(),
                    e.state(),
                    if e.crc_ok() { "ok" } else { "FAUX" }
                )?;
            }
        }
        match self.active() {
            Some(a) => write!(f, "active={a} -> ota_{}", self.entries[a].slot()),
            None => f.write_str("active=aucune -> factory"),
        }
    }
}
