#![no_std]
#![no_main]
#![feature(asm_experimental_arch)]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

#[cfg(feature = "esp-storage-cs")]
compile_error!(
    "la Rust A écrit la flash comme IDF depuis la PSRAM, sans la section critique d'esp-storage"
);

// Essai 08b, Rust A : l'essai 08a sans sa charge de flash, et avec une
// vraie mise à jour (module `ota`). Une image envoyée par `POST /firmware`
// s'installe dans le slot qui n'a pas démarré, `otadata` la désigne en état
// `NEW`, et la carte redémarre dessus. Wi-Fi seul par défaut, comme
// coffeeflow pendant une mise à jour.
//
// Deux tas : la SRAM interne (`esp_alloc::HEAP`) pour la radio, la PSRAM
// (`PSRAM_HEAP`) pour Slint. L'allocateur global choisit selon le cœur ; les
// malloc des blobs radio passent par `HEAP`, qui n'a que de la SRAM
// interne.
//
// Le cœur 1 ne journalise rien lui-même : esp-println écrit sous une section
// critique partagée par les deux cœurs, et le cœur 0 resterait bloqué,
// interruptions masquées, le temps que l'autre vide sa ligne.

use alloc::boxed::Box;
use alloc::rc::Rc;
use core::alloc::{GlobalAlloc, Layout};
use core::sync::atomic::{AtomicU32, Ordering::Relaxed};

use embassy_executor::Spawner;
use embassy_time::{Duration, Instant, Timer};
use esp_backtrace as _;
use esp_hal::clock::CpuClock;
use esp_hal::dma::{BurstConfig, DmaDescriptor, DmaTxBuffer, Owner, Preparation};
use esp_hal::gpio::{Level, Output, OutputConfig};
use esp_hal::i2c::master::{Config as I2cConfig, I2c};
use esp_hal::interrupt::{self, InterruptHandler, Priority};
use esp_hal::lcd_cam::LcdCam;
use esp_hal::lcd_cam::lcd::dpi::{Config as DpiConfig, Dpi, Format, FrameTiming};
use esp_hal::lcd_cam::lcd::{ClockMode, Phase, Polarity};
use esp_hal::peripherals::{Interrupt, BT, WIFI};
use esp_hal::psram::{Psram, PsramConfig, PsramMode, SpiRamFreq};
use esp_hal::system::{Cpu, Stack};
use esp_hal::time::Rate;
use esp_hal::timer::timg::TimerGroup;
use log::info;
use slint::ComponentHandle;
use slint::platform::software_renderer::{
    LineBufferProvider, MinimalSoftwareWindow, RepaintBufferType, Rgb565Pixel,
};
use slint::platform::{Platform, PointerEventButton, WindowAdapter, WindowEvent};
use static_cell::StaticCell;

extern crate alloc;

use trial_08b_ota::otadata::Otadata;
use trial_08b_ota::{MainWindow, demo, xip};

#[cfg_attr(feature = "sans-ble", allow(dead_code))]
mod ble;
mod ota;
mod wifi;

esp_bootloader_esp_idf::esp_app_desc!();

const H_RES: usize = 800;
const V_RES: usize = 480;
const FB_PX: usize = H_RES * V_RES;
const FB_BYTES: usize = FB_PX * 2;

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

// Fond de l'UI (Theme.bg, #16110C) en RGB565, avant le premier rendu.
const BG_565: u16 = ((0x16 >> 3) << 11) | ((0x11 >> 2) << 5) | (0x0C >> 3);

// Bounce buffers et descripteurs en SRAM interne (.bss). Seules les
// interruptions y touchent une fois l'affichage démarré.
static mut BOUNCE: [[u32; BB_BYTES / 4]; 2] = [[0; BB_BYTES / 4]; 2];
static mut RING: [DmaDescriptor; N_RING + 1] = [DmaDescriptor::EMPTY; N_RING + 1];

// Ligne que Slint compose en SRAM interne.
static mut LINE: [Rgb565Pixel; H_RES] = [Rgb565Pixel(0); H_RES];

// Adresse du framebuffer PSRAM, posée une fois avant le démarrage.
static FB: AtomicU32 = AtomicU32::new(0);
// Prochain bloc de BB_LINES lignes à recopier.
static NEXT_CHUNK: AtomicU32 = AtomicU32::new(0);
// Fins de buffer reçues depuis le dernier VSYNC ; sa parité désigne le buffer
// que le DMA vient de finir.
static EOF_COUNT: AtomicU32 = AtomicU32::new(0);

// Mesures de la recopie, lues par le rapport.
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

/// Dernières recopies en retard : écart depuis la fin de buffer précédente,
/// durée de la recopie, et PC interrompu. Écrit par l'interruption seule, lu
/// par le rapport ; une entrée écrasée avant lecture est simplement perdue.
const LATE_LOG: usize = 8;
static LATE_GAP_US: [AtomicU32; LATE_LOG] = [const { AtomicU32::new(0) }; LATE_LOG];
static LATE_COPY_US: [AtomicU32; LATE_LOG] = [const { AtomicU32::new(0) }; LATE_LOG];
static LATE_PC: [AtomicU32; LATE_LOG] = [const { AtomicU32::new(0) }; LATE_LOG];
// Rangs de OUT_DSCR à l'entrée et après la recopie, sur un octet chacun.
static LATE_DSCR: [AtomicU32; LATE_LOG] = [const { AtomicU32::new(0) }; LATE_LOG];

// Mesures du rendu, écrites par la boucle de rendu et lues par le rapport,
// sur le même exécuteur. Les valeurs « PERIOD » repartent de zéro à chaque
// rapport.
static PARTIAL_N_PERIOD: AtomicU32 = AtomicU32::new(0);
static PARTIAL_SUM_US_PERIOD: AtomicU32 = AtomicU32::new(0);
static PARTIAL_MAX_US_PERIOD: AtomicU32 = AtomicU32::new(0);
static PARTIAL_PX_PERIOD: AtomicU32 = AtomicU32::new(0);
static PARTIAL_MAX_US: AtomicU32 = AtomicU32::new(0);
static FULL_N: AtomicU32 = AtomicU32::new(0);
static FULL_LAST_US: AtomicU32 = AtomicU32::new(0);
static FULL_MAX_US: AtomicU32 = AtomicU32::new(0);
static TOUCH_PRESSES: AtomicU32 = AtomicU32::new(0);
static TOUCH_ERRORS: AtomicU32 = AtomicU32::new(0);
static PAGE: AtomicU32 = AtomicU32::new(0);

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

/// Octets de la pile du cœur 1 atteints au moins une fois, au-dessus de sa
/// zone basse.
fn stack1_used() -> u32 {
    let base = STACK1_BASE.load(Relaxed);
    APP_STACK_BYTES as u32 - APP_STACK_LOW_BYTES - stack_untouched(base + APP_STACK_LOW_BYTES)
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

/// Recopie le prochain bloc de lignes du framebuffer dans le bounce buffer
/// `bb`. Le CPU lit le framebuffer à travers le même cache que celui qui
/// l'écrit : pas de writeback nécessaire.
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

/// Fin de trame. Une trame en phase a reçu exactement BB_PER_FRAME fins de
/// buffer, et les deux buffers contiennent déjà les blocs 0 et 1 de la
/// suivante. Sinon, on relance le DMA en phase pendant le blanking vertical
/// (~2,9 ms), comme lcd_rgb_panel_try_restart_transmission() d'IDF.
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

/// Tactile GT911, lu en polling comme dans coffeeflow (l'IRQ reste tenue
/// basse depuis le reset, pour l'adresse 0x5D).
struct Gt911 {
    addr: u8,
}

enum Touch {
    /// Pas de nouvelle mesure depuis la dernière lecture.
    Unchanged,
    Released,
    Pressed(i32, i32),
}

const GT911_PRODUCT_ID: [u8; 2] = [0x81, 0x40];
const GT911_STATUS: [u8; 2] = [0x81, 0x4E];
const GT911_POINT1: [u8; 2] = [0x81, 0x50];
// coffeeflow décale l'appui de 10 px vers le bas : l'écran est vu en
// plongée, et le doigt tombe un peu au-dessus de la cible.
const TOUCH_Y_OFFSET: i32 = 10;

impl Gt911 {
    fn probe(i2c: &mut I2c<'_, esp_hal::Blocking>) -> Option<Self> {
        [0x5D, 0x14].into_iter().find_map(|addr| {
            let mut id = [0u8; 4];
            match i2c.write_read(addr, &GT911_PRODUCT_ID, &mut id) {
                Ok(()) if &id[..3] == b"911" => Some(Self { addr }),
                _ => None,
            }
        })
    }

    fn read(&self, i2c: &mut I2c<'_, esp_hal::Blocking>) -> Result<Touch, esp_hal::i2c::master::Error> {
        let mut status = [0u8];
        i2c.write_read(self.addr, &GT911_STATUS, &mut status)?;
        if status[0] & 0x80 == 0 {
            return Ok(Touch::Unchanged);
        }
        let touch = if status[0] & 0x0F == 0 {
            Touch::Released
        } else {
            let mut p = [0u8; 4];
            i2c.write_read(self.addr, &GT911_POINT1, &mut p)?;
            let x = u16::from_le_bytes([p[0], p[1]]) as i32;
            let y = u16::from_le_bytes([p[2], p[3]]) as i32 + TOUCH_Y_OFFSET;
            Touch::Pressed(x.min(H_RES as i32 - 1), y.min(V_RES as i32 - 1))
        };
        i2c.write(self.addr, &[GT911_STATUS[0], GT911_STATUS[1], 0])?;
        Ok(touch)
    }
}

/// Rendu ligne par ligne : Slint compose la partie modifiée de chaque ligne
/// dans `LINE`, en SRAM interne, puis elle est recopiée d'un bloc dans le
/// framebuffer affiché. La recopie des bounce buffers ne voit donc jamais
/// une zone à moitié dessinée, seulement des lignes anciennes ou nouvelles.
struct LineToFb(*mut Rgb565Pixel);

impl LineBufferProvider for LineToFb {
    type TargetPixel = Rgb565Pixel;

    fn process_line(
        &mut self,
        line: usize,
        range: core::ops::Range<usize>,
        render_fn: impl FnOnce(&mut [Rgb565Pixel]),
    ) {
        // Seule la boucle de rendu touche LINE.
        let line_buf = unsafe { &mut *(&raw mut LINE) };
        let buf = &mut line_buf[range.clone()];
        render_fn(buf);
        let dst = self.0.wrapping_add(line * H_RES + range.start);
        unsafe { core::ptr::copy_nonoverlapping(buf.as_ptr(), dst, buf.len()) };
    }
}

/// Plateforme Slint minimale : une seule fenêtre, et le temps d'embassy.
struct Board {
    window: Rc<MinimalSoftwareWindow>,
}

impl Platform for Board {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
        Ok(self.window.clone())
    }

    fn duration_since_start(&self) -> core::time::Duration {
        core::time::Duration::from_micros(Instant::now().as_micros())
    }
}

/// Une ligne courte à la fois, espacées de 20 ms : esp-println écrit sous
/// section critique, et une ligne qui ne tient pas dans la FIFO UART
/// (128 octets) bloquerait les interruptions le temps de la vider.
#[embassy_executor::task]
#[allow(
    clippy::large_stack_frames,
    reason = "futures d'embassy et de trouble-host : clippy compte l'état de la future, qui vit dans l'arène de l'exécuteur ; trames réelles mesurées (emit-stack-sizes) de 3,1 Ko au plus"
)]
async fn report() {
    let start = Instant::now();
    let (mut last_frames, mut last_req) = (0, 0);
    let mut last_late = 0;
    let mut ble_report = ble::ReportState::new();
    loop {
        Timer::after(Duration::from_secs(10)).await;
        let frames = FRAMES.load(Relaxed);
        info!(
            "{}s tr={}/s ret={} recal={} copie={}/{}us ecart={}us",
            start.elapsed().as_secs(),
            (frames - last_frames) / 10,
            LATE.load(Relaxed),
            RESYNCS.load(Relaxed),
            COPY_MAX_PERIOD_US.swap(0, Relaxed),
            COPY_MAX_US.load(Relaxed),
            GAP_MAX_PERIOD_US.swap(0, Relaxed),
        );
        last_frames = frames;

        Timer::after(Duration::from_millis(20)).await;
        let n = PARTIAL_N_PERIOD.swap(0, Relaxed);
        let sum = PARTIAL_SUM_US_PERIOD.swap(0, Relaxed);
        let px = PARTIAL_PX_PERIOD.swap(0, Relaxed);
        info!(
            "partiel n={} moy={}us max={}/{}us zone={}px",
            n,
            sum.checked_div(n).unwrap_or(0),
            PARTIAL_MAX_US_PERIOD.swap(0, Relaxed),
            PARTIAL_MAX_US.load(Relaxed),
            px.checked_div(n).unwrap_or(0),
        );

        Timer::after(Duration::from_millis(20)).await;
        info!(
            "complet n={} dernier={}us max={}us ecran={} appuis={} err_tactile={}",
            FULL_N.load(Relaxed),
            FULL_LAST_US.load(Relaxed),
            FULL_MAX_US.load(Relaxed),
            PAGE.load(Relaxed),
            TOUCH_PRESSES.load(Relaxed),
            TOUCH_ERRORS.load(Relaxed),
        );

        // Tas interne (radio) : libre et minimum libre, comme l'essai 05.
        // Tas PSRAM (Slint) : utilisé et plus haut niveau.
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
        let req = wifi::HTTP_REQUESTS.load(Relaxed);
        info!(
            "wifi {} sta={}/{} req={}/s err={} dhcp={}",
            wifi::AP_STATE_NAMES[wifi::AP_STATE.load(Relaxed) as usize],
            wifi::STATIONS.load(Relaxed),
            wifi::STATION_EVENTS_LOST.load(Relaxed),
            req.wrapping_sub(last_req) / 10,
            wifi::HTTP_ERRORS.load(Relaxed),
            wifi::DHCP_ERRORS.load(Relaxed),
        );
        last_req = req;

        Timer::after(Duration::from_millis(20)).await;
        info!(
            "ota {} {}/{} octets",
            ota::STATE_NAMES[ota::STATE.load(Relaxed) as usize],
            ota::RECEIVED.load(Relaxed),
            ota::EXPECTED.load(Relaxed),
        );

        Timer::after(Duration::from_millis(20)).await;
        let stack0_top = &raw const _stack_start_cpu0 as u32;
        let stack0_low = STACK0_LOW.load(Relaxed);
        let (low_words, low_first) = stack_low_writes(STACK1_BASE.load(Relaxed));
        info!(
            "pile0 {}/{} pile1 {}/{} bas={}@{} {}",
            stack0_top - stack0_low - stack_untouched(stack0_low),
            stack0_top - (&raw const _stack_end_cpu0 as u32),
            stack1_used(),
            APP_STACK_BYTES,
            low_words,
            low_first,
            RadioIrqMap::read(),
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

        ble::report(&mut ble_report).await;
    }
}

/// Réponse de `/stats` : compteurs cumulés depuis le démarrage, que la page
/// compare d'une réponse à l'autre. Le tas est le tas interne, celui de la
/// radio.
fn stats_json(out: &mut impl core::fmt::Write) -> core::fmt::Result {
    let heap = esp_alloc::HEAP.stats();
    write!(
        out,
        "{{\"t_ms\":{},\"trames\":{},\"retards\":{},\"recalages\":{},\"copie_max_us\":{},\
         \"rendu_max_us\":{},\"tas_libre\":{},\"tas_min\":{},\"pile1\":{},\"stations\":{},\
         \"requetes\":{},\"erreurs_http\":{}",
        Instant::now().as_millis(),
        FRAMES.load(Relaxed),
        LATE.load(Relaxed),
        RESYNCS.load(Relaxed),
        COPY_MAX_US.load(Relaxed),
        PARTIAL_MAX_US.load(Relaxed),
        esp_alloc::HEAP.free(),
        heap.size - heap.max_usage,
        stack1_used(),
        wifi::STATIONS.load(Relaxed),
        wifi::HTTP_REQUESTS.load(Relaxed),
        wifi::HTTP_ERRORS.load(Relaxed),
    )?;
    write!(
        out,
        ",\"ota\":\"{}\",\"ota_recus\":{},\"ota_attendus\":{}",
        ota::STATE_NAMES[ota::STATE.load(Relaxed) as usize],
        ota::RECEIVED.load(Relaxed),
        ota::EXPECTED.load(Relaxed),
    )?;
    ble::stats_json(out)?;
    out.write_str("}")
}

/// Correspondance des interruptions radio dans la matrice de chaque cœur,
/// affichée cœur 0/cœur 1 (16 = désactivée chez esp-hal). Montre sur quel
/// cœur la radio les reçoit.
struct RadioIrqMap([[u8; 2]; 4]);

impl RadioIrqMap {
    fn read() -> Self {
        let sources = [
            Interrupt::BT_BB as usize,
            Interrupt::RWBLE as usize,
            Interrupt::WIFI_MAC as usize,
            Interrupt::WIFI_PWR as usize,
        ];
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

impl core::fmt::Display for RadioIrqMap {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let [[bb0, bb1], [ble0, ble1], [mac0, mac1], [pwr0, pwr1]] = self.0;
        write!(
            f,
            "irq bb {bb0}/{bb1} rwble {ble0}/{ble1} mac {mac0}/{mac1} pwr {pwr0}/{pwr1}"
        )
    }
}

/// Tas de Slint, en PSRAM après le code et le framebuffer.
static PSRAM_HEAP: esp_alloc::EspHeap = esp_alloc::EspHeap::empty();

/// Bus de données de la PSRAM sur ESP32-S3 : toute adresse de `PSRAM_HEAP`
/// y tombe, aucune de `esp_alloc::HEAP`.
const PSRAM_DATA_BUS: core::ops::Range<usize> = 0x3C00_0000..0x3E00_0000;

/// Allocateur global : le cœur 0 (Slint, rendu, rapport) alloue en PSRAM, le
/// cœur 1 (radio) en SRAM interne. La libération suit l'adresse du bloc,
/// quel que soit le cœur qui le rend. Les malloc des blobs radio ne passent
/// pas par ici : ils vont droit à `esp_alloc::HEAP`.
struct PerCoreHeap;

unsafe impl GlobalAlloc for PerCoreHeap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        match Cpu::current() {
            Cpu::ProCpu => unsafe { PSRAM_HEAP.alloc(layout) },
            _ => unsafe { esp_alloc::HEAP.alloc(layout) },
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if PSRAM_DATA_BUS.contains(&(ptr as usize)) {
            unsafe { PSRAM_HEAP.dealloc(ptr, layout) }
        } else {
            unsafe { esp_alloc::HEAP.dealloc(ptr, layout) }
        }
    }
}

#[global_allocator]
static ALLOCATOR: PerCoreHeap = PerCoreHeap;

const APP_STACK_BYTES: usize = 16384;
const APP_STACK_LOW_BYTES: u32 = 256;

// Tas interne de l'essai 05 en combiné : 128 Ko, dont 72 récupérés du
// bootloader et le reste en .bss, pris sur la pile du cœur 0.
const HEAP_RECLAIMED: usize = 73744;
const HEAP_TOTAL: usize = 128 * 1024;

/// Périphériques radio confiés au cœur 1.
struct Radio {
    wifi: WIFI<'static>,
    bt: BT<'static>,
}

#[allow(
    clippy::large_stack_frames,
    reason = "futures d'embassy et de trouble-host : clippy compte l'état de la future, qui vit dans l'arène de l'exécuteur ; trames réelles mesurées (emit-stack-sizes) de 3,1 Ko au plus"
)]
mod radio {
    use super::*;

    /// Démarre la radio sur le cœur 1 : le BLE d'abord, comme l'exemple de
    /// coexistence d'esp-radio, puis le point d'accès et ses tâches réseau. La
    /// tâche porte ensuite les liens GATT. Avec `sans-ble`, le Wi-Fi seul,
    /// comme le mode Wi-Fi de coffeeflow, qui désinitialise le BLE : c'est
    /// le mode d'une mise à jour.
    #[embassy_executor::task]
    pub async fn radio_task(spawner: Spawner, radio: Radio) {
        #[cfg(not(feature = "sans-ble"))]
        {
            let connector = ble::connector(radio.bt);
            wifi::start(spawner, radio.wifi);
            ble::run(connector).await;
        }
        #[cfg(feature = "sans-ble")]
        {
            let _ = radio.bt;
            wifi::start(spawner, radio.wifi);
        }
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

    // Le bootloader de l'écran (console IDF désactivée) remet GPIO43 en
    // GPIO, sortie coupée : on rebranche U0TXD avant le premier journal.
    // esp-println écrit ensuite directement dans la FIFO d'UART0. Le pilote
    // vit jusqu'à la fin de main, qui ne rend jamais la main.
    let _console = esp_hal::uart::UartTx::new(peripherals.UART0, esp_hal::uart::Config::default())
        .unwrap()
        .with_tx(peripherals.GPIO43);

    // Le tas interne d'abord, comme l'essai 05 : 128 Ko pour la radio.
    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: HEAP_RECLAIMED);
    esp_alloc::heap_allocator!(size: HEAP_TOTAL - HEAP_RECLAIMED);

    let psram = Psram::new(
        peripherals.PSRAM,
        PsramConfig {
            mode: PsramMode::OctalSpi,
            ram_frequency: SpiRamFreq::Freq80m,
            ..Default::default()
        },
    );
    // Le slot démarré se lit avant le remappage du code en PSRAM.
    let mut storage = ota::storage(peripherals.FLASH);
    let layout = ota::layout(&mut storage);
    info!("reset: {:?}", esp_hal::system::reset_reason());
    if let Some(layout) = &layout {
        info!(
            "ota: démarré sur ota_{}, otadata à 0x{:x}, ota_0 0x{:x}+0x{:x}, ota_1 0x{:x}+0x{:x}",
            layout.booted,
            layout.otadata,
            layout.slots[0].0,
            layout.slots[0].1,
            layout.slots[1].0,
            layout.slots[1].1,
        );
        match Otadata::read(&mut storage, layout.otadata) {
            Ok(otadata) => info!("otadata: {otadata}"),
            Err(err) => info!("otadata illisible: {err:?}"),
        }
    }

    let (psram_start, psram_size) = psram.raw_parts();
    info!("PSRAM: {psram_size} octets à {psram_start:p}");

    // Le code et les rodata passent en PSRAM avant tout autre usage, et en
    // occupent le début. Le framebuffer suit, puis le tas de Slint.
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
    // IDF ne garde le cache actif pendant une écriture que si plus aucune
    // page ne mappe la flash ; la dernière reste au bootloader (essai 08a).
    info!("ota: pages MMU encore sur la flash: {}", xip::flash_mappings());
    // La flash ne s'écrit qu'une fois le code en PSRAM.
    match layout {
        Some(layout) => ota::init(storage, layout),
        None => info!("ota: pas de mise à jour possible"),
    }
    let heap_start = code_bytes + FB_BYTES;
    assert!(psram_size > heap_start + 1024 * 1024, "PSRAM absente ou trop petite");
    let fb: *mut Rgb565Pixel = psram_start.wrapping_add(code_bytes).cast();
    for i in 0..FB_PX {
        unsafe { fb.cast::<u16>().add(i).write(BG_565) };
    }
    FB.store(fb as u32, Relaxed);

    unsafe {
        PSRAM_HEAP.add_region(esp_alloc::HeapRegion::new(
            psram_start.wrapping_add(heap_start),
            psram_size - heap_start,
            esp_alloc::MemoryCapability::External.into(),
        ));
    }
    info!(
        "tas: {} octets en PSRAM (Slint), {} en interne (radio)",
        psram_size - heap_start,
        esp_alloc::HEAP.free()
    );

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0, peripherals.FROM_CPU_INTR0);

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

    let touch = Gt911::probe(&mut i2c);
    match &touch {
        Some(t) => info!("GT911 à 0x{:02x}", t.addr),
        None => info!("GT911 absent : pas de changement d'écran"),
    }

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
    info!("bounce: 2 x {BB_LINES} lignes ({BB_BYTES} octets) en SRAM, {N_RING} descripteurs");

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
    info!("DPI démarré ; Slint dessine sur le cœur de la recopie");

    // La radio part sur le cœur 1 une fois l'affichage lancé : esp-hal y
    // active les interruptions radio (sur le cœur qui crée le contrôleur),
    // le Wi-Fi y épingle sa tâche (wifi_task_core_id = cœur courant), et
    // task_cpu y épingle la tâche du contrôleur BLE.
    static APP_CORE_STACK: StaticCell<Stack<APP_STACK_BYTES>> = StaticCell::new();
    static APP_EXECUTOR: StaticCell<esp_rtos::embassy::Executor> = StaticCell::new();
    let app_stack = APP_CORE_STACK.init(Stack::new());
    let app_stack_low = app_stack.mem.as_mut_ptr().cast::<u32>();
    paint_stack(app_stack_low, APP_STACK_BYTES / 4);
    STACK1_BASE.store(app_stack_low as u32, Relaxed);
    let radio = Radio {
        wifi: peripherals.WIFI,
        bt: peripherals.BT,
    };
    esp_rtos::start_second_core(
        peripherals.CPU_CTRL,
        peripherals.FROM_CPU_INTR1,
        app_stack,
        move || {
            let executor = APP_EXECUTOR.init(esp_rtos::embassy::Executor::new());
            executor.run(|spawner| spawner.spawn(radio::radio_task(spawner, radio).unwrap()));
        },
    );

    // Le rendu ligne par ligne réécrit le framebuffer affiché : seule la
    // zone modifiée est à redessiner, comme en ReusedBuffer.
    let window = MinimalSoftwareWindow::new(RepaintBufferType::ReusedBuffer);
    window.set_size(slint::PhysicalSize::new(H_RES as u32, V_RES as u32));
    slint::platform::set_platform(Box::new(Board { window: window.clone() }))
        .expect("plateforme Slint déjà posée");
    let ui = MainWindow::new().unwrap();
    ui.show().unwrap();
    let _demo = demo::run(&ui, ble::probe_celsius);
    Timer::after(Duration::from_millis(20)).await;
    info!("Slint: ligne par ligne, 4 Hz par groupe, radio et mise à jour sur le coeur 1");

    spawner.spawn(report().unwrap());
    spawner.spawn(ota::log_task().unwrap());
    render_loop(&window, &ui, fb, i2c, touch).await
}

/// Boucle de rendu, sur le cœur de la recopie : minuteurs et animations de
/// Slint, tactile toutes les 20 ms, puis rendu de ce qui a changé. Chaque
/// tour se termine par une pause de 10 ms, même après un rendu long : le
/// rapport, sur le même exécuteur, doit pouvoir passer.
async fn render_loop(
    window: &MinimalSoftwareWindow,
    ui: &MainWindow,
    fb: *mut Rgb565Pixel,
    mut i2c: I2c<'static, esp_hal::Blocking>,
    touch: Option<Gt911>,
) -> ! {
    let mut last_page = -1;
    // Le rendu qui suit un changement d'écran est complet.
    let mut full_pending = false;
    let mut pressed: Option<slint::LogicalPosition> = None;
    let mut iteration = 0u32;
    loop {
        slint::platform::update_timers_and_animations();

        iteration = iteration.wrapping_add(1);
        if let (Some(touch), true) = (&touch, iteration.is_multiple_of(2)) {
            match touch.read(&mut i2c) {
                Ok(Touch::Unchanged) => {}
                Ok(Touch::Pressed(x, y)) => {
                    let position = slint::LogicalPosition::new(x as f32, y as f32);
                    let event = match pressed.replace(position) {
                        Some(_) => WindowEvent::PointerMoved { position },
                        None => {
                            TOUCH_PRESSES.fetch_add(1, Relaxed);
                            WindowEvent::PointerPressed { position, button: PointerEventButton::Left }
                        }
                    };
                    window.dispatch_event(event);
                }
                Ok(Touch::Released) => {
                    if let Some(position) = pressed.take() {
                        window.dispatch_event(WindowEvent::PointerReleased {
                            position,
                            button: PointerEventButton::Left,
                        });
                        window.dispatch_event(WindowEvent::PointerExited);
                    }
                }
                Err(_) => {
                    TOUCH_ERRORS.fetch_add(1, Relaxed);
                }
            }
        }

        let page = ui.get_page();
        if page != last_page {
            last_page = page;
            full_pending = true;
            PAGE.store(page as u32, Relaxed);
        }

        window.draw_if_needed(|renderer| {
            let t0 = esp_hal::time::Instant::now();
            let region = renderer.render_by_line(LineToFb(fb));
            let us = t0.elapsed().as_micros() as u32;
            if full_pending {
                full_pending = false;
                FULL_N.fetch_add(1, Relaxed);
                FULL_LAST_US.store(us, Relaxed);
                FULL_MAX_US.fetch_max(us, Relaxed);
            } else {
                let px: u32 = region.iter().map(|(_, s)| s.width * s.height).sum();
                PARTIAL_N_PERIOD.fetch_add(1, Relaxed);
                PARTIAL_SUM_US_PERIOD.fetch_add(us, Relaxed);
                PARTIAL_PX_PERIOD.fetch_add(px, Relaxed);
                PARTIAL_MAX_US_PERIOD.fetch_max(us, Relaxed);
                PARTIAL_MAX_US.fetch_max(us, Relaxed);
            }
        });

        Timer::after(Duration::from_millis(10)).await;
    }
}
