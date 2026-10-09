//! Charge du cœur 0 qui imite `screen` sans LCD (feature `core0-load`) :
//!
//! - une tâche embassy qui, toutes les 10 ms, alloue et libère quelques Ko
//!   (en PSRAM avec `psram-heap`) et calcule ~5 ms ;
//! - une tâche qui attend les événements du cœur 1 (annonce BLE, évènement
//!   Wi-Fi) dans un canal statique : chaque envoi réveille l'exécuteur du
//!   cœur 0 depuis l'autre cœur, comme les services de `screen` ;
//! - une interruption de niveau 3 à ~820 Hz qui recopie de la mémoire, avec
//!   `Instant::now()` au début et à la fin, comme `on_dma_eof` de l'écran.

use alloc::vec::Vec;
use core::hint::black_box;
use core::sync::atomic::{AtomicPtr, AtomicU32, Ordering::Relaxed};

use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};
use embassy_time::{Duration, Timer};
use esp_hal::{
    interrupt::{InterruptHandler, Priority},
    peripherals::TIMG1,
    time::Duration as HalDuration,
    timer::{PeriodicTimer, timg::TimerGroup},
};
use static_cell::StaticCell;

/// Origine d'un événement du cœur 1.
#[derive(Clone, Copy)]
pub enum Event {
    Adv,
    Wifi,
}

static EVENTS: Channel<CriticalSectionRawMutex, Event, 16> = Channel::new();

/// Interruptions de charge, durée max d'une interruption (µs), événements
/// reçus par le cœur 0.
pub static IRQS: AtomicU32 = AtomicU32::new(0);
pub static IRQ_MAX_US: AtomicU32 = AtomicU32::new(0);
pub static EVENTS_SEEN: AtomicU32 = AtomicU32::new(0);

/// Appelé depuis le cœur 1. Canal plein : l'événement est perdu, le réveil
/// du cœur 0 a déjà eu lieu.
pub fn notify(event: Event) {
    let _ = EVENTS.try_send(event);
}

/// Octets recopiés par interruption, comme un bounce buffer de l'écran
/// (800 px × 20 lignes × 2). La durée réelle se lit dans `IRQ_MAX_US`.
const COPY_BYTES: usize = 32000;
const COPY_WORDS: usize = COPY_BYTES / 4;
/// 820 Hz.
const IRQ_PERIOD_US: u64 = 1_000_000 / 820;

/// Source : PSRAM avec `psram-heap`, SRAM sinon. Destination toujours en SRAM.
/// Taille de l'écran : 800 × 480 × 2 octets, soit 24 blocs de 20 lignes. Sans
/// PSRAM, la source en SRAM n'a qu'un bloc.
#[cfg(feature = "psram-heap")]
const BLOCKS: usize = 24;
#[cfg(not(feature = "psram-heap"))]
const BLOCKS: usize = 1;
static NEXT_BLOCK: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(0);
#[cfg(not(feature = "psram-heap"))]
static mut SRAM_SOURCE: [u32; COPY_WORDS] = [0x5a5a_a5a5; COPY_WORDS];
static SRC: AtomicPtr<u32> = AtomicPtr::new(core::ptr::null_mut());
static mut DEST: [u32; COPY_WORDS] = [0; COPY_WORDS];

/// Interruption de niveau 3 sur le cœur 0. Pas de verrou : l'horodatage en
/// µs bruts et les différences saturées évitent la panique d'`Instant` quand
/// l'autre cœur lit aussi le SYSTIMER (voir la recopie d'affichage).
extern "C" fn on_tick() {
    let t0 = esp_hal::time::Instant::now()
        .duration_since_epoch()
        .as_micros();
    let timg = unsafe { &*esp32s3::TIMG1::ptr() };
    timg.int_clr().write(|w| w.t(0).clear_bit_by_one());
    // Le matériel retire l'alarme quand elle sonne : sans la réarmer, une
    // seule interruption partirait.
    timg.t(0).config().modify(|_, w| w.alarm_en().set_bit());
    // Comme la recopie d'affichage : lire la PSRAM (avec `psram-heap`) vers
    // un tampon en SRAM.
    // Bloc suivant du « framebuffer » à chaque interruption, comme l'écran qui
    // parcourt ses 768 Ko par tranches de 20 lignes : le cache voit défiler
    // toute la zone, pas toujours les mêmes 32 Ko.
    let block = NEXT_BLOCK.load(Relaxed);
    NEXT_BLOCK.store((block + 1) % BLOCKS, Relaxed);
    let src = SRC.load(Relaxed).wrapping_add(block * COPY_WORDS);
    unsafe { core::ptr::copy_nonoverlapping(src, (&raw mut DEST).cast::<u32>(), COPY_WORDS) };
    // Personne ne relit DEST : sans cette fuite du pointeur, le compilateur
    // supprime la recopie (12 µs mesurés au lieu de plusieurs centaines).
    core::hint::black_box(&raw mut DEST);
    let us = esp_hal::time::Instant::now()
        .duration_since_epoch()
        .as_micros()
        .saturating_sub(t0) as u32;
    IRQS.fetch_add(1, Relaxed);
    IRQ_MAX_US.fetch_max(us, Relaxed);
}

/// Prépare la source et arme le TIMG1 sur le cœur qui appelle (le
/// cœur 0) : l'interruption est liée au cœur courant.
pub fn start_interrupt(timg1: TIMG1<'static>) {
    #[cfg(feature = "psram-heap")]
    {
        // Alloué par le cœur 0 : donc en PSRAM.
        let src = alloc::vec![0x5a5a_a5a5u32; COPY_WORDS * BLOCKS].leak();
        SRC.store(src.as_mut_ptr(), Relaxed);
    }
    #[cfg(not(feature = "psram-heap"))]
    SRC.store((&raw mut SRAM_SOURCE).cast(), Relaxed);

    static TIMER: StaticCell<PeriodicTimer<'static, esp_hal::Blocking>> = StaticCell::new();
    let timer = TIMER.init(PeriodicTimer::new(TimerGroup::new(timg1).timer0));
    timer.set_interrupt_handler(InterruptHandler::new(on_tick, Priority::Priority3));
    timer.start(HalDuration::from_micros(IRQ_PERIOD_US)).unwrap();
    timer.listen();
}

/// Alloue et libère quelques Ko puis calcule ~5 ms, toutes les 10 ms.
#[embassy_executor::task]
pub async fn work() {
    let mut round = 0u32;
    loop {
        Timer::after(Duration::from_millis(10)).await;
        round = round.wrapping_add(1);
        {
            let mut buffer: Vec<u8> = Vec::with_capacity(1024 * (1 + (round % 6) as usize));
            buffer.resize(buffer.capacity(), round as u8);
            black_box(&buffer);
        }
        let start = embassy_time::Instant::now();
        let mut acc = round;
        while start.elapsed() < Duration::from_millis(5) {
            for i in 0..256u32 {
                acc = acc.wrapping_mul(1664525).wrapping_add(i ^ 1013904223);
            }
            black_box(acc);
        }
    }
}

/// Attend les événements du cœur 1.
#[embassy_executor::task]
pub async fn events() {
    loop {
        EVENTS.receive().await;
        EVENTS_SEEN.fetch_add(1, Relaxed);
    }
}
