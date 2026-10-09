//! Rendu ligne par ligne : Slint compose la partie modifiée de chaque ligne
//! dans `LINE`, en SRAM interne, puis elle est recopiée d'un bloc dans le
//! framebuffer affiché. La recopie des bounce buffers ne voit donc jamais
//! une zone à moitié dessinée, seulement des lignes anciennes ou nouvelles.

use slint::platform::software_renderer::{LineBufferProvider, Rgb565Pixel};

use crate::board::H_RES;

// Ligne que Slint compose en SRAM interne.
static mut LINE: [Rgb565Pixel; H_RES] = [Rgb565Pixel(0); H_RES];

pub struct LineToFb(pub *mut Rgb565Pixel);

impl LineBufferProvider for LineToFb {
    type TargetPixel = Rgb565Pixel;

    fn process_line(
        &mut self,
        line: usize,
        range: core::ops::Range<usize>,
        render_fn: impl FnOnce(&mut [Rgb565Pixel]),
    ) {
        // Seule la boucle de rendu touche LINE. `&mut LINE` est refusé sur un
        // `static mut` (édition 2024), d'où le passage par le pointeur brut.
        #[allow(clippy::deref_addrof, reason = "&mut LINE est refusé sur un static mut")]
        let line_buf = unsafe { &mut *(&raw mut LINE) };
        let buf = &mut line_buf[range.clone()];
        render_fn(buf);
        let dst = self.0.wrapping_add(line * H_RES + range.start);
        unsafe { core::ptr::copy_nonoverlapping(buf.as_ptr(), dst, buf.len()) };
    }
}
