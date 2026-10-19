//! Watchdog RTC. `esp_hal::init` coupe tous les watchdogs : sans cela, un
//! cœur 0 figé laisserait la carte immobile. On le réarme au début de `main`
//! (panique à 6 s, reset 2 s plus tard) ; la boucle de rendu le nourrit.

use core::cell::RefCell;

use critical_section::Mutex;
use esp_hal::peripherals::RTC_TIMER;
use esp_hal::rtc_cntl::{Rtc, RwdtStage, RwdtStageAction};

// `Rwdt` ne se construit pas hors d'esp-hal : on garde le `Rtc` entier.
static RTC: Mutex<RefCell<Option<Rtc<'static>>>> = Mutex::new(RefCell::new(None));

/// Délais largement au-dessus d'un tour de la boucle de rendu (10 ms plus un
/// rendu complet de quelques dizaines de ms) : 8 s au total, comme avant.
/// Avant le reset, une interruption qui panique : la trace montre où le cœur 0
/// est resté. Si elle ne vient pas (interruptions masquées), le reset suit.
const WARNING: esp_hal::time::Duration = esp_hal::time::Duration::from_secs(6);
const RESET_AFTER_WARNING: esp_hal::time::Duration = esp_hal::time::Duration::from_secs(2);

#[esp_hal::handler(priority = esp_hal::interrupt::Priority::Priority2)]
fn on_starving() {
    panic!("watchdog RTC : boucle de rendu du cœur 0 arrêtée depuis 6 s");
}

pub fn arm(rtc_timer: RTC_TIMER<'static>) {
    let mut rtc = Rtc::new(rtc_timer);
    rtc.rwdt.enable();
    rtc.rwdt.set_timeout(RwdtStage::Stage0, WARNING);
    rtc.rwdt
        .set_stage_action(RwdtStage::Stage0, RwdtStageAction::Interrupt);
    rtc.rwdt.set_timeout(RwdtStage::Stage1, RESET_AFTER_WARNING);
    rtc.rwdt
        .set_stage_action(RwdtStage::Stage1, RwdtStageAction::ResetSystem);
    rtc.set_interrupt_handler(on_starving);
    rtc.rwdt.listen();
    rtc.rwdt.feed();
    critical_section::with(|cs| RTC.borrow_ref_mut(cs).replace(rtc));
}

/// Quelques écritures de registre, sous section critique : sans effet sur la
/// recopie d'affichage.
pub fn feed() {
    critical_section::with(|cs| {
        if let Some(rtc) = RTC.borrow_ref_mut(cs).as_mut() {
            rtc.rwdt.feed();
        }
    });
}
