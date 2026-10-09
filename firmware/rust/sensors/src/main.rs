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
use sensors_logic::machine::TICK_PERIOD_US;
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

    // Sorties sûres avant tout le reste, CAN compris. Le latch du SSR
    // chaudière est préchargé à l'état inactif (HIGH, le niveau bas chauffe)
    // avant que le pilote de sortie ne s'active, avec la pull-up du chemin
    // HW-399 ; la vanne est basse. `Output::new` pose le niveau avant
    // d'activer la sortie.
    let heater = Output::new(
        peripherals.GPIO3,
        HEATER_INACTIVE,
        OutputConfig::default().with_pull(Pull::Up),
    );
    let valve = Output::new(peripherals.GPIO9, Level::Low, OutputConfig::default());
    // LED du XIAO, allumée au niveau bas, visible du haut de la machine sans
    // console : fixe dès ici, elle clignote une fois la boucle de logique
    // lancée. Fixe puis éteinte au bout de 3 s : blocage, puis watchdog.
    let led = Output::new(peripherals.GPIO21, Level::Low, OutputConfig::default());

    // esp_hal::init a coupé les watchdogs : on réarme celui du RTC tout de suite.
    watchdog::arm(peripherals.RTC_TIMER);
    clock::init();

    // TWAI : 500 kbit/s, tout accepter (filtre par défaut), RX GPIO8 et TX
    // GPIO7. Démarré tôt, en mode bloquant, pour les jalons du démarrage ; il
    // passe en asynchrone juste avant la boucle de logique.
    let mut twai = TwaiConfiguration::new(
        peripherals.TWAI0,
        peripherals.GPIO8,
        peripherals.GPIO7,
        BaudRate::B500K,
        TwaiMode::Normal,
    )
    .start();
    can::milestone(&mut twai, 1);

    // U0TXD sur GPIO43 (D6 du shield), pour la console. GPIO44, le RX de
    // l'UART0, est le débitmètre : on ne le branche pas. Le pilote vit
    // jusqu'à la fin de main, qui ne rend jamais la main.
    esp_println::logger::init_logger_from_env();
    let _console = esp_hal::uart::UartTx::new(peripherals.UART0, esp_hal::uart::Config::default())
        .unwrap()
        .with_tx(peripherals.GPIO43);
    can::milestone(&mut twai, 2);

    let lockout_active = lockout::init();
    can::milestone(&mut twai, 3);
    info!(
        "sensors {}, reset: {:?}, verrou {}, validation {}",
        protocol::FIRMWARE_VERSION,
        esp_hal::system::reset_reason(),
        if lockout_active { "actif" } else { "libre" },
        if VALIDATE { "oui" } else { "non (no-validate)" },
    );

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0, peripherals.FROM_CPU_INTR0);
    can::milestone(&mut twai, 4);

    // NEW ou PENDING_VERIFY. On ne valide jamais l'image ici : la logique ne
    // le fait qu'après un PING/PONG confirmé sur le bus, sinon elle revient
    // en arrière au bout de 30 s.
    let mut ota = FlashOta::new(peripherals.FLASH);
    can::milestone(&mut twai, 5);
    let ota_pending = ota.pending_state();
    can::milestone(&mut twai, 6);
    if let Some(state) = ota_pending {
        info!("image en attente de confirmation, état otadata {state}");
    }

    // Bus I2C du dimmer. Ses seules pull-ups sont les 4,7 kΩ du module Grove
    // branché sur R1 : ni pull-up interne ni autre jeu en plus, d'où des
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
    let i2c = I2c::new(
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
    can::milestone(&mut twai, 7);

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
    can::milestone(&mut twai, 8);
    flow::init(peripherals.IO_MUX, peripherals.GPIO44);
    can::milestone(&mut twai, 9);
    machine.ready();

    let twai = twai.stop().into_async().start();
    spawner.spawn(can::task(twai).unwrap());
    // Le dernier jalon part par la tâche CAN : il prouve aussi l'émission
    // asynchrone et l'exécuteur.
    let _ = can::TX.try_send(can::milestone_frame(10));

    logic(machine, led).await
}

/// Reçoit les trames, déroule le tick de sécurité et exécute les
/// redémarrages demandés.
async fn logic(machine: &'static mut Machine<Board>, mut led: Output<'static>) -> ! {
    let mut next_tick_us = clock::now_us() + TICK_PERIOD_US;
    let mut next_blink_us = clock::now_us();
    loop {
        if clock::now_us() >= next_blink_us {
            led.toggle();
            next_blink_us = clock::now_us() + 500_000;
        }
        // Réveil toutes les 10 ms au plus : c'est la cadence de la tâche
        // pression du C++, et la précision du tick de sécurité.
        if let Ok(frame) = with_timeout(Duration::from_millis(10), can::RX.receive()).await {
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
