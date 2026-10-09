//! Affichage : flux RGB 800x480 du LCD_CAM par DMA, depuis deux bounce
//! buffers de 20 lignes en SRAM interne que l'interruption de fin de buffer
//! remplit à partir du framebuffer en PSRAM (cœur 0). Pourquoi des bounce
//! buffers : le DMA ne lit pas la PSRAM assez vite pendant que le cache
//! d'instructions s'en sert aussi.

pub mod render;

use core::sync::atomic::{AtomicU32, Ordering::Relaxed};

use esp_hal::dma::{BurstConfig, DmaDescriptor, DmaTxBuffer, Owner, Preparation};
use esp_hal::gpio::{AnyPin, Level};
use esp_hal::interrupt::{self, InterruptHandler, Priority};
use esp_hal::lcd_cam::LcdCam;
use esp_hal::lcd_cam::lcd::dpi::{Config as DpiConfig, Dpi, DpiTransfer, Format, FrameTiming};
use esp_hal::lcd_cam::lcd::{ClockMode, Phase, Polarity};
use esp_hal::peripherals::{DMA_CH0, Interrupt, LCD_CAM};
use esp_hal::time::Rate;
use log::info;

use crate::board::{H_RES, V_RES};

pub const FB_PX: usize = H_RES * V_RES;
pub const FB_BYTES: usize = FB_PX * 2;

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
const _: () = assert!(FB_PX.is_multiple_of(BB_PX) && BB_PER_FRAME.is_multiple_of(2));
const _: () = assert!(BB_BYTES.is_multiple_of(DESC_PER_BB) && DESC_BYTES <= 4095);

// Sur ESP32-S3, IDF relance le DMA LCD_LL_FIFO_DEPTH + 1 pixels après le
// début de la trame : la FIFO du LCD en contient déjà autant
// (RGB_LCD_NEEDS_SEPARATE_RESTART_LINK dans esp_lcd_panel_rgb.c).
const RESTART_SKIP_BYTES: usize = (16 + 1) * 2;

// Bounce buffers et descripteurs en SRAM interne (.bss). Seules les
// interruptions y touchent une fois l'affichage démarré.
static mut BOUNCE: [[u32; BB_BYTES / 4]; 2] = [[0; BB_BYTES / 4]; 2];
static mut RING: [DmaDescriptor; N_RING + 1] = [DmaDescriptor::EMPTY; N_RING + 1];

// Adresse du framebuffer PSRAM, posée une fois avant le démarrage.
static FB: AtomicU32 = AtomicU32::new(0);
// Prochain bloc de BB_LINES lignes à recopier.
static NEXT_CHUNK: AtomicU32 = AtomicU32::new(0);
// Fins de buffer reçues depuis le dernier VSYNC ; sa parité désigne le buffer
// que le DMA vient de finir.
static EOF_COUNT: AtomicU32 = AtomicU32::new(0);
static LAST_EOF_US: AtomicU32 = AtomicU32::new(0);

// Mesures de la recopie, lues par le rapport.
/// Trames affichées (un VSYNC chacune).
pub static FRAMES: AtomicU32 = AtomicU32::new(0);
/// Recopies terminées après que le DMA est revenu sur le buffer : des lignes
/// périmées sont parties vers la dalle.
pub static LATE: AtomicU32 = AtomicU32::new(0);
/// Trames qui n'ont pas reçu exactement BB_PER_FRAME fins de buffer : le DMA
/// a été relancé en phase pendant le blanking vertical.
pub static REALIGNS: AtomicU32 = AtomicU32::new(0);
/// Recopie la plus lente depuis le démarrage.
pub static COPY_MAX_US: AtomicU32 = AtomicU32::new(0);
/// Recopie la plus lente et écart le plus long entre deux fins de buffer
/// depuis le dernier rapport (lus par `swap(0)`). L'écart nominal est de
/// 1 220 µs ; il mesure le retard d'entrée dans l'interruption, que la durée
/// de recopie ne voit pas.
pub static COPY_MAX_PERIOD_US: AtomicU32 = AtomicU32::new(0);
pub static GAP_MAX_PERIOD_US: AtomicU32 = AtomicU32::new(0);

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
pub struct BounceRing;

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
    if offset.is_multiple_of(size) && offset / size <= RESTART_DESC {
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
/// buffer.
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
    let t0 = esp_hal::time::Instant::now();
    let now = t0.duration_since_epoch().as_micros() as u32;
    let gap = now.wrapping_sub(LAST_EOF_US.swap(now, Relaxed));

    dma_tx().out_int().clr().write(|w| w.out_eof().clear_bit_by_one());
    let eof = EOF_COUNT.fetch_add(1, Relaxed);
    let bb = (eof % 2) as usize;

    fill_bounce(bb);
    let us = t0.elapsed().as_micros() as u32;

    if in_buffer(dma_current(out_dscr()), bb) {
        LATE.fetch_add(1, Relaxed);
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
        REALIGNS.fetch_add(1, Relaxed);
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

/// Broches du flux RGB de la dalle (board.h du C++).
pub struct Pins {
    pub hsync: AnyPin<'static>,
    pub vsync: AnyPin<'static>,
    pub de: AnyPin<'static>,
    pub pclk: AnyPin<'static>,
    /// De data0 à data15, dans l'ordre du câblage.
    pub data: [AnyPin<'static>; 16],
}

/// Transfert DMA en cours : le lâcher arrêterait l'affichage.
pub struct Display {
    _transfer: DpiTransfer<'static, BounceRing, esp_hal::Blocking>,
}

/// Démarre le flux RGB. `fb` est le framebuffer RGB565 en PSRAM, déjà
/// rempli : les deux premiers bounce buffers en sont tirés tout de suite.
/// Le démarrage se fait en phase avec le VSYNC, comme IDF.
#[allow(
    clippy::large_stack_frames,
    reason = "appelée une fois au démarrage : le constructeur DPI, déplacé à chaque with_*, laisse 1,6 Ko de pile à main, qui n'en manque pas"
)]
pub fn start(lcd_cam: LCD_CAM<'static>, dma: DMA_CH0<'static>, pins: Pins, fb: *mut u8) -> Display {
    FB.store(fb as u32, Relaxed);
    let [d0, d1, d2, d3, d4, d5, d6, d7, d8, d9, d10, d11, d12, d13, d14, d15] = pins.data;
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

    let lcd_cam = LcdCam::new(lcd_cam);
    let dpi = Dpi::new(lcd_cam.lcd, dma, dpi_config)
        .unwrap()
        .with_hsync(pins.hsync)
        .with_vsync(pins.vsync)
        .with_de(pins.de)
        .with_pclk(pins.pclk)
        .with_data0(d0)
        .with_data1(d1)
        .with_data2(d2)
        .with_data3(d3)
        .with_data4(d4)
        .with_data5(d5)
        .with_data6(d6)
        .with_data7(d7)
        .with_data8(d8)
        .with_data9(d9)
        .with_data10(d10)
        .with_data11(d11)
        .with_data12(d12)
        .with_data13(d13)
        .with_data14(d14)
        .with_data15(d15);

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

    let transfer = match dpi.send(true, ring) {
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
    Display { _transfer: transfer }
}
