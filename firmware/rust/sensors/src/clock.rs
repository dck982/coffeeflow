//! Microsecondes depuis le démarrage de l'application, comme
//! `esp_timer_get_time`. Le compteur matériel (systimer) n'est pas garanti à
//! zéro après un reset logiciel : on retranche sa valeur à l'entrée dans `main`.

use core::cell::Cell;

use critical_section::{CriticalSection, Mutex};
use esp_hal::time::Instant;

static ORIGIN_US: Mutex<Cell<u64>> = Mutex::new(Cell::new(0));

fn raw_us() -> u64 {
    Instant::now().duration_since_epoch().as_micros()
}

/// À appeler une fois, au début de `main`.
pub fn init() {
    critical_section::with(|cs| ORIGIN_US.borrow(cs).set(raw_us()));
}

/// Pour l'interruption du débitmètre, déjà en section critique.
pub fn now_us_in(cs: CriticalSection<'_>) -> i64 {
    (raw_us() - ORIGIN_US.borrow(cs).get()) as i64
}

pub fn now_us() -> i64 {
    critical_section::with(now_us_in)
}
