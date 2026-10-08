#![no_std]
#![no_main]
#![feature(asm_experimental_arch)]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

// Essai 04 : l'affichage de l'essai 03, variante un cœur, inchangé, avec un
// scan BLE actif en parallèle. Le cœur 0 garde la recopie des bounce buffers
// et le dessin continu des rectangles et du compteur (pire cas de charge
// PSRAM). Le cœur 1 porte la radio : ses interruptions, la tâche du
// contrôleur BLE et l'hôte trouble-host, dans son propre exécuteur embassy.
//
// On mesure ce que la radio coûte à la recopie : retards, recalages et
// recopie la plus lente, à comparer aux 885 µs de l'essai 03. Le cœur 1 ne
// journalise rien lui-même : esp-println écrit sous une section critique
// partagée par les deux cœurs, et le cœur 0 resterait bloqué, interruptions
// masquées, le temps que l'autre vide sa ligne. (esp-radio peut encore
// écrire une ligne d'avertissement depuis le cœur 1, rarement.)

use core::cell::RefCell;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering::Relaxed};

use bt_hci::controller::ExternalController;
use critical_section::Mutex;

use embassy_executor::Spawner;
use embedded_graphics::mono_font::MonoTextStyleBuilder;
use embedded_graphics::mono_font::ascii::FONT_10X20;
use embedded_graphics::pixelcolor::raw::RawU16;
use embedded_graphics::pixelcolor::{IntoStorage, Rgb565};
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;
use embedded_graphics::text::{Baseline, Text};
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

// Côté BLE, écrit par le cœur 1 et lu par le rapport du cœur 0.
static BLE_SCANNING: AtomicBool = AtomicBool::new(false);
static ADV_REPORTS: AtomicU32 = AtomicU32::new(0);
static SEEN: Mutex<RefCell<[Option<Seen>; 2]>> = Mutex::new(RefCell::new([None, None]));

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
            "mem libre={} min={} ble={} adv={}/s entree={}/{}/{}/{}/{}/{}",
            esp_alloc::HEAP.free(),
            heap.size - heap.max_usage,
            if BLE_SCANNING.load(Relaxed) { "scan" } else { "init" },
            adv.wrapping_sub(last_adv) / 10,
            ENTRY_DSCR[0].load(Relaxed),
            ENTRY_DSCR[1].load(Relaxed),
            ENTRY_DSCR[2].load(Relaxed),
            ENTRY_DSCR[3].load(Relaxed),
            ENTRY_DSCR[4].load(Relaxed),
            ENTRY_DSCR[5].load(Relaxed),
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
        for (kind, dev) in ["balance", "scace"].iter().zip(seen) {
            if let Some(dev) = dev {
                Timer::after(Duration::from_millis(20)).await;
                info!(
                    "{kind} {} {} {}dBm",
                    core::str::from_utf8(dev.name()).unwrap_or("?"),
                    Mac(dev.addr),
                    dev.rssi,
                );
            }
        }
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

/// Dernière annonce reçue d'un appareil reconnu.
#[derive(Clone, Copy)]
struct Seen {
    name: [u8; 20],
    name_len: u8,
    addr: [u8; 6],
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
        Some(0)
    } else if name == b"SCACE" {
        Some(1)
    } else {
        None
    }
}

/// Reçoit les annonces et réponses de scan dans le contexte de l'hôte, sur
/// le cœur 1. Garde la dernière annonce de chaque appareil reconnu ; le
/// rapport la prend et la journalise au plus une fois toutes les 10 s.
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
            let Some(kind) = classify(name) else { continue };
            let mut dev = Seen {
                name: [0; 20],
                name_len: name.len().min(20) as u8,
                addr: [0; 6],
                rssi: report.rssi,
            };
            dev.name[..dev.name_len as usize].copy_from_slice(&name[..dev.name_len as usize]);
            dev.addr.copy_from_slice(report.addr.raw());
            critical_section::with(|cs| SEEN.borrow_ref_mut(cs)[kind] = Some(dev));
        }
    }
}

const APP_STACK_BYTES: usize = 16384;
const APP_STACK_LOW_BYTES: u32 = 256;
const CONNECTIONS_MAX: usize = 1;
const L2CAP_CHANNELS_MAX: usize = 1;

/// Scan actif continu (fenêtre = intervalle), sur le modèle de l'exemple
/// `ble/scanner` d'esp-hal. Tourne dans l'exécuteur du cœur 1.
#[embassy_executor::task]
async fn ble_scan(bt: BT<'static>) {
    let config = esp_radio::ble::Config::default().with_task_cpu(Cpu::AppCpu);
    let connector = BleConnector::new(bt, config).unwrap();
    let controller: ExternalController<_, 1> = ExternalController::new(connector);

    // Adresse aléatoire fixe, comme l'exemple : l'essai ne se connecte pas.
    let address = Address::random([0xff, 0x8f, 0x1b, 0x05, 0xe4, 0xff]);
    let mut resources: HostResources<_, DefaultPacketPool, CONNECTIONS_MAX, L2CAP_CHANNELS_MAX> =
        HostResources::new();
    let stack = trouble_host::new(controller, &mut resources)
        .set_random_address(address)
        .build();
    let mut runner = stack.runner();
    let mut scanner = Scanner::new(stack.central());

    let _ = embassy_futures::join::join(runner.run_with_handler(&ScanHandler), async {
        let config = ScanConfig {
            active: true,
            phys: PhySet::M1,
            interval: Duration::from_secs(1),
            window: Duration::from_secs(1),
            ..Default::default()
        };
        let _session = scanner.scan(&config).await.unwrap();
        BLE_SCANNING.store(true, Relaxed);
        loop {
            Timer::after(Duration::from_secs(1)).await;
        }
    })
    .await;
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
    assert!(psram_size >= FB_PX * 2, "PSRAM absente ou trop petite");

    // Le framebuffer occupe le début de la PSRAM ; rien d'autre n'y vit.
    let mut fb = Fb(psram_start as *mut u16);
    let t0 = Instant::now();
    let _ = fb.fill_contiguous(
        &fb.bounding_box(),
        (0..FB_PX).map(|i| RawU16::new(pattern(i % H_RES, i / H_RES)).into()),
    );
    info!("mire dessinée en {} ms", t0.elapsed().as_millis());
    FB.store(psram_start as u32, Relaxed);

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
            executor.run(|spawner| spawner.spawn(ble_scan(bt).unwrap()));
        },
    );

    spawner.spawn(report().unwrap());
    let mut scene = Scene::new();
    loop {
        scene.draw_step(&mut fb);
        embassy_futures::yield_now().await;
    }
}
