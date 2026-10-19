//! Infusion guidée : la routine d'avant-infusion (rinçage du porte-filtre,
//! attente de la consigne, préparation) suivie écran par écran, avec le
//! déroulé enregistré pour la capture HF. La purge sans porte-filtre qui
//! précède se fait hors du guide : sans porte-filtre, l'eau gicle autour de
//! la tasse et mouille la balance.
//!
//! La machine à états est pure : [`Guided`] reçoit l'heure monotone, les
//! pesées horodatées, l'instantané et la configuration, et rend des
//! [`Request`] que [`crate::core::Core`] exécute (tare, infusion, abandon). Les actions soumises au cœur (purge, infusion) et leurs
//! refus sont orchestrés par le cœur, qui rend compte par les méthodes
//! `purge_started`, `refused`, `tare_sent`... Rien ici n'alloue.
//!
//! Les boutons ne sont que des préconditions : la balance et la température
//! activent **suivant** ou **démarrer**, elles ne font jamais avancer le guide
//! seules.

use alloc::format;
use alloc::string::{String, ToString};

use crate::config::Config;
use crate::core::ActionStatus;
use crate::json::format_g;
use crate::snapshot::{CycleState, Snapshot, brew_ready};

/// Durée d'un rinçage, confiée au cycle de purge qui s'arrête seul.
pub const RINSE_US: i64 = 5_000_000;
/// Poids au-dessus duquel une tasse est posée.
pub const CUP_PLACED_G: f32 = 30.0;
/// Poids sous lequel la tasse est retirée. Plus bas que zéro, pour qu'une
/// oscillation de la balance autour de zéro ne passe pas pour un retrait.
pub const CUP_REMOVED_G: f32 = -5.0;
/// Bande de poids où une balance tarée est considérée revenue à zéro.
const TARE_BAND_G: f32 = 2.0;
/// Durée maximale pendant laquelle le poids est ignoré après une tare.
const TARE_SETTLE_US: i64 = 2_000_000;
/// Fenêtre et amplitude de la stabilité du poids avant de tarer.
const STABLE_WINDOW_US: i64 = 1_000_000;
const STABLE_SPREAD_G: f32 = 0.5;
/// Délai entre l'arrêt d'un rinçage et la lecture du poids gagné.
const WEIGHT_READ_DELAY_US: i64 = 2_000_000;
/// Abandon sans appui.
const IDLE_ABANDON_US: i64 = 30 * 60 * 1_000_000;
/// Durée d'affichage d'un refus du cœur.
const REFUSAL_US: i64 = 5_000_000;
/// Au plus ce pas est compté pour les durées cumulées : un pas de tâche
/// manqué ne gonfle pas le temps de chauffe ou de balance perdue.
const MAX_STEP_US: i64 = 250_000;
/// Marge sous la consigne à partir de laquelle un rinçage est possible.
const RINSE_MARGIN_C: f32 = 5.0;
/// Pesées gardées pour juger de la stabilité (plusieurs secondes à 10 Hz).
const WEIGHT_WINDOW: usize = 32;
/// Écart au poids de la tasse pesée au rinçage sous lequel la tasse revenue
/// est reconnue. Fixe plutôt que proportionnel : il couvre les gouttes
/// d'une tasse rincée, quelle que soit la tasse.
pub const CUP_MATCH_G: f32 = 1.5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Screen {
    #[default]
    Rinse,
    Wait,
}

/// Une valeur par consigne affichable ; le texte est choisi par le
/// présentateur.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Hint {
    #[default]
    ConnectScale,
    WaitRinseTemp,
    PlaceCup,
    PressNextRinse,
    CupRemovedEarly,
    WaitSetpoint,
    Prepare,
    WaitBrewTemp,
    PressStart,
}

/// Ce que l'écran affiche du guide.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct View {
    pub active: bool,
    pub screen: Screen,
    pub hint: Hint,
    /// Le bouton d'action (**suivant** ou **démarrer**) est utilisable. Vrai
    /// pendant un rinçage, où le bouton **arrêter** le remplace.
    pub button_enabled: bool,
    /// Un rinçage du guide est en cours.
    pub purging: bool,
    pub purge_elapsed_ms: u32,
    /// Température minimale d'un rinçage (consigne moins 5 °C).
    pub rinse_min_c: f32,
    /// Secondes depuis la fin du rinçage.
    pub since_rinse_s: u32,
    /// La tasse revenue avant l'infusion pèse le poids relevé au rinçage.
    pub cup_recognized: bool,
    /// Refus du cœur à afficher en rouge sous le bouton.
    pub refusal: Option<ActionStatus>,
}

/// Demande du guide au cœur.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Request {
    /// Tarer la balance vide.
    Tare,
    /// Lancer l'infusion : la tare demandée par **démarrer** est absorbée.
    Start,
    /// Abandonner le guide (défaut, mise à jour, inactivité...).
    Abandon,
}

/// Ce que l'appui sur **suivant** ou **démarrer** doit exécuter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Press {
    /// Bouton inactif ou rinçage en cours : rien à faire.
    Ignored,
    /// Tarer la balance, tasse sur le plateau, puis lancer le rinçage.
    Rinse,
    /// Tarer, puis lancer l'infusion une fois la tare absorbée. Lancée avant,
    /// l'infusion garderait le poids de la tasse comme poids de départ et
    /// verrait la tare comme une balance qui recule.
    Start,
}

/// Le rinçage enregistré. Les valeurs inconnues sont NaN.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PurgeRecord {
    pub start_us: i64,
    pub duration_us: i64,
    pub volume_ml: f32,
    pub weight_g: f32,
    pub ntc_start_c: f32,
    pub ntc_min_c: f32,
    pub ntc_end_c: f32,
    pub pressure_max_bar: f32,
}

impl PurgeRecord {
    const EMPTY: Self = Self {
        start_us: 0,
        duration_us: 0,
        volume_ml: f32::NAN,
        weight_g: f32::NAN,
        ntc_start_c: f32::NAN,
        ntc_min_c: f32::NAN,
        ntc_end_c: f32::NAN,
        pressure_max_bar: 0.0,
    };
}

/// Déroulé du guide : des instants (µs monotones) et des valeurs résumées,
/// aucun échantillon.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Session {
    pub opened_us: i64,
    pub opened_at_unix_s: i64,
    pub setpoint_c: f32,
    pub ntc_open_c: f32,
    /// Poids de la tasse vide à l'appui sur **suivant** ; NaN avant.
    pub cup_g: f32,
    pub rinse: PurgeRecord,
    pub wait_ntc_min_c: f32,
    pub setpoint_reached_us: Option<i64>,
    pub heater_on_us: i64,
    pub early_removals: u16,
    pub cup_removed_us: Option<i64>,
    pub cup_returned_us: Option<i64>,
    pub start_pressed_us: i64,
    pub scale_lost_us: i64,
}

impl Session {
    const fn new() -> Self {
        Self {
            opened_us: 0,
            opened_at_unix_s: 0,
            setpoint_c: 0.0,
            ntc_open_c: f32::NAN,
            cup_g: f32::NAN,
            rinse: PurgeRecord::EMPTY,
            wait_ntc_min_c: f32::NAN,
            setpoint_reached_us: None,
            heater_on_us: 0,
            early_removals: 0,
            cup_removed_us: None,
            cup_returned_us: None,
            start_pressed_us: 0,
            scale_lost_us: 0,
        }
    }

    /// Objet JSON de la clé `session` de `/hf-capture`. Les instants sont en
    /// millisecondes relatives au début de la capture (négatifs avant elle),
    /// arrondies à l'entier.
    pub fn to_json(&self, capture_start_us: i64) -> String {
        let rel = |t: i64| ms(t - capture_start_us).to_string();
        let rel_opt = |t: Option<i64>| t.map_or_else(|| "null".to_string(), rel);
        let p = &self.rinse;
        let rinse = format!(
            "{{\"start_ms\":{},\"duration_ms\":{},\"volume_ml\":{},\"weight_g\":{},\
\"ntc_start_c\":{},\"ntc_min_c\":{},\"ntc_end_c\":{},\"pressure_max_bar\":{}}}",
            rel(p.start_us),
            ms(p.duration_us),
            num(p.volume_ml),
            num(p.weight_g),
            num(p.ntc_start_c),
            num(p.ntc_min_c),
            num(p.ntc_end_c),
            num(p.pressure_max_bar),
        );
        format!(
            "{{\"opened_ms\":{},\"opened_at_unix_s\":{},\"setpoint_c\":{},\"ntc_open_c\":{},\
\"cup_g\":{},\"rinse\":{rinse},\"wait\":{{\"ntc_min_c\":{},\"setpoint_reached_ms\":{},\
\"heater_on_ms\":{},\"early_removals\":{}}},\"cup_removed_ms\":{},\"cup_returned_ms\":{},\
\"start_pressed_ms\":{},\"scale_lost_ms\":{}}}",
            rel(self.opened_us),
            self.opened_at_unix_s,
            num(self.setpoint_c),
            num(self.ntc_open_c),
            num(round_tenth(self.cup_g)),
            num(self.wait_ntc_min_c),
            rel_opt(self.setpoint_reached_us),
            ms(self.heater_on_us),
            self.early_removals,
            rel_opt(self.cup_removed_us),
            rel_opt(self.cup_returned_us),
            rel(self.start_pressed_us),
            ms(self.scale_lost_us),
        )
    }
}

/// Microsecondes en millisecondes entières, arrondies au plus proche.
fn ms(us: i64) -> i64 {
    libm::round(us as f64 / 1000.0) as i64
}

/// Au dixième de gramme, la résolution de la balance.
fn round_tenth(value: f32) -> f32 {
    libm::roundf(value * 10.0) / 10.0
}

fn num(value: f32) -> String {
    if value.is_finite() {
        format_g(f64::from(value), 6)
    } else {
        "null".to_string()
    }
}

/// Suivi de la tasse sur la balance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Cup {
    Present,
    /// Poids sous le seuil de retrait, pas encore taré.
    Removed,
    /// Tare à vide envoyée : il faut une tasse (poids au-dessus du seuil de
    /// pose) pour revenir à `Present`.
    TaredEmpty,
}

/// Rinçage en cours.
#[derive(Debug, Clone, Copy)]
struct Rinse {
    started_us: i64,
    start_volume_ml: f32,
}

#[derive(Debug, Clone, Copy)]
struct Reading {
    at_us: i64,
    weight_g: f32,
}

/// Machine à états du guide.
#[derive(Debug, Clone, Copy)]
pub struct Guided {
    active: bool,
    screen: Screen,
    session: Session,
    last_press_us: i64,
    last_step_us: i64,
    rinse: Option<Rinse>,
    /// Instant de lecture du poids gagné, 2 s après l'arrêt du rinçage. La
    /// balance a été tarée tasse posée : le poids lu est l'eau du rinçage.
    weight_due_us: Option<i64>,
    rinse_end_us: i64,
    last_weight: Option<Reading>,
    window: [Reading; WEIGHT_WINDOW],
    window_len: usize,
    window_next: usize,
    /// Une tare d'ouverture est due : à l'ouverture, et à l'écran du
    /// rinçage chaque fois que la balance revient, car on ne sait pas ce qui
    /// a été posé entre-temps.
    open_tare: bool,
    /// Le suivi du retrait de la tasse commence avec la tare d'ouverture.
    tracking: bool,
    cup: Cup,
    /// Une tare à vide a suivi un retrait depuis la consigne atteinte : le
    /// poids lu est celui de la tasse revenue.
    empty_tare_after_setpoint: bool,
    /// La tasse était reconnue à l'appui sur **démarrer** ; gardé pendant
    /// la tare qui précède l'infusion.
    cup_recognized_at_start: bool,
    settling_since_us: Option<i64>,
    /// Une tare a été refusée : traité comme une balance absente jusqu'à
    /// ce qu'elle se reconnecte.
    tare_blocked: bool,
    blocked_saw_absence: bool,
    refusal: Option<(ActionStatus, i64)>,
    /// **Démarrer** est pressé : l'infusion part quand la tare est absorbée.
    start_pending: bool,
}

impl Default for Guided {
    fn default() -> Self {
        Self::new()
    }
}

impl Guided {
    pub const fn new() -> Self {
        Self {
            active: false,
            screen: Screen::Rinse,
            session: Session::new(),
            last_press_us: 0,
            last_step_us: 0,
            rinse: None,
            weight_due_us: None,
            rinse_end_us: 0,
            last_weight: None,
            window: [Reading {
                at_us: 0,
                weight_g: 0.0,
            }; WEIGHT_WINDOW],
            window_len: 0,
            window_next: 0,
            open_tare: false,
            tracking: false,
            cup: Cup::Present,
            empty_tare_after_setpoint: false,
            cup_recognized_at_start: false,
            settling_since_us: None,
            tare_blocked: false,
            blocked_saw_absence: false,
            refusal: None,
            start_pending: false,
        }
    }

    pub fn active(&self) -> bool {
        self.active
    }

    /// Un rinçage du guide est en cours.
    pub fn purging(&self) -> bool {
        self.rinse.is_some()
    }

    /// Ouvre le guide à l'écran 1. `wall_unix_s` vaut 0 si l'heure murale
    /// n'est pas connue. La tare d'ouverture part au pas suivant.
    pub fn open(&mut self, now_us: i64, wall_unix_s: i64, s: &Snapshot, c: &Config) {
        *self = Self::new();
        self.active = true;
        self.open_tare = true;
        self.last_press_us = now_us;
        self.last_step_us = now_us;
        self.last_weight = self.last_weight_of(s);
        self.session.opened_us = now_us;
        self.session.opened_at_unix_s = wall_unix_s;
        self.session.setpoint_c = c.brew_temperature_c;
        self.session.ntc_open_c = ntc_of(s);
    }

    /// Le guide se termine ; le déroulé est jeté.
    pub fn close(&mut self) {
        self.active = false;
        self.rinse = None;
    }

    /// Appui sur l'écran : repousse l'abandon pour inactivité.
    pub fn note_press(&mut self, now_us: i64) {
        self.last_press_us = now_us;
    }

    /// Pesée reçue de la balance.
    pub fn on_weight(&mut self, now_us: i64, weight_g: f32) {
        if !self.active {
            return;
        }
        self.last_weight = Some(Reading {
            at_us: now_us,
            weight_g,
        });
        if let Some(since) = self.settling_since_us {
            // Une pesée antérieure à la tare ne prouve rien : la balance a
            // fini de revenir à zéro quand elle lit une valeur proche de zéro
            // après l'envoi.
            if now_us > since && libm::fabsf(weight_g) <= TARE_BAND_G {
                self.settling_since_us = None;
            } else {
                return;
            }
        }
        self.push_window(Reading {
            at_us: now_us,
            weight_g,
        });
        if !self.tracking {
            return;
        }
        match self.cup {
            Cup::Present if weight_g < CUP_REMOVED_G => {
                self.cup = Cup::Removed;
                if self.screen == Screen::Wait {
                    if self.session.setpoint_reached_us.is_none() {
                        self.session.early_removals = self.session.early_removals.saturating_add(1);
                    } else if self.session.cup_removed_us.is_none() {
                        self.session.cup_removed_us = Some(now_us);
                    }
                }
            }
            // La tasse est revenue avant que la tare à vide soit partie : la
            // balance lit de nouveau près de zéro, ou davantage.
            Cup::Removed if weight_g > -TARE_BAND_G => self.cup_returned(now_us),
            Cup::TaredEmpty if weight_g > CUP_PLACED_G => self.cup_returned(now_us),
            // La tare n'a pas pris : la balance lit toujours un retrait.
            Cup::TaredEmpty if weight_g < CUP_REMOVED_G => self.cup = Cup::Removed,
            _ => {}
        }
    }

    fn cup_returned(&mut self, now_us: i64) {
        self.cup = Cup::Present;
        if self.screen == Screen::Wait
            && self.session.cup_removed_us.is_some()
            && self.session.setpoint_reached_us.is_some()
        {
            self.session.cup_returned_us = Some(now_us);
        }
    }

    fn push_window(&mut self, reading: Reading) {
        self.window[self.window_next] = reading;
        self.window_next = (self.window_next + 1) % WEIGHT_WINDOW;
        self.window_len = (self.window_len + 1).min(WEIGHT_WINDOW);
    }

    /// Variation de 0,5 g au plus sur la dernière seconde, avec au moins une
    /// pesée dans la fenêtre et une pesée plus ancienne qui en prouve la
    /// durée.
    fn weight_stable(&self, now_us: i64) -> bool {
        let mut min = f32::INFINITY;
        let mut max = f32::NEG_INFINITY;
        let mut recent = false;
        for k in 0..self.window_len {
            let index = (self.window_next + WEIGHT_WINDOW - 1 - k) % WEIGHT_WINDOW;
            let reading = self.window[index];
            min = min.min(reading.weight_g);
            max = max.max(reading.weight_g);
            if reading.at_us >= now_us - STABLE_WINDOW_US {
                recent = true;
            } else {
                return recent && max - min <= STABLE_SPREAD_G;
            }
        }
        false
    }

    fn last_weight_of(&self, s: &Snapshot) -> Option<Reading> {
        s.scale_present.then_some(Reading {
            at_us: 0,
            weight_g: s.weight_g,
        })
    }

    /// Une tare vient de partir.
    pub fn tare_sent(&mut self, now_us: i64) {
        self.settling_since_us = Some(now_us);
        self.window_len = 0;
        self.window_next = 0;
        if self.cup == Cup::Removed {
            self.cup = Cup::TaredEmpty;
            if self.screen == Screen::Wait && self.session.setpoint_reached_us.is_some() {
                self.empty_tare_after_setpoint = true;
            }
        }
    }

    /// Le cœur a refusé la tare : la balance compte comme absente.
    pub fn tare_refused(&mut self) {
        self.tare_blocked = true;
        self.blocked_saw_absence = false;
    }

    /// Le cœur a refusé l'action du bouton.
    pub fn refused(&mut self, now_us: i64, status: ActionStatus) {
        self.start_pending = false;
        self.refusal = Some((status, now_us));
    }

    /// Le cœur a accepté le rinçage demandé par [`Press::Rinse`], balance
    /// tarée tasse posée.
    pub fn purge_started(&mut self, now_us: i64, s: &Snapshot) {
        self.refusal = None;
        // La tasse est sur le plateau et tarée : un retrait passe sous zéro.
        self.cup = Cup::Present;
        let mut record = PurgeRecord::EMPTY;
        record.start_us = now_us;
        record.ntc_start_c = ntc_of(s);
        record.ntc_min_c = ntc_of(s);
        self.session.rinse = record;
        self.rinse = Some(Rinse {
            started_us: now_us,
            start_volume_ml: s.volume_ml,
        });
    }

    /// La tare de **démarrer** est partie : l'infusion suivra.
    pub fn start_requested(&mut self) {
        self.start_pending = true;
    }

    /// La tasse revenue pèse, à [`CUP_MATCH_G`] près, celle pesée au
    /// rinçage. Seulement après une tare à vide qui suit la consigne : avant,
    /// la balance compte encore la tasse tarée au rinçage.
    fn cup_recognized(&self) -> bool {
        if self.start_pending {
            return self.cup_recognized_at_start;
        }
        self.screen == Screen::Wait
            && self.empty_tare_after_setpoint
            && self.settling_since_us.is_none()
            && self.session.cup_g.is_finite()
            && self.last_weight.is_some_and(|reading| {
                libm::fabsf(reading.weight_g - self.session.cup_g) <= CUP_MATCH_G
            })
    }

    /// L'infusion est partie : fige et rend le déroulé. Le guide se termine.
    pub fn started(&mut self, now_us: i64) -> Session {
        let mut session = self.session;
        session.start_pressed_us = now_us;
        self.close();
        session
    }

    fn rinse_possible(&self, s: &Snapshot, c: &Config) -> bool {
        s.boiler_temperature_valid
            && s.boiler_temperature_c >= c.brew_temperature_c - RINSE_MARGIN_C
    }

    fn scale_ok(&self, s: &Snapshot) -> bool {
        s.scale_present && !self.tare_blocked
    }

    fn cup_placed(&self) -> bool {
        self.last_weight
            .is_some_and(|reading| reading.weight_g > CUP_PLACED_G)
    }

    /// Consigne et bouton de l'écran courant.
    fn evaluate(&self, s: &Snapshot, c: &Config) -> (Hint, bool) {
        let scale = self.scale_ok(s);
        let rinse_ok = self.rinse_possible(s, c);
        match self.screen {
            Screen::Rinse => {
                // Tant que la tare d'ouverture n'est pas absorbée, le poids
                // lu ne pèse pas encore la tasse.
                let tared = !self.open_tare && self.settling_since_us.is_none();
                let placed = tared && self.cup_placed();
                let hint = if !scale {
                    Hint::ConnectScale
                } else if !rinse_ok {
                    Hint::WaitRinseTemp
                } else if !placed {
                    Hint::PlaceCup
                } else {
                    Hint::PressNextRinse
                };
                (hint, scale && placed && rinse_ok)
            }
            Screen::Wait => {
                let cup = self.cup == Cup::Present;
                let ready = brew_ready(s, c);
                if self.session.setpoint_reached_us.is_none() {
                    let hint = if !scale {
                        Hint::ConnectScale
                    } else if !cup {
                        Hint::CupRemovedEarly
                    } else {
                        Hint::WaitSetpoint
                    };
                    (hint, false)
                } else {
                    let prepared = cup
                        && self.session.cup_removed_us.is_some()
                        && self.session.cup_returned_us.is_some();
                    let hint = if !scale {
                        Hint::ConnectScale
                    } else if !prepared {
                        Hint::Prepare
                    } else if !ready {
                        Hint::WaitBrewTemp
                    } else {
                        Hint::PressStart
                    };
                    (hint, scale && prepared && ready && !self.start_pending)
                }
            }
        }
    }

    /// Ce que le guide affiche à `now_us`.
    pub fn view(&self, now_us: i64, s: &Snapshot, c: &Config) -> View {
        if !self.active {
            return View::default();
        }
        let (hint, enabled) = self.evaluate(s, c);
        let purge_elapsed_ms = self.rinse.map_or(0, |r| {
            ((now_us - r.started_us).clamp(0, RINSE_US) / 1000) as u32
        });
        let since_rinse_s = if self.screen == Screen::Wait {
            ((now_us - self.rinse_end_us).max(0) / 1_000_000) as u32
        } else {
            0
        };
        View {
            active: true,
            screen: self.screen,
            hint,
            button_enabled: enabled || self.rinse.is_some(),
            purging: self.rinse.is_some(),
            purge_elapsed_ms,
            rinse_min_c: c.brew_temperature_c - RINSE_MARGIN_C,
            since_rinse_s,
            cup_recognized: self.cup_recognized(),
            refusal: self.refusal.map(|(status, _)| status),
        }
    }

    /// Appui sur le bouton d'action : ce que le cœur doit exécuter.
    pub fn press(&mut self, now_us: i64, s: &Snapshot, c: &Config) -> Press {
        if !self.active || self.rinse.is_some() || self.start_pending {
            return Press::Ignored;
        }
        self.last_press_us = now_us;
        let (_, enabled) = self.evaluate(s, c);
        if !enabled {
            return Press::Ignored;
        }
        self.refusal = None;
        match self.screen {
            Screen::Rinse => {
                // Le poids avant la tare du rinçage est celui de la tasse
                // vide, depuis la tare d'ouverture.
                self.session.cup_g = self.last_weight.map_or(f32::NAN, |r| r.weight_g);
                Press::Rinse
            }
            Screen::Wait => {
                self.cup_recognized_at_start = self.cup_recognized();
                Press::Start
            }
        }
    }

    /// Pas périodique.
    pub fn tick(&mut self, now_us: i64, s: &Snapshot, c: &Config) -> Option<Request> {
        if !self.active {
            return None;
        }
        if s.lockout
            || s.flash_active
            || s.provisioning
            || !s.sensors_alive
            || now_us - self.last_press_us >= IDLE_ABANDON_US
        {
            return Some(Request::Abandon);
        }
        let step = (now_us - self.last_step_us).clamp(0, MAX_STEP_US);
        self.last_step_us = now_us;
        if !s.scale_present {
            self.session.scale_lost_us += step;
            if self.screen == Screen::Rinse && self.rinse.is_none() {
                self.open_tare = true;
            }
            if self.tare_blocked {
                self.blocked_saw_absence = true;
            }
        } else if self.tare_blocked && self.blocked_saw_absence {
            self.tare_blocked = false;
            self.blocked_saw_absence = false;
        }
        if let Some((_, at)) = self.refusal
            && now_us - at >= REFUSAL_US
        {
            self.refusal = None;
        }
        if let Some(since) = self.settling_since_us
            && now_us - since >= TARE_SETTLE_US
        {
            self.settling_since_us = None;
        }
        self.read_rinse_weight(now_us, s);
        self.step_rinse(now_us, s);
        if self.screen == Screen::Wait && self.rinse.is_none() {
            self.step_wait(step, s, c);
        }
        if self.start_pending && self.settling_since_us.is_none() {
            return Some(Request::Start);
        }
        // Tare d'ouverture : le plateau compte pour vide. Une tasse déjà
        // posée lit alors près de zéro ; la soulever passe sous le seuil de
        // retrait et tare à vide.
        if self.open_tare && self.settling_since_us.is_none() && self.scale_ok(s) {
            self.open_tare = false;
            self.tracking = true;
            self.cup = Cup::TaredEmpty;
            return Some(Request::Tare);
        }
        // Tare à vide d'une tasse retirée, une fois la balance stable. Aucune
        // nouvelle tare tant que la précédente n'a pas été absorbée.
        if self.tracking
            && self.cup == Cup::Removed
            && self.settling_since_us.is_none()
            && self.scale_ok(s)
            && self.weight_stable(now_us)
        {
            return Some(Request::Tare);
        }
        None
    }

    fn read_rinse_weight(&mut self, now_us: i64, s: &Snapshot) {
        let Some(due_us) = self.weight_due_us else {
            return;
        };
        if now_us < due_us {
            return;
        }
        self.weight_due_us = None;
        if let (true, Some(reading)) = (s.scale_present, self.last_weight) {
            self.session.rinse.weight_g = reading.weight_g;
        }
    }

    fn step_rinse(&mut self, now_us: i64, s: &Snapshot) {
        let Some(rinse) = self.rinse else {
            return;
        };
        let record = &mut self.session.rinse;
        let ntc = ntc_of(s);
        record.ntc_min_c = lowest(record.ntc_min_c, ntc);
        if s.pressure_valid && s.pressure_bar > record.pressure_max_bar {
            record.pressure_max_bar = s.pressure_bar;
        }
        if s.cycle_state == CycleState::Purge {
            return;
        }
        // Le cœur a arrêté la pompe (durée écoulée, relâchement ou refus des
        // capteurs) : le rinçage est fini, l'écran suivant s'ouvre. La durée
        // vient du cycle quand il la fige, au pas près sinon.
        record.duration_us = if s.cycle_state == CycleState::Finished {
            i64::from(s.cycle_elapsed_ms) * 1000
        } else {
            now_us - rinse.started_us
        };
        let end_us = rinse.started_us + record.duration_us;
        record.volume_ml = s.volume_ml - rinse.start_volume_ml;
        record.ntc_end_c = ntc;
        record.ntc_min_c = lowest(record.ntc_min_c, ntc);
        self.weight_due_us = Some(end_us + WEIGHT_READ_DELAY_US);
        self.rinse = None;
        self.refusal = None;
        self.rinse_end_us = end_us;
        self.screen = Screen::Wait;
    }

    fn step_wait(&mut self, step_us: i64, s: &Snapshot, c: &Config) {
        let ntc = ntc_of(s);
        self.session.wait_ntc_min_c = lowest(self.session.wait_ntc_min_c, ntc);
        if s.heater_on {
            self.session.heater_on_us += step_us;
        }
        if self.session.setpoint_reached_us.is_none() && brew_ready(s, c) {
            self.session.setpoint_reached_us = Some(self.last_step_us);
        }
    }
}

/// Plus petite des deux valeurs connues ; NaN est inconnu.
fn lowest(current: f32, value: f32) -> f32 {
    if !value.is_finite() || value >= current {
        current
    } else {
        value
    }
}

fn ntc_of(s: &Snapshot) -> f32 {
    if s.boiler_temperature_valid {
        s.boiler_temperature_c
    } else {
        f32::NAN
    }
}
