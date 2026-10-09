//! Présentation et gestes partagés entre la carte et le simulateur. Le cœur
//! reste seul responsable des refus, des actionneurs et de la persistance.

use super::{AppWindow, Segment, Tile};
use alloc::{format, rc::Rc, string::String, vec::Vec};
use screen_core::{
    config::{self, Config, PreinfusionMode, RampdownMode},
    core::{Action, ActionCommand, ActionSource, Input, Reply},
    snapshot::{CycleState, Freshness, Snapshot},
};
use slint::{Color, ModelRc, SharedString, VecModel};

/// Demande locale à exécuter par l'assemblage, après le traitement des gestes.
#[derive(Debug, Clone, PartialEq)]
pub enum UiRequest {
    Core(Input),
    RestartLcd,
}

/// Quatre groupes espacés : chacun revient à 4 Hz, même si le rendu est lent.
/// Un retard ne provoque jamais une rafale de rattrapage sur le framebuffer.
#[derive(Default)]
pub struct Refresh {
    next_us: u64,
    group: u8,
}
impl Refresh {
    pub fn due(&mut self, now_us: u64) -> Option<u8> {
        if now_us < self.next_us {
            return None;
        }
        let group = self.group;
        self.group = (self.group + 1) % 4;
        self.next_us = (now_us / 62_500).saturating_add(1).saturating_mul(62_500);
        Some(group)
    }
    pub fn invalidate(&mut self) {
        self.next_us = 0;
    }
}

/// Navigation locale ; aucune transition ne lance de cycle implicitement.
#[derive(Default)]
pub struct Presenter {
    pub page: i32,
    pub settings_page: usize,
    editor: Option<usize>,
    editor_value: String,
    editor_error: String,
    editor_fresh: bool,
    choice: usize,
    menu: usize,
    confirm: bool,
    last_touch_us: u64,
    pub refresh: Refresh,
}

fn core_action(action: Action) -> UiRequest {
    UiRequest::Core(Input::Action {
        command: ActionCommand::new(action),
        source: ActionSource::Ui,
    })
}
fn number(v: f32, digits: usize, unit: &str) -> String {
    if !v.is_finite() {
        return "-".into();
    }
    format!("{v:.digits$}{unit}").replace('.', ",")
}
fn tile(name: &str, value: String, detail: &str, enabled: bool, tone: i32) -> Tile {
    Tile {
        name: name.into(),
        value: value.into(),
        detail: detail.into(),
        enabled,
        tone,
    }
}
fn model<T: Clone + 'static>(v: Vec<T>) -> ModelRc<T> {
    Rc::new(VecModel::from(v)).into()
}
fn hex(v: u32) -> Color {
    Color::from_rgb_u8((v >> 16) as u8, (v >> 8) as u8, v as u8)
}
fn freshness(v: Freshness) -> &'static str {
    match v {
        Freshness::Fresh => "valide",
        Freshness::Stale => "périmé",
        Freshness::Missing => "absent",
    }
}

// L'ordre est celui des six tuiles par page, pas celui du document HTTP.
const EDITS: [[usize; 6]; 4] = [
    [0, 1, 2, 3, 4, 5],
    [6, 100, 7, 8, 9, 999],
    [101, 10, 11, 12, 13, 14],
    [102, 103, 104, 105, 15, 999],
];
const NAMES: [&str; 16] = [
    "cible temps",
    "pompe remplissage",
    "cible poids",
    "pompe pré-inf.",
    "cible pression",
    "pompe infusion",
    "pression remplissage",
    "durée remplissage",
    "échéance pré-inf.",
    "cible chaudière",
    "avance temps",
    "avance poids",
    "chute pression",
    "pompe purge",
    "purge max",
    "précharge chauffe",
];
const UNITS: [&str; 16] = [
    "s", "%", "g", "%", "bar", "%", "bar", "s", "s", "°C", "s", "g", "bar", "%", "s", "s",
];
fn decimal(e: usize) -> bool {
    matches!(e, 2 | 4 | 6 | 9 | 10 | 11 | 12 | 15)
}
fn value(c: &Config, e: usize) -> f32 {
    match e {
        0 => c.target_time_s as f32,
        1 => c.filling_pump_pct as f32,
        2 => c.target_weight_g,
        3 => c.preinfusion_pump_pct as f32,
        4 => c.target_pressure_bar,
        5 => c.brew_pump_pct as f32,
        6 => c.filling_pressure_bar,
        7 => c.filling_time_s as f32,
        8 => c.preinfusion_time_s as f32,
        9 => c.brew_temperature_c,
        10 => c.rampdown_lead_time_s,
        11 => c.rampdown_lead_weight_g,
        12 => c.rampdown_pressure_drop_bar,
        13 => c.purge_pump_pct as f32,
        14 => c.purge_max_s as f32,
        15 => c.brew_preheat_time_s,
        _ => 0.0,
    }
}
fn set_value(c: &mut Config, e: usize, v: f32) {
    match e {
        0 => c.target_time_s = v as u16,
        1 => c.filling_pump_pct = v as u8,
        2 => c.target_weight_g = v,
        3 => c.preinfusion_pump_pct = v as u8,
        4 => c.target_pressure_bar = v,
        5 => c.brew_pump_pct = v as u8,
        6 => c.filling_pressure_bar = v,
        7 => c.filling_time_s = v as u16,
        8 => c.preinfusion_time_s = v as u16,
        9 => c.brew_temperature_c = v,
        10 => c.rampdown_lead_time_s = v,
        11 => c.rampdown_lead_weight_g = v,
        12 => c.rampdown_pressure_drop_bar = v,
        13 => c.purge_pump_pct = v as u8,
        14 => c.purge_max_s = v as u16,
        15 => c.brew_preheat_time_s = v,
        _ => {}
    }
}

// Le fuseau de la machine suit les règles CET/CEST : dernier dimanche de
// mars/octobre à 01:00 UTC. L'UI ne dépend ainsi pas de libc sur la carte.
fn civil(days: i64) -> (i64, i64, i64) {
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    (y + i64::from(month <= 2), month, day)
}
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = year - i64::from(month <= 2);
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let m = month + if month > 2 { -3 } else { 9 };
    let doy = (153 * m + 2) / 5 + day - 1;
    era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719468
}
fn local_parts(unix_s: i64) -> (i64, i64, i64, i64) {
    let (year, _, _) = civil(unix_s.div_euclid(86400));
    let last_sunday = |month| {
        let last = days_from_civil(year, month, 31);
        last - (last + 4).rem_euclid(7)
    };
    let start = last_sunday(3) * 86400 + 3600;
    let end = last_sunday(10) * 86400 + 3600;
    let local = unix_s
        + if unix_s >= start && unix_s < end {
            7200
        } else {
            3600
        };
    let (_, month, day) = civil(local.div_euclid(86400));
    let seconds = local.rem_euclid(86400);
    (month, day, seconds / 3600, seconds % 3600 / 60)
}
/// Heure locale de l'écran ; chaîne vide avant la synchronisation SNTP.
pub fn clock_text(unix_s: Option<i64>) -> String {
    unix_s.map_or_else(String::new, |s| {
        let (_, _, hour, minute) = local_parts(s);
        format!("{hour:02}:{minute:02}")
    })
}
/// Date et heure de départ, dans le même fuseau que le bandeau.
pub fn shot_date_text(unix_s: i64) -> String {
    if unix_s <= 0 {
        return String::new();
    }
    let (month, day, hour, minute) = local_parts(unix_s);
    format!("{day:02}/{month:02} · {hour:02}:{minute:02}")
}

impl Presenter {
    fn edit(&mut self, e: usize, c: &Config) {
        self.editor = Some(e);
        self.editor_value = number(value(c, e), if decimal(e) { 1 } else { 0 }, "");
        self.editor_error.clear();
        self.editor_fresh = true;
        self.page = 5;
    }
    /// Traite les callbacks `command` de Slint. Le résultat doit être transmis
    /// au cœur avec la même horloge que l'instantané, puis `feedback` reçoit
    /// son refus éventuel. La purge libérée est toujours transmise, même si
    /// une faute vient de préempter l'écran.
    pub fn command(
        &mut self,
        id: i32,
        payload: &str,
        now_us: u64,
        s: &Snapshot,
        c: &Config,
    ) -> Option<UiRequest> {
        self.last_touch_us = now_us;
        self.refresh.invalidate();
        let i = payload.parse::<usize>().unwrap_or(0);
        if id == 3 {
            return Some(core_action(Action::PurgeRelease));
        }
        if s.lockout
            || s.flash_active
            || s.provisioning
            || (!s.sensors_alive && s.screen_uptime_s >= 2)
        {
            return None;
        }
        if s.cycle_state.is_active() && !matches!(id, 9 | 20) {
            return None;
        }
        match id {
            1 => Some(core_action(Action::StartBrew)),
            2 => Some(core_action(Action::PurgePress)),
            4 => {
                self.page = 4;
                None
            }
            5 => {
                self.edit(if s.scale_present { 2 } else { 0 }, c);
                None
            }
            6 | 7 => {
                let mut c = *c;
                let d = if id == 6 { -1.0 } else { 1.0 };
                if s.scale_present {
                    c.target_weight_g = (c.target_weight_g + d * 0.5).clamp(10.0, 100.0);
                } else {
                    c.target_time_s = (c.target_time_s as f32 + d).clamp(5.0, 60.0) as u16;
                }
                Some(UiRequest::Core(Input::PutConfig(c)))
            }
            8 => {
                self.page = 3;
                None
            }
            9 => Some(core_action(if s.cycle_state == CycleState::Finished {
                Action::DismissSummary
            } else {
                Action::StopBrew
            })),
            10 => {
                self.page = if matches!(self.page, 5 | 6) {
                    3
                } else if self.page == 7 {
                    4
                } else {
                    0
                };
                self.editor = None;
                None
            }
            11 => Some(UiRequest::Core(Input::WifiEnabled(!s.wifi_enabled))),
            12 => {
                self.settings_page = i.min(3);
                None
            }
            13 if i < 6 => {
                let e = EDITS[self.settings_page][i];
                match e {
                    0..=15 => self.edit(e, c),
                    100 | 101 => {
                        self.choice = e;
                        self.page = 6;
                    }
                    102 => self.confirm = true,
                    104 => return Some(UiRequest::RestartLcd),
                    _ => {}
                }
                None
            }
            14 => {
                if let Some(e) = self.editor {
                    if payload == "back" {
                        self.editor_value.pop();
                        self.editor_fresh = false;
                    } else if payload == "," {
                        if decimal(e) && !self.editor_value.contains(',') {
                            if self.editor_fresh {
                                self.editor_value.clear();
                            }
                            if self.editor_value.is_empty() {
                                self.editor_value.push('0');
                            }
                            self.editor_value.push(',');
                            self.editor_fresh = false;
                        }
                    } else if payload.len() == 1 && payload.as_bytes()[0].is_ascii_digit() {
                        if self.editor_fresh {
                            self.editor_value.clear();
                            self.editor_fresh = false;
                        }
                        if self.editor_value.len() < 7 {
                            self.editor_value.push_str(payload);
                        }
                    }
                    self.editor_error.clear();
                }
                None
            }
            15 => {
                let e = self.editor?;
                let Ok(v) = self.editor_value.replace(',', ".").parse::<f32>() else {
                    self.editor_error = "valeur invalide".into();
                    return None;
                };
                if !v.is_finite()
                    || v < 0.0
                    || (!decimal(e) && v != v as u32 as f32)
                    || (!decimal(e) && v > 255.0)
                {
                    self.editor_error = "valeur hors limites".into();
                    return None;
                }
                let mut candidate = *c;
                set_value(&mut candidate, e, v);
                if config::validate(&candidate).is_some() {
                    self.editor_error = "valeur hors limites".into();
                    return None;
                }
                Some(UiRequest::Core(Input::PutConfig(candidate)))
            }
            16 if matches!(i, 3..=5) => {
                self.menu = i;
                self.page = 7;
                None
            }
            17 => {
                let mut c = *c;
                if self.choice == 100 {
                    if i < 2 {
                        c.preinfusion_mode = PreinfusionMode(c.preinfusion_mode.0 ^ (1 << i));
                    }
                } else {
                    c.rampdown_mode = match i {
                        1 => RampdownMode::Time,
                        2 => RampdownMode::Weight,
                        3 => RampdownMode::PressureDrop,
                        _ => RampdownMode::None,
                    };
                }
                Some(UiRequest::Core(Input::PutConfig(c)))
            }
            18 => match self.menu {
                3 => Some(core_action(if i == 0 {
                    Action::ResetDimmer
                } else {
                    Action::RecalibrateDimmer
                })),
                4 => Some(core_action(if i == 0 {
                    Action::OpenMaintenanceValve
                } else {
                    Action::CloseMaintenanceValve
                })),
                5 => {
                    let mut c = *c;
                    c.heating_enabled = i == 0;
                    Some(UiRequest::Core(Input::PutConfig(c)))
                }
                _ => None,
            },
            19 => {
                self.confirm = false;
                if i == 1 {
                    Some(UiRequest::Core(Input::ForgetNetwork))
                } else {
                    None
                }
            }
            20 => None,
            _ => None,
        }
    }
    /// Affiche le refus sans fermer prématurément l'éditeur ou le menu.
    pub fn feedback(&mut self, reply: Reply) {
        match reply {
            Reply::Config(r) if r.status == config::ConfigStatus::Ok => {
                if self.page == 5 || (self.page == 6 && self.choice == 101) {
                    self.page = 3;
                    self.editor = None;
                } else if self.page == 7 {
                    self.page = 4;
                }
            }
            Reply::Config(_) => self.editor_error = "modification refusée pendant un cycle".into(),
            Reply::Action(r)
                if r.status == screen_core::core::ActionStatus::Ok
                    && self.page == 7
                    && self.menu == 3 =>
            {
                self.page = 4
            }
            Reply::Action(r) if r.status != screen_core::core::ActionStatus::Ok => {
                self.editor_error = "commande indisponible".into()
            }
            _ => {}
        }
    }
    /// Rafraîchit les priorités immédiatement, et un seul groupe de valeurs.
    /// Appeler avant chaque rendu, sur le même cœur que Slint.
    pub fn update(
        &mut self,
        app: &AppWindow,
        now_us: u64,
        s: &Snapshot,
        c: &Config,
        provisioning_ssid: &str,
        wall_clock_text: &str,
    ) {
        let cycle = match s.cycle_state {
            CycleState::Purge => Some(2),
            CycleState::Idle => None,
            _ if s.shot.active => Some(1),
            _ => Some(2),
        };
        if let Some(page) = cycle {
            self.page = page;
        } else if matches!(self.page, 1 | 2) {
            self.page = 0;
        }
        app.set_page(self.page);
        app.set_settings_page(self.settings_page as i32);
        self.fullscreen(app, s, provisioning_ssid);
        let idle_s = now_us.saturating_sub(self.last_touch_us) / 1_000_000;
        let dim = !s.cycle_state.is_active()
            && !s.flash_active
            && !s.provisioning
            && idle_s >= c.dim_after_s as u64;
        app.set_dimmed(dim);
        app.set_standby(dim && idle_s >= c.standby_after_s as u64);
        app.set_standby_offset(((idle_s % 120) as i32 - 60).abs() - 30);
        app.set_confirm_visible(self.confirm);
        app.set_confirm_title("réinitialiser le réseau".into());
        app.set_confirm_body("effacer les identifiants wifi enregistrés ?".into());
        match self.refresh.due(now_us) {
            Some(0) => self.status(app, s, c, wall_clock_text),
            Some(1) => self.content(app, s, c),
            Some(2) => self.diagnostics(app, s, c),
            Some(3) => self.settings(app, c),
            _ => {}
        }
    }
    fn fullscreen(&mut self, app: &AppWindow, s: &Snapshot, ssid: &str) {
        let (title, body, fault, progress) = if s.lockout {
            ("verrou de sécurité", "la pompe est restée en marche plus de 60 s. couper la machine à l'interrupteur principal, puis la rallumer".into(),true,false)
        } else if !s.sensors_alive && s.screen_uptime_s >= 3 {
            (
                "module interne injoignable",
                "le module interne ne répond plus. les commandes de pompe et de vanne sont coupées"
                    .into(),
                true,
                false,
            )
        } else if s.flash_active {
            (
                "mise à jour",
                format!(
                    "{} · {} / {} octets\nne pas couper la machine",
                    match s.flash_target {
                        screen_core::snapshot::FlashTarget::Screen => "écran",
                        screen_core::snapshot::FlashTarget::Sensors => "capteurs",
                        _ => "firmware",
                    },
                    s.flash_bytes_done,
                    s.flash_bytes_total
                ),
                false,
                true,
            )
        } else if s.screen_uptime_s < 2 {
            ("coffeeflow", String::new(), false, false)
        } else if s.provisioning {
            (
                "configuration wifi",
                format!("connectez-vous au réseau {ssid}\net entrez vos identifiants"),
                false,
                false,
            )
        } else {
            ("", String::new(), false, false)
        };
        let visible = !title.is_empty();
        if visible && !s.cycle_state.is_active() {
            self.page = 0;
        }
        app.set_full_visible(visible);
        app.set_full_title(title.into());
        app.set_full_body(body.into());
        app.set_full_fault(fault);
        app.set_full_progress(progress);
        if progress {
            app.set_progress(s.flash_bytes_done as f32 / s.flash_bytes_total.max(1) as f32);
        }
    }
    fn status(&self, app: &AppWindow, s: &Snapshot, c: &Config, clock: &str) {
        app.set_clock(if s.time_known {
            clock.into()
        } else {
            "".into()
        });
        app.set_weight(if s.scale_present {
            number(s.weight_g, 1, " g").into()
        } else {
            SharedString::default()
        });
        app.set_pressure(if s.pressure_valid {
            number(s.pressure_bar, 1, " bar").into()
        } else {
            "-".into()
        });
        let cold = s.boiler_temperature_c < 50.0;
        let temp = if cold {
            s.boiler_sensor_temperature_c.min(50.0)
        } else {
            s.boiler_temperature_c
        };
        app.set_temperature(if s.boiler_temperature_valid {
            number(temp, 1, "°").into()
        } else {
            "-".into()
        });
        let color = if !s.boiler_temperature_valid {
            0x6B615A
        } else if cold {
            0xA485D0
        } else if s.boiler_temperature_c > c.brew_temperature_c + 1.0 {
            0xB9412F
        } else if s.brew_temperature_ready {
            0x4CAF50
        } else if s.boiler_temperature_c >= c.brew_temperature_c - 5.0 {
            0x849B82
        } else {
            0x4A9BB5
        };
        let color = hex(color);
        app.set_temperature_color(if s.boiler_temperature_freshness == Freshness::Stale {
            Color::from_argb_u8(128, color.red(), color.green(), color.blue())
        } else {
            color
        });
        app.set_heating((s.heating_power_pct / 100.0).clamp(0.0, 1.0));
        app.set_wifi_enabled(s.wifi_enabled);
        app.set_wifi_connected(
            s.network_state == screen_core::snapshot::NetworkState::StaConnected,
        );
    }
    fn content(&self, app: &AppWindow, s: &Snapshot, c: &Config) {
        app.set_target(if s.scale_present {
            number(c.target_weight_g, 1, " g").into()
        } else {
            format!("{} s", c.target_time_s).into()
        });
        app.set_brew_label(if s.scale_present {
            format!("infuser · {:.0} g", c.target_weight_g).into()
        } else {
            format!("infuser · {} s", c.target_time_s).into()
        });
        app.set_brew_enabled(
            s.sensors_alive
                && !s.lockout
                && s.dimmer_ready
                && s.dimmer_valid
                && c.heating_enabled
                && s.boiler_temperature_valid
                && s.boiler_temperature_freshness == Freshness::Fresh
                && s.heating_power_capable
                && s.heating_freshness == Freshness::Fresh
                && (s.boiler_temperature_c - c.brew_temperature_c).abs() <= 1.0
                && s.brew_temperature_ready
                && !s.capture_cooldown,
        );
        app.set_minus_enabled(if s.scale_present {
            c.target_weight_g > 10.0
        } else {
            c.target_time_s > 5
        });
        app.set_plus_enabled(if s.scale_present {
            c.target_weight_g < 100.0
        } else {
            c.target_time_s < 60
        });
        app.set_warning(
            if s.dimmer_error_active {
                "erreur dimmer"
            } else if !s.dimmer_ready {
                "dimmer en calibration — vérifier le secteur"
            } else if !c.heating_enabled {
                "chauffage désactivé"
            } else if !s.brew_temperature_ready {
                "chaudière en chauffe"
            } else {
                ""
            }
            .into(),
        );
        app.set_detail(if s.scale_present {
            format!("cible · pré-infusion {} s · rampe", c.preinfusion_time_s).into()
        } else {
            "cible temps · balance absente".into()
        });
        app.set_finished(s.cycle_state == CycleState::Finished);
        app.set_close_enabled(!s.capture_cooldown);
        app.set_close_label(if s.capture_cooldown {
            format!(
                "fermer · {} s",
                s.capture_cooldown_remaining_ms.div_ceil(1000)
            )
            .into()
        } else {
            "fermer".into()
        });
        let phase = match s.cycle_state {
            CycleState::ThermalPreheat => "pré-chauffe",
            CycleState::Filling => "remplissage",
            CycleState::Preinfusion => "pré-infusion",
            CycleState::Rampdown => "rampe",
            CycleState::Finished if s.capture_cooldown => {
                if s.shot.drip_done {
                    "écoulement"
                } else {
                    "dernières gouttes"
                }
            }
            CycleState::Finished => "terminé",
            _ => "infusion",
        };
        app.set_phase(if s.cycle_state == CycleState::Purge {
            "purge".into()
        } else {
            phase.into()
        });
        app.set_started(shot_date_text(s.shot_start_unix_s).into());
        app.set_purge_time(
            if s.cycle_state == CycleState::Finished && s.last_shot_available {
                number(s.last_shot_weight_g, 1, " g").into()
            } else if s.cycle_weight_goal && s.cycle_state != CycleState::Purge {
                number(s.weight_g - s.cycle_start_weight_g, 1, " g").into()
            } else {
                number(s.cycle_elapsed_ms as f32 / 1000.0, 0, " s").into()
            },
        );
        app.set_legacy_split(
            s.cycle_weight_goal
                && s.cycle_state != CycleState::Purge
                && s.cycle_state != CycleState::Finished,
        );
        app.set_legacy_time(format!("{} s", s.cycle_elapsed_ms / 1000).into());
        app.set_purge_detail(
            format!(
                "{} · {}",
                number(s.flow_ml_s, 2, " ml/s"),
                number(s.pressure_bar, 1, " bar")
            )
            .into(),
        );
        if s.cycle_state == CycleState::Purge {
            app.set_progress(
                (s.cycle_elapsed_ms as f32 / (c.purge_max_s.max(1) as f32 * 1000.0))
                    .clamp(0.0, 1.0),
            );
        }
        if !s.shot.active && s.cycle_state != CycleState::Idle && s.cycle_state != CycleState::Purge
        {
            let progress = if s.cycle_state == CycleState::Finished {
                1.0
            } else if s.cycle_weight_goal {
                (s.weight_g - s.cycle_start_weight_g) / c.target_weight_g
            } else {
                s.cycle_elapsed_ms as f32 / (c.target_time_s.max(1) as f32 * 1000.0)
            };
            app.set_progress(progress.clamp(0.0, 1.0));
        }
        let shot = &s.shot;
        let total = if shot.pump_stopped() {
            shot.pump_stop_s
        } else {
            shot.elapsed_s
        };
        app.set_brew_values(model(alloc::vec![
            tile("poids", number(shot.cup_weight_g, 1, " g"), "", true, 0),
            tile(
                "temps infusion",
                number(shot.infusion_s, 1, " s"),
                "",
                true,
                0
            ),
            tile(
                "moyenne en tasse",
                number(shot.cup_mean_c, 1, "°"),
                "",
                true,
                0
            ),
            tile("temps total", number(total, 1, " s"), "", true, 0),
            tile(
                "débit infusion",
                number(shot.infusion_cup_g_s, 2, " g/s"),
                "",
                true,
                0
            ),
            tile(
                "baisse thermique",
                number(
                    -(shot.start_temperature_c - shot.min_temperature_c).max(0.0),
                    1,
                    "°"
                ),
                "",
                true,
                0
            )
        ]));
        app.set_live_values(model(alloc::vec![
            tile("chauffe", number(s.heating_power_pct, 0, " %"), "", true, 0),
            tile("pompe", format!("{} %", s.dimmer_pct), "", true, 0),
            tile(
                "débit balance",
                number(shot.cup_rate_g_s, 2, " g/s"),
                "",
                true,
                0
            ),
            tile("débit pompe", number(s.flow_ml_s, 2, " ml/s"), "", true, 0)
        ]));
        let names = [
            "PRÉ-CHAUFFE",
            "REMPLISSAGE",
            "PRÉ-INFUSION",
            "MONTÉE",
            "INFUSION",
            "GOUTTES",
        ];
        let mut segments = Vec::new();
        let scale = 736.0 / shot.axis_s.max(1.0);
        for (i, name) in names.iter().enumerate() {
            let start = shot.segment_start_s[i];
            let end = shot.segment_end_s[i];
            if !start.is_finite() || !end.is_finite() || end <= start {
                continue;
            }
            let x = (start * scale).clamp(0.0, 734.0);
            let width = ((end * scale).min(736.0) - x - 2.0).max(2.0);
            segments.push(Segment {
                x,
                width,
                name: if width > name.len() as f32 * 9.0 {
                    (*name).into()
                } else {
                    "".into()
                },
                value: if width > 40.0 {
                    number(end - start, 1, "").into()
                } else {
                    "".into()
                },
                tone: i as i32,
                active: s.cycle_state.is_active() && (end - shot.elapsed_s).abs() < 0.3,
            });
        }
        app.set_segments(model(segments));
    }
    fn diagnostics(&self, app: &AppWindow, s: &Snapshot, c: &Config) {
        app.set_diagnostics(model(alloc::vec![
            tile(
                "pression",
                number(s.pressure_bar, 1, " bar"),
                freshness(s.pressure_freshness),
                true,
                0
            ),
            tile(
                "chaudière",
                number(s.boiler_temperature_c, 1, "°"),
                freshness(s.boiler_temperature_freshness),
                true,
                0
            ),
            tile(
                "débit",
                number(s.flow_ml_s, 2, " ml/s"),
                freshness(s.flow_freshness),
                true,
                0
            ),
            tile(
                "pompe",
                format!("{} %", s.dimmer_pct),
                if s.dimmer_ready {
                    "valide"
                } else {
                    "calibration"
                },
                true,
                0
            ),
            tile(
                "vanne",
                if s.valve_open { "ouverte" } else { "fermée" }.into(),
                freshness(s.actuators_freshness),
                true,
                0
            ),
            tile(
                "chauffage",
                number(s.heating_power_pct, 0, " %"),
                if c.heating_enabled {
                    "activé"
                } else {
                    "désactivé"
                },
                true,
                0
            ),
            tile(
                "bus can",
                if s.sensors_alive { "OK" } else { "ERREUR" }.into(),
                "",
                true,
                if s.sensors_alive { 1 } else { 2 }
            ),
            tile(
                "balance",
                if s.scale_present {
                    "présente"
                } else {
                    "absente"
                }
                .into(),
                "",
                true,
                0
            ),
            tile(
                "versions",
                format!(
                    "0.4.0 / {}.{}.{}",
                    s.sensors_version_major, s.sensors_version_minor, s.sensors_version_patch
                ),
                "écran / sensors",
                true,
                0
            )
        ]));
        match self.menu {
            3 => {
                app.set_menu_title("dimmerlink".into());
                app.set_menu_body(if self.editor_error.is_empty() {
                    "la pompe est arrêtée avant la commande".into()
                } else {
                    self.editor_error.clone().into()
                });
                app.set_menu_hint("".into());
                app.set_menu_left("reset dimmerlink".into());
                app.set_menu_right("calibrer".into());
                app.set_menu_left_destructive(true);
                app.set_menu_left_enabled(true);
                app.set_menu_right_enabled(true);
            }
            4 => {
                app.set_menu_title("vanne · maintenance".into());
                app.set_menu_body(if !s.maintenance_valve_capable {
                    "module capteurs à mettre à jour".into()
                } else if s.maintenance_valve_open {
                    format!(
                        "vanne ouverte · fermeture dans {} s",
                        s.lease_remaining_ms.div_ceil(1000)
                    )
                    .into()
                } else if c.heating_enabled {
                    "désactiver le chauffage avant d'ouvrir".into()
                } else {
                    "vanne fermée · pompe arrêtée".into()
                });
                app.set_menu_hint(
                    "pompe arrêtée · ouvrir la buse vapeur pour laisser entrer l'air".into(),
                );
                app.set_menu_left(
                    if s.maintenance_valve_open {
                        "relancer 30 s"
                    } else {
                        "ouvrir 30 s"
                    }
                    .into(),
                );
                app.set_menu_right("fermer la vanne".into());
                app.set_menu_left_enabled(
                    s.sensors_alive
                        && !s.lockout
                        && s.maintenance_valve_capable
                        && !c.heating_enabled,
                );
                app.set_menu_right_enabled(s.maintenance_valve_open);
                app.set_menu_left_destructive(false);
            }
            5 => {
                app.set_menu_title("chauffage".into());
                app.set_menu_body(
                    if c.heating_enabled {
                        "chauffage actuellement activé"
                    } else {
                        "chauffage actuellement désactivé"
                    }
                    .into(),
                );
                app.set_menu_hint("".into());
                app.set_menu_left("activer".into());
                app.set_menu_right("désactiver".into());
                app.set_menu_left_enabled(!c.heating_enabled);
                app.set_menu_right_enabled(c.heating_enabled);
                app.set_menu_left_destructive(false);
            }
            _ => {}
        }
    }
    fn settings(&self, app: &AppWindow, c: &Config) {
        let mut tiles = Vec::new();
        for &e in &EDITS[self.settings_page] {
            let t = match e {
                0..=15 => tile(
                    NAMES[e],
                    number(
                        value(c, e),
                        if decimal(e) { 1 } else { 0 },
                        &format!(" {}", UNITS[e]),
                    ),
                    "",
                    true,
                    0,
                ),
                100 => tile(
                    "critères pré-inf.",
                    match c.preinfusion_mode.0 {
                        1 => "temps",
                        2 => "poids",
                        3 => "temps + poids",
                        _ => "aucune",
                    }
                    .into(),
                    "",
                    true,
                    0,
                ),
                101 => tile(
                    "stratégie rampe",
                    match c.rampdown_mode {
                        RampdownMode::None => "aucune",
                        RampdownMode::Time => "temps",
                        RampdownMode::Weight => "poids",
                        RampdownMode::PressureDrop => "chute pression",
                    }
                    .into(),
                    "",
                    true,
                    0,
                ),
                102 => tile("réinitialiser réseau", "effacer".into(), "", true, 2),
                103 => tile("calibrations", "depuis /config".into(), "", false, 0),
                104 => tile("réinitialiser LCD", "redémarrer".into(), "", true, 0),
                105 => tile("veille", "automatique".into(), "", false, 0),
                _ => continue,
            };
            tiles.push(t);
        }
        app.set_settings(model(tiles));
        if let Some(e) = self.editor {
            app.set_editor_title(NAMES[e].into());
            app.set_editor_unit(UNITS[e].into());
            app.set_editor_decimal(decimal(e));
            app.set_editor_value(self.editor_value.clone().into());
            app.set_editor_error(self.editor_error.clone().into());
        }
        if self.page == 6 {
            app.set_editor_title(
                if self.choice == 100 {
                    "pré-infusion"
                } else {
                    "stratégie rampe"
                }
                .into(),
            );
            let names: &[&str] = if self.choice == 100 {
                &["temps", "poids"]
            } else {
                &["aucune", "temps", "poids", "chute pression"]
            };
            let choices = names
                .iter()
                .enumerate()
                .map(|(i, n)| {
                    tile(
                        n,
                        String::new(),
                        "",
                        true,
                        if if self.choice == 100 {
                            c.preinfusion_mode.0 & (1 << i) != 0
                        } else {
                            c.rampdown_mode as usize == i
                        } {
                            1
                        } else {
                            0
                        },
                    )
                })
                .collect();
            app.set_choices(model(choices));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn ready() -> Snapshot {
        Snapshot {
            sensors_alive: true,
            screen_uptime_s: 5,
            ..Snapshot::default()
        }
    }
    #[test]
    fn groups_stagger_without_catch_up() {
        let mut r = Refresh::default();
        assert_eq!(r.due(0), Some(0));
        assert_eq!(r.due(62_499), None);
        assert_eq!(r.due(62_500), Some(1));
        assert_eq!(r.due(500_000), Some(2));
        assert_eq!(r.due(500_000), None);
        assert_eq!(r.due(562_500), Some(3));
        assert_eq!(r.due(625_000), Some(0));
    }
    #[test]
    fn keypad_replaces_initial_value_and_validates_bounds() {
        let mut p = Presenter::default();
        let s = ready();
        let c = Config::default();
        p.command(5, "", 0, &s, &c);
        p.command(14, ",", 0, &s, &c);
        assert_eq!(p.editor_value, "28");
        p.command(14, "9", 0, &s, &c);
        p.command(14, "9", 0, &s, &c);
        assert!(p.command(15, "", 0, &s, &c).is_none());
        assert_eq!(p.editor_error, "valeur hors limites");
        p.command(14, "back", 0, &s, &c);
        assert!(matches!(
            p.command(15, "", 0, &s, &c),
            Some(UiRequest::Core(Input::PutConfig(Config {
                target_time_s: 9,
                ..
            })))
        ));
    }
    #[test]
    fn integer_overflow_cannot_wrap_to_valid_pump() {
        let mut p = Presenter::default();
        let s = ready();
        let c = Config::default();
        p.settings_page = 0;
        p.command(13, "1", 0, &s, &c);
        for d in ["3", "5", "6"] {
            p.command(14, d, 0, &s, &c);
        }
        assert!(p.command(15, "", 0, &s, &c).is_none());
    }
    #[test]
    fn release_survives_preemption() {
        let mut p = Presenter::default();
        let mut s = ready();
        s.lockout = true;
        assert_eq!(
            p.command(3, "", 0, &s, &Config::default()),
            Some(core_action(Action::PurgeRelease))
        );
        assert!(p.command(1, "", 0, &s, &Config::default()).is_none());
    }
    #[test]
    fn wifi_is_session_input_and_never_navigation() {
        let mut p = Presenter {
            page: 3,
            ..Presenter::default()
        };
        let s = ready();
        assert_eq!(
            p.command(11, "", 0, &s, &Config::default()),
            Some(UiRequest::Core(Input::WifiEnabled(false)))
        );
        assert_eq!(p.page, 3);
    }
    #[test]
    fn active_cycle_only_accepts_stop_and_release() {
        let mut p = Presenter::default();
        let mut s = ready();
        s.cycle_state = CycleState::Brew;
        assert!(p.command(8, "", 0, &s, &Config::default()).is_none());
        assert_eq!(
            p.command(9, "", 0, &s, &Config::default()),
            Some(core_action(Action::StopBrew))
        );
    }
}

#[cfg(test)]
mod clock_tests {
    use super::*;
    #[test]
    fn cet_cest_boundaries_and_optional_clock() {
        assert_eq!(clock_text(None), "");
        let spring = days_from_civil(2026, 3, 29) * 86400 + 3600;
        assert_eq!(clock_text(Some(spring - 60)), "01:59");
        assert_eq!(clock_text(Some(spring)), "03:00");
        let autumn = days_from_civil(2026, 10, 25) * 86400 + 3600;
        assert_eq!(clock_text(Some(autumn - 60)), "02:59");
        assert_eq!(clock_text(Some(autumn)), "02:00");
        assert_eq!(
            shot_date_text(days_from_civil(2026, 12, 31) * 86400 + 23 * 3600),
            "01/01 · 00:00"
        );
    }
}
