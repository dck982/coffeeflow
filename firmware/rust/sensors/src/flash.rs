//! Flash du module : table de partitions, slot inactif, `otadata`.
//!
//! Le code s'exécute depuis la flash (ni PSRAM ni affichage ici) : esp-storage
//! garde sa section critique, qui masque les interruptions pendant chaque
//! opération (au plus 83 ms pour un bloc de 64 Ko effacé).

use esp_bootloader_esp_idf::partitions::{
    self, AppPartitionSubType, DataPartitionSubType, PartitionType,
};
use esp_storage::FlashStorage;
use log::{info, warn};
use sensors_logic::flash_rx::{FlashSink, HEAD_LEN};
use sha2::{Digest, Sha256};

use crate::otadata::{Otadata, State};
use crate::watchdog;

const BLOCK: u32 = FlashStorage::BLOCK_SIZE;
const SHA_BYTES: u32 = 32;

/// Où sont `otadata` et les deux slots, et lequel a démarré.
#[derive(Clone, Copy)]
struct Layout {
    otadata: u32,
    /// (début, taille) de `ota_0` et `ota_1`.
    slots: [(u32, u32); 2],
    /// Slot OTA démarré ; `None` si l'image tourne depuis `factory`.
    booted: Option<u32>,
}

impl Layout {
    /// Même choix qu'`esp_ota_get_next_update_partition(NULL)` : l'autre slot
    /// OTA, ou `ota_0` quand on tourne depuis `factory`.
    fn target(&self) -> u32 {
        match self.booted {
            Some(slot) => 1 - slot,
            None => 0,
        }
    }
}

/// Tampon de lecture aligné sur 4 : esp-storage lit alors directement, sans
/// copie de 4 Ko sur la pile.
#[repr(C, align(4))]
struct ReadBuf([u8; 1024]);

pub struct FlashOta {
    storage: FlashStorage<'static>,
    layout: Option<Layout>,
    read_buf: ReadBuf,
}

impl FlashOta {
    #[allow(
        clippy::large_stack_frames,
        reason = "la table de partitions (3 Ko) est lue une fois au démarrage"
    )]
    pub fn new(flash: esp_hal::peripherals::FLASH<'static>) -> Self {
        let mut storage = FlashStorage::new(flash);
        let layout = Self::read_layout(&mut storage);
        Self {
            storage,
            layout,
            read_buf: ReadBuf([0; 1024]),
        }
    }

    #[allow(
        clippy::large_stack_frames,
        reason = "la table de partitions (3 Ko) est lue une fois au démarrage"
    )]
    fn read_layout(storage: &mut FlashStorage<'_>) -> Option<Layout> {
        let mut table = [0u8; partitions::PARTITION_TABLE_MAX_LEN];
        let Ok(pt) = partitions::read_partition_table(storage, &mut table) else {
            warn!("table de partitions illisible");
            return None;
        };
        let find = |kind| {
            pt.find_partition(kind)
                .ok()
                .flatten()
                .map(|p| (p.offset(), p.len()))
        };
        let otadata = find(PartitionType::Data(DataPartitionSubType::Ota));
        let ota0 = find(PartitionType::App(AppPartitionSubType::Ota0));
        let ota1 = find(PartitionType::App(AppPartitionSubType::Ota1));
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
        match (otadata, ota0, ota1) {
            (Some((otadata, 0x2000)), Some(ota0), Some(ota1)) => {
                info!(
                    "partitions: otadata 0x{otadata:x}, ota_0 0x{:x}+0x{:x}, ota_1 0x{:x}+0x{:x}, démarrée sur {booted:?}",
                    ota0.0, ota0.1, ota1.0, ota1.1
                );
                Some(Layout {
                    otadata,
                    slots: [ota0, ota1],
                    booted,
                })
            }
            _ => {
                warn!("otadata, ota_0 ou ota_1 introuvable");
                None
            }
        }
    }

    /// État de la copie `otadata` active, si elle désigne bien le slot
    /// démarré. Sinon, le bootloader a démarré cette image pour une autre
    /// raison (retour en arrière, `factory`) et elle n'a rien à valider.
    /// Renvoie l'état brut (`NEW` = 0, `PENDING_VERIFY` = 1).
    pub fn pending_state(&mut self) -> Option<u32> {
        let layout = self.layout?;
        let booted = layout.booted?;
        let ota = Otadata::read(&mut self.storage, layout.otadata).ok()?;
        info!("otadata: {ota}");
        let active = ota.active()?;
        let entry = ota.entries[active];
        if entry.slot() != booted {
            return None;
        }
        match entry.state() {
            state @ (State::New | State::PendingVerify) => Some(match state {
                State::New => 0,
                _ => 1,
            }),
            _ => None,
        }
    }

    /// `esp_ota_mark_app_valid_cancel_rollback`.
    pub fn mark_valid(&mut self) -> bool {
        let Some(layout) = self.layout else {
            return false;
        };
        let result = Otadata::read(&mut self.storage, layout.otadata)
            .and_then(|mut ota| ota.mark_valid(&mut self.storage).map(|()| ota));
        match result {
            Ok(ota) => {
                info!("validée: {ota}");
                true
            }
            Err(err) => {
                warn!("validation impossible: {err:?}");
                false
            }
        }
    }

    /// Invalide l'image courante pour que le bootloader revienne sur la
    /// précédente. Sans effet si cette image n'est pas celle que désigne
    /// `otadata` (démarrée depuis `factory`, ou déjà validée plus tôt).
    pub fn invalidate(&mut self) {
        if self.pending_state().is_none() {
            return;
        }
        let Some(layout) = self.layout else { return };
        let result = Otadata::read(&mut self.storage, layout.otadata)
            .and_then(|mut ota| ota.mark_invalid(&mut self.storage).map(|()| ota));
        match result {
            Ok(ota) => info!("invalidée: {ota}"),
            Err(err) => warn!("invalidation impossible: {err:?}"),
        }
    }

    /// SHA-256 de fin d'image : celui qu'`esp_ota_end` contrôle. `size` est la
    /// taille de l'image, SHA compris.
    fn sha_ok(&mut self, start: u32, size: u32) -> bool {
        if size <= SHA_BYTES {
            return false;
        }
        let body = size - SHA_BYTES;
        let mut hasher = Sha256::new();
        let mut at = 0u32;
        while at < body {
            let n = (body - at).min(self.read_buf.0.len() as u32) as usize;
            if self
                .storage
                .read_nor(start + at, &mut self.read_buf.0[..n])
                .is_err()
            {
                return false;
            }
            hasher.update(&self.read_buf.0[..n]);
            at += n as u32;
            watchdog::feed();
        }
        let mut tail = [0u8; SHA_BYTES as usize];
        if self.storage.read_nor(start + body, &mut tail).is_err() {
            return false;
        }
        hasher.finalize().as_slice() == tail
    }
}

impl FlashSink for FlashOta {
    fn slot_size(&self) -> Option<u32> {
        let layout = self.layout?;
        Some(layout.slots[layout.target() as usize].1)
    }

    fn erase_slot(&mut self) -> bool {
        let Some(layout) = self.layout else {
            return false;
        };
        let (start, len) = layout.slots[layout.target() as usize];
        let end = start + len;
        // Par blocs de 64 Ko, pour nourrir le watchdog entre deux.
        let mut at = start;
        while at < end {
            let to = ((at / BLOCK + 1) * BLOCK).min(end);
            if self.storage.erase(at, to).is_err() {
                warn!("effacement en échec à 0x{at:x}");
                return false;
            }
            at = to;
            watchdog::feed();
        }
        true
    }

    fn write(&mut self, offset: u32, data: &[u8]) -> bool {
        let Some(layout) = self.layout else {
            return false;
        };
        let start = layout.slots[layout.target() as usize].0;
        self.storage.write_nor(start + offset, data).is_ok()
    }

    fn commit(&mut self, head: &[u8; HEAD_LEN], image_size: u32) -> bool {
        let Some(layout) = self.layout else {
            return false;
        };
        let target = layout.target();
        let start = layout.slots[target as usize].0;
        // Le SHA-256 ajouté par l'outil de génération est présent : on le
        // vérifie sur la flash relue, comme `esp_ota_end`.
        if head[23] == 1 && !self.sha_ok(start, image_size) {
            warn!("SHA-256 de l'image faux à la relecture");
            return false;
        }
        let result = Otadata::read(&mut self.storage, layout.otadata)
            .and_then(|mut ota| ota.set_boot(&mut self.storage, target).map(|()| ota));
        match result {
            Ok(ota) => {
                info!("otadata: {ota}");
                true
            }
            Err(err) => {
                warn!("bascule d'otadata impossible: {err:?}");
                false
            }
        }
    }
}
