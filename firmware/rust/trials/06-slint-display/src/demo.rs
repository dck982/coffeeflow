//! Infusion simulée : les valeurs de la maquette avancent de 50 ms à chaque
//! pas, sur un cycle de 50 s qui reprend au début. Partagé tel quel entre
//! l'essai et le simulateur sur Mac (`sim/`), qui compilent chacun le même
//! `.slint` : `crate::MainWindow` et les autres types générés y portent les
//! mêmes noms.

use alloc::rc::Rc;
use alloc::vec::Vec;
use core::cell::RefCell;
use core::fmt::Write;

use slint::{Color, ComponentHandle, Model, SharedString, Timer, TimerMode, VecModel};

use crate::{Cell, Live, MainWindow, Segment, Tile};

pub const STEP_MS: u32 = 50;

/// Sans fréquence réduite, toutes les valeurs changent un pas sur deux :
/// 10 Hz.
const FULL_RATE_PERIOD: u32 = 2;

/// Groupes de valeurs, mis à jour ensemble.
struct Group;

impl Group {
    const TOPBAR: u8 = 1 << 0;
    const FRISE: u8 = 1 << 1;
    const TILES: u8 = 1 << 2;
    const CELLS: u8 = 1 << 3;
    const ALL: u8 = Self::TOPBAR | Self::FRISE | Self::TILES | Self::CELLS;
}

/// Fréquence réduite (levier 3 de l'étape 6) : période de chaque groupe en
/// pas de 50 ms, et décalage. La fréquence se décidera valeur par valeur ;
/// en infusion, 2 à 4 Hz peuvent suffire. Ici, chaque groupe passe à 4 Hz
/// (2,5 Hz paraissait lent sur la carte), et les décalages font qu'un rendu
/// n'en couvre qu'un seul : Slint ne garde que trois zones modifiées, et
/// fusionne les autres en grandes bandes. Un pas sur cinq ne change rien. La
/// barre du haut porte aussi le poids de l'accueil.
const REDUCED_RATES: [(u8, u32, u32); 4] = [
    (Group::TOPBAR, 5, 0),
    (Group::FRISE, 5, 1),
    (Group::TILES, 5, 2),
    (Group::CELLS, 5, 3),
];
const CYCLE_TICKS: u32 = 1000;

/// Phases de la frise : libellés long, moyen et court, couleur, fin en
/// secondes depuis le départ, et libellé sombre (sur l'infusion).
struct Phase {
    labels: [&'static str; 3],
    color: u32,
    end_s: f32,
    dark: bool,
}

const PHASES: [Phase; 6] = [
    Phase { labels: ["CHAUFFE", "CHAUF.", "C"], color: 0xB9412F, end_s: 5.5, dark: false },
    Phase { labels: ["REMPLISSAGE", "REMPL.", "R"], color: 0x4A9BB5, end_s: 8.5, dark: false },
    Phase { labels: ["PRÉ-INFUSION", "PRÉ-INF.", "PI"], color: 0x453426, end_s: 12.5, dark: false },
    Phase { labels: ["MONTÉE", "MONT.", "M"], color: 0x8C5A22, end_s: 16.2, dark: false },
    Phase { labels: ["INFUSION", "INF.", "I"], color: 0xD98324, end_s: 40.0, dark: true },
    Phase { labels: ["GOUTTES", "GTTE", "G"], color: 0x6B615A, end_s: 45.0, dark: false },
];
const PHASE_NAMES: [&str; 6] =
    ["pré-chauffe", "remplissage", "pré-infusion", "montée", "infusion", "dernières gouttes"];
const PUMP_START_S: f32 = 5.5;
const PUMP_STOP_S: f32 = 40.0;
const DROPS_END_S: f32 = 45.0;

const FRISE_X: f32 = 32.0;
const FRISE_W: f32 = 736.0;
const SEGMENT_GAP: f32 = 2.0;

pub struct Demo {
    tick: u32,
    reduced_rate: bool,
    segments: Rc<VecModel<Segment>>,
    tiles: Rc<VecModel<Tile>>,
    cells: Rc<VecModel<Cell>>,
}

/// Valeur à une décimale, virgule française : `12,1`.
fn dec1(v: f32) -> Fixed {
    let tenths = if v >= 0.0 { (v * 10.0 + 0.5) as i32 } else { (v * 10.0 - 0.5) as i32 };
    let mut out = Fixed::new();
    let sign = if tenths < 0 { "-" } else { "" };
    let _ = write!(out, "{sign}{},{}", tenths.unsigned_abs() / 10, tenths.unsigned_abs() % 10);
    out
}

/// Valeur à deux décimales : `0,88`.
fn dec2(v: f32) -> Fixed {
    let hundredths = (v.max(0.0) * 100.0 + 0.5) as u32;
    let mut out = Fixed::new();
    let _ = write!(out, "{},{:02}", hundredths / 100, hundredths % 100);
    out
}

/// Texte court sans allocation, converti en `SharedString` à l'affectation.
struct Fixed {
    buf: [u8; 48],
    len: usize,
}

impl Fixed {
    fn new() -> Self {
        Self { buf: [0; 48], len: 0 }
    }

    fn as_str(&self) -> &str {
        core::str::from_utf8(&self.buf[..self.len]).unwrap_or("")
    }

    fn with(mut self, suffix: &str) -> SharedString {
        let _ = self.write_str(suffix);
        self.as_str().into()
    }
}

impl Write for Fixed {
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

/// Onde triangulaire de période `period` s, entre -1 et 1 : de quoi faire
/// bouger les valeurs sans fonctions trigonométriques.
fn wobble(t: f32, period: f32) -> f32 {
    let phase = (t / period) - (t / period) as u32 as f32;
    if phase < 0.5 { phase * 4.0 - 1.0 } else { 3.0 - phase * 4.0 }
}

fn ramp(t: f32, t0: f32, t1: f32) -> f32 {
    ((t - t0) / (t1 - t0)).clamp(0.0, 1.0)
}

/// Poids en tasse : rien avant la montée, ~1,4 g/s en infusion, puis les
/// gouttes.
fn weight_at(t: f32) -> f32 {
    let brew = (t.min(PUMP_STOP_S) - 16.2).max(0.0) * 1.38;
    let rise = ramp(t, 13.5, 16.2) * 1.2;
    let drops = ramp(t, PUMP_STOP_S, DROPS_END_S) * 1.6;
    rise + brew + drops
}

fn pressure_at(t: f32) -> f32 {
    if !(PUMP_START_S..PUMP_STOP_S + 1.5).contains(&t) {
        0.0
    } else if t < 8.5 {
        1.4
    } else if t < 12.5 {
        2.0
    } else if t < 16.2 {
        2.0 + ramp(t, 12.5, 16.2) * 7.0
    } else if t < PUMP_STOP_S {
        9.0 + 0.15 * wobble(t, 3.1)
    } else {
        9.0 * (1.0 - ramp(t, PUMP_STOP_S, PUMP_STOP_S + 1.5))
    }
}

fn temperature_at(t: f32) -> f32 {
    93.4 - ramp(t, 8.5, 30.0) * 2.4 + ramp(t, 30.0, 50.0) * 2.4 + 0.1 * wobble(t, 1.7)
}

impl Demo {
    pub fn new(ui: &MainWindow) -> Self {
        let segments = Rc::new(VecModel::from(Vec::new()));
        let tiles = Rc::new(VecModel::from(
            [
                ("poids", ""),
                ("temps infusion", ""),
                ("moyenne en tasse", ""),
                ("temps total", "pompe"),
                ("débit infusion", ""),
                ("baisse thermique", ""),
            ]
            .map(|(title, aside)| Tile { title: title.into(), aside: aside.into(), ..Default::default() })
            .to_vec(),
        ));
        let cells = Rc::new(VecModel::from(
            ["chauffe", "pompe", "débit balance", "débit pompe"]
                .map(|title| Cell { title: title.into(), ..Default::default() })
                .to_vec(),
        ));
        let live = ui.global::<Live>();
        live.set_segments(segments.clone().into());
        live.set_tiles(tiles.clone().into());
        live.set_cells(cells.clone().into());
        let demo = Self { tick: 0, reduced_rate: false, segments, tiles, cells };
        demo.apply(ui, Group::ALL);
        demo
    }

    /// Fréquence réduite par groupe (`REDUCED_RATES`), au lieu de 10 Hz pour
    /// toutes les valeurs.
    pub fn set_reduced_rate(&mut self, reduced_rate: bool) {
        self.reduced_rate = reduced_rate;
    }

    /// Avance d'un pas de 50 ms.
    pub fn step(&mut self, ui: &MainWindow) {
        self.tick = (self.tick + 1) % CYCLE_TICKS;
        let groups = if self.reduced_rate {
            REDUCED_RATES
                .iter()
                .filter(|(_, period, offset)| self.tick % period == *offset)
                .fold(0, |acc, (group, _, _)| acc | group)
        } else if self.tick % FULL_RATE_PERIOD == 0 {
            Group::ALL
        } else {
            0
        };
        if groups != 0 {
            self.apply(ui, groups);
        }
    }

    /// Se place à `seconds` du départ, pour les instantanés.
    pub fn seek(&mut self, ui: &MainWindow, seconds: f32) {
        self.tick = ((seconds * 1000.0) as u32 / STEP_MS) % CYCLE_TICKS;
        self.apply(ui, Group::ALL);
    }

    fn apply(&self, ui: &MainWindow, groups: u8) {
        let t = (self.tick * STEP_MS) as f32 / 1000.0;
        let live = ui.global::<Live>();
        let weight = weight_at(t);
        let pressure = pressure_at(t);
        let temperature = temperature_at(t);
        let pumping = (PUMP_START_S..PUMP_STOP_S).contains(&t);

        if groups & Group::TOPBAR != 0 {
            live.set_weight(dec1(weight).with(" g"));
            live.set_pressure(dec1(pressure).with(" bar"));
            live.set_temperature(dec1(temperature).with("°"));
            live.set_heat((0.35 + 0.3 * wobble(t, 7.0)).clamp(0.0, 1.0));
            live.set_hero(dec1(weight).with(" g"));
        }

        if groups & Group::FRISE != 0 {
            let current = PHASES.iter().position(|p| t < p.end_s);
            live.set_phase(match current {
                Some(i) => PHASE_NAMES[i].into(),
                None => "terminé".into(),
            });
            self.update_frise(ui, t, current);
        }
        if groups & Group::TILES != 0 {
            self.update_tiles(t, weight, temperature);
        }
        if groups & Group::CELLS != 0 {
            self.update_cells(t, pumping);
        }
    }

    fn update_tiles(&self, t: f32, weight: f32, temperature: f32) {
        let brew_s = (t.min(PUMP_STOP_S) - 16.2).max(0.0);
        let total_s = t.min(PUMP_STOP_S);
        let pump_s = (t.min(PUMP_STOP_S) - PUMP_START_S).max(0.0);
        let flow = if brew_s >= 1.0 { (weight_at(t.min(PUMP_STOP_S)) - 1.2) / brew_s } else { 0.0 };
        let unknown = || SharedString::from("-");
        self.set_tile(0, dec1(weight).with(" g"), "".into(), true);
        self.set_tile(1, dec1(brew_s).with(" s"), "".into(), false);
        self.set_tile(
            2,
            if weight > 1.0 { dec1(temperature - 3.5).with("°") } else { unknown() },
            "cible 90°".into(),
            false,
        );
        self.set_tile(3, dec1(total_s).with(" s"), "".into(), true);
        self.set_tile(
            4,
            if brew_s >= 1.0 { dec2(flow).with(" g/s") } else { unknown() },
            if brew_s >= 1.0 { dec2(flow * 1.6).with(" ml/s") } else { "".into() },
            false,
        );
        // Baisse thermique depuis le départ, et plage min-max.
        let min_t = temperature_at(t.min(30.0));
        let mut range = Fixed::new();
        let _ = write!(range, "{}-{}°", dec1(min_t).as_str(), dec1(93.4).as_str());
        self.set_tile(5, dec1(min_t - 93.4).with("°"), range.as_str().into(), false);
        let mut aside = Fixed::new();
        let _ = write!(aside, "pompe {}", dec1(pump_s).as_str());
        self.set_aside(3, aside.as_str().into());
    }

    fn update_cells(&self, t: f32, pumping: bool) {
        let heat_pct = (28.0 + 10.0 * wobble(t, 2.3)) as u32;
        let pump_pct = if pumping { (65.0 + 5.0 * wobble(t, 4.1)) as u32 } else { 0 };
        let scale_flow = if (16.2..DROPS_END_S).contains(&t) { 1.38 + 0.1 * wobble(t, 1.3) } else { 0.0 };
        let pump_flow = if pumping { 1.25 + 0.08 * wobble(t, 2.9) } else { 0.0 };
        let pct = |v: u32| {
            let mut s = Fixed::new();
            let _ = write!(s, "{v} %");
            SharedString::from(s.as_str())
        };
        self.set_cell(0, pct(heat_pct));
        self.set_cell(1, pct(pump_pct));
        self.set_cell(2, dec2(scale_flow).with(" g/s"));
        self.set_cell(3, dec2(pump_flow).with(" ml/s"));
    }

    fn update_frise(&self, ui: &MainWindow, t: f32, current: Option<usize>) {
        // Échelle : 4 s de réserve au-delà de l'arrêt prévu de la pompe, puis
        // la fin des gouttes remplit la largeur.
        let scale_s = if t < PUMP_STOP_S { PUMP_STOP_S + 4.0 } else { DROPS_END_S.max(t.min(DROPS_END_S)) };
        let px = |s: f32| FRISE_X + s / scale_s * FRISE_W;
        let live = ui.global::<Live>();
        live.set_show_reserve(t < PUMP_STOP_S);
        live.set_reserve_x(px(PUMP_STOP_S) + SEGMENT_GAP);

        let mut start = 0.0;
        let mut row = 0;
        for (i, phase) in PHASES.iter().enumerate() {
            if t <= start {
                break;
            }
            let end = t.min(phase.end_s);
            let (x, w) = (px(start), px(end) - px(start) - SEGMENT_GAP);
            let label = phase.labels.iter().find(|l| label_width(l) + 8.0 <= w).copied().unwrap_or("");
            let duration = dec1(end - start);
            let segment = Segment {
                label: label.into(),
                duration: if label_width(duration.as_str()) * 1.45 <= w + SEGMENT_GAP {
                    duration.as_str().into()
                } else {
                    SharedString::new()
                },
                color: Color::from_argb_encoded(0xFF00_0000 | phase.color),
                x,
                width: w.max(1.0),
                current: current == Some(i),
                dark_label: phase.dark,
            };
            if row < self.segments.row_count() {
                if self.segments.row_data(row).as_ref() != Some(&segment) {
                    self.segments.set_row_data(row, segment);
                }
            } else {
                self.segments.push(segment);
            }
            row += 1;
            start = phase.end_s;
        }
        while self.segments.row_count() > row {
            self.segments.remove(row);
        }
    }

    fn set_tile(&self, i: usize, value: SharedString, detail: SharedString, highlight: bool) {
        let mut tile = self.tiles.row_data(i).unwrap_or_default();
        if tile.value != value || tile.detail != detail || tile.highlight != highlight {
            (tile.value, tile.detail, tile.highlight) = (value, detail, highlight);
            self.tiles.set_row_data(i, tile);
        }
    }

    fn set_aside(&self, i: usize, aside: SharedString) {
        let mut tile = self.tiles.row_data(i).unwrap_or_default();
        if tile.aside != aside {
            tile.aside = aside;
            self.tiles.set_row_data(i, tile);
        }
    }

    fn set_cell(&self, i: usize, value: SharedString) {
        let mut cell = self.cells.row_data(i).unwrap_or_default();
        if cell.value != value {
            cell.value = value;
            self.cells.set_row_data(i, cell);
        }
    }
}

/// Largeur approchée d'un libellé en 18 px (Inter, ~0,62 em par capitale).
fn label_width(label: &str) -> f32 {
    label.chars().count() as f32 * 11.0
}

/// Lance la simulation sur un minuteur Slint à un pas toutes les 50 ms, que
/// la boucle de rendu (essai) ou la boucle d'événements (simulateur) fait
/// tourner, avec ou sans fréquence réduite (`Demo::set_reduced_rate`). Le
/// minuteur rendu doit rester en vie.
pub fn run(ui: &MainWindow, reduced_rate: bool) -> Timer {
    let mut demo = Demo::new(ui);
    demo.set_reduced_rate(reduced_rate);
    let demo = RefCell::new(demo);
    let weak = ui.as_weak();
    let timer = Timer::default();
    timer.start(TimerMode::Repeated, core::time::Duration::from_millis(STEP_MS as u64), move || {
        if let Some(ui) = weak.upgrade() {
            demo.borrow_mut().step(&ui);
        }
    });
    timer
}
