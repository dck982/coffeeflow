#![no_std]
#![no_main]
#![feature(asm_experimental_arch)]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

// Essai 04b : l'affichage de l'essai 03, variante un cœur, inchangé, avec
// deux connexions GATT simultanées sur le cœur 1, comme coffeeflow : la
// balance Acaia Lunar (pesées notifiées, heartbeat toutes les 2,5 s) et la
// sonde SCACE (une trame notifiée à 10 Hz). Le cœur 0 garde la recopie des
// bounce buffers et le dessin continu des rectangles et du compteur (pire
// cas de charge PSRAM). Le cœur 1 porte la radio : ses interruptions, la
// tâche du contrôleur BLE et l'hôte trouble-host, dans son propre exécuteur
// embassy.
//
// Le code et les rodata s'exécutent depuis la PSRAM (module `xip`), comme
// coffeeflow avec CONFIG_SPIRAM_XIP_FROM_PSRAM : les défauts de cache du
// code radio occupaient le bus flash/PSRAM bien plus longtemps en flash, au
// détriment de la recopie. La scène avance à 10 Hz, l'ordre de grandeur
// d'une UI en infusion ; `draw-continuous` rend le pire cas des essais 03 et
// 04. `exec-from-flash` désactive le XIP pour comparer.
//
// Un scan actif trouve d'abord les deux appareils, puis s'arrête. Chaque
// lien se connecte ensuite par adresse et se reconnecte seul s'il tombe.
// On mesure la tenue des liens (pertes, trames manquées) et ce qu'ils
// coûtent à la recopie : retards, recalages et recopie la plus lente, à
// comparer aux 940 µs de l'essai 04. Le cœur 1 ne journalise rien lui-même :
// esp-println écrit sous une section critique partagée par les deux cœurs,
// et le cœur 0 resterait bloqué, interruptions masquées, le temps que
// l'autre vide sa ligne. (esp-radio peut encore écrire une ligne
// d'avertissement depuis le cœur 1, rarement.)

use core::cell::RefCell;
use core::sync::atomic::{AtomicU32, Ordering::Relaxed};

use bt_hci::controller::ExternalController;
use bt_hci::param::Status;
use critical_section::Mutex;

use embassy_executor::Spawner;
use embassy_futures::select::{Either, Either3, select, select3};
use embedded_graphics::mono_font::MonoTextStyleBuilder;
use embedded_graphics::mono_font::ascii::FONT_10X20;
use embedded_graphics::pixelcolor::raw::RawU16;
use embedded_graphics::pixelcolor::{IntoStorage, Rgb565};
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;
use embedded_graphics::text::{Baseline, Text};
use embassy_time::{Duration, Instant, Timer, with_timeout};
use esp_backtrace as _;
use esp_hal::clock::CpuClock;
use esp_hal::dma::{BurstConfig, DmaDescriptor, DmaTxBuffer, Owner, Preparation};
use esp_hal::gpio::{Level, Output, OutputConfig};
use esp_hal::i2c::master::{Config as I2cConfig, I2c};
use esp_hal::interrupt::{self, InterruptHandler, Priority};
use esp_hal::lcd_cam::LcdCam;
use esp_hal::lcd_cam::lcd::dpi::{Config as DpiConfig, Dpi, Format, FrameTiming};
use esp_hal::lcd_cam::lcd::{ClockMode, Phase, Polarity};
use esp_hal::peripherals::Interrupt;
use esp_hal::peripherals::BT;
use esp_hal::psram::{Psram, PsramConfig, PsramMode, SpiRamFreq};
use esp_hal::system::{Cpu, Stack};
use esp_hal::time::Rate;
use esp_hal::timer::timg::TimerGroup;
use esp_radio::ble::controller::BleConnector;
use log::info;
use static_cell::StaticCell;
use trouble_host::prelude::*;

extern crate alloc;

use trial_04b_ble_gatt::xip;

esp_bootloader_esp_idf::esp_app_desc!();

const H_RES: usize = 800;
const V_RES: usize = 480;
const FB_PX: usize = H_RES * V_RES;

// Timings de coffeeflow (service_screen.cpp), en PCLK et en lignes.
const HSYNC_PULSE: usize = 48;
const HSYNC_BACK: usize = 88;
const HSYNC_FRONT: usize = 40;
const VSYNC_PULSE: usize = 3;
const VSYNC_BACK: usize = 32;
const VSYNC_FRONT: usize = 13;

// Deux bounce buffers de 20 lignes : 24 recopies par trame, 1,22 ms chacune
// pour se faire pendant que le DMA envoie l'autre buffer.
const BB_LINES: usize = 20;
const BB_PX: usize = BB_LINES * H_RES;
const BB_BYTES: usize = BB_PX * 2;
const BB_PER_FRAME: u32 = (FB_PX / BB_PX) as u32;
const DESC_PER_BB: usize = 8;
const DESC_BYTES: usize = BB_BYTES / DESC_PER_BB;
const N_RING: usize = 2 * DESC_PER_BB;
const RESTART_DESC: usize = N_RING;
const _: () = assert!(FB_PX % BB_PX == 0 && BB_PER_FRAME % 2 == 0);
const _: () = assert!(BB_BYTES % DESC_PER_BB == 0 && DESC_BYTES <= 4095);

// Sur ESP32-S3, IDF relance le DMA LCD_LL_FIFO_DEPTH + 1 pixels après le
// début de la trame : la FIFO du LCD en contient déjà autant
// (RGB_LCD_NEEDS_SEPARATE_RESTART_LINK dans esp_lcd_panel_rgb.c).
const RESTART_SKIP_BYTES: usize = (16 + 1) * 2;

// CH422G : l'adresse I2C choisit la fonction.
const CH422G_MODE: u8 = 0x24;
const CH422G_OUT: u8 = 0x38;
const CH422G_MODE_OUTPUTS: u8 = 0x01;
const EXIO_TP_RST: u8 = 1 << 1;
const EXIO_DISP: u8 = 1 << 2;
const EXIO_LCD_RST: u8 = 1 << 3;
// CAN_SEL (bit 5) n'apparaît dans aucun octet écrit : il reste bas, le
// transceiver CAN reste désélectionné.

// Bounce buffers et descripteurs en SRAM interne (.bss). Seules les
// interruptions y touchent une fois l'affichage démarré.
static mut BOUNCE: [[u32; BB_BYTES / 4]; 2] = [[0; BB_BYTES / 4]; 2];
static mut RING: [DmaDescriptor; N_RING + 1] = [DmaDescriptor::EMPTY; N_RING + 1];

// Adresse du framebuffer PSRAM, posée une fois avant le démarrage.
static FB: AtomicU32 = AtomicU32::new(0);
// Prochain bloc de 20 lignes à recopier (0..24).
static NEXT_CHUNK: AtomicU32 = AtomicU32::new(0);
// Fins de buffer reçues depuis le dernier VSYNC ; sa parité désigne le buffer
// que le DMA vient de finir.
static EOF_COUNT: AtomicU32 = AtomicU32::new(0);

// Mesures, lues par la boucle principale.
static FRAMES: AtomicU32 = AtomicU32::new(0);
static LATE: AtomicU32 = AtomicU32::new(0);
static RESYNCS: AtomicU32 = AtomicU32::new(0);
static COPY_MAX_US: AtomicU32 = AtomicU32::new(0);
// Recopie la plus lente et écart le plus long entre deux fins de buffer
// depuis le dernier rapport. L'écart nominal est de 1 220 µs ; il mesure le
// retard d'entrée dans l'interruption, que la durée de recopie ne voit pas.
static COPY_MAX_PERIOD_US: AtomicU32 = AtomicU32::new(0);
static GAP_MAX_PERIOD_US: AtomicU32 = AtomicU32::new(0);
static LAST_EOF_US: AtomicU32 = AtomicU32::new(0);
static STEPS: AtomicU32 = AtomicU32::new(0);
static DRAWN_PX: AtomicU32 = AtomicU32::new(0);

// Côté BLE, écrit par le cœur 1 et lu par le rapport du cœur 0. Les
// tableaux sont indexés par appareil : SCALE puis PROBE.
const SCALE: usize = 0;
const PROBE: usize = 1;
const DEVICE_NAMES: [&str; 2] = ["balance", "scace"];

const PHASE_SCAN: u32 = 1;
const PHASE_LINKS: u32 = 2;
static BLE_PHASE: AtomicU32 = AtomicU32::new(0);
static ADV_REPORTS: AtomicU32 = AtomicU32::new(0);
// Adresses retenues au scan, puis annonce à journaliser une fois.
static TARGETS: Mutex<RefCell<[Option<Address>; 2]>> = Mutex::new(RefCell::new([None, None]));
static SEEN: Mutex<RefCell<[Option<Seen>; 2]>> = Mutex::new(RefCell::new([None, None]));

const LINK_DOWN: u32 = 0;
const LINK_CONNECTING: u32 = 1;
const LINK_DISCOVERING: u32 = 2;
const LINK_SUBSCRIBED: u32 = 3;
const LINK_STATE_NAMES: [&str; 4] = ["coupe", "connexion", "decouverte", "abonne"];
static LINK_STATE: [AtomicU32; 2] = [const { AtomicU32::new(LINK_DOWN) }; 2];
static CONNECTS: [AtomicU32; 2] = [const { AtomicU32::new(0) }; 2];
// Échecs avant l'abonnement (connexion, MTU, découverte), et liens perdus
// après.
static FAILS: [AtomicU32; 2] = [const { AtomicU32::new(0) }; 2];
static DROPS: [AtomicU32; 2] = [const { AtomicU32::new(0) }; 2];
static NOTIFS: [AtomicU32; 2] = [const { AtomicU32::new(0) }; 2];
static INTERVAL_US: [AtomicU32; 2] = [const { AtomicU32::new(0) }; 2];
static LAST_ERROR: Mutex<RefCell<[Option<ErrText>; 2]>> =
    Mutex::new(RefCell::new([const { None }; 2]));

static WEIGHT_BITS: AtomicU32 = AtomicU32::new(f32::NAN.to_bits());
static WEIGHTS: AtomicU32 = AtomicU32::new(0);
static HEARTBEATS: AtomicU32 = AtomicU32::new(0);
static PROBE_CENTI_C: AtomicU32 = AtomicU32::new(SCACE_NO_TEMPERATURE as u16 as u32);
static PROBE_STATUS: AtomicU32 = AtomicU32::new(0);
static PROBE_LOST: AtomicU32 = AtomicU32::new(0);
static PROBE_INVALID: AtomicU32 = AtomicU32::new(0);

/// Dernières recopies en retard : écart depuis la fin de buffer précédente,
/// durée de la recopie, et PC interrompu. Écrit par l'interruption seule, lu
/// par le rapport ; une entrée écrasée avant lecture est simplement perdue.
const LATE_LOG: usize = 8;
static LATE_GAP_US: [AtomicU32; LATE_LOG] = [const { AtomicU32::new(0) }; LATE_LOG];
static LATE_COPY_US: [AtomicU32; LATE_LOG] = [const { AtomicU32::new(0) }; LATE_LOG];
static LATE_PC: [AtomicU32; LATE_LOG] = [const { AtomicU32::new(0) }; LATE_LOG];
// Rangs de OUT_DSCR à l'entrée et après la recopie, sur un octet chacun.
static LATE_DSCR: [AtomicU32; LATE_LOG] = [const { AtomicU32::new(0) }; LATE_LOG];
// Avance de OUT_DSCR à l'entrée de l'interruption (0..4, 5 = plus loin).
const ENTRY_SLOTS: usize = 6;
static ENTRY_DSCR: [AtomicU32; ENTRY_SLOTS] = [const { AtomicU32::new(0) }; ENTRY_SLOTS];

// Bas des piles peintes au démarrage, pour en mesurer le plus haut niveau.
const STACK_PAINT: u32 = 0xA5A5_5A5A;
static STACK0_LOW: AtomicU32 = AtomicU32::new(0);
static STACK1_BASE: AtomicU32 = AtomicU32::new(0);

unsafe extern "C" {
    static _stack_start_cpu0: u32;
    static _stack_end_cpu0: u32;
}

/// PC du code que l'interruption de niveau 3 a interrompu. Si une section
/// critique masquait l'interruption, c'est l'instruction qui suit sa fin :
/// la console (`espflash monitor --elf`) traduit l'adresse en fonction.
#[inline(always)]
fn interrupted_pc() -> u32 {
    let pc: u32;
    unsafe { core::arch::asm!("rsr.epc3 {0}", out(reg) pc, options(nomem, nostack)) };
    pc
}

fn paint_stack(low: *mut u32, words: usize) {
    for i in 0..words {
        unsafe { low.add(i).write_volatile(STACK_PAINT) };
    }
}

/// Mots modifiés dans les `APP_STACK_LOW_BYTES` du bas de la pile du
/// cœur 1, et décalage du premier (en octets). La garde d'esp-hal y est
/// (60 octets) ; d'autres écritures isolées s'y voient sans fausser la
/// mesure de remplissage, qui commence au-dessus.
fn stack_low_writes(base: u32) -> (u32, u32) {
    let (mut count, mut first) = (0, APP_STACK_LOW_BYTES);
    for offset in (0..APP_STACK_LOW_BYTES).step_by(4) {
        if unsafe { ((base + offset) as *const u32).read_volatile() } != STACK_PAINT {
            count += 1;
            first = first.min(offset);
        }
    }
    (count, first)
}

/// Octets jamais atteints au-dessus de `low` : la pile a utilisé tout le
/// reste.
fn stack_untouched(low: u32) -> u32 {
    let mut p = low as *const u32;
    while unsafe { p.read_volatile() } == STACK_PAINT {
        p = p.wrapping_add(1);
    }
    p as u32 - low
}

fn lcd_regs() -> &'static esp32s3::lcd_cam::RegisterBlock {
    unsafe { &*esp32s3::LCD_CAM::ptr() }
}

fn dma_tx() -> &'static esp32s3::dma::CH {
    unsafe { &*esp32s3::DMA::ptr() }.ch(0)
}

fn ring_ptr(i: usize) -> *mut DmaDescriptor {
    unsafe { (&raw mut RING).cast::<DmaDescriptor>().add(i) }
}

fn bounce_ptr(bb: usize) -> *mut u8 {
    unsafe { (&raw mut BOUNCE).cast::<[u32; BB_BYTES / 4]>().add(bb).cast() }
}

/// Anneau de descripteurs : 8 par bounce buffer, EOF sur le dernier de
/// chacun. Un descripteur de plus sert à la relance en phase : il saute les
/// 17 premiers pixels du buffer 0 puis rejoint l'anneau.
struct BounceRing;

impl BounceRing {
    fn new() -> Self {
        for i in 0..N_RING {
            let desc = unsafe { &mut *ring_ptr(i) };
            let (bb, k) = (i / DESC_PER_BB, i % DESC_PER_BB);
            desc.set_size(DESC_BYTES);
            desc.set_length(DESC_BYTES);
            desc.set_owner(Owner::Dma);
            desc.set_suc_eof(k == DESC_PER_BB - 1);
            desc.buffer = unsafe { bounce_ptr(bb).add(k * DESC_BYTES) };
            desc.next = ring_ptr((i + 1) % N_RING);
        }
        let restart = unsafe { &mut *ring_ptr(RESTART_DESC) };
        restart.set_size(DESC_BYTES - RESTART_SKIP_BYTES);
        restart.set_length(DESC_BYTES - RESTART_SKIP_BYTES);
        restart.set_owner(Owner::Dma);
        restart.set_suc_eof(false);
        restart.buffer = unsafe { bounce_ptr(0).add(RESTART_SKIP_BYTES) };
        restart.next = ring_ptr(1);
        Self
    }
}

unsafe impl DmaTxBuffer for BounceRing {
    type View = BounceRing;
    type Final = BounceRing;

    fn prepare(&mut self) -> Preparation {
        Preparation {
            start: ring_ptr(0),
            accesses_psram: false,
            burst_transfer: BurstConfig::default(),
            // Le DMA ne rend jamais la main sur ces descripteurs : il ne
            // doit ni vérifier ni effacer le bit owner.
            check_owner: Some(false),
            auto_write_back: false,
        }
    }

    fn into_view(self) -> Self::View {
        self
    }

    fn from_view(view: Self::View) -> Self::Final {
        view
    }
}

/// Recopie le prochain bloc de 20 lignes du framebuffer dans le bounce
/// buffer `bb`. Le CPU lit le framebuffer à travers le même cache que celui
/// qui l'écrit : pas de writeback nécessaire.
fn fill_bounce(bb: usize) {
    let chunk = NEXT_CHUNK.load(Relaxed) as usize;
    let src = (FB.load(Relaxed) as *const u32).wrapping_add(chunk * BB_BYTES / 4);
    unsafe { core::ptr::copy_nonoverlapping(src, bounce_ptr(bb).cast::<u32>(), BB_BYTES / 4) };
    NEXT_CHUNK.store(((chunk + 1) % BB_PER_FRAME as usize) as u32, Relaxed);
}

/// Rang dans l'anneau du descripteur à l'adresse `addr` (0..16, 16 étant
/// celui de la relance), ou `NOT_IN_RING`.
const NOT_IN_RING: usize = 99;

fn desc_index(addr: u32) -> usize {
    let offset = (addr as usize).wrapping_sub(ring_ptr(0) as usize);
    let size = core::mem::size_of::<DmaDescriptor>();
    if offset % size == 0 && offset / size <= RESTART_DESC {
        offset / size
    } else {
        NOT_IN_RING
    }
}

/// Le descripteur de rang `index` appartient-il au bounce buffer `bb` ?
fn in_buffer(index: usize, bb: usize) -> bool {
    index / DESC_PER_BB == bb && index < N_RING || bb == 0 && index == RESTART_DESC
}

/// Rang de OUT_DSCR. Malgré la doc du PAC (« descripteur en cours »), il
/// a deux crans d'avance sur le descripteur que le DMA lit : à l'entrée de
/// l'interruption de fin de buffer, il vaut déjà le troisième de l'autre
/// buffer (essai 04, et zéro fausse alarme de l'essai 03 avec ce décalage).
fn out_dscr() -> usize {
    desc_index(dma_tx().out_dscr().read().outlink_dscr().bits())
}

/// Rang du descripteur que le DMA lit, déduit de OUT_DSCR. OUT_DSCR ne
/// vaut le descripteur de relance (16) qu'à l'instant de la relance, quand
/// le DMA le lit lui-même ; il fait partie du buffer 0.
fn dma_current(out_dscr: usize) -> usize {
    match out_dscr {
        NOT_IN_RING | RESTART_DESC => out_dscr,
        i => (i + N_RING - 2) % N_RING,
    }
}

/// Fin de lecture d'un bounce buffer par le DMA : il passe à l'autre, on
/// recharge celui-ci. Si le DMA est revenu dessus avant la fin de la
/// recopie, le LCD a reçu des lignes périmées : c'est un retard.
extern "C" fn on_dma_eof() {
    let pc = interrupted_pc();
    let entry_dscr = out_dscr();
    let t0 = esp_hal::time::Instant::now();
    let now = t0.duration_since_epoch().as_micros() as u32;
    let gap = now.wrapping_sub(LAST_EOF_US.swap(now, Relaxed));

    dma_tx().out_int().clr().write(|w| w.out_eof().clear_bit_by_one());
    let eof = EOF_COUNT.fetch_add(1, Relaxed);
    let bb = (eof % 2) as usize;

    fill_bounce(bb);
    let us = t0.elapsed().as_micros() as u32;
    let end_dscr = out_dscr();

    // Avance de OUT_DSCR à l'entrée, en descripteurs depuis le début de
    // l'autre buffer : 2 attendu, plus si l'interruption entre en retard.
    let other_first = (1 - bb) * DESC_PER_BB;
    let entry = if entry_dscr == RESTART_DESC { 0 } else { entry_dscr };
    let advance = (entry + N_RING - other_first) % N_RING;
    ENTRY_DSCR[advance.min(ENTRY_SLOTS - 1)].fetch_add(1, Relaxed);

    if in_buffer(dma_current(end_dscr), bb) {
        let n = LATE.fetch_add(1, Relaxed) as usize % LATE_LOG;
        LATE_GAP_US[n].store(gap, Relaxed);
        LATE_COPY_US[n].store(us, Relaxed);
        LATE_PC[n].store(pc, Relaxed);
        LATE_DSCR[n].store((entry_dscr << 8 | end_dscr) as u32, Relaxed);
    }
    COPY_MAX_US.fetch_max(us, Relaxed);
    COPY_MAX_PERIOD_US.fetch_max(us, Relaxed);
    // La première fin de buffer d'une trame suit le blanking vertical
    // (~2,9 ms de plus) : son écart ne dit rien du retard d'entrée.
    if eof != 0 {
        GAP_MAX_PERIOD_US.fetch_max(gap, Relaxed);
    }
}

/// Fin de trame. Une trame en phase a reçu exactement 24 fins de buffer, et
/// les deux buffers contiennent déjà les blocs 0 et 1 de la suivante. Sinon,
/// on relance le DMA en phase pendant le blanking vertical (~2,9 ms), comme
/// lcd_rgb_panel_try_restart_transmission() d'IDF.
extern "C" fn on_vsync() {
    lcd_regs().lc_dma_int_clr().write(|w| w.lcd_vsync_int_clr().set_bit());
    FRAMES.fetch_add(1, Relaxed);

    if EOF_COUNT.load(Relaxed) != BB_PER_FRAME {
        RESYNCS.fetch_add(1, Relaxed);
        if NEXT_CHUNK.load(Relaxed) > 2 {
            NEXT_CHUNK.store(0, Relaxed);
        }
        if NEXT_CHUNK.load(Relaxed) == 0 {
            fill_bounce(0);
        }
        lcd_regs()
            .lcd_misc()
            .modify(|_, w| w.lcd_afifo_reset().set_bit());
        let dma = dma_tx();
        dma.out_link().modify(|_, w| w.outlink_stop().set_bit());
        dma.out_conf0().modify(|_, w| w.out_rst().set_bit());
        dma.out_conf0().modify(|_, w| w.out_rst().clear_bit());
        dma.out_link()
            .modify(|_, w| unsafe { w.outlink_addr().bits(ring_ptr(RESTART_DESC) as u32) });
        dma.out_link().modify(|_, w| w.outlink_start().set_bit());
        if NEXT_CHUNK.load(Relaxed) == 1 {
            fill_bounce(1);
        }
    }
    EOF_COUNT.store(0, Relaxed);
}

/// `Dpi::send` lance le DMA puis vide la FIFO du LCD, ce qui jette les
/// premiers pixels. On refait le démarrage dans l'ordre d'IDF : LCD arrêté
/// et FIFO vidée, DMA relancé depuis le premier descripteur, puis LCD.
fn restart_in_phase() {
    let lcd = lcd_regs();
    let dma = dma_tx();

    lcd.lcd_user()
        .modify(|_, w| w.lcd_start().clear_bit().lcd_update().set_bit());
    lcd.lcd_user().modify(|_, w| w.lcd_reset().set_bit());
    lcd.lcd_misc().modify(|_, w| w.lcd_afifo_reset().set_bit());

    dma.out_link().modify(|_, w| w.outlink_stop().set_bit());
    dma.out_conf0().modify(|_, w| w.out_rst().set_bit());
    dma.out_conf0().modify(|_, w| w.out_rst().clear_bit());
    dma.out_link()
        .modify(|_, w| unsafe { w.outlink_addr().bits(ring_ptr(0) as u32) });
    dma.out_link().modify(|_, w| w.outlink_start().set_bit());

    // IDF : 1 µs suffit au DMA pour remplir la FIFO du LCD.
    esp_hal::delay::Delay::new().delay_micros(1);
    lcd.lcd_user()
        .modify(|_, w| w.lcd_update().set_bit().lcd_start().set_bit());
}

const fn rgb565(r: u8, g: u8, b: u8) -> u16 {
    ((r as u16 >> 3) << 11) | ((g as u16 >> 2) << 5) | (b as u16 >> 3)
}

const WHITE: u16 = 0xFFFF;
const BLACK: u16 = 0x0000;
const RED: u16 = 0xF800;
const GREEN: u16 = 0x07E0;
const BLUE: u16 = 0x001F;
const GRID: u16 = rgb565(128, 128, 128);

const BANDS: [u16; 8] = [
    WHITE,
    rgb565(255, 255, 0),
    rgb565(0, 255, 255),
    GREEN,
    rgb565(255, 0, 255),
    RED,
    BLUE,
    BLACK,
];

/// Mire : huit bandes verticales, des dégradés R, V, B et gris pour repérer
/// un bit de donnée inversé ou manquant, un quadrillage tous les 40 px pour
/// voir un décalage, un cadre d'un pixel sur les bords, et un carré de
/// couleur différente dans chaque coin pour voir une inversion d'axe. Calculée
/// pixel par pixel, pour pouvoir restaurer le fond sous un objet qui bouge.
fn pattern(x: usize, y: usize) -> u16 {
    if x == 0 || y == 0 || x == H_RES - 1 || y == V_RES - 1 {
        return WHITE;
    }
    let in_square = |x0: usize, y0: usize| (x0..x0 + 16).contains(&x) && (y0..y0 + 16).contains(&y);
    if in_square(3, 3) {
        return RED;
    }
    if in_square(H_RES - 19, 3) {
        return GREEN;
    }
    if in_square(3, V_RES - 19) {
        return BLUE;
    }
    if in_square(H_RES - 19, V_RES - 19) {
        return WHITE;
    }
    if x % 40 == 0 || y % 40 == 0 {
        return GRID;
    }
    // 32 niveaux par dégradé, 25 px par niveau.
    let level = (x * 32 / H_RES) as u16;
    match y {
        0..320 => BANDS[x / (H_RES / BANDS.len())],
        320..360 => level << 11,
        360..400 => (level * 2) << 5,
        400..440 => level,
        _ => (level << 11) | ((level * 2) << 5) | level,
    }
}

/// Framebuffer PSRAM vu par embedded-graphics. Chaque pixel écrit est compté
/// pour mesurer le débit d'écriture.
#[derive(Clone, Copy)]
struct Fb(*mut u16);

// Un seul contexte dessine ; l'interruption de recopie ne fait que lire.
unsafe impl Send for Fb {}

impl Fb {
    fn clip(&self, area: &Rectangle) -> Option<(usize, usize, usize, usize)> {
        let area = area.intersection(&self.bounding_box());
        let br = area.bottom_right()?;
        let (x0, y0) = (area.top_left.x as usize, area.top_left.y as usize);
        Some((x0, y0, br.x as usize + 1, br.y as usize + 1))
    }

    fn restore(&mut self, area: &Rectangle) {
        if let Some((x0, y0, x1, y1)) = self.clip(area) {
            for y in y0..y1 {
                let row = unsafe { core::slice::from_raw_parts_mut(self.0.add(y * H_RES), H_RES) };
                for x in x0..x1 {
                    row[x] = pattern(x, y);
                }
            }
            DRAWN_PX.fetch_add(((x1 - x0) * (y1 - y0)) as u32, Relaxed);
        }
    }
}

impl OriginDimensions for Fb {
    fn size(&self) -> Size {
        Size::new(H_RES as u32, V_RES as u32)
    }
}

impl DrawTarget for Fb {
    type Color = Rgb565;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        let mut n = 0;
        for Pixel(p, color) in pixels {
            if (0..H_RES as i32).contains(&p.x) && (0..V_RES as i32).contains(&p.y) {
                unsafe { *self.0.add(p.y as usize * H_RES + p.x as usize) = color.into_storage() };
                n += 1;
            }
        }
        DRAWN_PX.fetch_add(n, Relaxed);
        Ok(())
    }

    fn fill_solid(&mut self, area: &Rectangle, color: Self::Color) -> Result<(), Self::Error> {
        if let Some((x0, y0, x1, y1)) = self.clip(area) {
            for y in y0..y1 {
                let row = unsafe { core::slice::from_raw_parts_mut(self.0.add(y * H_RES), H_RES) };
                row[x0..x1].fill(color.into_storage());
            }
            DRAWN_PX.fetch_add(((x1 - x0) * (y1 - y0)) as u32, Relaxed);
        }
        Ok(())
    }
}

/// Rectangle plein qui rebondit sur les bords. À chaque pas, la mire est
/// restaurée sous l'ancienne position.
struct Mover {
    rect: Rectangle,
    dx: i32,
    dy: i32,
    color: Rgb565,
}

impl Mover {
    const fn new(x: i32, y: i32, w: u32, h: u32, dx: i32, dy: i32, color: Rgb565) -> Self {
        Self {
            rect: Rectangle::new(Point::new(x, y), Size::new(w, h)),
            dx,
            dy,
            color,
        }
    }

    fn step(&mut self, fb: &mut Fb) {
        fb.restore(&self.rect);
        let size = self.rect.size;
        let mut p = self.rect.top_left + Point::new(self.dx, self.dy);
        if p.x < 0 || p.x + size.width as i32 > H_RES as i32 {
            self.dx = -self.dx;
            p.x = p.x.clamp(0, H_RES as i32 - size.width as i32);
        }
        if p.y < 0 || p.y + size.height as i32 > V_RES as i32 {
            self.dy = -self.dy;
            p.y = p.y.clamp(0, V_RES as i32 - size.height as i32);
        }
        self.rect.top_left = p;
        let _ = fb.fill_solid(&self.rect, self.color);
    }
}

/// Texte court sans allocation.
struct Line {
    buf: [u8; 32],
    len: usize,
}

impl core::fmt::Write for Line {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let end = self.len + s.len();
        if end > self.buf.len() {
            return Err(core::fmt::Error);
        }
        self.buf[self.len..end].copy_from_slice(s.as_bytes());
        self.len = end;
        Ok(())
    }
}

/// Scène : trois rectangles de tailles et vitesses différentes, et un
/// compteur de pas. Un pas déplace tout et réécrit le compteur.
struct Scene {
    movers: [Mover; 3],
    step: u32,
}

impl Scene {
    const fn new() -> Self {
        Self {
            movers: [
                Mover::new(40, 40, 160, 100, 3, 2, Rgb565::new(31, 40, 0)),
                Mover::new(500, 260, 80, 80, -2, 3, Rgb565::new(16, 0, 31)),
                Mover::new(200, 380, 260, 40, 4, -1, Rgb565::new(0, 50, 20)),
            ],
            step: 0,
        }
    }

    fn draw_step(&mut self, fb: &mut Fb) {
        use core::fmt::Write;

        for mover in &mut self.movers {
            mover.step(fb);
        }
        self.step = self.step.wrapping_add(1);
        let mut line = Line { buf: [0; 32], len: 0 };
        let _ = write!(line, "pas {:>10}", self.step);
        let style = MonoTextStyleBuilder::new()
            .font(&FONT_10X20)
            .text_color(Rgb565::WHITE)
            .background_color(Rgb565::BLACK)
            .build();
        let text = core::str::from_utf8(&line.buf[..line.len]).unwrap_or("");
        let _ = Text::with_baseline(text, Point::new(300, 200), style, Baseline::Top).draw(fb);
        STEPS.fetch_add(1, Relaxed);
    }
}

/// Une ligne courte toutes les 10 s : esp-println écrit sous section
/// critique, et une ligne qui ne tient pas dans la FIFO UART (128 octets)
/// bloquerait les interruptions le temps de la vider.
#[embassy_executor::task]
async fn report() {
    let start = Instant::now();
    let (mut last_frames, mut last_steps, mut last_px, mut last_adv) = (0, 0, 0, 0);
    let mut last_notifs = [0u32; 2];
    let mut last_late = 0;
    loop {
        Timer::after(Duration::from_secs(10)).await;
        let (frames, steps, px) = (
            FRAMES.load(Relaxed),
            STEPS.load(Relaxed),
            DRAWN_PX.load(Relaxed),
        );
        info!(
            "{}s tr={}/s ret={} recal={} copie={}/{}us ecart={}us pas={}/s ecr={}Ko/s",
            start.elapsed().as_secs(),
            (frames - last_frames) / 10,
            LATE.load(Relaxed),
            RESYNCS.load(Relaxed),
            COPY_MAX_PERIOD_US.swap(0, Relaxed),
            COPY_MAX_US.load(Relaxed),
            GAP_MAX_PERIOD_US.swap(0, Relaxed),
            (steps - last_steps) / 10,
            px.wrapping_sub(last_px) * 2 / 1024 / 10,
        );
        (last_frames, last_steps, last_px) = (frames, steps, px);

        // Une ligne à la fois : la précédente doit sortir de la FIFO UART
        // (~11 ms à 115 200 bauds) avant que la suivante ne s'y écrive.
        Timer::after(Duration::from_millis(20)).await;
        let heap = esp_alloc::HEAP.stats();
        let adv = ADV_REPORTS.load(Relaxed);
        info!(
            "mem libre={} min={} ble={} adv={}/s",
            esp_alloc::HEAP.free(),
            heap.size - heap.max_usage,
            ["init", "scan", "liens"][BLE_PHASE.load(Relaxed) as usize],
            adv.wrapping_sub(last_adv) / 10,
        );
        last_adv = adv;

        Timer::after(Duration::from_millis(20)).await;
        let stack0_top = &raw const _stack_start_cpu0 as u32;
        let stack0_low = STACK0_LOW.load(Relaxed);
        let stack1_base = STACK1_BASE.load(Relaxed);
        let (low_words, low_first) = stack_low_writes(stack1_base);
        info!(
            "pile0 {}/{} pile1 {}/{} bas={}@{} {}",
            stack0_top - stack0_low - stack_untouched(stack0_low),
            stack0_top - (&raw const _stack_end_cpu0 as u32),
            APP_STACK_BYTES as u32
                - APP_STACK_LOW_BYTES
                - stack_untouched(stack1_base + APP_STACK_LOW_BYTES),
            APP_STACK_BYTES,
            low_words,
            low_first,
            BtIrqMap::read(),
        );

        // Au plus quatre retards par rapport, les plus récents.
        let late = LATE.load(Relaxed);
        let first = late.saturating_sub(4).max(last_late);
        for i in first..late {
            let n = i as usize % LATE_LOG;
            Timer::after(Duration::from_millis(20)).await;
            let dscr = LATE_DSCR[n].load(Relaxed);
            info!(
                "retard {} ecart={}us copie={}us dscr={}/{} pc=0x{:08x}",
                i + 1,
                LATE_GAP_US[n].load(Relaxed),
                LATE_COPY_US[n].load(Relaxed),
                dscr >> 8,
                dscr & 0xff,
                LATE_PC[n].load(Relaxed),
            );
        }
        last_late = late;

        let seen = critical_section::with(|cs| SEEN.borrow(cs).take());
        for (kind, dev) in DEVICE_NAMES.iter().zip(seen) {
            if let Some(dev) = dev {
                Timer::after(Duration::from_millis(20)).await;
                info!(
                    "{kind} vue {} {} {}dBm",
                    core::str::from_utf8(dev.name()).unwrap_or("?"),
                    Mac(dev.addr.addr.into_inner()),
                    dev.rssi,
                );
            }
        }

        // Une ligne par appareil : état du lien, connexions, échecs avant
        // abonnement, pertes après abonnement, notifications par seconde,
        // intervalle de connexion, puis la donnée elle-même.
        for dev in [SCALE, PROBE] {
            Timer::after(Duration::from_millis(20)).await;
            let notifs = NOTIFS[dev].load(Relaxed);
            let rate = notifs.wrapping_sub(last_notifs[dev]) / 10;
            last_notifs[dev] = notifs;
            let link = LinkLine {
                state: LINK_STATE_NAMES[LINK_STATE[dev].load(Relaxed) as usize],
                connects: CONNECTS[dev].load(Relaxed),
                fails: FAILS[dev].load(Relaxed),
                drops: DROPS[dev].load(Relaxed),
                rate,
                interval_us: INTERVAL_US[dev].load(Relaxed),
            };
            if dev == SCALE {
                info!(
                    "balance {link} poids={:.1}g pesees={} hb={}",
                    f32::from_bits(WEIGHT_BITS.load(Relaxed)),
                    WEIGHTS.load(Relaxed),
                    HEARTBEATS.load(Relaxed),
                );
            } else {
                info!(
                    "scace {link} t={} etat={} manq={} inval={}",
                    Centi(PROBE_CENTI_C.load(Relaxed) as i16),
                    PROBE_STATUS.load(Relaxed),
                    PROBE_LOST.load(Relaxed),
                    PROBE_INVALID.load(Relaxed),
                );
            }
        }

        let errors = critical_section::with(|cs| LAST_ERROR.borrow(cs).take());
        for (kind, err) in DEVICE_NAMES.iter().zip(errors) {
            if let Some(err) = err {
                Timer::after(Duration::from_millis(20)).await;
                info!("{kind} erreur {err}");
            }
        }
    }
}

/// Résumé d'un lien BLE pour le rapport.
struct LinkLine {
    state: &'static str,
    connects: u32,
    fails: u32,
    drops: u32,
    rate: u32,
    interval_us: u32,
}

impl core::fmt::Display for LinkLine {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{} conn={} ech={} pertes={} notif={}/s int={}ms",
            self.state,
            self.connects,
            self.fails,
            self.drops,
            self.rate,
            self.interval_us / 1000,
        )
    }
}

/// Centièmes de °C, ou `-` sans température.
struct Centi(i16);

impl core::fmt::Display for Centi {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if self.0 == SCACE_NO_TEMPERATURE {
            return f.write_str("-");
        }
        let abs = self.0.unsigned_abs();
        let sign = if self.0 < 0 { "-" } else { "" };
        write!(f, "{sign}{}.{:02}C", abs / 100, abs % 100)
    }
}

/// Correspondance des interruptions BT dans la matrice de chaque cœur,
/// affichée cœur 0/cœur 1 (16 = désactivée chez esp-hal). Montre sur quel
/// cœur la radio les reçoit.
struct BtIrqMap([[u8; 2]; 2]);

impl BtIrqMap {
    fn read() -> Self {
        let sources = [Interrupt::BT_BB as usize, Interrupt::RWBLE as usize];
        let core0 = unsafe { &*esp32s3::INTERRUPT_CORE0::ptr() };
        let core1 = unsafe { &*esp32s3::INTERRUPT_CORE1::ptr() };
        Self(sources.map(|n| {
            [
                core0.core_0_intr_map(n).read().map().bits(),
                core1.core_1_intr_map(n).read().map().bits(),
            ]
        }))
    }
}

impl core::fmt::Display for BtIrqMap {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let [[bb0, bb1], [ble0, ble1]] = self.0;
        write!(f, "irq bb {bb0}/{bb1} rwble {ble0}/{ble1}")
    }
}

/// Adresse BLE, octets reçus en petit-boutiste, affichée dans l'ordre usuel.
struct Mac([u8; 6]);

impl core::fmt::Display for Mac {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let a = self.0;
        write!(
            f,
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            a[5], a[4], a[3], a[2], a[1], a[0]
        )
    }
}

/// Première annonce reçue d'un appareil reconnu.
#[derive(Clone, Copy)]
struct Seen {
    name: [u8; 20],
    name_len: u8,
    addr: Address,
    rssi: i8,
}

impl Seen {
    fn name(&self) -> &[u8] {
        &self.name[..self.name_len as usize]
    }
}

/// Mêmes critères de nom que coffeeflow : `ACAIA` ou `LUNAR` sans tenir
/// compte de la casse pour la balance (ble_scale.cpp), `SCACE` exact pour
/// la sonde (scace_ble.hpp).
fn classify(name: &[u8]) -> Option<usize> {
    let contains = |needle: &[u8]| {
        name.windows(needle.len())
            .any(|w| w.eq_ignore_ascii_case(needle))
    };
    if contains(b"ACAIA") || contains(b"LUNAR") {
        Some(SCALE)
    } else if name == b"SCACE" {
        Some(PROBE)
    } else {
        None
    }
}

/// Reçoit les annonces et réponses de scan dans le contexte de l'hôte, sur
/// le cœur 1. Retient l'adresse de chaque appareil reconnu à sa première
/// annonce : les liens s'y connectent ensuite sans scanner.
struct ScanHandler;

impl EventHandler for ScanHandler {
    fn on_adv_reports(&self, mut reports: LeAdvReportsIter<'_>) {
        while let Some(Ok(report)) = reports.next() {
            ADV_REPORTS.fetch_add(1, Relaxed);
            let name = AdStructure::decode(report.data).find_map(|ad| match ad {
                Ok(AdStructure::CompleteLocalName(n) | AdStructure::ShortenedLocalName(n)) => {
                    Some(n)
                }
                _ => None,
            });
            let Some(name) = name else { continue };
            let Some(dev) = classify(name) else { continue };
            let addr = Address {
                kind: report.addr_kind,
                addr: report.addr,
            };
            critical_section::with(|cs| {
                let mut targets = TARGETS.borrow_ref_mut(cs);
                if targets[dev].is_some() {
                    return;
                }
                targets[dev] = Some(addr);
                let mut seen = Seen {
                    name: [0; 20],
                    name_len: name.len().min(20) as u8,
                    addr,
                    rssi: report.rssi,
                };
                seen.name[..seen.name_len as usize]
                    .copy_from_slice(&name[..seen.name_len as usize]);
                SEEN.borrow_ref_mut(cs)[dev] = Some(seen);
            });
        }
    }
}

const APP_STACK_BYTES: usize = 16384;
const APP_STACK_LOW_BYTES: u32 = 256;
const CONNECTIONS_MAX: usize = 2;
const L2CAP_CHANNELS_MAX: usize = 1;

type Ctrl = ExternalController<BleConnector<'static>, 4>;
type BleStack<'a> = trouble_host::Stack<'a, Ctrl, DefaultPacketPool>;
type Client<'a> = GattClient<'a, Ctrl, DefaultPacketPool, 4>;

/// Texte d'erreur formaté sur le cœur 1, journalisé par le cœur 0.
type ErrText = heapless::String<64>;

fn err_text(stage: &str, err: impl core::fmt::Debug) -> ErrText {
    use core::fmt::Write;
    let mut text = ErrText::new();
    let _ = write!(text, "{stage}: {err:?}");
    text
}

/// Scan actif continu (fenêtre = intervalle), sur le modèle de l'exemple
/// `ble/scanner` d'esp-hal, jusqu'à avoir vu les deux appareils. Puis un
/// lien par appareil, en parallèle. Tourne dans l'exécuteur du cœur 1.
#[embassy_executor::task]
async fn ble(bt: BT<'static>) {
    let config = esp_radio::ble::Config::default().with_task_cpu(Cpu::AppCpu);
    let connector = BleConnector::new(bt, config).unwrap();
    let controller: Ctrl = ExternalController::new(connector);

    // Adresse aléatoire fixe, comme l'exemple : aucun appairage.
    let address = Address::random([0xff, 0x8f, 0x1b, 0x05, 0xe4, 0xff]);
    let mut resources: HostResources<_, DefaultPacketPool, CONNECTIONS_MAX, L2CAP_CHANNELS_MAX> =
        HostResources::new();
    let stack = trouble_host::new(controller, &mut resources)
        .set_random_address(address)
        .build();
    let mut runner = stack.runner();

    let _ = embassy_futures::join::join(runner.run_with_handler(&ScanHandler), async {
        scan_until_both(&stack).await;
        embassy_futures::join::join(link(&stack, SCALE), link(&stack, PROBE)).await;
    })
    .await;
}

async fn scan_until_both(stack: &BleStack<'_>) {
    let mut scanner = Scanner::new(stack.central());
    let config = ScanConfig {
        active: true,
        phys: PhySet::M1,
        interval: Duration::from_secs(1),
        window: Duration::from_secs(1),
        ..Default::default()
    };
    {
        let _session = scanner.scan(&config).await.unwrap();
        BLE_PHASE.store(PHASE_SCAN, Relaxed);
        while critical_section::with(|cs| TARGETS.borrow_ref(cs).iter().any(Option::is_none)) {
            Timer::after(Duration::from_millis(100)).await;
        }
    }
    // Lâcher la session demande l'arrêt du scan, que l'hôte envoie de son
    // côté : on lui laisse le temps avant la première connexion.
    Timer::after(Duration::from_millis(200)).await;
    BLE_PHASE.store(PHASE_LINKS, Relaxed);
}

/// Un lien : connexion par adresse (le contrôleur cherche l'appareil
/// lui-même, sans scan de l'hôte), découverte, abonnement, puis réception
/// jusqu'à la déconnexion. Recommence indéfiniment. Les deux liens partagent
/// l'initiateur du contrôleur : trouble-host sérialise leurs connexions.
async fn link(stack: &BleStack<'_>, dev: usize) {
    let target = critical_section::with(|cs| TARGETS.borrow_ref(cs)[dev]).unwrap();
    let accept = [target];
    let config = ConnectConfig {
        scan_config: ScanConfig {
            active: false,
            filter_accept_list: &accept,
            phys: PhySet::M1,
            interval: Duration::from_millis(60),
            window: Duration::from_millis(30),
            ..Default::default()
        },
        // Paramètres initiaux par défaut de NimBLE, ceux de coffeeflow.
        connect_params: RequestedConnParams {
            min_connection_interval: Duration::from_millis(30),
            max_connection_interval: Duration::from_millis(50),
            max_latency: 0,
            min_event_length: Duration::from_secs(0),
            max_event_length: Duration::from_secs(0),
            supervision_timeout: Duration::from_millis(2560),
        },
    };

    loop {
        LINK_STATE[dev].store(LINK_CONNECTING, Relaxed);
        let mut central = stack.central();
        let conn = match with_timeout(Duration::from_secs(10), central.connect(&config)).await {
            Ok(Ok(conn)) => conn,
            Ok(Err(e)) => {
                link_failed(dev, err_text("connexion", e));
                Timer::after(Duration::from_secs(2)).await;
                continue;
            }
            Err(_) => {
                link_failed(dev, err_text("connexion", "aucune reponse en 10 s"));
                continue;
            }
        };
        CONNECTS[dev].fetch_add(1, Relaxed);
        INTERVAL_US[dev].store(conn.params().conn_interval.as_micros() as u32, Relaxed);
        LINK_STATE[dev].store(LINK_DISCOVERING, Relaxed);

        let err = match Client::new(stack, &conn).await {
            Err(e) => err_text("mtu", e),
            Ok(client) => {
                let session = async {
                    if dev == SCALE {
                        scale_session(&client).await
                    } else {
                        probe_session(&client).await
                    }
                };
                match select3(client.task(), session, conn_events(&conn, dev)).await {
                    Either3::First(r) => err_text("gatt", r),
                    Either3::Second(text) => text,
                    Either3::Third(reason) => err_text("deconnexion", reason),
                }
            }
        };
        if LINK_STATE[dev].load(Relaxed) == LINK_SUBSCRIBED {
            DROPS[dev].fetch_add(1, Relaxed);
            set_error(dev, err);
        } else {
            link_failed(dev, err);
        }
        LINK_STATE[dev].store(LINK_DOWN, Relaxed);
        if conn.is_connected() {
            conn.disconnect();
            let _ = with_timeout(Duration::from_secs(5), conn_events(&conn, dev)).await;
        }
        Timer::after(Duration::from_secs(1)).await;
    }
}

fn link_failed(dev: usize, err: ErrText) {
    FAILS[dev].fetch_add(1, Relaxed);
    set_error(dev, err);
}

fn set_error(dev: usize, err: ErrText) {
    critical_section::with(|cs| LAST_ERROR.borrow_ref_mut(cs)[dev] = Some(err));
}

/// Vide la file d'événements de la connexion (deux places) et rend la cause
/// de la déconnexion. Retient l'intervalle accepté par l'appareil.
async fn conn_events(conn: &Connection<'_, DefaultPacketPool>, dev: usize) -> Status {
    loop {
        match conn.next().await {
            ConnectionEvent::Disconnected { reason } => return reason,
            ConnectionEvent::ConnectionParamsUpdated { conn_interval, .. } => {
                INTERVAL_US[dev].store(conn_interval.as_micros() as u32, Relaxed);
            }
            _ => {}
        }
    }
}

// Acaia Lunar (ble_scale.cpp). Service et caractéristique legacy, confirmés
// sur ACAIAL-C1EF4C9 ; repli sur le service moderne des Lunar récentes, à
// canaux d'écriture et de notification distincts. UUID 128 bits dans
// l'ordre de l'air.
const ACAIA_LEGACY_SERVICE: Uuid = Uuid::new_short(0x1820);
const ACAIA_LEGACY_CHAR: Uuid = Uuid::new_short(0x2a80);
const ACAIA_SERVICE: Uuid = Uuid::new_long([
    0x55, 0xe4, 0x05, 0xd2, 0xaf, 0x9f, 0xa9, 0x8f, 0xe5, 0x4a, 0x7d, 0xfe, 0x43, 0x53, 0x53, 0x49,
]);
const ACAIA_WRITE: Uuid = Uuid::new_long([
    0xb3, 0x9b, 0x72, 0x34, 0xbe, 0xec, 0xd4, 0xa8, 0xf4, 0x43, 0x41, 0x88, 0x43, 0x53, 0x53, 0x49,
]);
const ACAIA_NOTIFY: Uuid = Uuid::new_long([
    0x16, 0x96, 0x24, 0x47, 0xc6, 0x23, 0x61, 0xba, 0xd9, 0x4b, 0x4d, 0x1e, 0x43, 0x53, 0x53, 0x49,
]);
const ACAIA_HEARTBEAT: Duration = Duration::from_millis(2500);

/// Poignée d'écriture et caractéristique notifiée de la balance.
async fn scale_discover(client: &Client<'_>) -> Result<(u16, Characteristic<[u8]>), ErrText> {
    let legacy = client
        .services_by_uuid(&ACAIA_LEGACY_SERVICE)
        .await
        .map_err(|e| err_text("service", e))?;
    if let Some(service) = legacy.first() {
        let chr: Characteristic<[u8]> = client
            .characteristic_by_uuid(service, &ACAIA_LEGACY_CHAR)
            .await
            .map_err(|e| err_text("caracteristique", e))?;
        return Ok((chr.handle, chr));
    }
    let modern = client
        .services_by_uuid(&ACAIA_SERVICE)
        .await
        .map_err(|e| err_text("service moderne", e))?;
    let service = modern
        .first()
        .ok_or_else(|| err_text("service", "absent"))?;
    let write: Characteristic<[u8]> = client
        .characteristic_by_uuid(service, &ACAIA_WRITE)
        .await
        .map_err(|e| err_text("ecriture", e))?;
    let notify: Characteristic<[u8]> = client
        .characteristic_by_uuid(service, &ACAIA_NOTIFY)
        .await
        .map_err(|e| err_text("notification", e))?;
    Ok((write.handle, notify))
}

/// Trame Acaia : en-tête EF DD, commande, longueur facultative, charge,
/// puis les sommes des octets de rang pair et impair après la commande.
async fn acaia_send(
    client: &Client<'_>,
    handle: u16,
    cmd: u8,
    payload: &[u8],
    length_prefix: bool,
) -> Result<(), ErrText> {
    let mut frame = [0u8; 24];
    let mut n = 0;
    for b in [0xef, 0xdd, cmd] {
        frame[n] = b;
        n += 1;
    }
    if length_prefix {
        frame[n] = payload.len() as u8 + 1;
        n += 1;
    }
    frame[n..n + payload.len()].copy_from_slice(payload);
    n += payload.len();
    let (mut even, mut odd) = (0u8, 0u8);
    for (i, &b) in frame[3..n].iter().enumerate() {
        if i % 2 == 0 {
            even = even.wrapping_add(b);
        } else {
            odd = odd.wrapping_add(b);
        }
    }
    frame[n] = even;
    frame[n + 1] = odd;
    n += 2;
    client
        .write_handle_without_response(handle, &frame[..n])
        .await
        .map_err(|e| err_text("ecriture", e))
}

/// Identification, heartbeat et demande de notifications, envoyés ensemble
/// à l'abonnement puis toutes les 2,5 s, comme coffeeflow.
async fn acaia_heartbeat(client: &Client<'_>, handle: u16) -> Result<(), ErrText> {
    const ID: [u8; 15] = *b"012345678901234";
    const REQUEST: [u8; 8] = [0x00, 0x01, 0x01, 0x02, 0x02, 0x05, 0x03, 0x04];
    acaia_send(client, handle, 0x0b, &ID, false).await?;
    acaia_send(client, handle, 0x00, &[0], true).await?;
    acaia_send(client, handle, 0x0c, &REQUEST, true).await?;
    HEARTBEATS.fetch_add(1, Relaxed);
    Ok(())
}

async fn scale_session(client: &Client<'_>) -> ErrText {
    let (write_handle, notify) = match scale_discover(client).await {
        Ok(found) => found,
        Err(e) => return e,
    };
    let mut listener = match client.subscribe(&notify, false).await {
        Ok(listener) => listener,
        Err(e) => return err_text("abonnement", e),
    };
    LINK_STATE[SCALE].store(LINK_SUBSCRIBED, Relaxed);

    let mut rx = AcaiaRx::new();
    let mut next_heartbeat = Instant::now();
    loop {
        match select(listener.next(), Timer::at(next_heartbeat)).await {
            Either::First(notification) => {
                NOTIFS[SCALE].fetch_add(1, Relaxed);
                if rx.consume(notification.as_ref()) {
                    return err_text("balance", "deconnexion demandee (0x20)");
                }
            }
            Either::Second(()) => {
                if let Err(e) = acaia_heartbeat(client, write_handle).await {
                    return e;
                }
                next_heartbeat += ACAIA_HEARTBEAT;
            }
        }
    }
}

/// Recompose les trames Acaia réparties sur plusieurs notifications
/// (consume_rx de ble_scale.cpp).
struct AcaiaRx {
    buf: [u8; 128],
    len: usize,
}

impl AcaiaRx {
    const fn new() -> Self {
        Self {
            buf: [0; 128],
            len: 0,
        }
    }

    /// Vrai si la balance demande la déconnexion.
    fn consume(&mut self, data: &[u8]) -> bool {
        if data.len() > self.buf.len() {
            return false;
        }
        if self.len + data.len() > self.buf.len() {
            self.len = 0;
        }
        self.buf[self.len..self.len + data.len()].copy_from_slice(data);
        self.len += data.len();
        while self.len >= 3 {
            if self.buf[0] != 0xef || self.buf[1] != 0xdd {
                self.drop_front(1);
                continue;
            }
            if self.buf[2] == 0x20 {
                self.len = 0;
                return true;
            }
            if self.len < 6 {
                return false;
            }
            let payload_len = self.buf[3] as usize;
            let frame_len = payload_len + 5;
            if payload_len == 0 || frame_len > self.buf.len() {
                self.len = 0;
                return false;
            }
            if self.len < frame_len {
                return false;
            }
            if self.buf[2] == 12 {
                acaia_status(&self.buf[4..4 + payload_len]);
            }
            self.drop_front(frame_len);
        }
        false
    }

    fn drop_front(&mut self, n: usize) {
        self.buf.copy_within(n..self.len, 0);
        self.len -= n;
    }
}

/// Sous-messages d'une trame d'état : seule la pesée (type 5) est retenue.
fn acaia_status(payload: &[u8]) {
    let mut i = 0;
    while i < payload.len() {
        let kind = payload[i];
        i += 1;
        match kind {
            5 if i + 6 <= payload.len() => {
                WEIGHT_BITS.store(acaia_weight(&payload[i..i + 6]).to_bits(), Relaxed);
                WEIGHTS.fetch_add(1, Relaxed);
                i += 6;
            }
            7 if i + 3 <= payload.len() => i += 3,
            8 if i < payload.len() => i += 1,
            _ => break,
        }
    }
}

fn acaia_weight(data: &[u8]) -> f32 {
    let raw = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as f32;
    let weight = match data[4] {
        1 => raw / 10.0,
        2 => raw / 100.0,
        3 => raw / 1000.0,
        4 => raw / 10000.0,
        _ => raw,
    };
    if data[5] & 0x02 != 0 { -weight } else { weight }
}

// Sonde SCACE (common/scace_ble.hpp) : une trame de 12 octets à 10 Hz.
// UUID dans l'ordre de l'air.
const SCACE_SERVICE: Uuid = Uuid::new_long([
    0xfe, 0x01, 0x12, 0x4b, 0xb2, 0x61, 0x2e, 0xa2, 0xd8, 0x46, 0xa7, 0x7f, 0x0a, 0x12, 0x17, 0x50,
]);
const SCACE_FRAME: Uuid = Uuid::new_long([
    0x38, 0x9a, 0xa7, 0xae, 0x17, 0x6f, 0x21, 0xb2, 0x9a, 0x4c, 0x71, 0x11, 0x74, 0xf5, 0xfe, 0x84,
]);
const SCACE_FRAME_SIZE: usize = 12;
const SCACE_STATUS_MAX: u8 = 3;
const SCACE_NO_TEMPERATURE: i16 = i16::MIN;

async fn probe_session(client: &Client<'_>) -> ErrText {
    let services = match client.services_by_uuid(&SCACE_SERVICE).await {
        Ok(services) => services,
        Err(e) => return err_text("service", e),
    };
    let Some(service) = services.first() else {
        return err_text("service", "absent");
    };
    let frame: Characteristic<[u8]> = match client.characteristic_by_uuid(service, &SCACE_FRAME).await {
        Ok(chr) => chr,
        Err(e) => return err_text("caracteristique", e),
    };
    let mut listener = match client.subscribe(&frame, false).await {
        Ok(listener) => listener,
        Err(e) => return err_text("abonnement", e),
    };
    LINK_STATE[PROBE].store(LINK_SUBSCRIBED, Relaxed);

    // Les trames manquées se lisent dans les numéros de séquence. La file de
    // notifications de trouble-host n'a qu'une place : une trame non lue à
    // temps est écrasée par la suivante, et compte comme manquée.
    let mut last_seq: Option<u16> = None;
    loop {
        let notification = listener.next().await;
        NOTIFS[PROBE].fetch_add(1, Relaxed);
        let data = notification.as_ref();
        if data.len() < SCACE_FRAME_SIZE || data[10] > SCACE_STATUS_MAX {
            PROBE_INVALID.fetch_add(1, Relaxed);
            continue;
        }
        let seq = u16::from_le_bytes([data[0], data[1]]);
        if let Some(last) = last_seq {
            // Un saut énorme est un redémarrage de la sonde, pas une perte.
            let gap = seq.wrapping_sub(last).wrapping_sub(1);
            if gap < 1000 {
                PROBE_LOST.fetch_add(gap as u32, Relaxed);
            }
        }
        last_seq = Some(seq);
        PROBE_CENTI_C.store(i16::from_le_bytes([data[8], data[9]]) as u16 as u32, Relaxed);
        PROBE_STATUS.store(data[10] as u32, Relaxed);
    }
}

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    // Peindre la pile du cœur 0 avant tout le reste, sous la trame courante.
    // Le premier Ko, où esp-hal et esp-rtos posent leur garde, reste intact.
    {
        let sp: u32;
        unsafe { core::arch::asm!("mov {0}, a1", out(reg) sp, options(nomem, nostack)) };
        let low = (&raw const _stack_end_cpu0 as u32) + 1024;
        paint_stack(low as *mut u32, ((sp - 512 - low) / 4) as usize);
        STACK0_LOW.store(low, Relaxed);
    }

    esp_println::logger::init_logger_from_env();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 73744);
    info!("tas interne: {} octets libres", esp_alloc::HEAP.free());

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0, peripherals.FROM_CPU_INTR0);

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
    // occupent le début. Le framebuffer suit ; rien d'autre n'y vit.
    #[cfg(not(feature = "exec-from-flash"))]
    let code_bytes = {
        let t0 = Instant::now();
        let bytes = xip::run(psram_start, psram_size);
        info!(
            "XIP: {} pages de code et rodata en PSRAM en {} ms",
            xip::PAGES.load(Relaxed),
            t0.elapsed().as_millis()
        );
        bytes
    };
    #[cfg(feature = "exec-from-flash")]
    let code_bytes = 0;
    assert!(psram_size - code_bytes >= FB_PX * 2, "PSRAM absente ou trop petite");
    let mut fb = Fb(psram_start.wrapping_add(code_bytes).cast::<u16>());
    let t0 = Instant::now();
    let _ = fb.fill_contiguous(
        &fb.bounding_box(),
        (0..FB_PX).map(|i| RawU16::new(pattern(i % H_RES, i / H_RES)).into()),
    );
    info!("mire dessinée en {} ms", t0.elapsed().as_millis());
    FB.store(fb.0 as u32, Relaxed);

    // Mise sous tension de la dalle, comme board::panel_power_on() : GT911
    // IRQ bas pendant le reset (adresse 0x5D), DISP haut, LCD_RST et TP_RST
    // bas 100 ms puis hauts 200 ms. Chaque écriture porte l'octet complet.
    let _touch_irq = Output::new(peripherals.GPIO4, Level::Low, OutputConfig::default());
    let mut i2c = I2c::new(
        peripherals.I2C0,
        I2cConfig::default().with_frequency(Rate::from_khz(400)),
    )
    .unwrap()
    .with_sda(peripherals.GPIO8)
    .with_scl(peripherals.GPIO9);
    i2c.write(CH422G_MODE, &[CH422G_MODE_OUTPUTS]).unwrap();
    i2c.write(CH422G_OUT, &[EXIO_DISP]).unwrap();
    Timer::after(Duration::from_millis(100)).await;
    i2c.write(CH422G_OUT, &[EXIO_DISP | EXIO_LCD_RST | EXIO_TP_RST])
        .unwrap();
    Timer::after(Duration::from_millis(200)).await;
    info!("CH422G: dalle alimentée, CAN_SEL bas");

    let dpi_config = DpiConfig::default()
        .with_frequency(Rate::from_mhz(16))
        // Données émises sur le front descendant (pclk_active_neg d'IDF).
        .with_clock_mode(ClockMode {
            polarity: Polarity::IdleLow,
            phase: Phase::ShiftHigh,
        })
        .with_format(Format {
            enable_2byte_mode: true,
            ..Default::default()
        })
        .with_timing(FrameTiming {
            horizontal_active_width: H_RES,
            horizontal_total_width: HSYNC_PULSE + HSYNC_BACK + H_RES + HSYNC_FRONT,
            horizontal_blank_front_porch: HSYNC_PULSE + HSYNC_BACK,
            vertical_active_height: V_RES,
            vertical_total_height: VSYNC_PULSE + VSYNC_BACK + V_RES + VSYNC_FRONT,
            vertical_blank_front_porch: VSYNC_PULSE + VSYNC_BACK,
            hsync_width: HSYNC_PULSE,
            vsync_width: VSYNC_PULSE,
            hsync_position: 0,
        })
        .with_hsync_idle_level(Level::High)
        .with_vsync_idle_level(Level::High)
        .with_de_idle_level(Level::Low);

    let lcd_cam = LcdCam::new(peripherals.LCD_CAM);
    let dpi = Dpi::new(lcd_cam.lcd, peripherals.DMA_CH0, dpi_config)
        .unwrap()
        .with_hsync(peripherals.GPIO46)
        .with_vsync(peripherals.GPIO3)
        .with_de(peripherals.GPIO5)
        .with_pclk(peripherals.GPIO7)
        .with_data0(peripherals.GPIO14)
        .with_data1(peripherals.GPIO38)
        .with_data2(peripherals.GPIO18)
        .with_data3(peripherals.GPIO17)
        .with_data4(peripherals.GPIO10)
        .with_data5(peripherals.GPIO39)
        .with_data6(peripherals.GPIO0)
        .with_data7(peripherals.GPIO45)
        .with_data8(peripherals.GPIO48)
        .with_data9(peripherals.GPIO47)
        .with_data10(peripherals.GPIO21)
        .with_data11(peripherals.GPIO1)
        .with_data12(peripherals.GPIO2)
        .with_data13(peripherals.GPIO42)
        .with_data14(peripherals.GPIO41)
        .with_data15(peripherals.GPIO40);

    // Comme IDF : la durée de la phase de données est fixée par le DMA, pas
    // par un compteur de cycles. esp-hal ne pose pas ce bit en mode DPI.
    lcd_regs()
        .lcd_user()
        .modify(|_, w| w.lcd_always_out_en().set_bit());

    // Les deux buffers partent avec les blocs 0 et 1 de la trame.
    let ring = BounceRing::new();
    NEXT_CHUNK.store(0, Relaxed);
    fill_bounce(0);
    fill_bounce(1);
    info!(
        "bounce: 2 x {BB_LINES} lignes ({BB_BYTES} octets) en SRAM, {N_RING} descripteurs"
    );

    // Les interruptions sont branchées sur ce cœur, mais restent masquées
    // côté périphérique jusqu'au démarrage en phase.
    interrupt::bind_handler(
        Interrupt::DMA_OUT_CH0,
        InterruptHandler::new(on_dma_eof, Priority::Priority3),
    );
    interrupt::bind_handler(
        Interrupt::LCD_CAM,
        InterruptHandler::new(on_vsync, Priority::Priority3),
    );

    // Le transfert tourne tant que `_transfer` vit : le lâcher arrêterait
    // le DMA.
    let _transfer = match dpi.send(true, ring) {
        Ok(transfer) => transfer,
        Err((err, _, _)) => panic!("échec du démarrage DPI: {err:?}"),
    };
    restart_in_phase();
    EOF_COUNT.store(0, Relaxed);
    LAST_EOF_US.store(
        esp_hal::time::Instant::now().duration_since_epoch().as_micros() as u32,
        Relaxed,
    );
    dma_tx()
        .out_int()
        .clr()
        .write(|w| w.out_eof().clear_bit_by_one());
    lcd_regs()
        .lc_dma_int_clr()
        .write(|w| w.lcd_vsync_int_clr().set_bit());
    dma_tx().out_int().ena().modify(|_, w| w.out_eof().set_bit());
    lcd_regs()
        .lc_dma_int_ena()
        .modify(|_, w| w.lcd_vsync_int_ena().set_bit());
    // Laisser l'UART vider la ligne précédente : un info! qui attend la FIFO
    // bloque les interruptions et fait partir une recopie en retard.
    Timer::after(Duration::from_millis(200)).await;
    STEPS.store(0, Relaxed);
    DRAWN_PX.store(0, Relaxed);

    info!("DPI démarré ; dessin sur le cœur de la recopie");

    // La radio part sur le cœur 1 une fois l'affichage lancé : BleConnector
    // y branche les interruptions BT (esp-hal les active sur le cœur qui
    // appelle), et task_cpu y épingle la tâche du contrôleur.
    static APP_CORE_STACK: StaticCell<Stack<APP_STACK_BYTES>> = StaticCell::new();
    static APP_EXECUTOR: StaticCell<esp_rtos::embassy::Executor> = StaticCell::new();
    let app_stack = APP_CORE_STACK.init(Stack::new());
    let app_stack_low = app_stack.mem.as_mut_ptr().cast::<u32>();
    paint_stack(app_stack_low, APP_STACK_BYTES / 4);
    STACK1_BASE.store(app_stack_low as u32, Relaxed);
    let bt = peripherals.BT;
    esp_rtos::start_second_core(
        peripherals.CPU_CTRL,
        peripherals.FROM_CPU_INTR1,
        app_stack,
        move || {
            let executor = APP_EXECUTOR.init(esp_rtos::embassy::Executor::new());
            executor.run(|spawner| spawner.spawn(ble(bt).unwrap()));
        },
    );

    spawner.spawn(report().unwrap());
    let mut scene = Scene::new();
    #[cfg(not(feature = "draw-continuous"))]
    let mut ticker = embassy_time::Ticker::every(Duration::from_millis(100));
    loop {
        scene.draw_step(&mut fb);
        #[cfg(feature = "draw-continuous")]
        embassy_futures::yield_now().await;
        #[cfg(not(feature = "draw-continuous"))]
        ticker.next().await;
    }
}
