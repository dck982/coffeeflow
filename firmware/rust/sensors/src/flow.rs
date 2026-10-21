//! Débitmètre Digmesa sur GPIO 44 : front descendant, sans pull-up interne
//! (la pull-up de 1 kΩ et le filtre de la carte fournissent un 3,3 V
//! propre). Une interruption incrémente le compteur et date le dernier
//! front ; la boucle de sécurité le lit pour `STATUS_FLOW`.

use core::cell::{Cell, RefCell};

use critical_section::Mutex;
use esp_hal::gpio::{Event, Input, InputConfig, Io, Pull};
use esp_hal::handler;
use esp_hal::peripherals::{GPIO44, IO_MUX};
use sensors_logic::FlowCounter;

use crate::clock;

static PIN: Mutex<RefCell<Option<Input<'static>>>> = Mutex::new(RefCell::new(None));
static COUNTER: Mutex<Cell<FlowCounter>> = Mutex::new(Cell::new(FlowCounter::new()));

#[handler]
fn on_edge() {
    critical_section::with(|cs| {
        let mut pin = PIN.borrow_ref_mut(cs);
        let Some(pin) = pin.as_mut() else { return };
        if !pin.is_interrupt_set() {
            return;
        }
        pin.clear_interrupt();
        let mut counter = COUNTER.borrow(cs).get();
        // L'horloge vient du même compteur que celui des ticks.
        if counter.on_edge(clock::now_us_in(cs)) {
            COUNTER.borrow(cs).set(counter);
        }
    });
}

/// GPIO 44 est le RX par défaut de la console UART0 : `Input::new` remet le
/// pad en fonction GPIO, sinon il garde celle de U0RXD et l'interruption ne
/// voit jamais rien.
///
/// Le pad passe en GPIO avant l'installation du gestionnaire : dans l'ordre
/// inverse, la carte se bloquait dans `Input::new` jusqu'au watchdog.
pub fn init(io_mux: IO_MUX<'static>, pin: GPIO44<'static>) {
    let mut input = Input::new(pin, InputConfig::default().with_pull(Pull::None));
    let mut io = Io::new(io_mux);
    io.set_interrupt_handler(on_edge);
    critical_section::with(|cs| {
        input.listen(Event::FallingEdge);
        PIN.borrow_ref_mut(cs).replace(input);
    });
}

pub fn read() -> FlowCounter {
    critical_section::with(|cs| COUNTER.borrow(cs).get())
}
