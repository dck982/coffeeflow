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

/// Période du rapport, fixée au build par `REPORT_PERIOD_MS` (10 s par défaut).
const PERIOD_MS: u64 = match option_env!("REPORT_PERIOD_MS") {
    Some(text) => parse_ms(text),
    None => 10_000,
};

const fn parse_ms(text: &str) -> u64 {
    let bytes = text.as_bytes();
    let mut value = 0;
    let mut i = 0;
    while i < bytes.len() {
        assert!(
            bytes[i].is_ascii_digit(),
            "REPORT_PERIOD_MS doit être un entier décimal"
        );
        value = value * 10 + (bytes[i] - b'0') as u64;
        i += 1;
    }
    assert!(value >= 1000, "REPORT_PERIOD_MS : 1000 au moins");
    value
}

#[embassy_executor::task]
#[allow(
    clippy::large_stack_frames,
    reason = "futures d'embassy : clippy compte l'état de la future, qui vit dans l'arène de l'exécuteur ou la future de main, pas sur la pile"
)]
pub async fn report() {
    let start = Instant::now();
    let mut last_frames = 0;
    loop {
        Timer::after(Duration::from_millis(PERIOD_MS)).await;

        // Piles d'abord : si la carte plante peu après, c'est la mesure utile.
        info!(
            "pile c0 max={} c1 max={}",
            StackUsage(stack_watermark::usage(0)),
            StackUsage(stack_watermark::usage(1)),
        );
        // Piles des tâches créées par les blobs radio. Une tâche non
        // épinglée affiche `cœur=-1`.
        for task in esp_rtos::radio_stacks::task_stacks().into_iter().flatten() {
            Timer::after(Duration::from_millis(20)).await;
            let core = if task.core == esp_rtos::radio_stacks::NO_CORE {
                -1
            } else {
                task.core as i32
            };
            info!(
                "[pile] {} taille={} demandé={} max={} ({}%) cœur={}",
                task.name(),
                task.size,
                task.requested,
                task.used,
                task.used * 100 / task.size,
                core,
            );
        }

        Timer::after(Duration::from_millis(20)).await;
        let frames = display::FRAMES.load(Relaxed);
        info!(
            "{}s tr={}/s ret={} recal={} copie={}/{}us ecart={}us",
            start.elapsed().as_secs(),
            (frames - last_frames) as u64 * 1000 / PERIOD_MS,
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

struct StackUsage(stack_watermark::Usage);

impl core::fmt::Display for StackUsage {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.0 {
            stack_watermark::Usage::Unpainted => f.write_str("?"),
            stack_watermark::Usage::Used(used, size) => write!(f, "{used}/{size}"),
            stack_watermark::Usage::Invalid => f.write_str("invalide"),
        }
    }
}
