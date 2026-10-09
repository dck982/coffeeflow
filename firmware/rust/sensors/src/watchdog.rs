//! Watchdog RTC. `esp_hal::init` coupe tous les watchdogs : sans cela, une
//! boucle figée laisserait les sorties dans leur dernier état. On le réarme au
//! début de `main` (reset du système au bout de `TIMEOUT`) ; la boucle de
//! sécurité le nourrit, et l'effacement de la flash entre deux blocs.

use core::cell::RefCell;

use critical_section::Mutex;
use esp_hal::peripherals::RTC_TIMER;
use esp_hal::rtc_cntl::{Rtc, RwdtStage};

/// Largement plus que le tour de la boucle de sécurité (10 ms, plus une
/// écriture de flash de quelques ms ou un bloc de 64 Ko effacé en 83 ms),
/// assez court pour qu'un blocage coupe la chauffe vite : le reset remet les
/// broches au repos.
const TIMEOUT: esp_hal::time::Duration = esp_hal::time::Duration::from_secs(3);

// `Rwdt` ne se construit pas hors d'esp-hal : on garde le `Rtc` entier.
static RTC: Mutex<RefCell<Option<Rtc<'static>>>> = Mutex::new(RefCell::new(None));

pub fn arm(rtc_timer: RTC_TIMER<'static>) {
    let mut rtc = Rtc::new(rtc_timer);
    rtc.rwdt.set_timeout(RwdtStage::Stage0, TIMEOUT);
    rtc.rwdt.enable();
    rtc.rwdt.feed();
    critical_section::with(|cs| RTC.borrow_ref_mut(cs).replace(rtc));
}

/// Quelques écritures de registre, sous section critique.
pub fn feed() {
    critical_section::with(|cs| {
        if let Some(rtc) = RTC.borrow_ref_mut(cs).as_mut() {
            rtc.rwdt.feed();
        }
    });
}
