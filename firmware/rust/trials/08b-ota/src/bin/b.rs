#![no_std]
#![no_main]

// Essai 08b, Rust B : une image minimale, sans affichage ni radio, qui joue
// un cas de validation OTA choisi au build. Elle journalise l'état OTA au
// démarrage, puis :
//
// - `b-valide` : au bout de 5 s, passe sa copie d'`otadata` de
//   `PENDING_VERIFY` à `VALID`, et reste ;
// - `b-sans-validation` : redémarre au bout de 30 s sans se valider ; le
//   bootloader marque alors la copie `ABORTED` et revient sur l'image
//   précédente, comme la sonde de l'étape 9 ;
// - `b-panique` : panique au bout de 5 s ; `custom_halt` redémarre ;
// - `b-blocage` : arme le watchdog RTC à 5 s, puis se bloque au bout de
//   5 s ; le watchdog redémarre la carte.
//
// Le code s'exécute depuis la flash : esp-storage garde sa section critique
// (feature `esp-storage-cs`). Le cœur 1 n'est jamais démarré.

use esp_backtrace as _;
use esp_bootloader_esp_idf::partitions::{
    self, AppPartitionSubType, DataPartitionSubType, PartitionType,
};
use esp_hal::delay::Delay;
use esp_hal::main;
use esp_hal::rtc_cntl::{Rtc, RwdtStage};
use esp_hal::time::{Duration, Instant};
use esp_storage::FlashStorage;
use log::info;
use trial_08b_ota::otadata::{Otadata, State};

esp_bootloader_esp_idf::esp_app_desc!();

// La bibliothèque de l'essai lie `alloc` (Slint) ; la Rust B n'alloue rien,
// et son tas reste vide.
#[global_allocator]
static ALLOCATOR: esp_alloc::EspHeap = esp_alloc::EspHeap::empty();

const CASES: usize = cfg!(feature = "b-valide") as usize
    + cfg!(feature = "b-sans-validation") as usize
    + cfg!(feature = "b-panique") as usize
    + cfg!(feature = "b-blocage") as usize;
const _: () = assert!(
    CASES == 1,
    "un cas et un seul : --features b-valide, b-sans-validation, b-panique ou b-blocage"
);

const CASE: &str = if cfg!(feature = "b-valide") {
    "valide"
} else if cfg!(feature = "b-sans-validation") {
    "sans-validation"
} else if cfg!(feature = "b-panique") {
    "panique"
} else {
    "blocage"
};

#[main]
fn main() -> ! {
    esp_println::logger::init_logger_from_env();
    let peripherals = esp_hal::init(esp_hal::Config::default());

    // Le bootloader de l'écran (console IDF désactivée) remet GPIO43 en
    // GPIO, sortie coupée : on rebranche U0TXD avant le premier journal.
    let _console = esp_hal::uart::UartTx::new(peripherals.UART0, esp_hal::uart::Config::default())
        .unwrap()
        .with_tx(peripherals.GPIO43);

    let delay = Delay::new();
    let start = Instant::now();
    info!("Rust B, cas {CASE}, reset: {:?}", esp_hal::system::reset_reason());

    let mut storage = FlashStorage::new(peripherals.FLASH);
    let (otadata_at, booted) = locate(&mut storage);
    info!("Rust B: démarrée sur {booted:?}, otadata à 0x{otadata_at:x}");
    let mut ota = Otadata::read(&mut storage, otadata_at).unwrap();
    info!("otadata: {ota}");
    let state = own_state(&ota, booted);
    info!("Rust B: état {state:?}");

    let mut rtc = Rtc::new(peripherals.RTC_TIMER);
    if cfg!(feature = "b-blocage") {
        rtc.rwdt.set_timeout(RwdtStage::Stage0, Duration::from_secs(5));
        rtc.rwdt.enable();
        info!("watchdog RTC armé à 5 s");
    }

    delay.delay_millis(5000);

    if cfg!(feature = "b-valide") {
        if state == Some(State::PendingVerify) {
            match ota.mark_valid(&mut storage) {
                Ok(()) => info!("validée: {ota}"),
                Err(err) => info!("validation impossible: {err:?}, {ota}"),
            }
        } else {
            info!("rien à valider");
        }
        loop {
            info!("Rust B valide, {} s", start.elapsed().as_secs());
            delay.delay_millis(10_000);
        }
    } else if cfg!(feature = "b-sans-validation") {
        for left in (5..=25).rev().step_by(5) {
            info!("Rust B sans validation, redémarrage dans {left} s");
            delay.delay_millis(5000);
        }
        info!("redémarrage");
        delay.delay_millis(50);
        esp_hal::system::software_reset()
    } else if cfg!(feature = "b-panique") {
        panic!("panique voulue de la Rust B");
    } else {
        info!("blocage, interruptions masquées");
        let _masked = unsafe { critical_section::acquire() };
        #[allow(clippy::empty_loop)]
        loop {}
    }
}

/// `otadata` et le slot démarré, depuis la table de partitions.
fn locate(storage: &mut FlashStorage<'_>) -> (u32, AppPartitionSubType) {
    let mut table = [0u8; partitions::PARTITION_TABLE_MAX_LEN];
    let pt = partitions::read_partition_table(storage, &mut table).unwrap();
    let otadata = pt
        .find_partition(PartitionType::Data(DataPartitionSubType::Ota))
        .unwrap()
        .unwrap();
    let booted = match pt.booted_partition().unwrap().unwrap().partition_type() {
        PartitionType::App(app) => app,
        other => panic!("démarrée hors d'une app: {other:?}"),
    };
    (otadata.offset(), booted)
}

/// État de la copie active, si elle désigne bien le slot démarré. Sinon, le
/// bootloader a démarré cette image pour une autre raison (retour en
/// arrière, `factory`) et elle n'a rien à valider.
fn own_state(ota: &Otadata, booted: AppPartitionSubType) -> Option<State> {
    let slot = match booted {
        AppPartitionSubType::Ota0 => 0,
        AppPartitionSubType::Ota1 => 1,
        _ => return None,
    };
    let active = ota.active()?;
    (ota.entries[active].slot() == slot).then(|| ota.entries[active].state())
}
