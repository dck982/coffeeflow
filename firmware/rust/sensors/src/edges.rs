//! Fronts descendants des deux entrées à interruption, servis par l'unique
//! gestionnaire GPIO d'esp-hal (`Io::set_interrupt_handler`) :
//!
//! - débitmètre Digmesa sur GPIO 44, sans pull-up interne (la pull-up de 1 kΩ
//!   et le filtre de la carte fournissent un 3,3 V propre) ;
//! - tachymètre de la pompe MPB003 sur GPIO 3, sans pull interne (4,7 kΩ vers
//!   le 3,3 V sur la carte) ; le signal est inversé par un optocoupleur, donc
//!   le front utile est descendant. GPIO 3 est une broche de strapping JTAG,
//!   lue seulement si un eFuse est grillé (pas par défaut) : sans effet en
//!   entrée.
//!
//! Chaque interruption incrémente le compteur et date le dernier front, avec
//! la même horloge que les ticks ; la boucle de sécurité les lit pour
//! `STATUS_FLOW` et la vitesse de la pompe.

use core::cell::{Cell, RefCell};

use critical_section::Mutex;
use esp_hal::gpio::{Event, Input, InputConfig, Io, Pull};
use esp_hal::handler;
use esp_hal::peripherals::{GPIO3, GPIO44, IO_MUX};
use sensors_logic::{FlowCounter, TachCounter};

use crate::clock;

static FLOW_PIN: Mutex<RefCell<Option<Input<'static>>>> = Mutex::new(RefCell::new(None));
static TACH_PIN: Mutex<RefCell<Option<Input<'static>>>> = Mutex::new(RefCell::new(None));
static FLOW: Mutex<Cell<FlowCounter>> = Mutex::new(Cell::new(FlowCounter::new()));
static TACH: Mutex<Cell<TachCounter>> = Mutex::new(Cell::new(TachCounter::new()));

/// Si l'interruption de `pin` est levée, l'acquitte et rend l'instant du
/// front.
fn take_edge(
    cs: critical_section::CriticalSection<'_>,
    pin: &Mutex<RefCell<Option<Input<'static>>>>,
) -> Option<i64> {
    let mut pin = pin.borrow_ref_mut(cs);
    let pin = pin.as_mut()?;
    if !pin.is_interrupt_set() {
        return None;
    }
    pin.clear_interrupt();
    // L'horloge vient du même compteur que celui des ticks.
    Some(clock::now_us_in(cs))
}

#[handler]
fn on_edge() {
    critical_section::with(|cs| {
        if let Some(now_us) = take_edge(cs, &FLOW_PIN) {
            let mut counter = FLOW.borrow(cs).get();
            if counter.on_edge(now_us) {
                FLOW.borrow(cs).set(counter);
            }
        }
        if let Some(now_us) = take_edge(cs, &TACH_PIN) {
            let mut counter = TACH.borrow(cs).get();
            if counter.on_edge(now_us) {
                TACH.borrow(cs).set(counter);
            }
        }
    });
}

/// GPIO 44 est le RX par défaut de la console UART0 : `Input::new` remet le
/// pad en fonction GPIO, sinon il garde celle de U0RXD et l'interruption ne
/// voit jamais rien.
///
/// Les pads passent en GPIO avant l'installation du gestionnaire : dans
/// l'ordre inverse, la carte se bloquait dans `Input::new` jusqu'au watchdog.
pub fn init(io_mux: IO_MUX<'static>, flow_pin: GPIO44<'static>, tach_pin: GPIO3<'static>) {
    let mut flow = Input::new(flow_pin, InputConfig::default().with_pull(Pull::None));
    let mut tach = Input::new(tach_pin, InputConfig::default().with_pull(Pull::None));
    let mut io = Io::new(io_mux);
    io.set_interrupt_handler(on_edge);
    critical_section::with(|cs| {
        flow.listen(Event::FallingEdge);
        tach.listen(Event::FallingEdge);
        FLOW_PIN.borrow_ref_mut(cs).replace(flow);
        TACH_PIN.borrow_ref_mut(cs).replace(tach);
    });
}

pub fn flow() -> FlowCounter {
    critical_section::with(|cs| FLOW.borrow(cs).get())
}

pub fn tach() -> TachCounter {
    critical_section::with(|cs| TACH.borrow(cs).get())
}

/// Vitesse visée par la consigne, référence du filtre des fronts parasites.
pub fn set_tach_target_rpm(rpm: u16) {
    critical_section::with(|cs| {
        let mut counter = TACH.borrow(cs).get();
        counter.set_target_rpm(rpm);
        TACH.borrow(cs).set(counter);
    });
}
