#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

// Firmware de l'écran Waveshare 7" : UI, cœur et entrées locales sur le
// cœur 0 ; radio, réseau et flash sur le cœur 1.
//
// Deux cœurs. Le cœur 0 porte la recopie d'affichage (interruptions de
// niveau 3), le rendu Slint et la logique ; il alloue en PSRAM. Le cœur 1,
// porte la radio, le réseau et le flash ; il alloue en SRAM interne.

extern crate alloc;

mod board;
mod core1;
mod display;
mod report;
mod runtime;
mod stack_watermark;
mod ui;
mod watchdog;
mod xip;

use core::sync::atomic::Ordering::Relaxed;

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::clock::CpuClock;
use esp_hal::gpio::Pin;
use esp_hal::psram::{Psram, PsramConfig, PsramMode, SpiRamFreq};
use esp_hal::timer::timg::TimerGroup;
use log::info;
use screen::heap;
use slint::platform::software_renderer::Rgb565Pixel;

esp_bootloader_esp_idf::esp_app_desc!();

/// Fin d'une panique, après la trace d'esp-backtrace. Par défaut, il boucle
/// sans fin, interruptions masquées, et `esp_hal::init` a coupé tous les
/// watchdogs : la carte resterait figée. On redémarre, pour que le
/// bootloader revienne sur l'image précédente si celle-ci n'est pas validée.
/// 50 ms laissent l'UART vider sa FIFO.
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
    screen::serial_bridge::init_logger().expect("journal déjà installé");

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    // esp_hal::init a coupé les watchdogs : on réarme celui du RTC tout de
    // suite, la boucle de rendu le nourrira.
    watchdog::arm(peripherals.RTC_TIMER);

    // Le bootloader de l'écran (console IDF désactivée) remet GPIO43 en
    // GPIO, sortie coupée : on rebranche U0TXD avant le premier journal.
    // esp-println écrit ensuite directement dans la FIFO d'UART0. Le pilote
    // vit jusqu'à la fin de main, qui ne rend jamais la main.
    let console = esp_hal::uart::Uart::new(peripherals.UART0, esp_hal::uart::Config::default())
        .unwrap()
        .with_tx(peripherals.GPIO43)
        .with_rx(peripherals.GPIO44)
        .into_async();
    info!(
        "screen {}, reset: {:?}",
        protocol::FIRMWARE_VERSION,
        esp_hal::system::reset_reason()
    );

    // Le tas interne d'abord : 120 Ko pour la radio du cœur 1.
    heap::init_internal();

    let mut flash = screen::flash::Flash::new(
        unsafe { esp_storage::FlashStorage::new(peripherals.FLASH).multicore_ignore() },
        true,
    );
    let layout = screen::flash::layout(&mut flash).expect("table de partitions écran incompatible");

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

    // Le code et les rodata passent en PSRAM avant tout autre usage, et en
    // occupent le début. Le framebuffer suit, puis le tas du cœur 0. La
    // flash ne sera plus lue en marche normale, ce qui permet aussi de
    // l'écrire (OTA, NVS) sans arrêter la recopie d'affichage.
    let code_bytes = {
        let t0 = esp_hal::time::Instant::now();
        let bytes = xip::run(psram_start, psram_size);
        info!(
            "XIP: {} pages de code et rodata en PSRAM en {} ms",
            xip::PAGES.load(Relaxed),
            t0.elapsed().as_millis()
        );
        bytes
    };
    let heap_start = code_bytes + display::FB_BYTES;
    assert!(
        psram_size > heap_start + 1024 * 1024,
        "PSRAM absente ou trop petite"
    );
    let fb = psram_start.wrapping_add(code_bytes);
    // Noir avant le premier rendu : la dalle ne montre jamais de PSRAM
    // non initialisée.
    unsafe { fb.write_bytes(0, display::FB_BYTES) };
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

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0, peripherals.FROM_CPU_INTR0);

    // La radio démarre avant la dalle ; les initialisations des blobs peuvent
    // masquer longtemps les interruptions du cœur qui les crée.
    core1::start(
        peripherals.CPU_CTRL,
        peripherals.FROM_CPU_INTR1,
        peripherals.WIFI,
        peripherals.BT,
        flash,
        layout,
    );
    core1::wait_ready().await;
    let (console_rx, console_tx) = console.split();
    spawner.spawn(screen::serial_bridge::receive(console_rx).unwrap());
    spawner.spawn(screen::serial_bridge::transmit(console_tx).unwrap());

    // Bus I2C, CH422G, séquence de mise sous tension de la dalle, GT911.
    let mut board = board::init(
        peripherals.I2C0,
        peripherals.GPIO8,
        peripherals.GPIO9,
        peripherals.GPIO4,
    )
    .await;

    board
        .expander
        .select_can(true)
        .await
        .expect("sélection CAN");
    // esp-hal prend RX puis TX : RX = GPIO19, TX = GPIO20 (board.h du C++).
    let twai = esp_hal::twai::TwaiConfiguration::new(
        peripherals.TWAI0,
        peripherals.GPIO19,
        peripherals.GPIO20,
        esp_hal::twai::BaudRate::B500K,
        esp_hal::twai::TwaiMode::Normal,
    )
    .into_async()
    .start();
    spawner.spawn(screen::can::task(twai).unwrap());

    let _display = display::start(
        peripherals.LCD_CAM,
        peripherals.DMA_CH0,
        display::Pins {
            hsync: peripherals.GPIO46.degrade(),
            vsync: peripherals.GPIO3.degrade(),
            de: peripherals.GPIO5.degrade(),
            pclk: peripherals.GPIO7.degrade(),
            data: [
                peripherals.GPIO14.degrade(),
                peripherals.GPIO38.degrade(),
                peripherals.GPIO18.degrade(),
                peripherals.GPIO17.degrade(),
                peripherals.GPIO10.degrade(),
                peripherals.GPIO39.degrade(),
                peripherals.GPIO0.degrade(),
                peripherals.GPIO45.degrade(),
                peripherals.GPIO48.degrade(),
                peripherals.GPIO47.degrade(),
                peripherals.GPIO21.degrade(),
                peripherals.GPIO1.degrade(),
                peripherals.GPIO2.degrade(),
                peripherals.GPIO42.degrade(),
                peripherals.GPIO41.degrade(),
                peripherals.GPIO40.degrade(),
            ],
        },
        fb,
    );
    // Laisser l'UART vider la ligne précédente : un info! qui attend la FIFO
    // bloque les interruptions et fait partir une recopie en retard.
    Timer::after(Duration::from_millis(200)).await;
    info!("DPI démarré");

    let ui = ui::init();
    Timer::after(Duration::from_millis(20)).await;
    info!("Slint: rendu ligne par ligne");

    spawner.spawn(report::report().unwrap());
    spawner.spawn(board::ads1115_task(board.device()).unwrap());
    let runtime = runtime::Runtime::new(board.touch.is_some());
    ui::run(ui, fb.cast::<Rgb565Pixel>(), board.touch, runtime).await
}
