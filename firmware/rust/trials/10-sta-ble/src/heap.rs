//! Allocateur global de l'essai, comme `screen/src/heap.rs`.
//!
//! Sans `psram-heap` : tout va au tas interne (comportement des essais
//! précédents). Avec `psram-heap` : le cœur 0 alloue en PSRAM, le cœur 1 en
//! SRAM interne ; les `malloc` des blobs radio vont droit à
//! `esp_alloc::HEAP`, qui n'a que de la SRAM interne.

use core::alloc::{GlobalAlloc, Layout};

/// Tas interne, à créer avant la PSRAM et avant toute allocation : 120 Ko,
/// dont 73744 octets récupérés du bootloader et le reste en .bss.
pub fn init_internal() {
    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: crate::HEAP_RECLAIMED);
    esp_alloc::heap_allocator!(size: crate::HEAP_TOTAL - crate::HEAP_RECLAIMED);
}

/// Sans `psram-heap` : esp-alloc n'a plus son allocateur global (feature
/// coupée pour que `PerCoreHeap` puisse exister), on renvoie au tas interne.
#[cfg(not(feature = "psram-heap"))]
struct InternalOnly;

#[cfg(not(feature = "psram-heap"))]
unsafe impl GlobalAlloc for InternalOnly {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        unsafe { esp_alloc::HEAP.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { esp_alloc::HEAP.dealloc(ptr, layout) }
    }
}

#[cfg(not(feature = "psram-heap"))]
#[global_allocator]
static ALLOCATOR: InternalOnly = InternalOnly;

#[cfg(feature = "psram-heap")]
pub use psram::init_psram;

#[cfg(feature = "psram-heap")]
mod psram {
    use super::*;
    use esp_hal::system::Cpu;

    /// Tas du cœur 0, en PSRAM.
    static PSRAM_HEAP: esp_alloc::EspHeap = esp_alloc::EspHeap::empty();

    /// Bus de données de la PSRAM sur ESP32-S3 : toute adresse de
    /// `PSRAM_HEAP` y tombe, aucune de `esp_alloc::HEAP`.
    const PSRAM_DATA_BUS: core::ops::Range<usize> = 0x3C00_0000..0x3E00_0000;

    /// Allocateur global : le cœur 0 alloue en PSRAM, le cœur 1 en SRAM
    /// interne. La libération suit l'adresse du bloc, quel que soit le cœur
    /// qui le rend.
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

    /// Tas PSRAM sur `[start, start + size)`.
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
}
