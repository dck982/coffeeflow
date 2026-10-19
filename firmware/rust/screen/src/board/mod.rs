//! La carte Waveshare 7" : bus I2C partagé, expandeur CH422G, mise sous
//! tension de la dalle, tactile GT911 et ADS1115 de la chaudière. Le brochage
//! est celui de `board.h` et `service_screen.cpp` du C++.

pub mod ads1115;
pub mod ch422g;
pub mod gt911;
pub mod sht40;

use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Timer};
use esp_hal::Async;
use esp_hal::gpio::{Level, Output, OutputConfig};
use esp_hal::i2c::master::{Config as I2cConfig, I2c};
use esp_hal::peripherals::{GPIO4, GPIO8, GPIO9, I2C0};
use esp_hal::time::Rate;
use log::info;
use static_cell::StaticCell;

use ch422g::{Ch422g, EXIO_LCD_RST, EXIO_TP_RST};
use gt911::Gt911;

pub const H_RES: usize = 800;
pub const V_RES: usize = 480;

/// Bus I2C0 (SDA GPIO8, SCL GPIO9). Le mutex est sans masquage
/// d'interruptions : toutes les tâches qui s'en servent tournent sur
/// l'exécuteur du cœur 0, et une transaction asynchrone ne doit pas bloquer
/// la recopie de l'affichage.
pub type Bus = Mutex<NoopRawMutex, I2c<'static, Async>>;
/// Un composant du bus ; l'adresse est choisie à chaque appel.
pub type Device = I2cDevice<'static, NoopRawMutex, I2c<'static, Async>>;

pub struct Board {
    pub bus: &'static Bus,
    #[allow(dead_code, reason = "la tâche CAN y mettra CAN_SEL haut")]
    pub expander: Ch422g<Device>,
    /// Absent si le GT911 ne répond pas : l'écran vit sans toucher.
    pub touch: Option<Gt911<Device>>,
    // L'IRQ du GT911 reste tenue basse depuis le reset (adresse 0x5D).
    _touch_irq: Output<'static>,
}

impl Board {
    /// Un accès au bus pour un nouveau pilote (ADS1115, ...).
    #[allow(dead_code, reason = "utilisée par les tâches des tâches suivantes")]
    pub fn device(&self) -> Device {
        I2cDevice::new(self.bus)
    }
}

/// Crée le bus et met la dalle sous tension, comme `board::panel_power_on()`
/// du C++ : GT911 IRQ bas pendant le reset, DISP haut, LCD_RST et TP_RST bas
/// 100 ms puis hauts 200 ms. CAN_SEL reste bas.
#[allow(
    clippy::large_stack_frames,
    reason = "futures d'embassy : clippy compte l'état de la future, qui vit dans l'arène de l'exécuteur ou la future de main, pas sur la pile"
)]
pub async fn init(
    i2c0: I2C0<'static>,
    sda: GPIO8<'static>,
    scl: GPIO9<'static>,
    touch_irq: GPIO4<'static>,
) -> Board {
    let touch_irq = Output::new(touch_irq, Level::Low, OutputConfig::default());
    let i2c = I2c::new(i2c0, I2cConfig::default().with_frequency(Rate::from_khz(400)))
        .unwrap()
        .with_sda(sda)
        .with_scl(scl)
        .into_async();
    static BUS: StaticCell<Bus> = StaticCell::new();
    let bus: &'static Bus = BUS.init(Mutex::new(i2c));

    let mut expander = Ch422g::new(I2cDevice::new(bus)).await.expect("CH422G absent");
    expander.set_bit(EXIO_LCD_RST, false).await.expect("CH422G");
    expander.set_bit(EXIO_TP_RST, false).await.expect("CH422G");
    Timer::after(Duration::from_millis(100)).await;
    expander.set_bit(EXIO_LCD_RST, true).await.expect("CH422G");
    expander.set_bit(EXIO_TP_RST, true).await.expect("CH422G");
    Timer::after(Duration::from_millis(200)).await;
    info!("CH422G: dalle alimentée, CAN_SEL bas");

    let touch = Gt911::probe(I2cDevice::new(bus)).await;
    match &touch {
        Some(t) => info!("GT911 à 0x{:02x}", t.addr()),
        None => info!("GT911 absent : pas de toucher"),
    }

    Board { bus, expander, touch, _touch_irq: touch_irq }
}

/// Cycle de lecture A0, A1, A2 de l'ADS1115, sur l'exécuteur du cœur 0.
#[embassy_executor::task]
#[allow(
    clippy::large_stack_frames,
    reason = "futures d'embassy : clippy compte l'état de la future, qui vit dans l'arène de l'exécuteur ou la future de main, pas sur la pile"
)]
pub async fn ads1115_task(i2c: Device) -> ! {
    ads1115::run(i2c).await
}

/// Lecture du SHT40 du boîtier toutes les secondes, sur l'exécuteur du cœur 0.
#[embassy_executor::task]
#[allow(
    clippy::large_stack_frames,
    reason = "futures d'embassy : clippy compte l'état de la future, qui vit dans l'arène de l'exécuteur ou la future de main, pas sur la pile"
)]
pub async fn sht40_task(i2c: Device) -> ! {
    sht40::run(i2c).await
}
