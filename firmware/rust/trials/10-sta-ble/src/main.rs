#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

// Essai 10 : Wi-Fi station et BLE ensemble sur le cœur 1, hors du firmware
// `screen`, pour reproduire son plantage (cœur 1 : LoadStoreError dans
// `pm_process_tim` du blob Wi-Fi ; cœur 0 : PC nul dans la file de tâches
// d'esp-rtos). Hypothèse : débordement des piles des tâches créées par les
// blobs radio, alors que les essais précédents n'avaient combiné BLE et
// Wi-Fi qu'en point d'accès.
//
// Cœur 0 : initialisation, watchdog, tas, démarrage du cœur 1, puis une
// tâche de rapport. Ni LCD, ni Slint, ni PSRAM, ni écriture en flash.
// Étages BLE par features cargo (`ble-controller`, `ble-host`, `ble-scan`),
// `no-wifi` pour le BLE seul, `ps-min` pour l'économie d'énergie Wi-Fi.
//
// Le `esp-rtos` vendoré (`vendor/esp-rtos`) remplit de motif la pile de
// chaque tâche radio et la mesure ; `TRIAL_EXTRA_TASK_STACK=<octets>` au
// build en ajoute à chacune.

extern crate alloc;

#[cfg(feature = "ble-controller")]
mod ble;
mod breadcrumb;
mod core1;
mod heap;
#[cfg(feature = "core0-load")]
mod load;
#[cfg(not(feature = "no-wifi"))]
mod flash;
#[cfg(not(feature = "no-wifi"))]
mod nvs;
mod report;
mod stack_watermark;
mod stats;
#[cfg(not(feature = "no-wifi"))]
mod sta;
mod watchdog;
#[cfg(feature = "xip")]
mod xip;

use embassy_executor::Spawner;
use esp_backtrace as _;
use esp_hal::clock::CpuClock;
use esp_hal::timer::timg::TimerGroup;
use log::info;

esp_bootloader_esp_idf::esp_app_desc!();

/// Tas interne de l'écran : 120 Ko, dont 73744 octets récupérés du
/// bootloader et le reste en .bss.
const HEAP_RECLAIMED: usize = 73744;
const HEAP_TOTAL: usize = 120 * 1024;

/// Fin d'une panique, après la trace d'esp-backtrace : redémarrer, plutôt
/// que de boucler interruptions masquées. 50 ms laissent l'UART se vider.
#[unsafe(no_mangle)]
extern "Rust" fn custom_halt() -> ! {
    esp_hal::delay::Delay::new().delay_millis(50);
    esp_hal::system::software_reset()
}

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    // Avant tout le reste : le plus haut niveau de pile se mesure depuis ici.
    stack_watermark::paint_core0();
    esp_println::logger::init_logger_from_env();

    let peripherals = esp_hal::init(esp_hal::Config::default().with_cpu_clock(CpuClock::max()));

    // esp_hal::init a coupé les watchdogs : on réarme celui du RTC tout de
    // suite, la tâche de rapport le nourrira.
    watchdog::arm(peripherals.RTC_TIMER);

    // Le bootloader de l'écran (console IDF désactivée) remet GPIO43 en
    // GPIO, sortie coupée : on rebranche U0TXD avant le premier journal.
    let _console = esp_hal::uart::UartTx::new(peripherals.UART0, esp_hal::uart::Config::default())
        .unwrap()
        .with_tx(peripherals.GPIO43);
    info!("Essai 10, reset: {:?}", esp_hal::system::reset_reason());
    breadcrumb::report_previous();
    crumb!();

    heap::init_internal();
    info!("tas interne: {} octets", esp_alloc::HEAP.free());

    // PSRAM comme `screen` (octale, 80 MHz). Avec `xip`, le code et les
    // rodata sont copiés au début de la PSRAM avant tout autre usage de
    // celle-ci et avant le démarrage du cœur 1 ; le tas PSRAM du cœur 0
    // vient après. Sans `xip`, il occupe toute la PSRAM.
    #[cfg(feature = "psram-heap")]
    let _psram = {
        use esp_hal::psram::{Psram, PsramConfig, PsramMode, SpiRamFreq};
        let psram = Psram::new(
            peripherals.PSRAM,
            PsramConfig {
                mode: PsramMode::OctalSpi,
                ram_frequency: SpiRamFreq::Freq80m,
                ..Default::default()
            },
        );
        let (psram_start, psram_size) = psram.raw_parts();
        info!("PSRAM: {psram_size} octets à {psram_start:p}");
        #[cfg(feature = "xip")]
        let heap_start = {
            let t0 = esp_hal::time::Instant::now();
            let bytes = xip::run(psram_start, psram_size);
            info!(
                "XIP: {} pages de code et rodata en PSRAM en {} ms",
                xip::PAGES.load(core::sync::atomic::Ordering::Relaxed),
                t0.elapsed().as_millis()
            );
            bytes
        };
        #[cfg(not(feature = "xip"))]
        let heap_start = 0;
        assert!(
            psram_size > heap_start + 1024 * 1024,
            "PSRAM absente ou trop petite"
        );
        unsafe {
            heap::init_psram(
                psram_start.wrapping_add(heap_start),
                psram_size - heap_start,
            )
        };
        info!(
            "tas: {} octets en PSRAM (cœur 0), {} en interne (cœur 1)",
            psram_size - heap_start,
            esp_alloc::HEAP.free()
        );
        psram
    };

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0, peripherals.FROM_CPU_INTR0);

    crumb!();
    core1::start(
        peripherals.CPU_CTRL,
        peripherals.FROM_CPU_INTR1,
        peripherals.WIFI,
        peripherals.BT,
        peripherals.FLASH,
    );
    // Après la radio, comme l'écran qui démarre la dalle une fois le cœur 1
    // lancé.
    #[cfg(feature = "core0-load")]
    {
        load::start_interrupt(peripherals.TIMG1);
        spawner.spawn(load::work().unwrap());
        spawner.spawn(load::events().unwrap());
    }
    spawner.spawn(report::report().unwrap());
    loop {
        embassy_time::Timer::after_secs(3600).await;
    }
}
