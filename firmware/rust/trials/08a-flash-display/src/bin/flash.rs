// Charge d'une mise à jour de coffeeflow, rejouée dans la fin du slot de
// test, au-delà de l'image de l'essai. Coffeeflow fait `esp_ota_begin`, qui
// efface d'un coup toute la taille annoncée, puis un `esp_ota_write` par
// morceau de 1 024 octets reçu par HTTP. On refait les deux phases, sur 1 Mo
// (60 % d'une image de coffeeflow), puis on relit tout pour contrôler.
//
// Les cycles alternent l'effacement par secteurs de 4 Ko et par blocs de
// 64 Ko : `esp_partition_erase_range` d'IDF prend les blocs dès que
// l'alignement le permet, mais un effacement de bloc garde les interruptions
// masquées bien plus longtemps. Les écritures vont au plus vite, sans
// attendre de réseau : c'est la charge la plus dure.
//
// Deux modes, choisis au build :
//
// - esp-storage tel quel (par défaut) : chaque opération coupe tout le
//   reste, interruptions masquées sur ce cœur (la recopie des bounce buffers
//   comprise) et cœur 1 arrêté par `multicore_auto_park`, donc la radio
//   aussi ;
// - `comme-idf` : ce que fait IDF quand le code tourne depuis la PSRAM
//   (`spi1_start`, `!MMAP_EXECUTABLES_FROM_FLASH`). Le cache ne lit plus la
//   flash, il reste actif ; ni section critique, ni arrêt du cœur 1. IDF
//   prend un mutex sur SPI1 : ici, cette tâche est la seule à toucher la
//   flash. La fonction ROM attend la fin de l'opération en boucle active, et
//   bloque donc l'exécuteur du cœur 0 (rendu, rapport), mais pas les
//   interruptions.
//
// Entre deux opérations, la tâche rend la main : rendu, rapport et recalage
// de l'affichage passent.

use core::sync::atomic::{AtomicU32, Ordering::Relaxed};

use embassy_futures::yield_now;
use embassy_time::{Duration, Timer};
use esp_bootloader_esp_idf::partitions::{self, AppPartitionSubType, PartitionType};
use esp_storage::FlashStorage;
use log::info;

use super::{LATE, RESYNCS};

/// La zone commence à 2 Mo du début du slot (3 500 Ko) et fait 1 Mo.
const REGION_SLOT_OFFSET: u32 = 0x20_0000;
const REGION_BYTES: u32 = 0x10_0000;
const SECTOR: u32 = FlashStorage::SECTOR_SIZE;
const BLOCK: u32 = FlashStorage::BLOCK_SIZE;
/// Taille du morceau d'`esp_ota_write` dans coffeeflow (`firmware_handler`).
const CHUNK: usize = 1024;
const CHUNKS: u32 = REGION_BYTES / CHUNK as u32;
/// Affichage et radio seuls avant le premier cycle et entre deux cycles :
/// le rapport y donne la référence sans écriture.
const IDLE: Duration = Duration::from_secs(30);

/// Cycles terminés et erreurs d'esp-storage, pour le rapport HTTP.
pub static CYCLES: AtomicU32 = AtomicU32::new(0);
pub static ERRORS: AtomicU32 = AtomicU32::new(0);
pub static MISMATCHES: AtomicU32 = AtomicU32::new(0);

/// Slot d'app d'où l'essai a démarré.
pub struct Slot {
    offset: u32,
    len: u32,
    label: heapless::String<16>,
}

/// Pilote de la flash selon le mode de l'essai.
pub fn storage(flash: esp_storage::Flash<'static>) -> FlashStorage<'static> {
    let storage = FlashStorage::new(flash);
    #[cfg(not(feature = "comme-idf"))]
    let storage = storage.multicore_auto_park();
    // Le cœur 1 ne lit jamais la flash : son code et ses rodata sont en
    // PSRAM depuis `xip::run`, qui précède son démarrage.
    #[cfg(feature = "comme-idf")]
    let storage = unsafe { storage.multicore_ignore() };
    storage
}

/// Lit la table de partitions et le slot démarré. À appeler avant
/// `xip::run` : `booted_partition` lit la première entrée de la MMU, que le
/// remappage fait pointer en PSRAM.
pub fn booted_slot(storage: &mut FlashStorage<'_>, table: &mut [u8]) -> Option<Slot> {
    let Some(entry) = partitions::read_partition_table(storage, table)
        .and_then(|pt| pt.booted_partition())
        .ok()
        .flatten()
    else {
        info!("flash: table de partitions ou slot démarré illisible, rien n'est écrit");
        return None;
    };
    let PartitionType::App(AppPartitionSubType::Ota0 | AppPartitionSubType::Ota1) =
        entry.partition_type()
    else {
        info!("flash: démarré hors d'un slot OTA ({}), rien n'est écrit", entry.label_as_str());
        return None;
    };
    let mut label = heapless::String::new();
    let _ = label.push_str(entry.label_as_str());
    Some(Slot { offset: entry.offset(), len: entry.len(), label })
}

/// Début de la zone d'écriture, si elle tient dans le slot démarré et après
/// la fin de l'image (`image_end`, en flash physique).
pub fn region(slot: &Slot, image_end: u32) -> Option<u32> {
    let start = slot.offset + REGION_SLOT_OFFSET;
    let end = start + REGION_BYTES;
    info!(
        "flash: slot {} 0x{:06x}+0x{:06x}, image jusqu'à 0x{:06x}, zone 0x{:06x}..0x{:06x}",
        slot.label, slot.offset, slot.len, image_end, start, end
    );
    // IDF ne garde le cache actif que si plus aucune page ne mappe la flash.
    info!(
        "flash: mode {}, pages MMU encore sur la flash: {}",
        if cfg!(feature = "comme-idf") { "comme-idf" } else { "esp-storage" },
        crate::xip::flash_mappings(),
    );
    let fits = image_end != 0
        && image_end <= start
        && end <= slot.offset + slot.len
        && start.is_multiple_of(BLOCK);
    if !fits {
        info!("flash: zone hors du slot ou sur l'image, rien n'est écrit");
        return None;
    }
    Some(start)
}

/// Contenu pseudo-aléatoire du morceau `index` au cycle `cycle` : chaque
/// cycle écrit autre chose que le précédent, et la relecture le régénère.
fn fill(buf: &mut [u32; CHUNK / 4], cycle: u32, index: u32) {
    let mut x = (cycle << 20 | index).wrapping_mul(0x9E37_79B9) | 1;
    for word in buf.iter_mut() {
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        *word = x;
    }
}

fn as_bytes(buf: &[u32; CHUNK / 4]) -> &[u8] {
    unsafe { core::slice::from_raw_parts(buf.as_ptr().cast(), CHUNK) }
}

fn as_bytes_mut(buf: &mut [u32; CHUNK / 4]) -> &mut [u8] {
    unsafe { core::slice::from_raw_parts_mut(buf.as_mut_ptr().cast(), CHUNK) }
}

/// Durées d'une phase, en µs.
struct Phase {
    n: u32,
    sum: u64,
    max: u32,
    late: u32,
    resyncs: u32,
    start: esp_hal::time::Instant,
}

impl Phase {
    fn begin() -> Self {
        Self {
            n: 0,
            sum: 0,
            max: 0,
            late: LATE.load(Relaxed),
            resyncs: RESYNCS.load(Relaxed),
            start: esp_hal::time::Instant::now(),
        }
    }

    /// Une opération d'esp-storage. Sans `comme-idf`, la section critique
    /// englobe l'arrêt et la reprise du cœur 1 : une interruption de ce cœur
    /// en attente pendant l'opération ne part qu'une fois le cœur 1 relancé,
    /// et ne peut pas attendre un verrou qu'il tiendrait, arrêté.
    fn op(&mut self, f: impl FnOnce() -> Result<(), esp_storage::FlashStorageError>) {
        let t0 = esp_hal::time::Instant::now();
        #[cfg(not(feature = "comme-idf"))]
        let result = critical_section::with(|_| f());
        #[cfg(feature = "comme-idf")]
        let result = f();
        let us = t0.elapsed().as_micros() as u32;
        self.n += 1;
        self.sum += us as u64;
        self.max = self.max.max(us);
        if result.is_err() {
            ERRORS.fetch_add(1, Relaxed);
        }
    }

    fn avg(&self) -> u64 {
        self.sum.checked_div(self.n as u64).unwrap_or(0)
    }

    fn ms(&self) -> u64 {
        self.start.elapsed().as_millis()
    }

    fn late(&self) -> u32 {
        LATE.load(Relaxed) - self.late
    }

    fn resyncs(&self) -> u32 {
        RESYNCS.load(Relaxed) - self.resyncs
    }
}

/// Une ligne de journal par phase, espacées de 20 ms comme le rapport.
async fn log_gap() {
    Timer::after(Duration::from_millis(20)).await;
}

pub use task::run;

#[allow(
    clippy::large_stack_frames,
    reason = "les deux tampons de 1 Ko sont dans l'état de la future, hors pile ; le secteur de 4 Ko que read_nor et write_nor réservent ne sert qu'à un tampon non aligné, et les nôtres sont des [u32]. Pile réelle : pile0 du rapport"
)]
mod task {
    use super::*;

    #[embassy_executor::task]
    pub async fn run(mut storage: FlashStorage<'static>, start: u32) {
        let mut data = [0u32; CHUNK / 4];
        let mut back = [0u32; CHUNK / 4];
        for cycle in 1u32.. {
            Timer::after(IDLE).await;

            let (mode, step) = if cycle % 2 == 1 { ("secteurs", SECTOR) } else { ("blocs", BLOCK) };
            let mut erase = Phase::begin();
            for addr in (start..start + REGION_BYTES).step_by(step as usize) {
                erase.op(|| storage.erase(addr, addr + step));
                yield_now().await;
            }
            let erase_ms = erase.ms();

            let mut write = Phase::begin();
            for index in 0..CHUNKS {
                fill(&mut data, cycle, index);
                let addr = start + index * CHUNK as u32;
                write.op(|| storage.write_nor(addr, as_bytes(&data)));
                yield_now().await;
            }
            let write_ms = write.ms();

            let mut read = Phase::begin();
            let mut mismatches = 0;
            for index in 0..CHUNKS {
                let addr = start + index * CHUNK as u32;
                read.op(|| storage.read_nor(addr, as_bytes_mut(&mut back)));
                fill(&mut data, cycle, index);
                if back != data {
                    mismatches += 1;
                }
                yield_now().await;
            }
            let read_ms = read.ms();
            MISMATCHES.fetch_add(mismatches, Relaxed);
            CYCLES.store(cycle, Relaxed);

            info!(
                "flash c{} effacement {} n={} moy={}us max={}us {}ms ret=+{} recal=+{}",
                cycle,
                mode,
                erase.n,
                erase.avg(),
                erase.max,
                erase_ms,
                erase.late(),
                erase.resyncs(),
            );
            log_gap().await;
            info!(
                "flash c{} ecriture n={} moy={}us max={}us {}ms {}Ko/s ret=+{} recal=+{}",
                cycle,
                write.n,
                write.avg(),
                write.max,
                write_ms,
                (REGION_BYTES as u64 / 1024 * 1000).checked_div(write_ms).unwrap_or(0),
                write.late(),
                write.resyncs(),
            );
            log_gap().await;
            info!(
                "flash c{} relecture {}ms max={}us ecarts={} err={}",
                cycle,
                read_ms,
                read.max,
                mismatches,
                ERRORS.load(Relaxed),
            );
        }
    }
}
