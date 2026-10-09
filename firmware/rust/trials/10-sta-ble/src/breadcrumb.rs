//! Diagnostic : dernier point de passage de chaque cœur, gardé en RAM RTC
//! lente à travers un reset du watchdog. Au démarrage suivant, on affiche où
//! chaque cœur était quand la carte s'est figée, sans dépendre de la console
//! au moment du blocage.

use esp_hal::ram;

const MAGIC: u32 = 0xB5EA_D001;

// RAM RTC lente : accessible des deux cœurs, gardée par un reset système.
// [0] = MAGIC ; puis par cœur : fichier, longueur, ligne, passages, instant
// (ms depuis le démarrage).
#[ram(unstable(rtc_slow, persistent))]
static mut SLOTS: [u32; 11] = [0; 11];

fn slot(index: usize) -> *mut u32 {
    unsafe { core::ptr::addr_of_mut!(SLOTS).cast::<u32>().add(index) }
}
fn get(index: usize) -> u32 {
    unsafe { slot(index).read_volatile() }
}
fn set(index: usize, value: u32) {
    unsafe { slot(index).write_volatile(value) }
}

#[doc(hidden)]
#[inline(always)]
pub fn mark(file: &'static str, line: u32) {
    let base = 1 + 5 * esp_hal::system::Cpu::current() as usize;
    set(base + 4, (esp_hal::time::Instant::now().duration_since_epoch().as_millis()) as u32);
    set(base, file.as_ptr() as u32);
    set(base + 1, file.len() as u32);
    set(base + 2, line);
    set(base + 3, get(base + 3).wrapping_add(1));
}

/// Note le passage du cœur courant ici.
#[macro_export]
macro_rules! crumb {
    () => {
        $crate::breadcrumb::mark(file!(), line!())
    };
}

/// Affiche les points laissés par le démarrage précédent, puis les efface.
/// Les chaînes de `file!()` sont dans les rodata de la même image : on les
/// relit telles quelles.
pub fn report_previous() {
    if get(0) == MAGIC {
        for core in 0..2 {
            let base = 1 + 5 * core;
            let ptr = get(base) as *const u8;
            let len = get(base + 1) as usize;
            let file = if ptr.is_null() || len > 128 {
                "?"
            } else {
                unsafe { core::str::from_utf8(core::slice::from_raw_parts(ptr, len)).unwrap_or("?") }
            };
            esp_println::println!(
                "[diag] avant reset, cœur {core} : {file}:{} ({} passages, à {} ms)",
                get(base + 2),
                get(base + 3),
                get(base + 4)
            );
        }
    } else {
        esp_println::println!("[diag] aucun point de passage précédent");
    }
    for index in 1..11 {
        set(index, 0);
    }
    set(0, MAGIC);
}
