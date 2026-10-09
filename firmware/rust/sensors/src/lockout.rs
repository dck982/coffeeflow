//! Verrou de 60 s, tenu en mémoire RTC rapide : il survit à un reset
//! logiciel, au watchdog et à une chute de tension, et ne se lève qu'au
//! démarrage à froid (`ChipPowerOn`). Une commande RESET venue du bus ne le
//! lève donc pas, sinon un écran fou l'effacerait lui-même.
//!
//! `persistent` remet la zone à zéro au power-on, et seulement là. On ajoute un
//! nombre magique, comme le C++ : une mémoire RTC jamais initialisée (premier
//! démarrage après un flash par USB, par exemple) lève le verrou aussi.

use core::ptr::{addr_of_mut, read_volatile, write_volatile};

use esp_hal::ram;
use esp_hal::rtc_cntl::SocResetReason;
use esp_hal::system::reset_reason;
use sensors_logic::lockout_after_boot;

const MAGIC: u32 = 0xC0FF_EE02;

#[ram(unstable(rtc_fast, persistent))]
static mut RTC_MAGIC: u32 = 0;
#[ram(unstable(rtc_fast, persistent))]
static mut RTC_LOCKOUT: u32 = 0;

/// Relit le verrou au démarrage, et le remet à zéro s'il le faut. À appeler
/// une fois, avant tout autre usage.
pub fn init() -> bool {
    let power_on = reset_reason() == Some(SocResetReason::ChipPowerOn);
    // SAFETY: une seule tâche écrit ces deux mots, et `init` s'exécute avant
    // le démarrage des autres.
    unsafe {
        let magic_ok = read_volatile(addr_of_mut!(RTC_MAGIC)) == MAGIC;
        let active = read_volatile(addr_of_mut!(RTC_LOCKOUT)) != 0;
        let active = lockout_after_boot(power_on, magic_ok, active);
        write_volatile(addr_of_mut!(RTC_MAGIC), MAGIC);
        write_volatile(addr_of_mut!(RTC_LOCKOUT), u32::from(active));
        active
    }
}

/// Pose le verrou. Il n'existe pas de fonction pour le lever.
pub fn set() {
    // SAFETY: voir `init`.
    unsafe { write_volatile(addr_of_mut!(RTC_LOCKOUT), 1) }
}
