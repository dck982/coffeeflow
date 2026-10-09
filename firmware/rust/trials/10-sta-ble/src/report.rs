//! Rapport toutes les 2 s sur la console, qui nourrit aussi le watchdog.
//! Les lignes sont espacées de 20 ms : esp-println écrit sous section
//! critique, et une ligne qui ne tient pas dans la FIFO UART bloquerait les
//! interruptions le temps de la vider.

use core::sync::atomic::Ordering::Relaxed;

use embassy_time::{Duration, Instant, Timer};
use log::info;

use crate::{crumb, stack_watermark, stats, watchdog};

#[embassy_executor::task]
#[allow(
    clippy::large_stack_frames,
    reason = "futures d'embassy : la table des piles radio vit dans la future, donc dans l'arène de l'exécuteur"
)]
pub async fn report() {
    let start = Instant::now();
    loop {
        Timer::after(Duration::from_secs(2)).await;
        crumb!();
        watchdog::feed();
        let heap = esp_alloc::HEAP.stats();
        let (c0, c1) = (stack_watermark::usage(0), stack_watermark::usage(1));
        info!(
            "[rapport] t={}s tas libre={} min={} pile c0={}/{} c1={}/{}",
            start.elapsed().as_secs(),
            esp_alloc::HEAP.free(),
            heap.size - heap.max_usage,
            c0.map_or(0, |u| u.0),
            c0.map_or(0, |u| u.1),
            c1.map_or(0, |u| u.0),
            c1.map_or(0, |u| u.1),
        );
        Timer::after(Duration::from_millis(20)).await;
        info!(
            "[radio] wifi assoc={} deco={} echec={} ip={} ble adv={} scans={}",
            stats::ASSOCIATED.load(Relaxed),
            stats::DISCONNECTED.load(Relaxed),
            stats::FAILED.load(Relaxed),
            core::net::Ipv4Addr::from(stats::ADDRESS.load(Relaxed)),
            stats::ADV_REPORTS.load(Relaxed),
            stats::SCAN_CYCLES.load(Relaxed),
        );
        #[cfg(feature = "core0-load")]
        {
            Timer::after(Duration::from_millis(20)).await;
            info!(
                "[charge] irq={} max={}us evenements_c0={}",
                crate::load::IRQS.load(Relaxed),
                crate::load::IRQ_MAX_US.load(Relaxed),
                crate::load::EVENTS_SEEN.load(Relaxed),
            );
        }
        for task in esp_rtos::trial_stacks::task_stacks().into_iter().flatten() {
            Timer::after(Duration::from_millis(20)).await;
            let core = if task.core == esp_rtos::trial_stacks::NO_CORE {
                -1
            } else {
                task.core as i32
            };
            info!(
                "[pile] {} taille={} demandé={} max={} ({}%) cœur={} adresse={:#x}",
                task.name(),
                task.size,
                task.requested,
                task.used,
                task.used * 100 / task.size,
                core,
                task.base,
            );
        }
    }
}
