//! Exécution depuis la PSRAM (équivalent de `CONFIG_SPIRAM_XIP_FROM_PSRAM`
//! d'IDF) : les pages de flash qui portent le code et les rodata sont
//! copiées en PSRAM, puis leurs entrées MMU sont redirigées vers ces copies.
//!
//! Pourquoi : la flash (DIO) et la PSRAM (octale DDR) partagent le bus du
//! cache. Un défaut de cache d'instructions lu en flash occupe ce bus ~4×
//! plus longtemps qu'en PSRAM, et retarde d'autant la recopie des bounce
//! buffers. Une fois le code en PSRAM, la flash n'est plus lue en marche
//! normale, ce qui servira aussi aux écritures en flash.
//!
//! Sur l'ESP32-S3, le bus d'instructions (0x4200_0000) et celui de données
//! (0x3C00_0000) partagent la même table MMU de 512 entrées de 64 Ko :
//! rediriger une entrée déplace à la fois le code et les rodata de la page.

use core::sync::atomic::{AtomicU32, Ordering::Relaxed};

const MMU_TABLE: *mut u32 = 0x600c_5000 as *mut u32;
const MMU_ENTRIES: usize = 512;
const MMU_INVALID: u32 = 1 << 14;
const MMU_ACCESS_SPIRAM: u32 = 1 << 15;
const MMU_PAGE_NUMBER: u32 = 0x3fff;
const PAGE_BYTES: usize = 64 * 1024;
const DBUS_BASE: usize = 0x3c00_0000;
const EXTMEM_CACHE_STATE: *const u32 = (0x600c_4000 + 0x130) as *const u32;
const CACHE_STATE_IDLE: u32 = 1;

unsafe extern "C" {
    fn rom_Cache_Suspend_ICache() -> u32;
    fn Cache_Resume_ICache(autoload: u32);
    fn rom_Cache_Suspend_DCache() -> u32;
    fn Cache_Resume_DCache(autoload: u32);
    fn Cache_Invalidate_ICache_All();
    fn Cache_Invalidate_DCache_All();
    fn Cache_WriteBack_All();
}

/// Pages remappées, pour le journal.
pub static PAGES: AtomicU32 = AtomicU32::new(0);
/// Fin, en flash physique, de la dernière page de 64 Ko que l'app mappait au
/// démarrage : rien de l'image n'est au-delà.
pub static FLASH_END: AtomicU32 = AtomicU32::new(0);

/// Entrées MMU valides qui pointent encore vers la flash, y compris la
/// dernière, que `run` laisse au bootloader.
pub fn flash_mappings() -> u32 {
    (0..MMU_ENTRIES)
        .filter(|&entry| {
            let value = unsafe { MMU_TABLE.add(entry).read_volatile() };
            value & (MMU_INVALID | MMU_ACCESS_SPIRAM) == 0
        })
        .count() as u32
}

/// Copie les pages de flash du code et des rodata au début de la PSRAM et
/// y redirige leurs entrées MMU. `psram` est l'adresse de données où
/// esp-hal a placé la page physique 0 de la PSRAM (`Psram::raw_parts`).
/// Rend le nombre d'octets de PSRAM désormais occupés par le code : le
/// reste commence à `psram + octets`.
///
/// À appeler avant de démarrer le second cœur, et avant toute utilisation
/// de la PSRAM.
pub fn run(psram: *mut u8, psram_size: usize) -> usize {
    // Pages de flash de l'app : entrées valides de type flash. La dernière
    // entrée sert au bootloader pour lire la flash ; esp-hal l'ignore aussi.
    let mut flash_pages = [0u16; MMU_ENTRIES];
    let mut n = 0;
    let mut flash_end = 0;
    for entry in 0..MMU_ENTRIES - 1 {
        let value = unsafe { MMU_TABLE.add(entry).read_volatile() };
        if value & (MMU_INVALID | MMU_ACCESS_SPIRAM) == 0 {
            flash_pages[n] = entry as u16;
            n += 1;
            flash_end = flash_end.max(((value & MMU_PAGE_NUMBER) + 1) * PAGE_BYTES as u32);
        }
    }
    FLASH_END.store(flash_end, Relaxed);
    assert!(n * PAGE_BYTES <= psram_size / 2, "code trop gros pour la PSRAM");

    // Copie, caches actifs : la page k de la PSRAM reçoit la page de flash
    // vue à travers le bus de données, puis tout est réécrit en PSRAM.
    for (k, &entry) in flash_pages[..n].iter().enumerate() {
        let src = (DBUS_BASE + entry as usize * PAGE_BYTES) as *const u32;
        let dst = psram.wrapping_add(k * PAGE_BYTES).cast::<u32>();
        unsafe { core::ptr::copy_nonoverlapping(src, dst, PAGE_BYTES / 4) };
    }
    unsafe { Cache_WriteBack_All() };

    critical_section::with(|_| unsafe { remap(flash_pages.as_ptr(), n) });

    // Contrôle : le code et les rodata relus depuis la PSRAM sont ceux de
    // la flash, page par page, à travers leurs adresses d'origine.
    for (k, &entry) in flash_pages[..n].iter().enumerate() {
        let mapped = (DBUS_BASE + entry as usize * PAGE_BYTES) as *const u32;
        let copy = psram.wrapping_add(k * PAGE_BYTES).cast::<u32>();
        for w in (0..PAGE_BYTES / 4).step_by(997) {
            assert_eq!(unsafe { mapped.add(w).read_volatile() }, unsafe {
                copy.add(w).read_volatile()
            });
        }
    }
    PAGES.store(n as u32, Relaxed);
    n * PAGE_BYTES
}

/// Redirige les entrées MMU vers les pages 0..n de la PSRAM. Caches
/// suspendus, donc rien ne doit venir de la flash ni de la PSRAM : cette
/// fonction est en IRAM et n'appelle que la ROM.
#[esp_hal::ram]
unsafe fn remap(entries: *const u16, n: usize) {
    unsafe {
        let icache = rom_Cache_Suspend_ICache();
        while EXTMEM_CACHE_STATE.read_volatile() & 0xfff != CACHE_STATE_IDLE {}
        let dcache = rom_Cache_Suspend_DCache();
        while (EXTMEM_CACHE_STATE.read_volatile() >> 12) & 0xfff != CACHE_STATE_IDLE {}

        let mut k = 0;
        while k < n {
            let entry = *entries.add(k) as usize;
            MMU_TABLE.add(entry).write_volatile(MMU_ACCESS_SPIRAM | k as u32);
            k += 1;
        }

        Cache_Invalidate_ICache_All();
        Cache_Invalidate_DCache_All();
        Cache_Resume_DCache(dcache);
        Cache_Resume_ICache(icache);
    }
}
