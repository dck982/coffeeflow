//! Simulateur de l'essai 06 sur le Mac : même `.slint`, mêmes glyphes
//! pré-rendus et même infusion simulée (`src/demo.rs`) que sur la carte.
//!
//! - sans argument : fenêtre 800×480, rendu logiciel, valeurs à 10 Hz ;
//!   « accueil » et « infuser » changent d'écran, comme le doigt sur la carte ;
//!   `reduced-rate` : même fenêtre, à la fréquence réduite de la feature du
//!   même nom ;
//! - `snapshot <sortie.ppm> <écran 0|1> <secondes>` : une image de l'écran
//!   d'infusion (0) ou de l'accueil (1) à cet instant de l'infusion ;
//! - `bench` : temps de rendu partiel et complet sur le Mac, en RGB565, à
//!   10 Hz puis à fréquence réduite, avec `render` puis `render_by_line`.

extern crate alloc;

use std::io::Write;
use std::rc::Rc;
use std::time::Instant;

use slint::ComponentHandle;
use slint::platform::software_renderer::{
    LineBufferProvider, MinimalSoftwareWindow, RepaintBufferType, Rgb565Pixel,
};
use slint::platform::{Platform, WindowAdapter};

slint::include_modules!();

#[path = "../../src/demo.rs"]
mod demo;

const W: usize = 800;
const H: usize = 480;

/// Plateforme hors écran, pour les instantanés et la mesure.
struct Offscreen {
    window: Rc<MinimalSoftwareWindow>,
    start: Instant,
}

impl Platform for Offscreen {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
        Ok(self.window.clone())
    }

    fn duration_since_start(&self) -> core::time::Duration {
        self.start.elapsed()
    }
}

fn offscreen() -> (Rc<MinimalSoftwareWindow>, MainWindow) {
    let window = MinimalSoftwareWindow::new(RepaintBufferType::ReusedBuffer);
    window.set_size(slint::PhysicalSize::new(W as u32, H as u32));
    slint::platform::set_platform(Box::new(Offscreen { window: window.clone(), start: Instant::now() }))
        .unwrap();
    let ui = MainWindow::new().unwrap();
    ui.show().unwrap();
    (window, ui)
}

/// Rendu ligne par ligne, comme la feature `line-by-line` de l'essai : une
/// ligne composée à part, puis recopiée dans le framebuffer.
struct LineToFb<'a> {
    fb: &'a mut [Rgb565Pixel],
    line: [Rgb565Pixel; W],
}

impl LineBufferProvider for LineToFb<'_> {
    type TargetPixel = Rgb565Pixel;

    fn process_line(
        &mut self,
        line: usize,
        range: core::ops::Range<usize>,
        render_fn: impl FnOnce(&mut [Rgb565Pixel]),
    ) {
        render_fn(&mut self.line[range.clone()]);
        self.fb[line * W + range.start..line * W + range.end].copy_from_slice(&self.line[range]);
    }
}

/// Rend ce qui a changé ; renvoie la durée et le nombre de pixels redessinés.
fn render(window: &MinimalSoftwareWindow, fb: &mut [Rgb565Pixel], by_line: bool) -> (f64, u32) {
    let mut out = (0.0, 0);
    window.draw_if_needed(|renderer| {
        let t0 = Instant::now();
        let region = if by_line {
            renderer.render_by_line(LineToFb { fb: &mut *fb, line: [Rgb565Pixel(0); W] })
        } else {
            renderer.render(fb, W)
        };
        out = (
            t0.elapsed().as_secs_f64() * 1e6,
            region.iter().map(|(_, s)| s.width * s.height).sum(),
        );
    });
    out
}

fn snapshot(path: &str, page: i32, seconds: f32) {
    let (window, ui) = offscreen();
    let mut demo = demo::Demo::new(&ui);
    ui.set_page(page);
    demo.seek(&ui, seconds);
    let mut fb = vec![Rgb565Pixel(0); W * H];
    render(&window, &mut fb, false);

    let mut out = std::io::BufWriter::new(std::fs::File::create(path).unwrap());
    write!(out, "P6\n{W} {H}\n255\n").unwrap();
    for p in &fb {
        let (r, g, b) = (p.0 >> 11, (p.0 >> 5) & 0x3F, p.0 & 0x1F);
        out.write_all(&[(r << 3 | r >> 2) as u8, (g << 2 | g >> 4) as u8, (b << 3 | b >> 2) as u8])
            .unwrap();
    }
}

fn bench() {
    let (window, ui) = offscreen();
    let mut demo = demo::Demo::new(&ui);
    let mut fb = vec![Rgb565Pixel(0); W * H];
    render(&window, &mut fb, false);

    for (reduced_rate, by_line) in [(false, false), (false, true), (true, false), (true, true)] {
        demo.set_reduced_rate(reduced_rate);
        println!(
            "{}, {} :",
            if reduced_rate { "fréquence réduite" } else { "10 Hz" },
            if by_line { "render_by_line" } else { "render" }
        );
        // L'accueil d'abord : l'écran d'infusion est déjà affiché.
        for page in [1, 0] {
            ui.set_page(page);
            let (full_us, _) = render(&window, &mut fb, by_line);
            let (mut sum, mut max, mut px) = (0.0, 0.0f64, 0u64);
            // Seuls comptent les pas qui changent quelque chose : à 10 Hz,
            // un pas sur deux ; à fréquence réduite, quatre sur cinq.
            let mut n = 0;
            for _ in 0..1000 {
                demo.step(&ui);
                let (us, area) = render(&window, &mut fb, by_line);
                if area == 0 {
                    continue;
                }
                n += 1;
                sum += us;
                max = max.max(us);
                px += area as u64;
            }
            println!(
                "  écran {page}: complet {full_us:.0} µs, partiel moy {:.0} µs max {max:.0} µs, zone moy {} px",
                sum / n as f64,
                px / n as u64
            );
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("snapshot") => snapshot(
            &args[2],
            args.get(3).and_then(|s| s.parse().ok()).unwrap_or(0),
            args.get(4).and_then(|s| s.parse().ok()).unwrap_or(30.0),
        ),
        Some("bench") => bench(),
        mode => {
            let ui = MainWindow::new().unwrap();
            let _demo = demo::run(&ui, mode == Some("reduced-rate"));
            ui.run().unwrap();
        }
    }
}
