//! Rapport périodique sur la console : quelques lignes courtes toutes les
//! 10 s. Elles sont espacées de 20 ms : esp-println écrit sous section
//! critique, et une ligne qui ne tient pas dans la FIFO UART (128 octets)
//! bloquerait les interruptions le temps de la vider, ce qui ferait partir
//! une recopie d'affichage en retard.

use core::sync::atomic::Ordering::Relaxed;

use embassy_time::{Duration, Instant, Timer};
use log::info;

use crate::board::ads1115;
use crate::display;
use crate::heap::PSRAM_HEAP;
use crate::stack_watermark;
use crate::ui;

#[embassy_executor::task]
#[allow(
    clippy::large_stack_frames,
    reason = "futures d'embassy : clippy compte l'état de la future, qui vit dans l'arène de l'exécuteur ou la future de main, pas sur la pile"
)]
pub async fn report() {
    let start = Instant::now();
    let mut last_frames = 0;
    loop {
        Timer::after(Duration::from_secs(10)).await;
        screen::crumb!();
        let frames = display::FRAMES.load(Relaxed);
        info!(
            "{}s tr={}/s ret={} recal={} copie={}/{}us ecart={}us",
            start.elapsed().as_secs(),
            (frames - last_frames) / 10,
            display::LATE.load(Relaxed),
            display::REALIGNS.load(Relaxed),
            display::COPY_MAX_PERIOD_US.swap(0, Relaxed),
            display::COPY_MAX_US.load(Relaxed),
            display::GAP_MAX_PERIOD_US.swap(0, Relaxed),
        );
        last_frames = frames;

        Timer::after(Duration::from_millis(20)).await;
        info!(
            "rendus n={} dernier={}us max={}us appuis={} err_tactile={}",
            ui::RENDERS.load(Relaxed),
            ui::RENDER_LAST_US.load(Relaxed),
            ui::RENDER_MAX_US.load(Relaxed),
            ui::TOUCH_PRESSES.load(Relaxed),
            ui::TOUCH_ERRORS.load(Relaxed),
        );

        // Tas interne (radio) : libre et minimum libre. Tas PSRAM : utilisé
        // et plus haut niveau.
        Timer::after(Duration::from_millis(20)).await;
        let heap = esp_alloc::HEAP.stats();
        let psram = PSRAM_HEAP.stats();
        info!(
            "tas interne libre={} min={} psram {}/{} max={}",
            esp_alloc::HEAP.free(),
            heap.size - heap.max_usage,
            PSRAM_HEAP.used(),
            psram.size,
            psram.max_usage,
        );

        // Plus haut niveau de pile de chaque cœur, utilisé/total.
        Timer::after(Duration::from_millis(20)).await;
        let (c0, c0_size) = stack_watermark::usage(0).unwrap_or_default();
        let (c1, c1_size) = stack_watermark::usage(1).unwrap_or_default();
        info!("pile c0 max={c0}/{c0_size} c1 max={c1}/{c1_size}");

        Timer::after(Duration::from_millis(20)).await;
        info!(
            "ads 0x{:02x} a0={} a1={} a2={}{} cycles={} err={}",
            ads1115::ADDRESS.load(Relaxed),
            ads1115::raw(0),
            ads1115::raw(1),
            ads1115::raw(2),
            if ads1115::A2_OK.load(Relaxed) { "" } else { " (perime)" },
            ads1115::CYCLES.load(Relaxed),
            ads1115::ERRORS.load(Relaxed),
        );
    }
}
