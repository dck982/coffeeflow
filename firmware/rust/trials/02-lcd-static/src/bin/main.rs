#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

// Essai 02 : mire fixe sur la dalle RGB 800×480, framebuffer en PSRAM lu
// directement par le DMA (sans bounce buffer). Le DMA parcourt en boucle une
// chaîne circulaire de descripteurs qui couvre le framebuffer ; le processeur
// ne touche plus à rien une fois la mire dessinée.

use embassy_executor::Spawner;
use embassy_time::{Duration, Instant, Timer};
use esp_backtrace as _;
use esp_hal::clock::CpuClock;
use esp_hal::dma::{
    BurstConfig, DmaDescriptor, DmaTxBuffer, ExternalBurstConfig, InternalBurstConfig, Owner,
    Preparation,
};
use esp_hal::gpio::{Level, Output, OutputConfig};
use esp_hal::i2c::master::{Config as I2cConfig, I2c};
use esp_hal::lcd_cam::LcdCam;
use esp_hal::lcd_cam::lcd::dpi::{Config as DpiConfig, Dpi, Format, FrameTiming};
use esp_hal::lcd_cam::lcd::{ClockMode, Phase, Polarity};
use esp_hal::psram::{Psram, PsramConfig, PsramMode, SpiRamFreq};
use esp_hal::time::Rate;
use esp_hal::timer::timg::TimerGroup;
use log::info;
use static_cell::StaticCell;

extern crate alloc;

esp_bootloader_esp_idf::esp_app_desc!();

const H_RES: usize = 800;
const V_RES: usize = 480;
const FB_BYTES: usize = H_RES * V_RES * 2;

// Timings de coffeeflow (service_screen.cpp), en PCLK et en lignes.
const HSYNC_PULSE: usize = 48;
const HSYNC_BACK: usize = 88;
const HSYNC_FRONT: usize = 40;
const VSYNC_PULSE: usize = 3;
const VSYNC_BACK: usize = 32;
const VSYNC_FRONT: usize = 13;

// Rafales PSRAM de 64 octets, comme le driver IDF : chaque descripteur couvre
// un multiple de 64 octets, sous la limite de 4095 octets du champ taille.
const BURST: usize = 64;
const CHUNK: usize = 4096 - BURST;
const N_DESC: usize = FB_BYTES.div_ceil(CHUNK);

// CH422G : l'adresse I2C choisit la fonction.
const CH422G_MODE: u8 = 0x24;
const CH422G_OUT: u8 = 0x38;
const CH422G_MODE_OUTPUTS: u8 = 0x01;
const EXIO_TP_RST: u8 = 1 << 1;
const EXIO_DISP: u8 = 1 << 2;
const EXIO_LCD_RST: u8 = 1 << 3;
// CAN_SEL (bit 5) n'apparaît dans aucun octet écrit : il reste bas, le
// transceiver CAN reste désélectionné.

static DESCRIPTORS: StaticCell<[DmaDescriptor; N_DESC]> = StaticCell::new();

/// Chaîne circulaire de descripteurs sur un framebuffer en PSRAM. Le dernier
/// descripteur pointe sur le premier : avec `next_frame_en`, le LCD redemande
/// une trame et le DMA la lui fournit sans intervention logicielle.
struct FrameLoop {
    descriptors: &'static mut [DmaDescriptor; N_DESC],
}

impl FrameLoop {
    fn new(descriptors: &'static mut [DmaDescriptor; N_DESC], fb: *mut u8) -> Self {
        let head: *mut DmaDescriptor = descriptors.as_mut_ptr();
        for (i, desc) in descriptors.iter_mut().enumerate() {
            let offset = i * CHUNK;
            let len = CHUNK.min(FB_BYTES - offset);
            desc.set_size(len);
            desc.set_length(len);
            desc.set_owner(Owner::Dma);
            desc.set_suc_eof(i == N_DESC - 1);
            desc.buffer = unsafe { fb.add(offset) };
            desc.next = if i == N_DESC - 1 {
                head
            } else {
                unsafe { head.add(i + 1) }
            };
        }
        Self { descriptors }
    }
}

unsafe impl DmaTxBuffer for FrameLoop {
    type View = FrameLoop;
    type Final = FrameLoop;

    fn prepare(&mut self) -> Preparation {
        Preparation {
            start: self.descriptors.as_mut_ptr(),
            accesses_psram: true,
            burst_transfer: BurstConfig {
                external_memory: ExternalBurstConfig::Size64,
                internal_memory: InternalBurstConfig::Disabled,
            },
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

/// Recopie en PSRAM ce que le processeur a écrit à travers le cache, pour
/// que le DMA, qui lit la PSRAM sans passer par le cache, le voie.
fn cache_writeback(addr: *const u8, len: usize) {
    unsafe extern "C" {
        fn rom_Cache_WriteBack_Addr(addr: u32, size: u32);
        fn Cache_Suspend_DCache_Autoload() -> u32;
        fn Cache_Resume_DCache_Autoload(value: u32);
    }
    unsafe {
        let autoload = Cache_Suspend_DCache_Autoload();
        rom_Cache_WriteBack_Addr(addr as u32, len as u32);
        Cache_Resume_DCache_Autoload(autoload);
    }
}

/// `Dpi::send` lance le DMA puis vide la FIFO du LCD : les pixels déjà
/// poussés par le DMA sont perdus, et la chaîne circulaire garde ce retard
/// pour toujours (image décalée vers la droite d'une quantité aléatoire à
/// chaque démarrage). On refait le démarrage dans l'ordre d'IDF : LCD arrêté
/// et FIFO vidée, DMA relancé depuis le premier descripteur, puis LCD.
fn restart_in_phase(head: *mut DmaDescriptor) {
    let lcd = unsafe { &*esp32s3::LCD_CAM::ptr() };
    let dma = unsafe { &*esp32s3::DMA::ptr() }.ch(0);

    lcd.lcd_user()
        .modify(|_, w| w.lcd_start().clear_bit().lcd_update().set_bit());
    lcd.lcd_user().modify(|_, w| w.lcd_reset().set_bit());
    lcd.lcd_misc().modify(|_, w| w.lcd_afifo_reset().set_bit());

    dma.out_link().modify(|_, w| w.outlink_stop().set_bit());
    dma.out_conf0().modify(|_, w| w.out_rst().set_bit());
    dma.out_conf0().modify(|_, w| w.out_rst().clear_bit());
    dma.out_link()
        .modify(|_, w| unsafe { w.outlink_addr().bits(head as u32) });
    dma.out_link().modify(|_, w| w.outlink_start().set_bit());

    // IDF : 1 µs suffit au DMA pour remplir la FIFO du LCD.
    esp_hal::delay::Delay::new().delay_micros(1);
    lcd.lcd_user()
        .modify(|_, w| w.lcd_update().set_bit().lcd_start().set_bit());
}

#[inline(never)]
fn cold_code_probe(x: f32) {
    info!("témoin froid: {:.4} {:e}", x / 7.0, x * 1.0e-3);
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

/// Mire : huit bandes verticales, des dégradés R, V, B et gris pour repérer
/// un bit de donnée inversé ou manquant, un quadrillage tous les 40 px pour
/// voir un décalage, un cadre d'un pixel sur les bords, et un carré de
/// couleur différente dans chaque coin pour voir une inversion d'axe.
fn draw_test_pattern(fb: &mut [u16]) {
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
    for y in 0..V_RES {
        let row = &mut fb[y * H_RES..(y + 1) * H_RES];
        for (x, px) in row.iter_mut().enumerate() {
            // 32 niveaux par dégradé, 25 px par niveau.
            let level = (x * 32 / H_RES) as u16;
            *px = match y {
                0..320 => BANDS[x / (H_RES / BANDS.len())],
                320..360 => level << 11,
                360..400 => (level * 2) << 5,
                400..440 => level,
                _ => (level << 11) | ((level * 2) << 5) | level,
            };
            if x % 40 == 0 || y % 40 == 0 {
                *px = GRID;
            }
        }
    }

    let mut square = |x0: usize, y0: usize, color: u16| {
        for y in y0..y0 + 16 {
            fb[y * H_RES + x0..y * H_RES + x0 + 16].fill(color);
        }
    };
    square(3, 3, RED);
    square(H_RES - 19, 3, GREEN);
    square(3, V_RES - 19, BLUE);
    square(H_RES - 19, V_RES - 19, WHITE);

    for x in 0..H_RES {
        fb[x] = WHITE;
        fb[(V_RES - 1) * H_RES + x] = WHITE;
    }
    for y in 0..V_RES {
        fb[y * H_RES] = WHITE;
        fb[y * H_RES + H_RES - 1] = WHITE;
    }
}

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[esp_rtos::main]
async fn main(_spawner: Spawner) -> ! {
    esp_println::logger::init_logger_from_env();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 73744);

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
    assert!(psram_size >= FB_BYTES, "PSRAM absente ou trop petite");
    assert!(psram_start as usize % BURST == 0);

    // Le framebuffer occupe le début de la PSRAM ; rien d'autre n'y vit.
    let fb_ptr = psram_start;
    let fb = unsafe { core::slice::from_raw_parts_mut(fb_ptr as *mut u16, H_RES * V_RES) };
    let t0 = Instant::now();
    draw_test_pattern(fb);
    cache_writeback(fb_ptr, FB_BYTES);
    info!("mire dessinée en {} ms", t0.elapsed().as_millis());

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
    unsafe { &*esp32s3::LCD_CAM::ptr() }
        .lcd_user()
        .modify(|_, w| w.lcd_always_out_en().set_bit());

    let mut frame_loop = FrameLoop::new(
        DESCRIPTORS.init([DmaDescriptor::EMPTY; N_DESC]),
        fb_ptr,
    );
    let head = frame_loop.prepare().start;
    info!("DMA: {N_DESC} descripteurs de {CHUNK} octets, rafales de {BURST}");

    // Hypothèse en test : chaque défaut de cache sur le code en flash prend
    // le bus partagé avec la PSRAM, affame le DMA, et fait perdre des pixels
    // pour toujours. On passe donc une fois par le code de la boucle avant
    // de lancer le DMA, pour qu'il soit déjà en cache, et on ne journalise
    // rien juste après le démarrage.
    info!("en vie depuis {} s", 0);
    Timer::after(Duration::from_millis(1)).await;
    info!("DPI: démarrage à 16 MHz, ~31 trames/s ; témoin de code froid à 30 s");

    // Le transfert tourne tant que `_transfer` vit : le lâcher arrêterait
    // le DMA.
    let _transfer = match dpi.send(true, frame_loop) {
        Ok(transfer) => transfer,
        Err((err, _, _)) => panic!("échec du démarrage DPI: {err:?}"),
    };
    restart_in_phase(head);

    let start = Instant::now();
    loop {
        Timer::after(Duration::from_secs(10)).await;
        let secs = start.elapsed().as_secs();
        info!("en vie depuis {} s", secs);
        if secs == 30 {
            // Témoin : premier passage dans le formatage des flottants, du
            // code froid. Si l'hypothèse tient, l'image saute ici.
            cold_code_probe(secs as f32);
        }
    }
}
