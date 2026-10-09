// Essai 10 : module ajouté à la copie vendorée d'esp-rtos. Il garde la trace
// des piles que le scheduler alloue pour les tâches créées par les blobs
// radio (`task_create`), les remplit d'un motif à la création, et permet de
// relire jusqu'où chacune est descendue. Les interruptions s'exécutent sur la
// pile de la tâche courante : un débordement d'une de ces piles abîme le tas
// voisin sans que la tâche fautive s'en aperçoive.

use core::{
    sync::atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering::*},
};

/// Octets de pile ajoutés à chaque tâche radio, fixés au build par la
/// variable d'environnement `TRIAL_EXTRA_TASK_STACK` (0 par défaut).
pub(crate) const EXTRA_TASK_STACK: usize = match option_env!("TRIAL_EXTRA_TASK_STACK") {
    Some(text) => parse(text),
    None => 0,
};

/// Motif de remplissage des piles.
pub(crate) const PATTERN: u32 = 0xA5C3_5A3C;

const fn parse(text: &str) -> usize {
    let bytes = text.as_bytes();
    let mut value = 0;
    let mut i = 0;
    while i < bytes.len() {
        assert!(
            bytes[i].is_ascii_digit(),
            "TRIAL_EXTRA_TASK_STACK doit être un entier décimal"
        );
        value = value * 10 + (bytes[i] - b'0') as usize;
        i += 1;
    }
    value
}

const SLOTS: usize = 16;
const NAME_LEN: usize = 16;

struct Slot {
    /// 0 = libre. Sert de réservation (compare_exchange) puis d'identifiant.
    base: AtomicUsize,
    ready: AtomicBool,
    size: AtomicUsize,
    requested: AtomicUsize,
    guard_end: AtomicUsize,
    core: AtomicUsize,
    name: [AtomicU8; NAME_LEN],
    name_len: AtomicUsize,
}

#[allow(clippy::declare_interior_mutable_const, reason = "modèle d'initialisation du tableau")]
const EMPTY: Slot = Slot {
    base: AtomicUsize::new(0),
    ready: AtomicBool::new(false),
    size: AtomicUsize::new(0),
    requested: AtomicUsize::new(0),
    guard_end: AtomicUsize::new(0),
    core: AtomicUsize::new(NO_CORE),
    name: [const { AtomicU8::new(0) }; NAME_LEN],
    name_len: AtomicUsize::new(0),
};
static TABLE: [Slot; SLOTS] = [EMPTY; SLOTS];

/// Cœur inconnu : la tâche n'est pas épinglée.
pub const NO_CORE: usize = 255;

/// Enregistre une pile qui vient d'être allouée et la remplit du motif.
///
/// # Safety
/// `base..base + size` est une zone allouée à cette tâche, sans autre usage.
pub(crate) unsafe fn register(
    name: &str,
    base: usize,
    size: usize,
    requested: usize,
    guard_end: usize,
    core: usize,
) {
    let mut p = base as *mut u32;
    let end = (base + size) as *mut u32;
    while p < end {
        unsafe {
            p.write_volatile(PATTERN);
            p = p.add(1);
        }
    }
    for slot in &TABLE {
        if slot.base.compare_exchange(0, base, AcqRel, Relaxed).is_ok() {
            slot.size.store(size, Relaxed);
            slot.requested.store(requested, Relaxed);
            slot.guard_end.store(guard_end, Relaxed);
            slot.core.store(core, Relaxed);
            let n = name.len().min(NAME_LEN);
            for (cell, byte) in slot.name.iter().zip(name.as_bytes()[..n].iter()) {
                cell.store(*byte, Relaxed);
            }
            slot.name_len.store(n, Relaxed);
            slot.ready.store(true, Release);
            return;
        }
    }
}

/// Retire la pile libérée de la table.
pub(crate) fn unregister(base: usize) {
    for slot in &TABLE {
        if slot.base.load(Acquire) == base {
            slot.ready.store(false, Release);
            slot.base.store(0, Release);
        }
    }
}

/// État d'une pile de tâche radio.
#[derive(Clone, Copy)]
pub struct TaskStack {
    name: [u8; NAME_LEN],
    name_len: usize,
    /// Adresse du bas de la pile.
    pub base: usize,
    /// Taille réelle allouée, en octets.
    pub size: usize,
    /// Taille demandée par le blob, en octets.
    pub requested: usize,
    /// Cœur sur lequel la tâche est épinglée, `NO_CORE` sinon.
    pub core: usize,
    /// Octets utilisés au plus haut, depuis le haut de la pile.
    pub used: usize,
}

impl TaskStack {
    /// Nom donné par le blob, tronqué à 16 octets.
    pub fn name(&self) -> &str {
        core::str::from_utf8(&self.name[..self.name_len]).unwrap_or("?")
    }
}

/// Pour chaque tâche radio vivante : taille et plus haut niveau utilisé
/// (premier mot différent du motif depuis le bas, garde sautée). Lire la
/// pile d'une tâche qui tourne est sans danger : on compare des mots.
pub fn task_stacks() -> [Option<TaskStack>; SLOTS] {
    let mut out = [None; SLOTS];
    for (slot, out) in TABLE.iter().zip(out.iter_mut()) {
        if !slot.ready.load(Acquire) {
            continue;
        }
        let base = slot.base.load(Acquire);
        let size = slot.size.load(Relaxed);
        let mut p = slot.guard_end.load(Relaxed);
        let top = base + size;
        while p < top && unsafe { (p as *const u32).read_volatile() } == PATTERN {
            p += 4;
        }
        let mut name = [0; NAME_LEN];
        let name_len = slot.name_len.load(Relaxed);
        for (byte, cell) in name.iter_mut().zip(slot.name.iter()).take(name_len) {
            *byte = cell.load(Relaxed);
        }
        // La tâche a pu être détruite pendant la lecture.
        if !slot.ready.load(Acquire) || slot.base.load(Acquire) != base {
            continue;
        }
        *out = Some(TaskStack {
            name,
            name_len,
            base,
            size,
            requested: slot.requested.load(Relaxed),
            core: slot.core.load(Relaxed),
            used: top - p,
        });
    }
    out
}
