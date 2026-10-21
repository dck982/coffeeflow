#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

// Module capteurs de coffeeflow (XIAO ESP32-S3) : vanne, pompe (DimmerLink),
// chauffe, débitmètre, et les messages de sécurité du bus CAN. La logique est
// dans `sensors-logic`, testée sur le Mac ; ce crate ne fait que câbler le
// matériel.
//
// Une tâche porte la logique (trames reçues, tick de sécurité toutes les
// 100 ms), une autre le contrôleur TWAI.

mod board;
mod can;
mod clock;
mod flash;
mod flow;
mod led;
mod lockout;
mod otadata;
mod watchdog;

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer, with_timeout};
use esp_backtrace as _;
use esp_hal::clock::CpuClock;
use esp_hal::gpio::{Flex, Level, Output, OutputConfig, Pull};
use esp_hal::i2c::master::{Config as I2cConfig, I2c, SoftwareTimeout};
use esp_hal::time::Rate;
use esp_hal::timer::timg::TimerGroup;
use esp_hal::twai::{BaudRate, TwaiConfiguration, TwaiMode};
use log::{info, warn};
use sensors_logic::machine::{DIMMER_ADDR, DIMMER_REG_STATUS, TICK_PERIOD_US};
use sensors_logic::{Action, Config, Machine};
use static_cell::StaticCell;

use crate::board::{Board, HEATER_INACTIVE};
use crate::flash::FlashOta;

esp_bootloader_esp_idf::esp_app_desc!();

/// Fin d'une panique, après la trace d'esp-backtrace. Par défaut, il boucle
/// sans fin, interruptions masquées, et `esp_hal::init` a coupé tous les
/// watchdogs : la carte resterait figée, chauffe comprise. On redémarre, pour
/// que le bootloader revienne sur l'image précédente si celle-ci n'est pas
/// validée. 50 ms laissent l'UART vider sa FIFO.
#[unsafe(no_mangle)]
extern "Rust" fn custom_halt() -> ! {
    esp_hal::delay::Delay::new().delay_millis(50);
    esp_hal::system::software_reset()
}

const VALIDATE: bool = !cfg!(feature = "no-validate");

static MACHINE: StaticCell<Machine<Board>> = StaticCell::new();

#[allow(
    clippy::large_stack_frames,
    reason = "main prépare le matériel une fois ; clippy compte l'état de la future, qui vit dans l'arène de l'exécuteur"
)]
#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default().with_cpu_clock(CpuClock::_160MHz));

    // Sorties sûres avant tout le reste, CAN compris. La chauffe (HEATER_CMD)
    // et la vanne sont basses ; `Output::new` pose le niveau avant d'activer
    // la sortie. Pas de pull-up sur la chauffe : sans le GPIO, la résistance
    // de grille de Q3 tient la sortie BOILER coupée.
    let heater = Output::new(
        peripherals.GPIO4,
        HEATER_INACTIVE,
        OutputConfig::default().with_pull(Pull::None),
    );
    let valve = Output::new(peripherals.GPIO9, Level::Low, OutputConfig::default());
    // LED d'état de la carte, allumée dès ici (voir `led`).
    let led = Output::new(peripherals.GPIO2, Level::High, OutputConfig::default());

    // esp_hal::init a coupé les watchdogs : on réarme celui du RTC tout de suite.
    watchdog::arm(peripherals.RTC_TIMER);
    clock::init();
    let boot_us = clock::now_us();

    // Console sur l'USB du XIAO quand un hôte y est branché, sinon sur
    // U0TXD (GPIO43, non câblé sur la carte). GPIO44, le RX de l'UART0, est
    // le débitmètre : on ne le branche pas. Le pilote vit jusqu'à la fin de
    // main, qui ne rend jamais la main.
    esp_println::logger::init_logger_from_env();
    let _console = esp_hal::uart::UartTx::new(peripherals.UART0, esp_hal::uart::Config::default())
        .unwrap()
        .with_tx(peripherals.GPIO43);

    let lockout_active = lockout::init();
    info!(
        "sensors {}, reset: {:?}, verrou {}, validation {}",
        protocol::FIRMWARE_VERSION,
        esp_hal::system::reset_reason(),
        if lockout_active { "actif" } else { "libre" },
        if VALIDATE { "oui" } else { "non (no-validate)" },
    );

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0, peripherals.FROM_CPU_INTR0);

    // NEW ou PENDING_VERIFY. On ne valide jamais l'image ici : la logique ne
    // le fait qu'après un PING/PONG confirmé sur le bus, sinon elle revient
    // en arrière au bout de 30 s.
    let mut ota = FlashOta::new(peripherals.FLASH);
    let ota_pending = ota.pending_state();
    if let Some(state) = ota_pending {
        info!("image en attente de confirmation, état otadata {state}");
    }

    // TWAI : 500 kbit/s, tout accepter (filtre par défaut).
    let twai = TwaiConfiguration::new(
        peripherals.TWAI0,
        peripherals.GPIO8,
        peripherals.GPIO7,
        BaudRate::B500K,
        TwaiMode::Normal,
    )
    .into_async()
    .start();
    spawner.spawn(can::task(twai).unwrap());

    // Bus I2C du dimmer et des puces de la carte. Ses seules pull-ups sont
    // les 4,7 kΩ de la carte (R13/R14) : ni pull-up interne en plus, d'où des
    // broches `Flex` configurées à la main (le pilote n'y touche plus).
    let mut sda = Flex::new(peripherals.GPIO5);
    let mut scl = Flex::new(peripherals.GPIO6);
    for pin in [&mut sda, &mut scl] {
        pin.apply_output_config(
            &OutputConfig::default().with_drive_mode(esp_hal::gpio::DriveMode::OpenDrain),
        );
        pin.set_input_enable(true);
        pin.set_output_enable(true);
        pin.set_high();
    }
    let mut i2c = I2c::new(
        peripherals.I2C0,
        I2cConfig::default()
            .with_frequency(Rate::from_khz(100))
            .with_software_timeout(SoftwareTimeout::Transaction(
                esp_hal::time::Duration::from_millis(100),
            )),
    )
    .unwrap()
    .with_sda(sda)
    .with_scl(scl);
    let led = led::Led::new(led, boot_fault(&mut i2c), boot_us);

    let board = Board {
        valve,
        heater,
        i2c,
        ota,
    };
    let machine = MACHINE.init(Machine::new(
        board,
        Config {
            lockout_active,
            ota_pending,
            validate: VALIDATE,
        },
    ));
    machine.start();
    flow::init(peripherals.IO_MUX, peripherals.GPIO44);
    machine.ready();

    logic(machine, led).await
}

/// Une lecture de registre par puce, pour la LED : les puces de la carte
/// d'abord, puis le dimmer (son registre STATUS, comme sa relecture
/// périodique).
fn boot_fault(i2c: &mut I2c<'static, esp_hal::Blocking>) -> Option<led::BootFault> {
    let mut buf = [0u8; 2];
    let tmp102 = i2c
        .write_read(
            board::TMP102_ADDR,
            &[board::TMP102_REG_TEMPERATURE],
            &mut buf,
        )
        .is_ok();
    let ads1115 = i2c
        .write_read(board::ADS1115_ADDR, &[board::ADS1115_REG_CONFIG], &mut buf)
        .is_ok();
    let dimmer = i2c
        .write_read(DIMMER_ADDR as u8, &[DIMMER_REG_STATUS], &mut buf[..1])
        .is_ok();
    let fault = if !(tmp102 && ads1115) {
        Some(led::BootFault::Board)
    } else if !dimmer {
        Some(led::BootFault::Dimmer)
    } else {
        None
    };
    info!("i2c : tmp102 {tmp102}, ads1115 {ads1115}, dimmer {dimmer} -> led {fault:?}");
    fault
}

/// Reçoit les trames, déroule le tick de sécurité et exécute les
/// redémarrages demandés.
async fn logic(machine: &'static mut Machine<Board>, mut led: led::Led) -> ! {
    let mut next_tick_us = clock::now_us() + TICK_PERIOD_US;
    loop {
        led.update(clock::now_us());
        // Réveil toutes les 10 ms au plus : c'est la cadence de la tâche
        // pression du C++, et la précision du tick de sécurité.
        if let Ok(frame) = with_timeout(Duration::from_millis(10), can::RX.receive()).await {
            led.on_frame(clock::now_us());
            machine.on_frame(&frame);
            while let Ok(frame) = can::RX.try_receive() {
                machine.on_frame(&frame);
            }
        }
        if clock::now_us() >= next_tick_us {
            machine.tick_safety();
            // Délai après le travail, comme `vTaskDelay` en fin de boucle.
            next_tick_us = clock::now_us() + TICK_PERIOD_US;
        }
        machine.tick_pressure();
        watchdog::feed();

        match machine.take_action() {
            Action::None => {}
            Action::Reboot => reboot().await,
            Action::RollbackReboot => {
                // Laisser partir le LOG, puis invalider l'image et redémarrer.
                Timer::after(Duration::from_millis(50)).await;
                machine.hw.ota.invalidate();
                warn!("retour en arrière");
                reboot().await
            }
        }
    }
}

async fn reboot() -> ! {
    // Laisser partir le LOG ou l'accusé avant le redémarrage.
    Timer::after(Duration::from_millis(50)).await;
    esp_hal::system::software_reset()
}
