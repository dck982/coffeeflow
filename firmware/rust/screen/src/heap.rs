//! Deux tas : la SRAM interne pour la radio et le cœur 1, la PSRAM pour le
//! cœur 0 (Slint, logique, réseau applicatif).
//!
//! Pourquoi deux : le code des blobs radio et les tampons DMA doivent être en
//! SRAM interne, alors que la SRAM ne suffit pas à Slint. L'allocateur global
//! choisit selon le cœur ; les `malloc` des blobs radio ne passent pas par lui
//! mais vont droit à `esp_alloc::HEAP`, qui n'a que de la SRAM interne.

use core::alloc::{GlobalAlloc, Layout};

use esp_hal::system::Cpu;

/// Tas du cœur 0, en PSRAM après le code et le framebuffer.
pub static PSRAM_HEAP: esp_alloc::EspHeap = esp_alloc::EspHeap::empty();

/// Bus de données de la PSRAM sur ESP32-S3 : toute adresse de `PSRAM_HEAP`
/// y tombe, aucune de `esp_alloc::HEAP`.
const PSRAM_DATA_BUS: core::ops::Range<usize> = 0x3C00_0000..0x3E00_0000;

// Tas interne : 128 Ko, dont 72 récupérés du bootloader et le reste en .bss,
// pris sur la pile du cœur 0.
const HEAP_RECLAIMED: usize = 73744;
const HEAP_TOTAL: usize = 128 * 1024;

/// Allocateur global : le cœur 0 alloue en PSRAM, le cœur 1 en SRAM interne.
/// La libération suit l'adresse du bloc, quel que soit le cœur qui le rend.
struct PerCoreHeap;

unsafe impl GlobalAlloc for PerCoreHeap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        match Cpu::current() {
            Cpu::ProCpu => unsafe { PSRAM_HEAP.alloc(layout) },
            _ => unsafe { esp_alloc::HEAP.alloc(layout) },
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if PSRAM_DATA_BUS.contains(&(ptr as usize)) {
            unsafe { PSRAM_HEAP.dealloc(ptr, layout) }
        } else {
            unsafe { esp_alloc::HEAP.dealloc(ptr, layout) }
        }
    }
}

#[global_allocator]
static ALLOCATOR: PerCoreHeap = PerCoreHeap;

/// Tas interne, à créer avant la PSRAM et avant toute allocation.
pub fn init_internal() {
    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: HEAP_RECLAIMED);
    esp_alloc::heap_allocator!(size: HEAP_TOTAL - HEAP_RECLAIMED);
}

/// Tas PSRAM sur `[start, start + size)`, une fois le code déplacé en PSRAM
/// et le framebuffer réservé.
///
/// # Safety
/// La zone est de la PSRAM libre, que rien d'autre n'utilise.
pub unsafe fn init_psram(start: *mut u8, size: usize) {
    unsafe {
        PSRAM_HEAP.add_region(esp_alloc::HeapRegion::new(
            start,
            size,
            esp_alloc::MemoryCapability::External.into(),
        ));
    }
}
