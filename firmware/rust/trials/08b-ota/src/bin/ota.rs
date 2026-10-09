// Mise à jour par HTTP, comme `firmware_handler` de coffeeflow : l'image
// arrive dans le corps d'un `POST /firmware`, s'écrit au fil de l'eau dans
// l'autre slot, puis `otadata` désigne ce slot en état `NEW` et la carte
// redémarre. Avec le bootloader de l'écran (rollback actif), l'image démarre
// en `PENDING_VERIFY` et doit se valider, sinon le prochain démarrage revient
// ici.
//
// Différences avec IDF, voulues :
//
// - l'effacement se fait bloc de 64 Ko par bloc, juste avant d'y écrire, et
//   non d'un coup au début (`esp_ota_begin`) : jamais plus de 83 ms d'attente
//   active d'affilée ;
// - rien n'est effacé avant que l'en-tête soit lu et reconnu ;
// - le SHA-256 de fin d'image est vérifié sur le flux reçu, puis sur la
//   flash relue, avant de toucher à `otadata`.
//
// La flash est en mode « comme IDF » (essai 08a) : ni section critique, ni
// arrêt de l'autre cœur, le code des deux cœurs étant en PSRAM. Tout se
// passe sur le cœur 1, dans la tâche HTTP. Ce cœur ne journalise pas : les
// lignes passent par `notes`, que le cœur 0 écrit.

#![allow(
    clippy::large_stack_frames,
    reason = "futures de la tâche HTTP : clippy compte l'état de la future (tampon de 1 Ko, SHA-256), qui vit dans l'arène de l'exécuteur ; et la table de partitions de 3 Ko de `layout`, lue une fois sur la pile du cœur 0 (189 Ko)"
)]

use core::cell::RefCell;
use core::fmt::Write as _;
use core::sync::atomic::{AtomicU32, Ordering::Relaxed};

use embassy_futures::yield_now;
use embassy_net::tcp::TcpSocket;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Instant, Timer};
use esp_bootloader_esp_idf::partitions::{
    self, AppPartitionSubType, DataPartitionSubType, PartitionType,
};
use esp_storage::FlashStorage;
use log::info;
use sha2::{Digest, Sha256};
use trial_08b_ota::image::{self, HEAD_BYTES, SHA_BYTES};
use trial_08b_ota::otadata::Otadata;

const SECTOR: usize = FlashStorage::SECTOR_SIZE as usize;
const BLOCK: u32 = FlashStorage::BLOCK_SIZE;

/// Où sont `otadata` et les deux slots, et lequel a démarré.
#[derive(Clone, Copy)]
pub struct Layout {
    pub otadata: u32,
    /// (début, taille) de `ota_0` et `ota_1`.
    pub slots: [(u32, u32); 2],
    pub booted: u32,
}

/// Pilote de la flash « comme IDF ».
pub fn storage(flash: esp_storage::Flash<'static>) -> FlashStorage<'static> {
    // Le cœur 1 ne lit jamais la flash : son code et ses rodata sont en
    // PSRAM depuis `xip::run`, qui précède son démarrage.
    unsafe { FlashStorage::new(flash).multicore_ignore() }
}

/// Lit la table de partitions et le slot démarré. À appeler avant
/// `xip::run` : `booted_partition` lit la première entrée de la MMU, que le
/// remappage fait pointer en PSRAM.
pub fn layout(storage: &mut FlashStorage<'_>) -> Option<Layout> {
    let mut table = [0u8; partitions::PARTITION_TABLE_MAX_LEN];
    let Ok(pt) = partitions::read_partition_table(storage, &mut table) else {
        info!("ota: table de partitions illisible");
        return None;
    };
    let find = |kind| pt.find_partition(kind).ok().flatten().map(|p| (p.offset(), p.len()));
    let otadata = find(PartitionType::Data(DataPartitionSubType::Ota));
    let ota0 = find(PartitionType::App(AppPartitionSubType::Ota0));
    let ota1 = find(PartitionType::App(AppPartitionSubType::Ota1));
    let booted = match pt.booted_partition().ok().flatten().map(|p| p.partition_type()) {
        Some(PartitionType::App(AppPartitionSubType::Ota0)) => Some(0),
        Some(PartitionType::App(AppPartitionSubType::Ota1)) => Some(1),
        _ => None,
    };
    match (otadata, ota0, ota1, booted) {
        (Some((otadata, 0x2000)), Some(ota0), Some(ota1), Some(booted)) => {
            Some(Layout { otadata, slots: [ota0, ota1], booted })
        }
        _ => {
            info!("ota: otadata, ota_0, ota_1 ou slot démarré introuvable");
            None
        }
    }
}

/// Lignes de journal du cœur 1, écrites par `log_task` sur le cœur 0.
static NOTES: critical_section::Mutex<RefCell<heapless::Deque<heapless::String<160>, 8>>> =
    critical_section::Mutex::new(RefCell::new(heapless::Deque::new()));

fn note(args: core::fmt::Arguments<'_>) {
    let mut line = heapless::String::new();
    let _ = line.write_fmt(args);
    critical_section::with(|cs| {
        let _ = NOTES.borrow_ref_mut(cs).push_back(line);
    });
}

fn notes_empty() -> bool {
    critical_section::with(|cs| NOTES.borrow_ref(cs).is_empty())
}

/// Une ligne toutes les 20 ms au plus, comme le rapport.
#[embassy_executor::task]
pub async fn log_task() {
    loop {
        Timer::after(Duration::from_millis(20)).await;
        if let Some(line) = critical_section::with(|cs| NOTES.borrow_ref_mut(cs).pop_front()) {
            info!("ota: {line}");
        }
    }
}

/// État de la réception, pour `/stats` et le rapport.
pub const IDLE: u32 = 0;
pub const RECEIVING: u32 = 1;
pub const CHECKING: u32 = 2;
pub const REBOOTING: u32 = 3;
pub const FAILED: u32 = 4;
pub const STATE_NAMES: [&str; 5] = ["attente", "reception", "controle", "redemarrage", "echec"];
pub static STATE: AtomicU32 = AtomicU32::new(IDLE);
pub static RECEIVED: AtomicU32 = AtomicU32::new(0);
pub static EXPECTED: AtomicU32 = AtomicU32::new(0);

struct Installer {
    flash: Option<(FlashStorage<'static>, Layout)>,
    /// Une page de 4 Ko, alignée sur 4 : esp-storage écrit sans recopie.
    page: [u32; SECTOR / 4],
}

/// Une seule réception à la fois : la tâche HTTP qui prend le verrou
/// possède la flash jusqu'à la fin. Construit en place, jamais sur la pile.
static INSTALLER: Mutex<CriticalSectionRawMutex, Installer> =
    Mutex::new(Installer { flash: None, page: [0; SECTOR / 4] });

pub fn init(storage: FlashStorage<'static>, layout: Layout) {
    if let Ok(mut inst) = INSTALLER.try_lock() {
        inst.flash = Some((storage, layout));
    }
}

/// Ce que la réponse HTTP rapporte.
pub struct Done {
    pub bytes: u32,
    pub ms: u64,
    pub slot: u32,
    pub version: heapless::String<32>,
    pub project: heapless::String<32>,
}

#[derive(Debug)]
pub enum Refusal {
    Busy,
    Size,
    Head(&'static str),
    Network,
    Flash,
    Sha,
    Readback,
    Otadata,
}

impl Refusal {
    /// Pour la réponse JSON : sans guillemets.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Busy => "occupe",
            Self::Size => "taille",
            Self::Head(what) => what,
            Self::Network => "reseau",
            Self::Flash => "flash",
            Self::Sha => "sha256",
            Self::Readback => "relecture",
            Self::Otadata => "otadata",
        }
    }

    pub fn status(&self) -> &'static str {
        match self {
            Self::Busy => "409 Conflict",
            Self::Size | Self::Head(_) | Self::Network | Self::Sha => "400 Bad Request",
            Self::Flash | Self::Readback | Self::Otadata => "500 Internal Server Error",
        }
    }
}

fn bytes_of(page: &mut [u32; SECTOR / 4]) -> &mut [u8; SECTOR] {
    unsafe { &mut *(page as *mut [u32; SECTOR / 4]).cast() }
}

/// Reçoit `size` octets, dont `first` déjà lus avec l'en-tête HTTP, et les
/// installe dans le slot qui n'a pas démarré.
pub async fn receive(
    socket: &mut TcpSocket<'_>,
    first: &[u8],
    size: usize,
) -> Result<Done, Refusal> {
    let mut guard = INSTALLER.try_lock().map_err(|_| Refusal::Busy)?;
    let Installer { flash: Some((storage, layout)), page } = &mut *guard else {
        return Err(Refusal::Busy);
    };
    let mut inst = Target { storage, layout: *layout, page };
    STATE.store(RECEIVING, Relaxed);
    EXPECTED.store(size as u32, Relaxed);
    RECEIVED.store(0, Relaxed);
    let result = install(&mut inst, socket, first, size).await;
    STATE.store(if result.is_ok() { REBOOTING } else { FAILED }, Relaxed);
    if let Err(err) = &result {
        note(format_args!("refus {err:?} apres {} octets", RECEIVED.load(Relaxed)));
    }
    result
}

/// La flash, et la page, pendant une réception.
struct Target<'a> {
    storage: &'a mut FlashStorage<'static>,
    layout: Layout,
    page: &'a mut [u32; SECTOR / 4],
}

async fn install(
    inst: &mut Target<'_>,
    socket: &mut TcpSocket<'_>,
    first: &[u8],
    size: usize,
) -> Result<Done, Refusal> {
    let target = 1 - inst.layout.booted;
    let (slot_start, slot_len) = inst.layout.slots[target as usize];
    if size < HEAD_BYTES + SHA_BYTES || size > slot_len as usize {
        return Err(Refusal::Size);
    }
    let slot_end = slot_start + slot_len;
    let started = Instant::now();
    let hashed = size - SHA_BYTES;
    let mut sha = Sha256::new();
    let mut tail = [0u8; SHA_BYTES];
    let mut head = None;
    let mut done = 0usize;
    let mut fill = 0usize;
    let mut chunk = [0u8; 1024];
    let mut pending = first;

    while done < size {
        if pending.is_empty() {
            let n = socket.read(&mut chunk).await.map_err(|_| Refusal::Network)?;
            if n == 0 {
                return Err(Refusal::Network);
            }
            pending = &chunk[..n];
        }
        let n = pending.len().min(size - done).min(SECTOR - fill);
        let data = &pending[..n];
        pending = &pending[n..];

        if done < hashed {
            sha.update(&data[..n.min(hashed - done)]);
        }
        for (i, &b) in data.iter().enumerate() {
            if done + i >= hashed {
                tail[done + i - hashed] = b;
            }
        }
        bytes_of(inst.page)[fill..fill + n].copy_from_slice(data);
        fill += n;
        done += n;
        RECEIVED.store(done as u32, Relaxed);

        if head.is_none() && done >= HEAD_BYTES {
            let page = bytes_of(inst.page);
            let h = image::check_head(page[..HEAD_BYTES].try_into().unwrap())
                .map_err(Refusal::Head)?;
            note(format_args!(
                "image {} {} {} octets -> ota_{}",
                h.project, h.version, size, target
            ));
            head = Some(h);
        }
        if fill == SECTOR || done == size {
            let at = slot_start + (done - fill) as u32;
            flush(inst, at, fill, slot_end)?;
            fill = 0;
            yield_now().await;
        }
    }
    let ms = started.elapsed().as_millis();
    note(format_args!(
        "recu {} octets en {} ms ({} Ko/s)",
        size,
        ms,
        (size as u64 / 1024 * 1000).checked_div(ms).unwrap_or(0)
    ));
    if sha.finalize()[..] != tail {
        return Err(Refusal::Sha);
    }

    STATE.store(CHECKING, Relaxed);
    readback(inst, slot_start, size, &tail).await?;
    note(format_args!("relecture ok, SHA-256 de fin d'image juste"));

    let mut ota = Otadata::read(inst.storage, inst.layout.otadata)
        .map_err(|_| Refusal::Otadata)?;
    note(format_args!("avant {ota}"));
    let result = ota.set_boot(inst.storage, target);
    note(format_args!("apres {ota}"));
    result.map_err(|_| Refusal::Otadata)?;

    let head = head.unwrap();
    Ok(Done { bytes: size as u32, ms, slot: target, version: head.version, project: head.project })
}

/// Écrit la page (`len` octets, complétés à 4 par 0xFF) à `at`. Le bloc de
/// 64 Ko qui commence à `at` est effacé d'abord, sans dépasser le slot.
fn flush(inst: &mut Target<'_>, at: u32, len: usize, slot_end: u32) -> Result<(), Refusal> {
    if at.is_multiple_of(BLOCK) {
        let end = (at + BLOCK).min(slot_end);
        inst.storage.erase(at, end).map_err(|_| Refusal::Flash)?;
    }
    let padded = len.next_multiple_of(4);
    let page = bytes_of(inst.page);
    page[len..padded].fill(0xFF);
    inst.storage.write_nor(at, &page[..padded]).map_err(|_| Refusal::Flash)
}

/// Relit l'image écrite : son SHA-256 doit retomber sur celui de la fin.
async fn readback(
    inst: &mut Target<'_>,
    start: u32,
    size: usize,
    tail: &[u8; SHA_BYTES],
) -> Result<(), Refusal> {
    let hashed = size - SHA_BYTES;
    let mut sha = Sha256::new();
    let mut read_tail = [0u8; SHA_BYTES];
    let mut offset = 0;
    while offset < size {
        let n = (size - offset).min(SECTOR);
        let padded = n.next_multiple_of(4);
        inst.storage
            .read_nor(start + offset as u32, &mut bytes_of(inst.page)[..padded])
            .map_err(|_| Refusal::Flash)?;
        let page = bytes_of(inst.page);
        if offset < hashed {
            sha.update(&page[..n.min(hashed - offset)]);
        }
        for i in 0..n {
            if offset + i >= hashed {
                read_tail[offset + i - hashed] = page[i];
            }
        }
        offset += n;
        yield_now().await;
    }
    if sha.finalize()[..] != *tail || read_tail != *tail {
        return Err(Refusal::Readback);
    }
    Ok(())
}

/// Après la réponse : laisser le cœur 0 vider le journal, puis redémarrer.
pub async fn reboot() -> ! {
    let deadline = Instant::now() + Duration::from_secs(2);
    while !notes_empty() && Instant::now() < deadline {
        Timer::after(Duration::from_millis(20)).await;
    }
    Timer::after(Duration::from_millis(200)).await;
    esp_hal::system::software_reset()
}
