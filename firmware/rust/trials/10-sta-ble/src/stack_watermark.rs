//! Plus haut niveau des piles des deux cœurs, pour le rapport périodique.
//!
//! La SRAM est trop juste pour garder la pile de 57 Ko des essais : celle du
//! cœur 0 n'a plus que ce que le linker laisse, et elle porte maintenant Slint,
//! la logique et le JSON. On remplit chaque pile d'un motif au démarrage, puis
//! on cherche le mot le plus bas qui l'a perdu : la marge se lit sur carte au
//! lieu de se deviner.
//!
//! Le bas de chaque pile est laissé tel quel : esp-hal y place sa garde
//! (60 octets au-dessus du bas) et la surveille par un point d'arrêt sur
//! écriture. La peindre déclencherait une fausse alerte de débordement.

use core::sync::atomic::{AtomicUsize, Ordering::Relaxed};

const PATTERN: u32 = 0xA5C3_5A3C;
/// Au-delà de la garde d'esp-hal, alignée sur 16.
const GUARD_SKIP: usize = 128;
/// Ce qu'on laisse sous le pointeur de pile courant : la trame de `paint_core0`
/// et ce que pourrait y écrire une interruption pendant le remplissage.
const BELOW_SP: usize = 512;

struct Region {
    bottom: AtomicUsize,
    top: AtomicUsize,
}

impl Region {
    const fn new() -> Self {
        Self {
            bottom: AtomicUsize::new(0),
            top: AtomicUsize::new(0),
        }
    }
}

static CORE0: Region = Region::new();
static CORE1: Region = Region::new();

unsafe extern "C" {
    static _stack_end_cpu0: u32;
    static _stack_start_cpu0: u32;
}

/// Peint la pile du cœur 0 sous le pointeur courant. À appeler en tout début
/// de `main` : ce qui a déjà servi au-dessus compte comme utilisé, ce qui
/// surestime un peu, jamais l'inverse.
#[inline(never)]
pub fn paint_core0() {
    let bottom = core::ptr::addr_of!(_stack_end_cpu0) as usize;
    let top = core::ptr::addr_of!(_stack_start_cpu0) as usize;
    // Une variable locale vaut le pointeur de pile à une trame près : l'asm
    // en ligne n'est pas stable sur Xtensa, et BELOW_SP couvre l'écart.
    let marker = 0u8;
    let sp = core::hint::black_box(&marker) as *const u8 as usize;
    unsafe { fill(bottom + GUARD_SKIP, (sp - BELOW_SP) & !3) };
    CORE0.bottom.store(bottom, Relaxed);
    CORE0.top.store(top, Relaxed);
}

/// Peint toute la pile du cœur 1, avant qu'il ne démarre dessus.
pub fn paint_core1(bottom: *mut u8, bytes: usize) {
    let bottom = bottom as usize;
    unsafe { fill(bottom + GUARD_SKIP, bottom + bytes) };
    CORE1.bottom.store(bottom, Relaxed);
    CORE1.top.store(bottom + bytes, Relaxed);
}

unsafe fn fill(from: usize, to: usize) {
    let mut p = from as *mut u32;
    while (p as usize) < to {
        unsafe {
            p.write_volatile(PATTERN);
            p = p.add(1);
        }
    }
}

/// Octets utilisés au plus haut, et taille totale. `None` avant le
/// remplissage. Lire la pile de l'autre cœur pendant qu'il tourne est sans
/// danger : on ne fait que comparer des mots.
pub fn usage(core: usize) -> Option<(usize, usize)> {
    let region = if core == 0 { &CORE0 } else { &CORE1 };
    let bottom = region.bottom.load(Relaxed);
    let top = region.top.load(Relaxed);
    if bottom == 0 {
        return None;
    }
    let mut p = (bottom + GUARD_SKIP) as *const u32;
    // esp-hal dépose la fermeture de démarrage du cœur 1 au bas de sa pile,
    // juste au-dessus de la garde : ces octets ne sont pas de la pile utilisée.
    // On saute ce qui précède le motif, puis on mesure le motif intact.
    while core == 1 && (p as usize) < top && unsafe { p.read_volatile() } != PATTERN {
        p = unsafe { p.add(1) };
    }
    while (p as usize) < top && unsafe { p.read_volatile() } == PATTERN {
        p = unsafe { p.add(1) };
    }
    Some((top - p as usize, top - bottom))
}
