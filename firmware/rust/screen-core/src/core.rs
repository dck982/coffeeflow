//! Le cœur de l'écran : une machine à états sans matériel.
//!
//! # Frontière
//!
//! [`Core`] possède tout l'état logique (instantané, cycle d'infusion,
//! régulation de chauffe, capture HF, présence CAN, configuration). Il n'a ni
//! tâche, ni verrou, ni horloge propre :
//!
//! - **Entrées** : des appels de méthode (ou [`Core::handle`] avec une
//!   [`Input`]) qui portent un [`Clock`] (horloge monotone en µs, heure
//!   murale facultative). Trame CAN reçue, pesée ou échantillon SCACE BLE,
//!   codes ADS1115, appui tactile, action de l'UI ou de HTTP, pas du
//!   minuteur ([`Core::tick`], toutes les 50 ms), état du réseau.
//! - **Sorties** : [`Core::drain_outputs`] rend les [`Output`] produites
//!   depuis le dernier appel : trames CAN à émettre (l'appelant les envoie sur
//!   le TWAI, les recopie sur le pont série et les publie sur le WebSocket,
//!   comme `can_link::send_message` du C++), commandes BLE, demandes de
//!   radio, oubli du réseau, trames FLASH_CTRL des capteurs. L'état se lit par
//!   [`Core::snapshot`].
//! - **Configuration** : synchrone, via le [`ConfigStore`] possédé par le
//!   cœur (écriture NVS), pour que `/config` puisse répondre `500 storage`
//!   sans avoir rien changé.
//!
//! Un seul propriétaire sérialise les entrées et vide les sorties après chaque
//! appel. Les services matériels transmettent leurs événements au propriétaire
//! plutôt que de tenir un verrou pendant une écriture flash.
//!
//! ```
//! use screen_core::config::MemoryConfigStore;
//! use screen_core::core::{Clock, Core, Input, Output};
//!
//! let mut core = Core::new(MemoryConfigStore::default(), Clock::new(1_000_000));
//! // La plateforme fournit l'état réellement lu en NVS.
//! core.handle(Clock::new(1_000_000), Input::CredentialsPresent(false));
//! core.handle(Clock::new(1_050_000), Input::Tick);
//! let snapshot = core.snapshot(Clock::new(1_050_000));
//! assert!(snapshot.provisioning);
//! assert!(!snapshot.ble_enabled);
//! assert!(core.drain_outputs().iter().any(|out| matches!(out, Output::Radio(_))));
//! ```

use alloc::vec::Vec;

use protocol::messages::{
    ConfirmSensorsOtaPayload, DimmerCommand, DimmerCommandPayload, FlashCtrlPayload, LogPayload,
    MaintenanceValvePayload, NodeIdentityPayload, ReqStatusPayload, SetHeatingPayload,
    SetHeatingPowerPayload, SetPayload,
};
use protocol::{
    Dest, FIRMWARE_VERSION, Frame, LogCode, LogSeverity, Message, MessageType, Node,
    heating_protocol_supported,
};

use crate::calibration;
use crate::can_link::{self, Inbound, OtaConfirmState, Presence};
use crate::config::{
    BREW_TEMPERATURE_TOLERANCE_C, Config, ConfigManager, ConfigResult, ConfigStatus, ConfigStore,
    LoadReport,
};
use crate::events::{EventKind, EventLog};
use crate::hf_capture::{self, Capture, Origin, SampleMode};
use crate::machine::{self, Machine};
use crate::ntc;
use crate::sample_schedule::SampleSchedule;
use crate::scace;
use crate::shot_summary::{ShotSample, ShotSampleMode, ShotSummaryBuilder};
use crate::snapshot::{CycleState, FlashTarget, Freshness, NetworkState, Snapshot};
use crate::thermal::{self, Controller, Flow};

// --- Constantes -----------------------------------------------------------------

/// Lissage du débitmètre Digmesa : nombre d'impulsions de la fenêtre.
const FLOW_WINDOW_PULSES: u32 = 10;
const FLOW_SILENCE_MS: i64 = 3000;
/// Pas de la tâche de télémétrie.
pub const TELEMETRY_TICK_MS: u32 = 50;
const SCALE_PRESENT_MS: i64 = 2000;
const HF_CAPTURE_SCHEDULE: SampleSchedule = SampleSchedule::new(
    hf_capture::PERIOD_MS as i64 * 1000,
    TELEMETRY_TICK_MS as i64 * 1000 / 2,
);
/// Période de la paire NTC/A2 de l'ADS1115.
const BOILER_PAIR_PERIOD_MS: i64 = 100;
const FLOW_HISTORY_CAPACITY: usize = 16;
/// Largeur d'une commande de chauffe, en ms.
const HEATING_POWER_LEASE_MS: u16 = 1500;
const HEATING_STATUS_PERIOD_MS: u16 = 250;
/// Vie maximale d'une demande `set_heating`.
pub const SET_HEATING_MAX_MS: u16 = 30000;

// --- Types publics ----------------------------------------------------------------

/// Heure fournie à chaque appel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Clock {
    /// Horloge monotone en microsecondes depuis le boot : tout ce qui mesure
    /// une durée s'appuie dessus.
    pub mono_us: i64,
    /// Heure murale en secondes Unix, `None` tant que l'heure n'est pas
    /// connue (SNTP non synchronisé).
    pub wall_unix_s: Option<i64>,
}

impl Clock {
    pub const fn new(mono_us: i64) -> Self {
        Self {
            mono_us,
            wall_unix_s: None,
        }
    }

    pub const fn with_wall(mono_us: i64, wall_unix_s: i64) -> Self {
        Self {
            mono_us,
            wall_unix_s: Some(wall_unix_s),
        }
    }
}

/// Origine d'une action : HTTP ne peut jamais lancer d'infusion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionSource {
    Ui,
    Http,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TelemetryProfile {
    Idle,
    Active,
    Suspended,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    SetActuators,
    SetBrewActuators,
    SetHeating,
    StartBrew,
    StopBrew,
    ResetSensors,
    ResetDimmer,
    RecalibrateDimmer,
    PurgePress,
    PurgeRelease,
    Tare,
    DismissSummary,
    StartFlash,
    /// Vidange chaudière : vanne seule, pompe à 0, pour 30 s au plus. Exige le
    /// chauffage désactivé ; les capteurs coupent aussi la chauffe pendant
    /// l'ouverture.
    OpenMaintenanceValve,
    CloseMaintenanceValve,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionStatus {
    Ok,
    Unavailable,
    BusLost,
    Locked,
    DimmerNotReady,
    CycleActive,
    NoCycle,
    InvalidValue,
}

impl ActionStatus {
    /// Texte du champ `reason` de la réponse `/action`.
    pub const fn reason(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Unavailable => "unavailable",
            Self::BusLost => "bus_lost",
            Self::Locked => "locked",
            Self::DimmerNotReady => "dimmer_not_ready",
            Self::CycleActive => "cycle_active",
            Self::NoCycle => "no_cycle",
            Self::InvalidValue => "invalid_value",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BrewActuatorsCommand {
    pub pump_pct: u8,
    pub ttl_ms: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HeatingCommand {
    pub on: bool,
    pub duration_ms: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionCommand {
    pub action: Action,
    pub brew: BrewActuatorsCommand,
    pub heating: HeatingCommand,
}

impl ActionCommand {
    pub const fn new(action: Action) -> Self {
        Self {
            action,
            brew: BrewActuatorsCommand {
                pump_pct: 0,
                ttl_ms: 0,
            },
            heating: HeatingCommand {
                on: false,
                duration_ms: 0,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionResult {
    pub status: ActionStatus,
    pub reason: &'static str,
}

impl ActionResult {
    pub const fn of(status: ActionStatus) -> Self {
        Self {
            status,
            reason: status.reason(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticStatus {
    Ok,
    BusLost,
    DimmerNotReady,
    Locked,
}

/// Commande destinée à la tâche BLE.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BleCommand {
    /// Commande Acaia TARE (écrire [`crate::acaia::tare_frame`]).
    Tare,
    /// Pendant un cycle ou une capture, publier chaque pesée
    /// (`Session::set_active`) ; sinon une par seconde.
    SetActive(bool),
    /// `scace.enabled` est faux : couper la connexion à la sonde.
    DisconnectScace,
}

/// État Wi-Fi voulu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WifiRequest {
    /// Radio Wi-Fi arrêtée (bascule des réglages sur OFF).
    Off,
    /// Station, identifiants en NVS. Une panne ne relance jamais l'AP.
    Sta,
    /// Point d'accès de provisioning (aucun identifiant en NVS).
    Provisioning,
}

/// État radio voulu : Wi-Fi et BLE tournent ensemble, sauf en provisioning où
/// le BLE est coupé.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RadioRequest {
    pub wifi: WifiRequest,
    pub ble: bool,
}

/// Sorties du cœur, à exécuter par l'appelant dans l'ordre.
#[derive(Debug, Clone, PartialEq)]
pub enum Output {
    /// Trame CAN à émettre sur le bus ; recopier aussi sur le pont série et
    /// le WebSocket.
    Can(Frame),
    Ble(BleCommand),
    /// L'état radio voulu a changé.
    Radio(RadioRequest),
    /// Effacer les identifiants Wi-Fi, puis appeler
    /// [`Core::set_credentials_present`]`(false)`.
    ForgetNetwork,
    /// Acquittement des capteurs pour la mise à jour OTA proxy (FLASH_CTRL).
    SensorsFlashCtrl(FlashCtrlPayload),
}

/// Entrées, pour une boucle d'événements. Chaque variante correspond à la
/// méthode du même nom de [`Core`].
#[derive(Debug, Clone, PartialEq)]
pub enum Input {
    Tick,
    CanFrame(Frame),
    ScaleConnection(bool),
    ScaleWeight(f32),
    ScaceConnection(bool),
    ScaceSample(scace::Sample),
    BoilerNtc {
        a0_raw: i16,
        a1_raw: i16,
        read_ok: bool,
    },
    PressureA2 {
        raw: i16,
        read_ok: bool,
    },
    TouchReady(bool),
    TouchPress,
    Network {
        state: NetworkState,
        ipv4_address: u32,
    },
    BootTimeSyncing(bool),
    CredentialsPresent(bool),
    WifiEnabled(bool),
    HeapStats {
        free: u32,
        largest: u32,
        minimum: u32,
    },
    Action {
        command: ActionCommand,
        source: ActionSource,
    },
    PutConfig(Config),
    ResetConfig,
    ForgetNetwork,
    Event(EventKind),
}

/// Réponse synchrone d'une entrée.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Reply {
    None,
    Action(ActionResult),
    Config(ConfigResult),
}

/// Début de mise à jour : voir [`Core::begin_flash`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlashStart {
    Refused,
    Started,
    /// Cible capteurs : un STOP est parti. Appeler [`Core::poll_flash_stop`]
    /// toutes les 25 ms, 20 fois au plus, puis
    /// [`Core::cancel_pending_flash`].
    AwaitingStop,
}

#[derive(Debug, Clone, Copy, Default)]
struct LastShot {
    available: bool,
    weight_g: f32,
    duration_ms: u32,
    flow_ml_s: f32,
    unix_s: i64,
}

#[derive(Debug, Clone, Copy)]
struct FlowSample {
    pulses: u32,
    edge_ms: u16,
}

#[derive(Debug, Clone, Copy)]
struct PendingFlash {
    target: FlashTarget,
    total: u32,
    old_actuators_received_us: i64,
    old_heating_received_us: i64,
}

#[derive(Debug, Clone, Copy, Default)]
struct Timestamps {
    pressure_received_us: i64,
    boiler_received_us: i64,
    pressure_last_valid_us: i64,
    flow_received_us: i64,
    actuators_received_us: i64,
    heating_received_us: i64,
    heating_deadline_us: i64,
    last_sensors_message_us: i64,
    last_flow_edge_received_us: i64,
    scale_received_us: i64,
    /// Dernière trame SCACE avec température, 0 sinon.
    scace_received_us: i64,
}

struct Periods {
    flow: u16,
    actuators: u16,
}

const fn periods_for(profile: TelemetryProfile) -> Periods {
    match profile {
        TelemetryProfile::Idle => Periods {
            flow: 1000,
            actuators: 1000,
        },
        TelemetryProfile::Active => Periods {
            flow: 100,
            actuators: 200,
        },
        TelemetryProfile::Suspended => Periods {
            flow: 0,
            actuators: 0,
        },
    }
}

fn age_ms(received_us: i64, now: i64) -> u32 {
    if received_us == 0 {
        u32::MAX
    } else {
        ((now - received_us) / 1000) as u32
    }
}

fn freshness(received_us: i64, period_ms: u16, now: i64) -> Freshness {
    if received_us == 0 || now - received_us > 3 * 1000 * 1000 {
        return Freshness::Missing;
    }
    if now - received_us > i64::from(2 * u32::from(period_ms)) * 1000 {
        Freshness::Stale
    } else {
        Freshness::Fresh
    }
}

fn cycle_state(state: machine::State) -> CycleState {
    match state {
        machine::State::Idle => CycleState::Idle,
        machine::State::ThermalPreheat => CycleState::ThermalPreheat,
        machine::State::Filling => CycleState::Filling,
        machine::State::Preinfusion => CycleState::Preinfusion,
        machine::State::Brew => CycleState::Brew,
        machine::State::Rampdown => CycleState::Rampdown,
        machine::State::Finished => CycleState::Finished,
        machine::State::Purge => CycleState::Purge,
    }
}

fn sample_mode(state: machine::State) -> SampleMode {
    match state {
        machine::State::ThermalPreheat => SampleMode::ThermalPreheat,
        machine::State::Filling => SampleMode::Filling,
        machine::State::Preinfusion => SampleMode::Preinfusion,
        machine::State::Brew => SampleMode::Infusion,
        machine::State::Rampdown => SampleMode::RampDown,
        _ => SampleMode::Purge,
    }
}

fn shot_sample(sample: &hf_capture::Sample) -> ShotSample {
    ShotSample {
        t_s: sample.t_ms as f32 / 1000.0,
        mode: match sample.mode {
            SampleMode::ThermalPreheat => ShotSampleMode::Preheat,
            SampleMode::Filling => ShotSampleMode::Filling,
            SampleMode::Preinfusion => ShotSampleMode::Preinfusion,
            SampleMode::Infusion | SampleMode::RampDown => ShotSampleMode::Infusion,
            SampleMode::Cooldown => ShotSampleMode::Cooldown,
            SampleMode::Purge => ShotSampleMode::Other,
        },
        pump_pct: sample.pump_pct_reported,
        pressure_valid: sample.flags & hf_capture::flags::PRESSURE_VALID != 0,
        pressure_bar: sample.pressure_bar,
        scale_present: sample.flags & hf_capture::flags::SCALE_PRESENT != 0,
        weight_g: sample.weight_g,
        volume_ml: sample.volume_ml,
        temperature_valid: sample.flags & hf_capture::flags::BOILER_VALID != 0,
        temperature_c: sample.boiler_temperature_c,
    }
}

/// Identifiant numérique d'un champ de configuration dans l'`arg16` du LOG
/// `CONFIG_REJECTED`.
fn config_field_arg(field: Option<&str>) -> u16 {
    const FIELDS: &[(&str, u16)] = &[
        ("version", 1),
        ("brew.target_weight_g", 2),
        ("brew.target_time_s", 3),
        ("brew.pump_pct", 4),
        ("preinfusion.mode", 5),
        ("preinfusion.time_s", 6),
        ("preinfusion.pump_pct", 8),
        ("rampdown.mode", 9),
        ("rampdown.lead_time_s", 10),
        ("rampdown.lead_weight_g", 11),
        ("rampdown.pressure_drop_bar", 12),
        ("purge.pump_pct", 13),
        ("purge.max_s", 14),
        ("ui.dim_after_s", 15),
        ("ui.standby_after_s", 16),
        ("profiles", 17),
        ("brew", 18),
        ("preinfusion", 19),
        ("rampdown", 20),
        ("purge", 21),
        ("ui", 22),
        ("filling.time_s", 23),
        ("filling.pressure_bar", 24),
        ("filling.pump_pct", 25),
        ("filling", 26),
        ("brew.target_pressure_bar", 27),
        ("heating.enabled", 28),
        ("heating.brew_temperature_c", 29),
        ("heating", 30),
        ("heating.brew_preheat_time_s", 31),
    ];
    let Some(field) = field else {
        return 0;
    };
    FIELDS
        .iter()
        .find(|(name, _)| *name == field)
        .map_or(0, |(_, id)| *id)
}

// --- Le cœur ------------------------------------------------------------------------

/// Voir la documentation du module.
pub struct Core<S: ConfigStore> {
    /// Stockage NVS de la configuration.
    pub store: S,
    /// Journal des derniers événements (page de service).
    pub events: EventLog,
    config: ConfigManager,
    load_report: LoadReport,
    clock: Clock,
    snapshot: Snapshot,
    t: Timestamps,
    thermal: Controller,
    last_heating_power_send_us: i64,
    last_thermal_step_us: i64,
    last_heating_power_sent: u16,
    flow_history: Vec<FlowSample>,
    profile: TelemetryProfile,
    profile_dirty: bool,
    last_request_us: i64,
    machine: Machine,
    last_cycle_set_us: i64,
    cycle_set_on: bool,
    shot_start_unix_s: i64,
    shot_summary_pending: bool,
    hf: Capture,
    shot_builder: ShotSummaryBuilder,
    last_shot: LastShot,
    scace_centi_c: i16,
    flash_active: bool,
    pending_flash: Option<PendingFlash>,
    last_ota_confirm_us: i64,
    presence: Presence,
    ble_active: bool,
    scale_writable: bool,
    credentials_present: bool,
    wifi_enabled: bool,
    last_radio: Option<RadioRequest>,
    out: Vec<Output>,
}

impl<S: ConfigStore> Core<S> {
    /// Crée le cœur : charge la configuration (migrations, normalisation),
    /// publie l'événement `Boot` et prépare le tampon de capture HF.
    ///
    /// Les identifiants Wi-Fi ne sont pas connus ici : l'appelant les lit en
    /// NVS puis appelle [`Core::set_credentials_present`], ce qui émet la
    /// première [`Output::Radio`].
    pub fn new(mut store: S, clock: Clock) -> Self {
        let (config, load_report) = ConfigManager::init(&mut store);
        let mut core = Self {
            store,
            events: EventLog::new(),
            config,
            load_report,
            clock,
            snapshot: Snapshot::default(),
            t: Timestamps::default(),
            thermal: Controller::new(),
            last_heating_power_send_us: 0,
            last_thermal_step_us: 0,
            last_heating_power_sent: 0,
            flow_history: Vec::with_capacity(FLOW_HISTORY_CAPACITY),
            profile: TelemetryProfile::Idle,
            profile_dirty: true,
            last_request_us: 0,
            machine: Machine::new(),
            last_cycle_set_us: 0,
            cycle_set_on: false,
            shot_start_unix_s: 0,
            shot_summary_pending: false,
            hf: Capture::new(),
            shot_builder: ShotSummaryBuilder::new(),
            last_shot: LastShot::default(),
            scace_centi_c: 0,
            flash_active: false,
            pending_flash: None,
            last_ota_confirm_us: 0,
            presence: Presence::new(clock.mono_us),
            ble_active: false,
            scale_writable: true,
            credentials_present: true,
            wifi_enabled: true,
            last_radio: None,
            out: Vec::new(),
        };
        core.hf
            .samples
            .reserve_exact(usize::from(hf_capture::CAPACITY));
        core.snapshot.screen_version_major = FIRMWARE_VERSION.major;
        core.snapshot.screen_version_minor = FIRMWARE_VERSION.minor;
        core.snapshot.screen_version_patch = FIRMWARE_VERSION.patch;
        core.snapshot.wifi_enabled = true;
        core.snapshot.ble_enabled = true;
        core.events
            .push(EventKind::Boot, (clock.mono_us / 1000) as u32);
        // Annonce notre identité au boot ; le PONG qui suit valide aussi une
        // image OTA des capteurs en attente.
        core.send_ping();
        core
    }

    /// Ce que le chargement de la configuration a trouvé (migration,
    /// normalisation, échec d'écriture).
    pub fn load_report(&self) -> LoadReport {
        self.load_report
    }

    /// Sorties produites depuis le dernier appel.
    pub fn drain_outputs(&mut self) -> Vec<Output> {
        core::mem::take(&mut self.out)
    }

    fn enter(&mut self, clock: Clock) {
        self.clock = clock;
        let known = clock.wall_unix_s.is_some();
        if known && !self.snapshot.time_known {
            self.events.push(EventKind::TimeKnown, self.uptime_ms());
        }
        self.snapshot.time_known = known;
    }

    fn now(&self) -> i64 {
        self.clock.mono_us
    }

    fn now_ms(&self) -> u64 {
        (self.clock.mono_us / 1000) as u64
    }

    fn uptime_ms(&self) -> u32 {
        (self.clock.mono_us / 1000) as u32
    }

    // --- Émission CAN ---------------------------------------------------------------

    fn emit(&mut self, message: Message) {
        self.out.push(Output::Can(message.encode()));
    }

    fn send_log(&mut self, code: LogCode, severity: LogSeverity, arg16: u16, arg32: u32) {
        let message = Message::Log(LogPayload {
            code: code as u8,
            severity: severity as u8,
            arg16,
            arg32,
        });
        self.out.push(Output::Can(
            message.encode_routed(Node::Screen, Dest::Broadcast),
        ));
    }

    fn identity(&self) -> NodeIdentityPayload {
        can_link::identity(FIRMWARE_VERSION, (self.now() / 1_000_000) as u32)
    }

    fn send_ping(&mut self) {
        let identity = self.identity();
        self.emit(Message::Ping(identity));
    }

    fn send_pong(&mut self) {
        let identity = self.identity();
        self.emit(Message::Pong(identity));
    }

    fn send_request(&mut self, target: MessageType, period_ms: u16) {
        self.emit(Message::ReqStatus(ReqStatusPayload::new(target, period_ms)));
    }

    fn send_set(&mut self, dimmer: u8, ttl_ms: u16, origin: Origin) {
        if dimmer > 0 {
            self.begin_hf_capture(origin, dimmer);
            self.set_telemetry_profile(TelemetryProfile::Active);
        } else {
            self.note_hf_capture_stop_request();
        }
        self.emit(Message::Set(SetPayload {
            pump_pct: dimmer,
            ttl_ms,
        }));
    }

    fn send_maintenance_valve(&mut self, open: bool) {
        // Durée accordée à chaque appui : le plafond protocolaire, sans
        // renouvellement.
        self.emit(Message::MaintenanceValve(MaintenanceValvePayload {
            open,
            duration_ms: if open {
                protocol::messages::MAINTENANCE_VALVE_MAX_MS
            } else {
                0
            },
        }));
    }

    fn send_heating(&mut self, on: bool, duration_ms: u16) {
        self.emit(Message::SetHeating(SetHeatingPayload { on, duration_ms }));
    }

    fn send_heating_power(&mut self, power_permille: u16, restart_window: bool) {
        self.emit(Message::SetHeatingPower(SetHeatingPowerPayload {
            power_permille,
            lease_ms: HEATING_POWER_LEASE_MS,
            restart_window,
        }));
    }

    // --- Profil de télémétrie ----------------------------------------------------

    fn set_telemetry_profile(&mut self, profile: TelemetryProfile) {
        if self.profile != profile {
            self.profile = profile;
            self.profile_dirty = true;
        }
        // La balance suffit à une cadence sobre au repos ; pendant un cycle,
        // chaque notification est publiée plutôt qu'un échantillon par
        // seconde.
        let active = profile == TelemetryProfile::Active;
        if active != self.ble_active {
            self.ble_active = active;
            self.out.push(Output::Ble(BleCommand::SetActive(active)));
        }
    }

    fn capture_profile(&self) -> TelemetryProfile {
        if self.flash_active {
            TelemetryProfile::Suspended
        } else if self.machine.active() || self.hf.active {
            TelemetryProfile::Active
        } else {
            TelemetryProfile::Idle
        }
    }

    // --- Capture HF ----------------------------------------------------------------

    fn begin_hf_capture(&mut self, origin: Origin, dimmer: u8) -> bool {
        let config = self.config.get();
        let mut started = false;
        if !self.hf.active || self.hf.cooldown {
            if self.hf.cooldown && self.shot_summary_pending {
                self.remember_completed_shot();
                self.shot_summary_pending = false;
            }
            let now = self.now();
            let started_unix = if self.snapshot.time_known {
                self.clock.wall_unix_s.unwrap_or(0)
            } else {
                0
            };
            let capture = &mut self.hf;
            capture.samples.clear();
            capture.origin = origin;
            capture.started_at_us = now;
            capture.ended_at_us = 0;
            capture.started_at_unix_s = started_unix;
            capture.ended_at_unix_s = 0;
            capture.next_sample_us = now;
            capture.cooldown_ends_at_us = 0;
            capture.volume_ml_reference = 0.0;
            capture.dropped_samples = 0;
            capture.pump_pct_commanded = dimmer;
            capture.active = true;
            capture.complete = false;
            capture.actuator_seen_on = false;
            capture.cooldown = false;
            self.snapshot.capture_cooldown = false;
            // Seule une infusion a un résumé ; une purge ou une commande de
            // banc l'efface pour que l'écran ne montre pas un cycle étranger.
            if origin == Origin::Brew {
                self.shot_builder.reset(
                    config.target_pressure_bar,
                    if config.heating_enabled {
                        config.brew_preheat_time_s
                    } else {
                        0.0
                    },
                );
            } else {
                self.shot_builder.clear();
            }
            self.snapshot.shot = *self.shot_builder.summary();
            self.snapshot.shot_start_unix_s = started_unix;
            started = true;
        } else if self.hf.active {
            self.hf.pump_pct_commanded = dimmer;
        }
        started
    }

    fn note_hf_capture_stop_request(&mut self) {
        if self.hf.active {
            self.hf.pump_pct_commanded = 0;
        }
    }

    fn start_cooldown(&mut self, now: i64) {
        let cooldown_us = if self.hf.origin == Origin::Brew {
            hf_capture::BREW_COOLDOWN_US
        } else {
            hf_capture::OTHER_COOLDOWN_US
        };
        self.hf.cooldown = true;
        self.hf.cooldown_ends_at_us = now + cooldown_us;
        self.snapshot.capture_cooldown = true;
    }

    fn finish_hf_capture_without_pump(&mut self) {
        let now = self.now();
        if self.hf.active && !self.hf.cooldown && !self.hf.actuator_seen_on {
            self.start_cooldown(now);
        }
    }

    fn scale_present_raw(&self, now: i64) -> bool {
        self.snapshot.scale_connected
            && self.t.scale_received_us != 0
            && now - self.t.scale_received_us <= SCALE_PRESENT_MS * 1000
    }

    fn tick_hf_capture(&mut self) {
        let now = self.now();
        if !self.hf.active {
            return;
        }
        if self.hf.cooldown && now >= self.hf.cooldown_ends_at_us {
            self.hf.active = false;
            self.hf.complete = true;
            self.hf.cooldown = false;
            self.hf.ended_at_us = self.hf.cooldown_ends_at_us;
            self.hf.ended_at_unix_s = if self.snapshot.time_known {
                self.clock.wall_unix_s.unwrap_or(0)
            } else {
                0
            };
            self.snapshot.capture_cooldown = false;
            if self.shot_summary_pending {
                self.remember_completed_shot();
                self.shot_summary_pending = false;
            }
            let profile = self.capture_profile();
            self.set_telemetry_profile(profile);
            return;
        }
        if !HF_CAPTURE_SCHEDULE.due(now, self.hf.next_sample_us) {
            return;
        }
        self.hf.next_sample_us = HF_CAPTURE_SCHEDULE.next(now, self.hf.next_sample_us);
        if self.hf.samples.len() == usize::from(hf_capture::CAPACITY) {
            self.hf.dropped_samples = self.hf.dropped_samples.saturating_add(1);
            return;
        }
        let s = self.snapshot;
        // La référence est prise avec le premier échantillon, plutôt qu'au
        // début de la capture : le premier volume exporté est donc toujours
        // exactement nul.
        if self.hf.samples.is_empty() {
            self.hf.volume_ml_reference = s.volume_ml;
        }
        let scale_flag = s.scale_connected
            && self.t.scale_received_us != 0
            && now - self.t.scale_received_us <= SCALE_PRESENT_MS * 1000;
        let boiler_fresh = s.boiler_temperature_valid
            && freshness(self.t.boiler_received_us, BOILER_PAIR_PERIOD_MS as u16, now)
                == Freshness::Fresh;
        let mut flags = 0u8;
        if s.pressure_valid {
            flags |= hf_capture::flags::PRESSURE_VALID;
        }
        if s.flow_valid {
            flags |= hf_capture::flags::FLOW_VALID;
        }
        if scale_flag {
            flags |= hf_capture::flags::SCALE_PRESENT;
        }
        if boiler_fresh {
            flags |= hf_capture::flags::BOILER_VALID;
        }
        if s.heater_on {
            flags |= hf_capture::flags::HEATER_ON;
        }
        if s.pressure_a2_valid {
            flags |= hf_capture::flags::PRESSURE_A2_READ;
        }
        if scace::is_valid(s.scace_connected, self.t.scace_received_us, now) {
            flags |= hf_capture::flags::SCACE_VALID;
        }
        let silent = self.t.last_flow_edge_received_us == 0
            || now - self.t.last_flow_edge_received_us > FLOW_SILENCE_MS * 1000;
        let sample = hf_capture::Sample {
            t_ms: ((now - self.hf.started_at_us) / 1000) as u32,
            pressure_raw: s.pressure_raw,
            xdb401_temperature_raw: s.xdb401_temperature_raw,
            boiler_ntc_a0_raw: s.boiler_ntc_a0_raw,
            boiler_ntc_a1_raw: s.boiler_ntc_a1_raw,
            pressure_a2_raw: s.pressure_a2_raw,
            scace_centi_c: self.scace_centi_c,
            boiler_temperature_age_ms: age_ms(self.t.boiler_received_us, now),
            flow_pulse_count: s.flow_pulse_count,
            flow_last_edge_age_ms: age_ms(self.t.last_flow_edge_received_us, now),
            pressure_bar: s.pressure_bar,
            xdb401_temperature_c: s.xdb401_temperature_c,
            boiler_temperature_c: s.boiler_temperature_c,
            heating_power_pct: s.heating_power_pct,
            volume_ml: s.volume_ml - self.hf.volume_ml_reference,
            flow_ml_s: if silent { 0.0 } else { s.flow_ml_s },
            weight_g: s.weight_g,
            pump_pct_commanded: self.hf.pump_pct_commanded,
            pump_pct_reported: s.dimmer_pct,
            mode: if self.hf.cooldown {
                SampleMode::Cooldown
            } else {
                sample_mode(self.machine.state())
            },
            flags,
        };
        self.hf.samples.push(sample);
        if self.hf.origin == Origin::Brew {
            self.shot_builder.add(&shot_sample(&sample));
            self.snapshot.shot = *self.shot_builder.summary();
        }
    }

    // --- Dernière infusion -----------------------------------------------------------

    fn remember_completed_shot(&mut self) {
        use machine::StopReason;
        if self.machine.state() != machine::State::Finished
            || self.machine.stop_reason() == StopReason::PurgeReleased
            || self.machine.stop_reason() == StopReason::PurgeTimeout
        {
            return;
        }
        let weight = self.snapshot.weight_g - self.machine.starting_weight_g();
        let duration = self.machine.elapsed_ms(self.now_ms());
        self.last_shot = LastShot {
            available: true,
            weight_g: weight,
            duration_ms: duration,
            flow_ml_s: self.snapshot.flow_ml_s,
            unix_s: self.shot_start_unix_s,
        };
        self.snapshot.last_shot_available = true;
        self.snapshot.last_shot_weight_g = weight;
        self.snapshot.last_shot_duration_ms = duration;
        self.snapshot.last_shot_flow_ml_s = self.snapshot.flow_ml_s;
        self.snapshot.last_shot_unix_s = self.shot_start_unix_s;
    }

    fn defer_completed_shot(&mut self) {
        use machine::StopReason;
        if self.machine.state() == machine::State::Finished
            && self.machine.stop_reason() != StopReason::PurgeReleased
            && self.machine.stop_reason() != StopReason::PurgeTimeout
        {
            self.shot_summary_pending = true;
        }
    }

    fn finalize_pending_shot(&mut self) {
        if !self.shot_summary_pending {
            return;
        }
        self.remember_completed_shot();
        self.shot_summary_pending = false;
    }

    fn update_cycle_snapshot(&mut self) {
        let now_ms = self.now_ms();
        self.snapshot.cycle_state = cycle_state(self.machine.state());
        self.snapshot.cycle_elapsed_ms = self.machine.elapsed_ms(now_ms);
        self.snapshot.cycle_phase_elapsed_ms = self.machine.phase_elapsed_ms(now_ms);
        self.snapshot.cycle_start_weight_g = self.machine.starting_weight_g();
        self.snapshot.cycle_weight_goal = self.machine.weight_goal();
    }

    // --- Pas de la tâche de télémétrie ---------------------------------------------

    /// Pas périodique (toutes les [`TELEMETRY_TICK_MS`] ms) : renouvelle les
    /// demandes de STATUS, entretient la présence, confirme l'image OTA des
    /// capteurs, expire la demande de chauffe, fait avancer le cycle, la
    /// régulation de chauffe et la capture HF.
    pub fn tick(&mut self, clock: Clock) {
        self.enter(clock);
        let now = self.now();
        if self.profile_dirty || now - self.last_request_us >= 1000 * 1000 {
            let p = periods_for(self.profile);
            self.send_request(MessageType::StatusFlow, p.flow);
            self.send_request(MessageType::StatusActuators, p.actuators);
            self.send_request(MessageType::StatusHeating, HEATING_STATUS_PERIOD_MS);
            self.profile_dirty = false;
            self.last_request_us = now;
        }
        let presence = self.presence.tick(now);
        if presence.send_ping {
            self.send_ping();
        }
        if presence.lost {
            self.send_log(LogCode::PresenceLost, LogSeverity::Warn, 0, 0);
            self.events
                .push(EventKind::CanPresenceLost, self.uptime_ms());
        }

        let s = self.snapshot_now();
        let confirm = OtaConfirmState {
            sensors_alive: s.sensors_alive,
            sensors_version: protocol::Version::new(
                s.sensors_version_major,
                s.sensors_version_minor,
                s.sensors_version_patch,
            ),
            actuators_fresh: s.actuators_freshness == Freshness::Fresh,
            heating_fresh: s.heating_freshness == Freshness::Fresh,
            valve_open: s.valve_open,
            pump_pct: s.pump_pct,
            heater_on: s.heater_on,
            heating_power_capable: s.heating_power_capable,
            heating_power_accepted_permille_nonzero: s.heating_power_accepted_pct > 0.0,
            heating_lease_remaining_ms: s.heating_lease_remaining_ms,
        };
        if now - self.last_ota_confirm_us >= can_link::OTA_CONFIRM_PERIOD_US && confirm.ready() {
            self.emit(Message::ConfirmSensorsOta(
                ConfirmSensorsOtaPayload::default(),
            ));
            self.last_ota_confirm_us = now;
        }

        if self.t.heating_deadline_us != 0
            && (now >= self.t.heating_deadline_us
                || self.t.heating_received_us == 0
                || now - self.t.heating_received_us > 750 * 1000)
        {
            self.t.heating_deadline_us = 0;
            self.snapshot.heating_requested = false;
            self.send_heating(false, 0);
        }
        self.tick_machine();
        self.tick_thermal();
        self.tick_hf_capture();
    }

    fn tick_machine(&mut self) {
        let now = self.now();
        let now_ms = self.now_ms();
        let input = self.machine_input(now);
        let was_active = self.machine.active();
        let output = self.machine.tick(now_ms, &input);
        let active = self.machine.active();
        let mut stop_reason = machine::StopReason::None;
        if was_active && !active {
            self.defer_completed_shot();
            stop_reason = self.machine.stop_reason();
        }
        self.update_cycle_snapshot();
        let mut send = false;
        if (active && now - self.last_cycle_set_us >= 100_000) || (!active && self.cycle_set_on) {
            self.last_cycle_set_us = now;
            self.cycle_set_on = active;
            send = true;
        }
        let capture_active = self.hf.active;
        let origin = if self.machine.state() == machine::State::Purge {
            Origin::Purge
        } else {
            Origin::Brew
        };
        if was_active && !active {
            let purge = stop_reason == machine::StopReason::PurgeReleased
                || stop_reason == machine::StopReason::PurgeTimeout;
            self.send_log(
                if purge {
                    LogCode::PurgeStopped
                } else {
                    LogCode::BrewStopped
                },
                LogSeverity::Info,
                stop_reason as u16,
                0,
            );
        }
        if send {
            self.send_set(output.dimmer, output.ttl_ms, origin);
        }
        let profile = if self.flash_active {
            TelemetryProfile::Suspended
        } else if active || capture_active {
            TelemetryProfile::Active
        } else {
            TelemetryProfile::Idle
        };
        self.set_telemetry_profile(profile);
    }

    fn machine_input(&self, now: i64) -> machine::Input {
        // `scale_present` du snapshot est dérivé à la lecture ; les actions de
        // la machine lisent l'état brut et appliquent la même règle, sinon un
        // cycle démarré juste après une mesure de balance partirait en mode
        // temps. La boucle PI peut tenir la dernière pression valide pendant
        // une brève erreur I2C, jamais au-delà de 300 ms.
        let pressure_fresh =
            self.t.pressure_last_valid_us != 0 && now - self.t.pressure_last_valid_us <= 300 * 1000;
        machine::Input {
            weight_g: self.snapshot.weight_g,
            scale_present: self.scale_present_raw(now),
            pressure_bar: self.snapshot.pressure_bar,
            pressure_valid: pressure_fresh,
        }
    }

    fn tick_thermal(&mut self) {
        let now = self.now();
        if now - self.last_thermal_step_us < 250 * 1000 {
            return;
        }
        self.last_thermal_step_us = now;
        let config = self.config.get();
        let s = self.snapshot_now();
        let filling = matches!(s.cycle_state, CycleState::Filling | CycleState::Preinfusion);
        let infusing = matches!(s.cycle_state, CycleState::Brew | CycleState::Rampdown);
        let mode = match s.cycle_state {
            CycleState::ThermalPreheat => thermal::Mode::ThermalPreheat,
            CycleState::Purge => thermal::Mode::Purge,
            _ if infusing => thermal::Mode::Infusion,
            _ if filling => thermal::Mode::Brew,
            _ => thermal::Mode::Idle,
        };
        let can_heat = config.heating_enabled
            && !self.flash_active
            && !s.heating_requested
            && !s.lockout
            && s.sensors_alive
            && s.heating_power_capable
            && s.heating_freshness == Freshness::Fresh;
        let flowmeter_live = s.flow_valid
            && s.flow_freshness == Freshness::Fresh
            && s.actuators_freshness == Freshness::Fresh
            && s.pump_pct > 0;
        let flow_recent = s.flow_last_edge_age_ms <= 500;
        // La pause de pré-infusion garde la pompe sous son seuil de débit :
        // sans impulsion, le débit est nul et non inconnu. Ailleurs, le repli
        // à 45 % couvre un débitmètre muet pendant l'écoulement.
        let paused = s.cycle_state == CycleState::Preinfusion && flowmeter_live && !flow_recent;
        let output = self.thermal.step(
            now_ms_of(now),
            s.boiler_temperature_c,
            config.brew_temperature_c,
            s.boiler_temperature_valid && s.boiler_temperature_freshness == Freshness::Fresh,
            can_heat,
            mode,
            Flow::new(
                if paused { 0.0 } else { s.flow_ml_s },
                flowmeter_live && (flow_recent || paused),
            ),
        );
        self.snapshot.heating_power_pct = f32::from(output.power_permille) / 10.0;
        self.snapshot.brew_temperature_ready = output.ready;
        // Le drapeau de nouvelle période n'est envoyé qu'à un module qui
        // l'annonce : un module plus ancien rejetterait la trame de 5 octets.
        // Perdu, il laisse seulement le comportement antérieur (attente de la
        // période suivante).
        let restart_window = output.restart_window && s.heating_window_restart_capable;
        if (output.power_permille != self.last_heating_power_sent
            || restart_window
            || (output.power_permille > 0 && now - self.last_heating_power_send_us >= 500 * 1000))
            && s.sensors_alive
            && s.heating_power_capable
        {
            self.send_heating_power(output.power_permille, restart_window);
            self.last_heating_power_sent = output.power_permille;
            self.last_heating_power_send_us = now;
        }
    }

    // --- Entrées : CAN --------------------------------------------------------------

    /// Trame reçue du bus (ou du pont série). Seules celles émises par les
    /// capteurs et adressées à l'écran ou en broadcast comptent ; toute trame
    /// attribuable aux capteurs entretient la présence, y compris un type
    /// futur que cette image ne comprend pas.
    pub fn on_can_frame(&mut self, clock: Clock, frame: &Frame) {
        self.enter(clock);
        let inbound = can_link::classify(frame);
        if inbound == Inbound::Ignored {
            return;
        }
        self.mark_presence();
        let message = match inbound {
            Inbound::Message(message) => message,
            Inbound::Malformed { message_type } => {
                // Un PING illisible reçoit quand même un PONG, comme le C++.
                if message_type == MessageType::Ping {
                    self.send_pong();
                }
                return;
            }
            _ => return,
        };
        match message {
            Message::Ping(identity) => {
                self.on_pong_identity(&identity);
                self.send_pong();
            }
            // DLC 0 reste le format des images antérieures à v0.2.30.
            Message::PingLegacy => self.send_pong(),
            Message::Pong(identity) => {
                if frame.len == 8 && frame.data[0] == Node::Sensors as u8 {
                    self.presence.note_roundtrip();
                    self.on_pong_identity(&identity);
                }
            }
            Message::StatusPressure(payload) => self.on_status_pressure(&payload),
            Message::StatusFlow(payload) => self.on_status_flow(&payload),
            Message::StatusActuators(payload) => self.on_status_actuators(&payload),
            Message::StatusHeating(payload) => self.on_status_heating(&payload),
            Message::Log(payload) => self.on_log(&payload),
            Message::FlashCtrl(payload) => self.out.push(Output::SensorsFlashCtrl(payload)),
            _ => {}
        }
    }

    fn mark_presence(&mut self) {
        let now = self.now();
        if self.presence.mark(now) {
            self.send_log(LogCode::PresenceRestored, LogSeverity::Info, 0, 0);
            self.events
                .push(EventKind::CanPresenceRestored, self.uptime_ms());
        }
    }

    /// Un PONG des capteurs n'a pas à être confirmé par le même identifiant :
    /// il faut seulement que le nœud annoncé soit les capteurs.
    fn on_pong_identity(&mut self, identity: &NodeIdentityPayload) {
        let Some(version) = can_link::sensors_identity(identity) else {
            return;
        };
        let now = self.now();
        self.t.last_sensors_message_us = now;
        if identity.uptime_s < self.snapshot.sensors_uptime_s
            || !heating_protocol_supported(version.major, version.minor, version.patch)
        {
            self.t.heating_received_us = 0;
            self.snapshot.heating_capable = false;
            self.snapshot.heating_power_capable = false;
            self.snapshot.heating_window_restart_capable = false;
            self.snapshot.heater_on = false;
            self.snapshot.heating_power_accepted_pct = 0.0;
            self.snapshot.heating_lease_remaining_ms = 0;
        }
        self.snapshot.sensors_version_major = version.major;
        self.snapshot.sensors_version_minor = version.minor;
        self.snapshot.sensors_version_patch = version.patch;
        self.snapshot.sensors_uptime_s = identity.uptime_s;
    }

    /// L'écran ne demande plus STATUS_PRESSURE : la pression vient de A2. Un
    /// paquet reçu quand même ne garde que ses codes bruts, pour le
    /// diagnostic.
    fn on_status_pressure(&mut self, payload: &protocol::messages::StatusPressurePayload) {
        self.t.last_sensors_message_us = self.now();
        self.snapshot.pressure_raw = payload.pressure_raw;
        self.snapshot.temperature_raw = payload.temperature_raw;
        self.snapshot.xdb401_temperature_raw = payload.temperature_raw;
    }

    fn on_status_flow(&mut self, payload: &protocol::messages::StatusFlowPayload) {
        let now = self.now();
        self.t.last_sensors_message_us = now;
        self.t.flow_received_us = now;
        self.snapshot.flow_valid = (payload.flags & 0x01) != 0;
        let changed = self.flow_history.is_empty()
            || payload.pulse_count != self.snapshot.flow_pulse_count
            || self
                .flow_history
                .last()
                .is_some_and(|last| payload.last_edge_ms != last.edge_ms);
        if !self.flow_history.is_empty() && payload.pulse_count < self.snapshot.flow_pulse_count {
            self.flow_history.clear(); // redémarrage du XIAO
        }
        self.snapshot.flow_pulse_count = payload.pulse_count;
        self.snapshot.volume_ml =
            payload.pulse_count as f32 * 1000.0 / calibration::FLOW_PULSES_PER_LITER;
        if changed {
            if self.flow_history.len() == FLOW_HISTORY_CAPACITY {
                self.flow_history.remove(0);
            }
            self.flow_history.push(FlowSample {
                pulses: payload.pulse_count,
                edge_ms: payload.last_edge_ms,
            });
            self.t.last_flow_edge_received_us = now;
            self.snapshot.flow_ml_s = self.compute_flow_ml_s();
        }
    }

    fn compute_flow_ml_s(&self) -> f32 {
        let count = self.flow_history.len();
        if count < 2 {
            return 0.0;
        }
        let latest = self.flow_history[count - 1];
        let mut anchor = None;
        for i in (0..count - 1).rev() {
            if latest.pulses.wrapping_sub(self.flow_history[i].pulses) >= FLOW_WINDOW_PULSES {
                anchor = Some(self.flow_history[i]);
                break;
            }
        }
        let Some(anchor) = anchor else {
            return 0.0;
        };
        let delta_pulses = latest.pulses.wrapping_sub(anchor.pulses);
        let delta_ms = latest.edge_ms.wrapping_sub(anchor.edge_ms);
        if delta_ms == 0 {
            return 0.0;
        }
        // pulses/L ÷ ms : convertir d'abord les millisecondes en secondes,
        // puis les litres en millilitres. Le résultat est bien en ml/s.
        delta_pulses as f32 * 1_000_000.0
            / (calibration::FLOW_PULSES_PER_LITER * f32::from(delta_ms))
    }

    fn on_status_actuators(&mut self, payload: &protocol::messages::StatusActuatorsPayload) {
        use protocol::messages::StatusActuatorsPayload as P;
        let now = self.now();
        self.t.last_sensors_message_us = now;
        self.t.actuators_received_us = now;
        self.snapshot.valve_open = payload.valve_open;
        self.snapshot.dimmer_pct = payload.pump_pct;
        self.snapshot.pump_pct = payload.pump_pct;
        self.snapshot.lease_remaining_ms = payload.lease_remaining_ms;
        self.snapshot.continuous_on_ms = payload.continuous_on_ms;
        self.snapshot.lockout = (payload.flags & P::FLAG_LOCKOUT) != 0;
        self.snapshot.dimmer_ready = (payload.flags & P::FLAG_DIMMER_READY) != 0;
        self.snapshot.dimmer_valid = (payload.flags & P::FLAG_DIMMER_PRESENT) != 0;
        self.snapshot.dimmer_error_active = (payload.flags & P::FLAG_DIMMER_ERROR) != 0;
        self.snapshot.maintenance_valve_open =
            (payload.flags & P::FLAG_MAINTENANCE_VALVE_OPEN) != 0;
        self.snapshot.maintenance_valve_capable =
            (payload.flags & P::FLAG_MAINTENANCE_VALVE_CAPABLE) != 0;
        if self.hf.active && payload.pump_pct > 0 {
            self.hf.actuator_seen_on = true;
        }
        if self.hf.active && !self.hf.cooldown && self.hf.actuator_seen_on && payload.pump_pct == 0
        {
            self.start_cooldown(now);
        }
        let profile = self.capture_profile();
        self.set_telemetry_profile(profile);
    }

    fn on_status_heating(&mut self, payload: &protocol::messages::StatusHeatingPayload) {
        let now = self.now();
        self.t.last_sensors_message_us = now;
        self.t.heating_received_us = now;
        self.snapshot.heating_capable = true;
        self.snapshot.heating_power_capable = payload.fine_power_capable;
        self.snapshot.heating_window_restart_capable = payload.window_restart_capable;
        self.snapshot.heater_on = payload.heater_on;
        self.snapshot.heating_power_accepted_pct = f32::from(payload.power_permille) / 10.0;
        self.snapshot.heating_lease_remaining_ms = payload.lease_remaining_ms;
    }

    fn on_log(&mut self, payload: &LogPayload) {
        if payload.code != LogCode::TwaiErrorCounters as u8 {
            return;
        }
        self.snapshot.sensors_twai_rx_errors = u16::from((payload.arg16 >> 8) as u8);
        self.snapshot.sensors_twai_tx_errors = u16::from((payload.arg16 & 0xFF) as u8);
        self.snapshot.sensors_twai_bus_errors = payload.arg32;
    }

    /// Le module capteurs a-t-il répondu à un PING par un PONG ?
    pub fn peer_roundtrip_confirmed(&self) -> bool {
        self.presence.peer_roundtrip_confirmed()
    }

    /// Présence CAN perdue (au boot, avant toute trame, elle l'est).
    pub fn presence_lost(&self) -> bool {
        self.presence.lost()
    }

    // --- Entrées : mesures locales et BLE -----------------------------------------

    /// Paire NTC de la chaudière lue sur A0/A1. Renvoie la validité.
    pub fn on_boiler_ntc_reading(
        &mut self,
        clock: Clock,
        a0_raw: i16,
        a1_raw: i16,
        read_ok: bool,
    ) -> bool {
        self.enter(clock);
        let reading = ntc::boiler_temperature(a0_raw, a1_raw, read_ok);
        self.snapshot.boiler_temperature_valid = reading.valid;
        if read_ok {
            self.snapshot.boiler_ntc_a0_raw = a0_raw;
            self.snapshot.boiler_ntc_a1_raw = a1_raw;
        }
        if reading.valid {
            self.snapshot.boiler_temperature_c = reading.temperature_c;
            self.snapshot.boiler_sensor_temperature_c =
                calibration::boiler_sensor_temperature_c(reading.temperature_c);
            self.t.boiler_received_us = self.now();
        }
        reading.valid
    }

    /// Code A2 (XDB401 analogique), lu avec la paire NTC.
    pub fn on_pressure_a2_reading(&mut self, clock: Clock, raw: i16, read_ok: bool) {
        self.enter(clock);
        let now = self.now();
        self.t.pressure_received_us = now;
        self.snapshot.pressure_a2_valid = read_ok;
        self.snapshot.pressure_valid = read_ok;
        if read_ok {
            self.snapshot.pressure_a2_raw = raw;
            self.snapshot.pressure_bar = ntc::pressure_bar_from_a2(raw);
            self.t.pressure_last_valid_us = now;
        }
    }

    /// Connexion ou perte de la balance. La définition de présence reste au
    /// cœur : connectée et une pesée reçue depuis moins de deux secondes.
    pub fn update_scale_connection(&mut self, clock: Clock, connected: bool) {
        self.enter(clock);
        self.snapshot.scale_connected = connected;
        if !connected {
            self.t.scale_received_us = 0;
        }
    }

    pub fn update_scale_weight(&mut self, clock: Clock, weight_g: f32) {
        self.enter(clock);
        self.snapshot.weight_g = weight_g;
        self.t.scale_received_us = self.now();
    }

    /// Le client BLE peut-il écrire sur la balance (abonnement actif) ? Sans
    /// quoi la tare est refusée. Vrai par défaut.
    pub fn set_scale_writable(&mut self, writable: bool) {
        self.scale_writable = writable;
    }

    pub fn update_scace_connection(&mut self, clock: Clock, connected: bool) {
        self.enter(clock);
        self.snapshot.scace_connected = connected;
        if !connected {
            self.t.scace_received_us = 0;
        }
    }

    /// Échantillon de la sonde SCACE. `ok` est faux pour une trame sans
    /// température (pont hors tension, NTC ouverte ou en court-circuit).
    pub fn update_scace_temperature(&mut self, clock: Clock, centi_c: i16, ok: bool) {
        self.enter(clock);
        if ok {
            self.scace_centi_c = centi_c;
            self.snapshot.scace_temperature_c = f32::from(centi_c) / 100.0;
            self.t.scace_received_us = self.now();
        } else {
            self.t.scace_received_us = 0;
        }
    }

    pub fn set_touch_ready(&mut self, ready: bool) {
        self.snapshot.touch_ready = ready;
    }

    pub fn note_touch_press(&mut self) {
        self.snapshot.touch_press_count = self.snapshot.touch_press_count.wrapping_add(1);
    }

    /// Tas interne : valeurs du moment, fournies par la plateforme.
    pub fn set_heap_stats(&mut self, free: u32, largest: u32, minimum: u32) {
        self.snapshot.internal_heap_free = free;
        self.snapshot.internal_heap_largest = largest;
        self.snapshot.internal_heap_minimum = minimum;
    }

    pub fn set_boot_time_syncing(&mut self, syncing: bool) {
        self.snapshot.boot_time_syncing = syncing;
    }

    /// Publie un événement dans le journal (réseau, heure...).
    pub fn note_event(&mut self, clock: Clock, kind: EventKind) {
        self.enter(clock);
        self.events.push(kind, self.uptime_ms());
    }

    // --- Radio et réseau ---------------------------------------------------------------

    /// État du réseau et adresse IPv4 (octet de poids faible = premier octet).
    pub fn update_network_status(&mut self, state: NetworkState, ipv4_address: u32) {
        self.snapshot.network_state = state;
        self.snapshot.ipv4_address = ipv4_address;
    }

    /// Des identifiants Wi-Fi sont-ils en NVS ? Sans eux : point d'accès de
    /// provisioning et BLE coupé. Une panne STA ne relance jamais l'AP ; seul
    /// l'oubli du réseau ([`Core::forget_network`]) repasse ici avec `false`.
    pub fn set_credentials_present(&mut self, present: bool) {
        self.credentials_present = present;
        self.refresh_radio();
    }

    /// Bascule Wi-Fi de la session (icône des réglages). Non mémorisée :
    /// chaque démarrage remet le Wi-Fi à ON. Sans effet en provisioning.
    pub fn set_wifi_enabled(&mut self, enabled: bool) {
        self.wifi_enabled = enabled;
        self.refresh_radio();
    }

    /// État radio voulu actuellement.
    pub fn radio_request(&self) -> RadioRequest {
        if !self.credentials_present {
            RadioRequest {
                wifi: WifiRequest::Provisioning,
                ble: false,
            }
        } else {
            RadioRequest {
                wifi: if self.wifi_enabled {
                    WifiRequest::Sta
                } else {
                    WifiRequest::Off
                },
                ble: true,
            }
        }
    }

    fn refresh_radio(&mut self) {
        self.snapshot.provisioning = !self.credentials_present;
        self.snapshot.ble_enabled = self.credentials_present;
        self.snapshot.wifi_enabled = self.wifi_enabled;
        let request = self.radio_request();
        if self.last_radio != Some(request) {
            self.last_radio = Some(request);
            self.out.push(Output::Radio(request));
        }
    }

    /// Oublier le réseau (réglages) : demande l'effacement des identifiants.
    /// La plateforme les efface puis appelle
    /// [`Core::set_credentials_present`]`(false)`.
    pub fn forget_network(&mut self) {
        self.out.push(Output::ForgetNetwork);
    }

    // --- Configuration ---------------------------------------------------------------

    pub fn config(&self) -> Config {
        self.config.get()
    }

    /// Écrit une configuration validée, tout ou rien. Refusée (`Busy`) pendant
    /// une infusion ou une purge. Ramène à zéro l'état « prêt » si la consigne
    /// de température change, et coupe la chauffe si elle est désactivée.
    pub fn put_config(&mut self, clock: Clock, candidate: &Config) -> ConfigResult {
        self.enter(clock);
        if self.snapshot_now().cycle_state.is_active() {
            return ConfigResult::new(ConfigStatus::Busy, "cycle");
        }
        for _ in 0..2 {
            let previous = self.config.get();
            let result = self
                .config
                .apply(&mut self.store, candidate, previous.revision);
            if result.status != ConfigStatus::StaleRevision {
                if result.status == ConfigStatus::Ok
                    && (!candidate.heating_enabled
                        || candidate.brew_temperature_c != previous.brew_temperature_c)
                {
                    self.snapshot.brew_temperature_ready = false;
                    if !candidate.heating_enabled {
                        self.snapshot.heating_power_pct = 0.0;
                        self.snapshot.heating_requested = false;
                        self.t.heating_deadline_us = 0;
                        self.send_heating_power(0, false);
                        self.send_heating(false, 0);
                    }
                }
                return result;
            }
        }
        ConfigResult::new(ConfigStatus::StaleRevision, "revision")
    }

    /// Remet les valeurs par défaut (nouvelle révision).
    pub fn reset_config(&mut self, clock: Clock) -> ConfigResult {
        self.enter(clock);
        self.config.reset(&mut self.store)
    }

    pub fn note_http_auth_refused(&mut self) {
        self.send_log(LogCode::HttpAuthRefused, LogSeverity::Warn, 0, 0);
    }

    pub fn note_config_rejected(&mut self, field: Option<&str>) {
        self.send_log(
            LogCode::ConfigRejected,
            LogSeverity::Warn,
            config_field_arg(field),
            0,
        );
    }

    // --- Actions ----------------------------------------------------------------------

    /// Compatibilité avec l'écran de service : purge à pression maintenue.
    pub fn set_diagnostic_purge(
        &mut self,
        clock: Clock,
        enabled: bool,
        pump_pct: u8,
    ) -> DiagnosticStatus {
        let mut command = ActionCommand::new(if enabled {
            Action::PurgePress
        } else {
            Action::PurgeRelease
        });
        command.brew.pump_pct = pump_pct;
        match self
            .perform_action(clock, &command, ActionSource::Ui)
            .status
        {
            ActionStatus::Ok => DiagnosticStatus::Ok,
            ActionStatus::BusLost => DiagnosticStatus::BusLost,
            ActionStatus::Locked => DiagnosticStatus::Locked,
            _ => DiagnosticStatus::DimmerNotReady,
        }
    }

    /// Exécute une action, acceptée ou refusée avec un motif. Une infusion ne
    /// peut jamais être lancée depuis HTTP ([`ActionSource::Http`]) ; elle est
    /// permise Wi-Fi actif.
    pub fn perform_action(
        &mut self,
        clock: Clock,
        command: &ActionCommand,
        source: ActionSource,
    ) -> ActionResult {
        self.enter(clock);
        use ActionStatus as St;
        let result = |status| ActionResult::of(status);
        if command.action == Action::StartBrew && source == ActionSource::Http {
            return result(St::Unavailable);
        }
        if command.action == Action::Tare {
            let present = self.snapshot_now().scale_present;
            if !present || !self.scale_writable {
                return result(St::Unavailable);
            }
            self.out.push(Output::Ble(BleCommand::Tare));
            return result(St::Ok);
        }
        if command.action == Action::DismissSummary {
            if self.shot_summary_pending || self.hf.cooldown {
                return result(St::CycleActive);
            }
            let ok = self.machine.dismiss();
            if ok {
                self.snapshot.last_shot_available = false;
                self.last_shot.available = false;
            }
            self.update_cycle_snapshot();
            return result(if ok { St::Ok } else { St::NoCycle });
        }
        let snapshot = self.snapshot_now();
        let cycle_busy = snapshot.cycle_state.is_active();
        if command.action == Action::SetHeating {
            if !command.heating.on {
                self.t.heating_deadline_us = 0;
                self.snapshot.heating_requested = false;
                self.send_heating(false, 0);
                return result(St::Ok);
            }
            if command.heating.duration_ms == 0 || command.heating.duration_ms > SET_HEATING_MAX_MS
            {
                return result(St::InvalidValue);
            }
            if !self.config.get().heating_enabled {
                return result(St::Unavailable);
            }
            if self.flash_active {
                return result(St::CycleActive);
            }
            if !snapshot.sensors_alive {
                return result(St::BusLost);
            }
            if snapshot.lockout {
                return result(St::Locked);
            }
            if !snapshot.heating_capable
                || snapshot.heating_freshness != Freshness::Fresh
                || snapshot.heater_on
                || snapshot.heating_requested
                || snapshot.heating_power_pct > 0.0
                || snapshot.heating_power_accepted_pct > 0.0
            {
                return result(St::Unavailable);
            }
            self.t.heating_deadline_us = self.now() + i64::from(command.heating.duration_ms) * 1000;
            self.snapshot.heating_requested = true;
            self.send_heating(true, command.heating.duration_ms);
            return result(St::Ok);
        }
        if command.action == Action::CloseMaintenanceValve {
            if !snapshot.sensors_alive {
                return result(St::BusLost);
            }
            self.send_maintenance_valve(false);
            return result(St::Ok);
        }
        if command.action == Action::OpenMaintenanceValve {
            if self.flash_active {
                return result(St::CycleActive);
            }
            if !snapshot.sensors_alive {
                return result(St::BusLost);
            }
            if snapshot.lockout {
                return result(St::Locked);
            }
            if !snapshot.maintenance_valve_capable
                || snapshot.actuators_freshness != Freshness::Fresh
                || self.config.get().heating_enabled
                || snapshot.heating_requested
            {
                return result(St::Unavailable);
            }
            if cycle_busy
                || snapshot.capture_cooldown
                || snapshot.pump_pct != 0
                || (snapshot.valve_open && !snapshot.maintenance_valve_open)
            {
                return result(St::CycleActive);
            }
            self.send_maintenance_valve(true);
            return result(St::Ok);
        }
        if command.action == Action::ResetSensors {
            if !snapshot.sensors_alive {
                return result(St::BusLost);
            }
            if cycle_busy || snapshot.capture_cooldown || self.flash_active {
                return result(St::CycleActive);
            }
            self.send_set(0, 0, Origin::SetActuators);
            self.emit(Message::Reset);
            return result(St::Ok);
        }
        if command.action == Action::ResetDimmer || command.action == Action::RecalibrateDimmer {
            if !snapshot.sensors_alive {
                return result(St::BusLost);
            }
            if cycle_busy || snapshot.capture_cooldown || self.flash_active {
                return result(St::CycleActive);
            }
            // Coupe explicitement les sorties avant une maintenance qui peut
            // rendre le DimmerLink indisponible pendant quelques secondes.
            self.send_set(0, 0, Origin::SetActuators);
            self.emit(Message::DimmerCommand(DimmerCommandPayload {
                command: if command.action == Action::ResetDimmer {
                    DimmerCommand::Reset
                } else {
                    DimmerCommand::Recalibrate
                },
            }));
            return result(St::Ok);
        }
        if command.action == Action::StopBrew {
            let stopped_during_preheat = snapshot.cycle_state == CycleState::ThermalPreheat;
            let ok = self.machine.stop(self.now_ms());
            if ok {
                self.defer_completed_shot();
            }
            self.update_cycle_snapshot();
            self.cycle_set_on = false;
            if ok {
                self.send_set(0, 0, Origin::SetActuators);
            }
            if ok && stopped_during_preheat {
                self.finish_hf_capture_without_pump();
            }
            if ok {
                self.send_log(
                    LogCode::BrewStopped,
                    LogSeverity::Info,
                    machine::StopReason::Manual as u16,
                    0,
                );
            }
            return result(if ok { St::Ok } else { St::NoCycle });
        }
        if command.action == Action::PurgeRelease {
            let ok = self.machine.purge_release(self.now_ms());
            self.update_cycle_snapshot();
            self.cycle_set_on = false;
            if ok {
                self.send_set(0, 0, Origin::SetActuators);
                self.send_log(
                    LogCode::PurgeStopped,
                    LogSeverity::Info,
                    machine::StopReason::PurgeReleased as u16,
                    0,
                );
            }
            return result(if ok { St::Ok } else { St::NoCycle });
        }
        if command.action == Action::StartBrew || command.action == Action::PurgePress {
            if self.flash_active {
                return result(St::CycleActive);
            }
            if !snapshot.sensors_alive {
                return result(St::BusLost);
            }
            if snapshot.lockout {
                return result(St::Locked);
            }
            if snapshot.maintenance_valve_open {
                return result(St::CycleActive);
            }
            if command.action == Action::StartBrew {
                let config = self.config.get();
                if !config.heating_enabled
                    || !snapshot.brew_temperature_ready
                    || !snapshot.heating_power_capable
                    || snapshot.heating_freshness != Freshness::Fresh
                    || !snapshot.boiler_temperature_valid
                    || snapshot.boiler_temperature_freshness != Freshness::Fresh
                    || libm::fabsf(snapshot.boiler_temperature_c - config.brew_temperature_c)
                        > BREW_TEMPERATURE_TOLERANCE_C
                {
                    return result(St::Unavailable);
                }
            }
            if !snapshot.dimmer_ready || !snapshot.dimmer_valid {
                return result(St::DimmerNotReady);
            }
            let wall_s = self.clock.wall_unix_s.unwrap_or(0);
            self.finalize_pending_shot();
            let machine_config = machine::Config::from(&self.config.get());
            let now = self.now();
            let ok = if command.action == Action::StartBrew {
                let input = self.machine_input(now);
                self.machine.start(self.now_ms(), &machine_config, &input)
            } else {
                self.machine.purge_press(self.now_ms(), &machine_config)
            };
            if ok && command.action == Action::StartBrew {
                self.shot_start_unix_s = if self.snapshot.time_known { wall_s } else { 0 };
            }
            self.update_cycle_snapshot();
            if ok && command.action == Action::StartBrew {
                self.begin_hf_capture(Origin::Brew, 0);
                self.set_telemetry_profile(TelemetryProfile::Active);
            }
            if ok {
                self.send_log(
                    if command.action == Action::StartBrew {
                        LogCode::BrewStarted
                    } else {
                        LogCode::PurgeStarted
                    },
                    LogSeverity::Info,
                    0,
                    0,
                );
            }
            return result(if ok { St::Ok } else { St::CycleActive });
        }
        if command.action != Action::SetActuators && command.action != Action::SetBrewActuators {
            return result(St::Unavailable);
        }
        if command.brew.pump_pct > 100 {
            return result(St::InvalidValue);
        }
        let snapshot = self.snapshot_now();
        if !snapshot.sensors_alive {
            return result(St::BusLost);
        }
        if snapshot.lockout {
            return result(St::Locked);
        }
        if snapshot.cycle_state.is_active() || self.flash_active || snapshot.maintenance_valve_open
        {
            return result(St::CycleActive);
        }
        self.send_set(
            command.brew.pump_pct,
            command.brew.ttl_ms,
            Origin::SetActuators,
        );
        result(St::Ok)
    }

    // --- Mise à jour OTA ----------------------------------------------------------------

    /// Démarre une mise à jour (écran ou capteurs). Refusée pendant un cycle,
    /// une capture en récupération, ou si les actionneurs ne sont pas au
    /// repos. Pour les capteurs, le STOP part et il faut attendre l'écho :
    /// voir [`FlashStart::AwaitingStop`].
    pub fn begin_flash(&mut self, clock: Clock, target: FlashTarget, total: u32) -> FlashStart {
        self.enter(clock);
        let s = self.snapshot_now();
        if target == FlashTarget::None
            || total == 0
            || self.flash_active
            || self.pending_flash.is_some()
            || s.cycle_state.is_active()
            || s.capture_cooldown
        {
            return FlashStart::Refused;
        }
        if s.heating_requested
            || (s.actuators_freshness == Freshness::Fresh && (s.valve_open || s.pump_pct != 0))
            || (s.heating_freshness == Freshness::Fresh && s.heater_on)
        {
            return FlashStart::Refused;
        }
        if target == FlashTarget::Sensors {
            if !s.sensors_alive
                || s.actuators_freshness != Freshness::Fresh
                || (s.heating_capable && s.heating_freshness != Freshness::Fresh)
            {
                return FlashStart::Refused;
            }
            self.pending_flash = Some(PendingFlash {
                target,
                total,
                old_actuators_received_us: self.t.actuators_received_us,
                old_heating_received_us: self.t.heating_received_us,
            });
            self.emit(Message::Stop);
            return FlashStart::AwaitingStop;
        }
        // Un écran qui ne décode plus le CAN doit garder son propre chemin
        // OTA.
        self.emit(Message::Stop);
        self.activate_flash(target, total);
        FlashStart::Started
    }

    /// Les capteurs ont-ils confirmé l'arrêt (nouvel écho d'actionneurs, vanne
    /// fermée, pompe à zéro, chauffe éteinte) ? Active alors la mise à jour.
    pub fn poll_flash_stop(&mut self, clock: Clock) -> bool {
        self.enter(clock);
        let Some(pending) = self.pending_flash else {
            return false;
        };
        let echo = self.snapshot_now();
        let stopped = self.t.actuators_received_us > pending.old_actuators_received_us
            && echo.actuators_freshness == Freshness::Fresh
            && !echo.valve_open
            && echo.dimmer_pct == 0
            && (!echo.heating_capable
                || (self.t.heating_received_us > pending.old_heating_received_us
                    && echo.heating_freshness == Freshness::Fresh
                    && !echo.heater_on));
        if !stopped {
            return false;
        }
        self.pending_flash = None;
        self.activate_flash(pending.target, pending.total);
        true
    }

    /// Abandonne l'attente de [`Core::poll_flash_stop`] (délai dépassé).
    pub fn cancel_pending_flash(&mut self) {
        self.pending_flash = None;
    }

    fn activate_flash(&mut self, target: FlashTarget, total: u32) {
        self.flash_active = true;
        self.snapshot.flash_active = true;
        self.snapshot.flash_target = target;
        self.snapshot.flash_bytes_done = 0;
        self.snapshot.flash_bytes_total = total;
        self.profile = TelemetryProfile::Suspended;
        self.profile_dirty = true;
        self.send_set(0, 0, Origin::SetActuators);
        self.send_heating(false, 0);
    }

    pub fn update_flash_progress(&mut self, done: u32) {
        if self.snapshot.flash_active {
            self.snapshot.flash_bytes_done = done;
        }
    }

    pub fn finish_flash(&mut self) {
        self.flash_active = false;
        self.snapshot.flash_active = false;
        self.snapshot.flash_target = FlashTarget::None;
        self.profile = TelemetryProfile::Idle;
        self.profile_dirty = true;
    }

    // --- Lecture ---------------------------------------------------------------------

    /// Instantané complet, âges et fraîcheurs calculés à `clock`.
    pub fn snapshot(&mut self, clock: Clock) -> Snapshot {
        self.enter(clock);
        self.snapshot_now()
    }

    fn snapshot_now(&self) -> Snapshot {
        let now = self.now();
        let mut result = self.snapshot;
        result.last_shot_available = self.last_shot.available;
        result.last_shot_weight_g = self.last_shot.weight_g;
        result.last_shot_duration_ms = self.last_shot.duration_ms;
        result.last_shot_flow_ml_s = self.last_shot.flow_ml_s;
        result.last_shot_unix_s = self.last_shot.unix_s;
        result.capture_cooldown_remaining_ms =
            if self.hf.cooldown && self.hf.cooldown_ends_at_us > now {
                ((self.hf.cooldown_ends_at_us - now) / 1000) as u32
            } else {
                0
            };
        let periods = periods_for(self.profile);
        let t = &self.t;
        result.pressure_age_ms = age_ms(t.pressure_received_us, now);
        result.boiler_temperature_age_ms = age_ms(t.boiler_received_us, now);
        result.boiler_temperature_freshness = if result.boiler_temperature_valid {
            freshness(t.boiler_received_us, BOILER_PAIR_PERIOD_MS as u16, now)
        } else {
            Freshness::Missing
        };
        if result.boiler_temperature_freshness != Freshness::Fresh {
            result.boiler_temperature_valid = false;
        }
        result.flow_age_ms = age_ms(t.flow_received_us, now);
        result.actuators_age_ms = age_ms(t.actuators_received_us, now);
        result.heating_age_ms = age_ms(t.heating_received_us, now);
        result.heating_freshness = freshness(t.heating_received_us, HEATING_STATUS_PERIOD_MS, now);
        if result.heating_freshness != Freshness::Fresh {
            result.heater_on = false;
            result.heating_lease_remaining_ms = 0;
        }
        result.pressure_freshness = if result.pressure_valid {
            freshness(t.pressure_received_us, BOILER_PAIR_PERIOD_MS as u16, now)
        } else {
            Freshness::Missing
        };
        result.flow_freshness = if result.flow_valid {
            freshness(t.flow_received_us, periods.flow, now)
        } else {
            Freshness::Missing
        };
        result.actuators_freshness = freshness(t.actuators_received_us, periods.actuators, now);
        result.sensors_alive =
            t.last_sensors_message_us != 0 && now - t.last_sensors_message_us <= 3 * 1000 * 1000;
        result.flow_last_edge_age_ms = age_ms(t.last_flow_edge_received_us, now);
        result.scale_age_ms = age_ms(t.scale_received_us, now);
        result.scale_present = self.scale_present_raw(now);
        result.scace_age_ms = age_ms(t.scace_received_us, now);
        result.scace_valid = scace::is_valid(result.scace_connected, t.scace_received_us, now);
        if t.last_flow_edge_received_us == 0
            || now - t.last_flow_edge_received_us > FLOW_SILENCE_MS * 1000
        {
            result.flow_ml_s = 0.0;
        }
        result.screen_uptime_s = (now / 1_000_000) as u32;
        result
    }

    /// Instantané de la capture HF.
    pub fn hf_capture_info(&self) -> hf_capture::Info {
        self.hf.info()
    }

    /// Échantillon `index`, disponible seulement une fois la capture complète.
    pub fn hf_capture_sample(&self, index: u16) -> Option<hf_capture::Sample> {
        if self.hf.complete {
            self.hf.samples.get(usize::from(index)).copied()
        } else {
            None
        }
    }

    /// Les échantillons de la capture complète (vide si elle est active ou
    /// absente).
    pub fn hf_capture_samples(&self) -> &[hf_capture::Sample] {
        if self.hf.complete {
            &self.hf.samples
        } else {
            &[]
        }
    }

    /// Entrée générique pour une boucle d'événements. Le temps passe par
    /// `clock`.
    pub fn handle(&mut self, clock: Clock, input: Input) -> Reply {
        match input {
            Input::Tick => self.tick(clock),
            Input::CanFrame(frame) => self.on_can_frame(clock, &frame),
            Input::ScaleConnection(c) => self.update_scale_connection(clock, c),
            Input::ScaleWeight(w) => self.update_scale_weight(clock, w),
            Input::ScaceConnection(c) => self.update_scace_connection(clock, c),
            Input::ScaceSample(s) => self.update_scace_temperature(clock, s.centi_c, s.ok),
            Input::BoilerNtc {
                a0_raw,
                a1_raw,
                read_ok,
            } => {
                self.on_boiler_ntc_reading(clock, a0_raw, a1_raw, read_ok);
            }
            Input::PressureA2 { raw, read_ok } => self.on_pressure_a2_reading(clock, raw, read_ok),
            Input::TouchReady(r) => self.set_touch_ready(r),
            Input::TouchPress => self.note_touch_press(),
            Input::Network {
                state,
                ipv4_address,
            } => self.update_network_status(state, ipv4_address),
            Input::BootTimeSyncing(s) => self.set_boot_time_syncing(s),
            Input::CredentialsPresent(p) => self.set_credentials_present(p),
            Input::WifiEnabled(e) => self.set_wifi_enabled(e),
            Input::HeapStats {
                free,
                largest,
                minimum,
            } => self.set_heap_stats(free, largest, minimum),
            Input::Action { command, source } => {
                return Reply::Action(self.perform_action(clock, &command, source));
            }
            Input::PutConfig(config) => return Reply::Config(self.put_config(clock, &config)),
            Input::ResetConfig => return Reply::Config(self.reset_config(clock)),
            Input::ForgetNetwork => self.forget_network(),
            Input::Event(kind) => self.note_event(clock, kind),
        }
        Reply::None
    }
}

fn now_ms_of(now_us: i64) -> u64 {
    (now_us / 1000) as u64
}
