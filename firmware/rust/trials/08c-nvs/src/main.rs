#![no_std]
#![no_main]
#![allow(
    clippy::large_stack_frames,
    reason = "secteur de 4 Ko et table de partitions de 3 Ko sur la pile de main, lus une fois au démarrage"
)]

// Essai 08c : esp-nvs contre le NVS d'IDF, dans les deux sens. Un cas par
// build :
//
// - `c-factice` : une zone de 24 Ko à 2 Mo du début du slot démarré, où le
//   Mac a écrit une image de `nvs_partition_gen` (données factices,
//   `nvs/donnees.py`). La Rust relit tout, écrit, modifie, efface, puis
//   rouvre et relit. Le Mac relit ensuite la zone avec le parseur d'IDF ;
// - `c-reel-lecture` : le vrai NVS, à travers une flash qui refuse toute
//   écriture. Noms, types et longueurs seulement, puis le Wi-Fi de la
//   maison avec les identifiants de coffeeflow, jamais journalisés ;
// - `c-reel-ecriture` : le vrai NVS, une clé de test écrite et relue.
//
// Ni affichage ni radio, sauf le Wi-Fi de `c-reel-lecture`. Le code
// s'exécute depuis la flash, sur un seul cœur.

extern crate alloc;

#[cfg(feature = "c-factice")]
mod attendu;
mod flash;
#[cfg(feature = "c-reel-lecture")]
mod sta;

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::Ordering::Relaxed;

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::clock::CpuClock;
use esp_hal::timer::timg::TimerGroup;
use esp_nvs::{Get, ItemType, Key, Nvs};
use esp_storage::FlashStorage;
use log::info;

use flash::{Flash, Layout, REFUSED, WRITES, region_crc};

esp_bootloader_esp_idf::esp_app_desc!();

const CASES: usize = cfg!(feature = "c-factice") as usize
    + cfg!(feature = "c-reel-lecture") as usize
    + cfg!(feature = "c-reel-ecriture") as usize;
const _: () = assert!(
    CASES == 1,
    "un cas et un seul : --features c-factice, c-reel-lecture ou c-reel-ecriture"
);

const CASE: &str = if cfg!(feature = "c-factice") {
    "factice"
} else if cfg!(feature = "c-reel-lecture") {
    "reel-lecture"
} else {
    "reel-ecriture"
};

#[cfg(feature = "c-factice")]
/// Zone factice : à 2 Mo du début du slot démarré, la taille du vrai NVS.
const ZONE_AT: u32 = 0x20_0000;
#[cfg(feature = "c-factice")]
const ZONE_LEN: u32 = 0x6000;

#[cfg(feature = "c-reel-ecriture")]
/// Clé de `c-reel-ecriture`, dans un espace que coffeeflow n'ouvre pas.
const TEST_NS: &str = "rust8c";
#[cfg(feature = "c-reel-ecriture")]
const TEST_KEY: &str = "essai";
#[cfg(feature = "c-reel-ecriture")]
const TEST_VALUE: u32 = 0x08C0_FFEE;

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    esp_println::logger::init_logger_from_env();
    let peripherals = esp_hal::init(esp_hal::Config::default().with_cpu_clock(CpuClock::max()));

    // Le bootloader de l'écran (console IDF désactivée) remet GPIO43 en
    // GPIO, sortie coupée : on rebranche U0TXD avant le premier journal.
    let _console = esp_hal::uart::UartTx::new(peripherals.UART0, esp_hal::uart::Config::default())
        .unwrap()
        .with_tx(peripherals.GPIO43);

    // Le tas de l'exemple Wi-Fi d'esp-radio : esp-nvs y tient ses pages.
    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 64 * 1024);
    esp_alloc::heap_allocator!(size: 36 * 1024);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0, peripherals.FROM_CPU_INTR0);

    info!("Essai 08c, cas {CASE}, reset: {:?}", esp_hal::system::reset_reason());
    let mut storage = FlashStorage::new(peripherals.FLASH);
    let layout = flash::layout(&mut storage);
    info!(
        "démarré sur {} 0x{:x}+0x{:x}, nvs 0x{:x}+0x{:x}",
        layout.booted_name, layout.booted.0, layout.booted.1, layout.nvs.0, layout.nvs.1
    );

    #[cfg(feature = "c-factice")]
    factice(storage, &layout);
    #[cfg(feature = "c-reel-ecriture")]
    reel_ecriture(storage, &layout);
    #[cfg(feature = "c-reel-lecture")]
    if let Some((ssid, password)) = reel_lecture(storage, &layout) {
        sta::start(spawner, peripherals.WIFI, &ssid, &password);
    }
    let _ = spawner;

    loop {
        Timer::after(Duration::from_secs(60)).await;
        info!("cas {CASE} terminé");
    }
}

/// Ouvre la partition ; journalise les écritures que l'ouverture a faites
/// ou tentées.
fn open(storage: FlashStorage<'static>, at: u32, len: u32, writable: bool) -> Option<Nvs<Flash>> {
    let (writes, refused) = (WRITES.load(Relaxed), REFUSED.load(Relaxed));
    let result = Nvs::new(at as usize, len as usize, Flash::new(storage, writable));
    info!(
        "ouverture ({}) : {} écriture(s), {} refusée(s)",
        if writable { "écriture permise" } else { "lecture seule" },
        WRITES.load(Relaxed) - writes,
        REFUSED.load(Relaxed) - refused
    );
    match result {
        Ok(nvs) => Some(nvs),
        Err(err) => {
            info!("ouverture impossible : {err:?}");
            None
        }
    }
}

/// Espace, clé, type et longueur de chaque entrée. Les valeurs sont lues
/// pour leur longueur, jamais affichées.
fn list(nvs: &mut Nvs<Flash>) {
    let entries: Vec<_> = nvs.typed_entries().collect();
    info!("{} entrée(s)", entries.len());
    for entry in entries {
        let (ns, key, kind) = match entry {
            Ok(entry) => entry,
            Err(err) => {
                info!("  entrée illisible : {err:?}");
                continue;
            }
        };
        let len = match kind {
            ItemType::U8 | ItemType::I8 => Ok(1),
            ItemType::U16 | ItemType::I16 => Ok(2),
            ItemType::U32 | ItemType::I32 => Ok(4),
            ItemType::U64 | ItemType::I64 => Ok(8),
            ItemType::Sized => <Nvs<Flash> as Get<String>>::get(nvs, &ns, &key).map(|s| s.len()),
            _ => <Nvs<Flash> as Get<Vec<u8>>>::get(nvs, &ns, &key).map(|b| b.len()),
        };
        match len {
            Ok(len) => info!("  {}/{} {kind} {len} octet(s)", ns.as_str(), key.as_str()),
            Err(err) => info!("  {}/{} {kind} illisible : {err:?}", ns.as_str(), key.as_str()),
        }
    }
}

#[cfg(not(feature = "c-reel-ecriture"))]
/// SSID et mot de passe de `wifi/credentials` (`Credentials` de
/// net_wifi.cpp : ssid[33] puis password[64], terminés par un zéro).
/// Journalise leurs longueurs seulement.
fn credentials(nvs: &mut Nvs<Flash>) -> Option<(String, String)> {
    let blob: Vec<u8> = match nvs.get(&Key::from_str("wifi"), &Key::from_str("credentials")) {
        Ok(blob) => blob,
        Err(err) => {
            info!("wifi/credentials : {err:?}");
            return None;
        }
    };
    if blob.len() != 97 {
        info!("wifi/credentials : {} octets, 97 attendus", blob.len());
        return None;
    }
    let field = |bytes: &[u8]| {
        let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
        String::from_utf8(bytes[..end].to_vec()).ok()
    };
    let (Some(ssid), Some(password)) = (field(&blob[..33]), field(&blob[33..])) else {
        info!("wifi/credentials : pas de l'UTF-8");
        return None;
    };
    info!("wifi/credentials : SSID de {} car., mot de passe de {} car.", ssid.len(), password.len());
    Some((ssid, password))
}

#[cfg(feature = "c-factice")]
fn factice(mut storage: FlashStorage<'static>, layout: &Layout) {
    let (slot, slot_len) = layout.booted;
    let at = slot + ZONE_AT;
    let image_end = flash::image_end(&mut storage, slot);
    info!(
        "zone 0x{at:x}+0x{ZONE_LEN:x}, fin de l'image 0x{image_end:x}, fin du slot 0x{:x}",
        slot + slot_len
    );
    assert!(
        image_end <= at && at + ZONE_LEN <= slot + slot_len,
        "zone sur l'image ou hors du slot"
    );
    info!("crc de la zone au départ : 0x{:08x}", region_crc(&mut storage, at, ZONE_LEN));

    let Some(mut nvs) = open(storage, at, ZONE_LEN, true) else { return };
    list(&mut nvs);
    let ecarts_avant = attendu::controler(&mut nvs, &attendu::avant(), false);
    let _ = credentials(&mut nvs);
    info!("relecture de l'image d'IDF : {ecarts_avant} écart(s)");

    // Chaque écriture USB redémarre la carte, et l'essai peut tourner avant
    // que la console soit ouverte. Il n'écrit que dans une zone intacte ;
    // sinon, il contrôle seulement l'état d'après.
    if ecarts_avant != 0 {
        info!("zone pas intacte : rien n'est écrit, contrôle de l'état d'après seulement");
        let ecarts = attendu::controler(&mut nvs, &attendu::apres(), true);
        info!("résultat : zone déjà passée, après {ecarts} écart(s)");
        return;
    }
    let erreurs = attendu::modifier(&mut nvs);
    info!("écritures : {erreurs} erreur(s)");

    // Une nouvelle ouverture repart de la flash seule.
    let storage = nvs.into_inner().into_inner();
    let Some(mut nvs) = open(storage, at, ZONE_LEN, true) else { return };
    list(&mut nvs);
    let ecarts_apres = attendu::controler(&mut nvs, &attendu::apres(), true);
    let mut storage = nvs.into_inner().into_inner();
    info!("crc de la zone à la fin : 0x{:08x}", region_crc(&mut storage, at, ZONE_LEN));
    info!(
        "résultat : avant {ecarts_avant} écart(s), écritures {erreurs} erreur(s), après {ecarts_apres} écart(s)"
    );
    info!("zone à relire sur le Mac : esptool read-flash 0x{at:x} 0x{ZONE_LEN:x}");
}

#[cfg(feature = "c-reel-lecture")]
fn reel_lecture(mut storage: FlashStorage<'static>, layout: &Layout) -> Option<(String, String)> {
    let (at, len) = layout.nvs;
    let crc = region_crc(&mut storage, at, len);
    info!("crc du nvs au départ : 0x{crc:08x}");
    let mut nvs = open(storage, at, len, false)?;
    list(&mut nvs);
    let found = credentials(&mut nvs);
    let mut storage = nvs.into_inner().into_inner();
    let after = region_crc(&mut storage, at, len);
    info!(
        "crc du nvs à la fin : 0x{after:08x} ({})",
        if after == crc { "inchangé" } else { "CHANGÉ" }
    );
    found
}

#[cfg(feature = "c-reel-ecriture")]
fn reel_ecriture(mut storage: FlashStorage<'static>, layout: &Layout) {
    let (at, len) = layout.nvs;
    info!("crc du nvs au départ : 0x{:08x}", region_crc(&mut storage, at, len));
    let Some(mut nvs) = open(storage, at, len, true) else { return };
    let (ns, key) = (Key::from_str(TEST_NS), Key::from_str(TEST_KEY));
    match nvs.set(&ns, &key, TEST_VALUE) {
        Ok(()) => info!("{TEST_NS}/{TEST_KEY} = 0x{TEST_VALUE:08x} écrit"),
        Err(err) => info!("{TEST_NS}/{TEST_KEY} : écriture impossible, {err:?}"),
    }
    let storage = nvs.into_inner().into_inner();
    let Some(mut nvs) = open(storage, at, len, true) else { return };
    match <Nvs<Flash> as Get<u32>>::get(&mut nvs, &ns, &key) {
        Ok(v) if v == TEST_VALUE => info!("{TEST_NS}/{TEST_KEY} relu après réouverture : ok"),
        other => info!("{TEST_NS}/{TEST_KEY} relu : {other:?}"),
    }
    list(&mut nvs);
    let mut storage = nvs.into_inner().into_inner();
    info!(
        "crc du nvs à la fin : 0x{:08x}, {} écriture(s) en tout",
        region_crc(&mut storage, at, len),
        WRITES.load(Relaxed)
    );
}

/// Fin d'une panique : redémarrer (essai 08b), plutôt que boucler.
#[unsafe(no_mangle)]
extern "Rust" fn custom_halt() -> ! {
    esp_hal::delay::Delay::new().delay_millis(50);
    esp_hal::system::software_reset()
}
