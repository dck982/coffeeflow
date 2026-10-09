#![no_std]
#![no_main]
#![feature(asm_experimental_arch)]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

// Essai 06 : Slint sur l'affichage de l'essai 04b (recopie des bounce
// buffers par interruption, code en PSRAM), sans la radio. Tout tourne sur
// le cœur 0 : Slint dessine dans le framebuffer PSRAM une maquette de l'UI
// de coffeeflow, dont les valeurs changent à 10 Hz (`demo`). Un bouton
// tactile change d'écran, ce qui force un redessin complet.
//
// On mesure le temps de rendu partiel (mise à jour des valeurs) et complet
// (changement d'écran), les compteurs de recopie de l'essai 04, le tas
// (PSRAM et interne) et la pile du cœur 0. Variantes, par feature :
//
// - par défaut, Slint dessine dans le framebuffer affiché (ReusedBuffer) ;
// - `swapped-buffers` lui en donne un second, échangé au début d'une trame ;
// - `line-by-line` lui fait composer chaque ligne dans un tampon en SRAM
//   interne, recopié ensuite dans le framebuffer affiché ;
// - `reduced-rate`, combinable avec les deux autres, baisse la fréquence de
//   mise à jour des valeurs, groupe par groupe (`demo::REDUCED_RATES`).

use alloc::boxed::Box;
use alloc::rc::Rc;
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
use esp_hal::peripherals::Interrupt;
use esp_hal::psram::{Psram, PsramConfig, PsramMode, SpiRamFreq};
use esp_hal::time::Rate;
use esp_hal::timer::timg::TimerGroup;
use log::info;
use slint::ComponentHandle;
use slint::platform::software_renderer::{MinimalSoftwareWindow, RepaintBufferType, Rgb565Pixel};
#[cfg(feature = "line-by-line")]
use slint::platform::software_renderer::LineBufferProvider;
use slint::platform::{Platform, PointerEventButton, WindowAdapter, WindowEvent};

extern crate alloc;

use trial_06_slint_display::{MainWindow, demo, xip};

esp_bootloader_esp_idf::esp_app_desc!();

#[cfg(all(feature = "swapped-buffers", feature = "line-by-line"))]
compile_error!("`swapped-buffers` et `line-by-line` sont deux variantes exclusives");

const H_RES: usize = 800;
const V_RES: usize = 480;
const FB_PX: usize = H_RES * V_RES;
const FB_BYTES: usize = FB_PX * 2;
#[cfg(not(feature = "swapped-buffers"))]
const FB_COUNT: usize = 1;
#[cfg(feature = "swapped-buffers")]
const FB_COUNT: usize = 2;

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

// Ligne que Slint compose en SRAM interne, en `line-by-line`.
#[cfg(feature = "line-by-line")]
static mut LINE: [Rgb565Pixel; H_RES] = [Rgb565Pixel(0); H_RES];

// Adresse du framebuffer recopié, et celle du suivant à afficher (0 si
// aucun échange n'attend). L'échange se fait au bloc 0 d'une trame : à partir
// de là, plus rien n'est lu dans l'ancien framebuffer.
static FB: AtomicU32 = AtomicU32::new(0);
static PENDING_FB: AtomicU32 = AtomicU32::new(0);
// Prochain bloc de 20 lignes à recopier (0..24).
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
// Attente de l'échange de framebuffers, la plus longue (SwappedBuffers).
static SWAP_WAIT_MAX_US_PERIOD: AtomicU32 = AtomicU32::new(0);
static TOUCH_PRESSES: AtomicU32 = AtomicU32::new(0);
static TOUCH_ERRORS: AtomicU32 = AtomicU32::new(0);
static PAGE: AtomicU32 = AtomicU32::new(0);

// Bas de la pile du cœur 0, peinte au démarrage pour en mesurer le plus haut
// niveau.
const STACK_PAINT: u32 = 0xA5A5_5A5A;
static STACK0_LOW: AtomicU32 = AtomicU32::new(0);

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
/// qui l'écrit : pas de writeback nécessaire. Au bloc 0, on passe au
/// framebuffer en attente, s'il y en a un.
fn fill_bounce(bb: usize) {
    let chunk = NEXT_CHUNK.load(Relaxed) as usize;
    if chunk == 0 {
        let pending = PENDING_FB.swap(0, Relaxed);
        if pending != 0 {
            FB.store(pending, Relaxed);
        }
    }
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
#[cfg(feature = "line-by-line")]
struct LineToFb(*mut Rgb565Pixel);

#[cfg(feature = "line-by-line")]
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
async fn report() {
    let start = Instant::now();
    let mut last_frames = 0;
    let mut last_late = 0;
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
            "partiel n={} moy={}us max={}/{}us zone={}px attente={}us",
            n,
            sum.checked_div(n).unwrap_or(0),
            PARTIAL_MAX_US_PERIOD.swap(0, Relaxed),
            PARTIAL_MAX_US.load(Relaxed),
            px.checked_div(n).unwrap_or(0),
            SWAP_WAIT_MAX_US_PERIOD.swap(0, Relaxed),
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

        Timer::after(Duration::from_millis(20)).await;
        let [psram_used, psram_size, int_used, int_size, max_usage] = heap_usage();
        let stack0_top = &raw const _stack_start_cpu0 as u32;
        let stack0_low = STACK0_LOW.load(Relaxed);
        info!(
            "tas psram {}/{} interne {}/{} max={} pile0 {}/{}",
            psram_used,
            psram_size,
            int_used,
            int_size,
            max_usage,
            stack0_top - stack0_low - stack_untouched(stack0_low),
            stack0_top - (&raw const _stack_end_cpu0 as u32),
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
    }
}

/// Tas utilisé et taille de la région PSRAM (0) et de la région interne
/// (1), puis le plus haut niveau atteint, toutes régions confondues.
fn heap_usage() -> [usize; 5] {
    let heap = esp_alloc::HEAP.stats();
    let region = |i: usize| heap.region_stats[i].as_ref().map_or((0, 0), |r| (r.used, r.size));
    let ((psram_used, psram_size), (int_used, int_size)) = (region(0), region(1));
    [psram_used, psram_size, int_used, int_size, heap.max_usage]
}

const HEAP_RECLAIMED: usize = 73744;

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
    // occupent le début. Les framebuffers suivent, puis le tas.
    #[cfg(not(feature = "exec-from-flash"))]
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
    #[cfg(feature = "exec-from-flash")]
    let code_bytes = 0;
    let heap_start = code_bytes + FB_COUNT * FB_BYTES;
    assert!(psram_size > heap_start + 1024 * 1024, "PSRAM absente ou trop petite");
    let fbs: [*mut Rgb565Pixel; FB_COUNT] =
        core::array::from_fn(|i| psram_start.wrapping_add(code_bytes + i * FB_BYTES).cast());
    for fb in fbs {
        for i in 0..FB_PX {
            unsafe { fb.cast::<u16>().add(i).write(BG_565) };
        }
    }
    FB.store(fbs[0] as u32, Relaxed);

    // Le tas PSRAM d'abord : esp-alloc sert chaque allocation depuis la
    // première région qui a la place, donc Slint vit en PSRAM. La SRAM
    // interne récupérée du bootloader ne sert qu'en second.
    unsafe {
        esp_alloc::HEAP.add_region(esp_alloc::HeapRegion::new(
            psram_start.wrapping_add(heap_start),
            psram_size - heap_start,
            esp_alloc::MemoryCapability::External.into(),
        ));
    }
    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: HEAP_RECLAIMED);
    info!(
        "tas: {} octets en PSRAM, {} en interne",
        psram_size - heap_start,
        HEAP_RECLAIMED
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
    info!(
        "bounce: 2 x {BB_LINES} lignes ({BB_BYTES} octets) en SRAM, {FB_COUNT} framebuffer(s)"
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
    info!("DPI démarré ; Slint dessine sur le cœur de la recopie");

    // Le rendu ligne par ligne réécrit lui aussi le framebuffer affiché :
    // seule la zone modifiée est à redessiner, comme en ReusedBuffer.
    #[cfg(not(feature = "swapped-buffers"))]
    let repaint = RepaintBufferType::ReusedBuffer;
    #[cfg(feature = "swapped-buffers")]
    let repaint = RepaintBufferType::SwappedBuffers;
    let window = MinimalSoftwareWindow::new(repaint);
    window.set_size(slint::PhysicalSize::new(H_RES as u32, V_RES as u32));
    slint::platform::set_platform(Box::new(Board { window: window.clone() }))
        .expect("plateforme Slint déjà posée");
    let ui = MainWindow::new().unwrap();
    ui.show().unwrap();
    let reduced_rate = cfg!(feature = "reduced-rate");
    let _demo = demo::run(&ui, reduced_rate);
    Timer::after(Duration::from_millis(20)).await;
    info!(
        "Slint: {repaint:?}, ligne par ligne={}, fréquence réduite={reduced_rate}",
        cfg!(feature = "line-by-line")
    );

    spawner.spawn(report().unwrap());
    render_loop(&window, &ui, fbs, i2c, touch).await
}

/// Boucle de rendu, sur le cœur de la recopie : minuteurs et animations de
/// Slint, tactile toutes les 20 ms, puis rendu de ce qui a changé. Chaque
/// tour se termine par une pause de 10 ms, même après un rendu long : le
/// rapport, sur le même exécuteur, doit pouvoir passer.
async fn render_loop(
    window: &MinimalSoftwareWindow,
    ui: &MainWindow,
    fbs: [*mut Rgb565Pixel; FB_COUNT],
    mut i2c: I2c<'static, esp_hal::Blocking>,
    touch: Option<Gt911>,
) -> ! {
    let mut back = FB_COUNT - 1;
    let mut last_page = -1;
    // Rendus restants à compter comme complets : un en ReusedBuffer, deux en
    // SwappedBuffers, où le rendu suivant rattrape l'autre framebuffer.
    let mut full_left = 0;
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
            full_left = FB_COUNT;
            PAGE.store(page as u32, Relaxed);
        }

        let mut drawn = false;
        window.draw_if_needed(|renderer| {
            let t0 = esp_hal::time::Instant::now();
            #[cfg(not(feature = "line-by-line"))]
            let region = {
                let fb = unsafe { core::slice::from_raw_parts_mut(fbs[back], FB_PX) };
                renderer.render(fb, H_RES)
            };
            #[cfg(feature = "line-by-line")]
            let region = renderer.render_by_line(LineToFb(fbs[back]));
            let us = t0.elapsed().as_micros() as u32;
            if full_left > 0 {
                full_left -= 1;
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
            drawn = true;
        });

        // SwappedBuffers : le framebuffer rendu s'affiche au début de la
        // prochaine trame, et l'ancien ne se réécrit qu'une fois libéré.
        if FB_COUNT == 2 && drawn {
            PENDING_FB.store(fbs[back] as u32, Relaxed);
            let t0 = Instant::now();
            while PENDING_FB.load(Relaxed) != 0 {
                Timer::after(Duration::from_millis(1)).await;
            }
            SWAP_WAIT_MAX_US_PERIOD.fetch_max(t0.elapsed().as_micros() as u32, Relaxed);
            back = 1 - back;
        }

        Timer::after(Duration::from_millis(10)).await;
    }
}
