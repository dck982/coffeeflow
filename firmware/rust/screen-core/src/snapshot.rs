//! Instantané cohérent de tout ce que la machine sait : l'UI, `/telemetry` et
//! le WebSocket le lisent, aucun consommateur ne lit l'état interne champ par
//! champ.

use crate::shot_summary::ShotSummary;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Freshness {
    Fresh,
    Stale,
    #[default]
    Missing,
}

impl Freshness {
    pub const fn text(self) -> &'static str {
        match self {
            Self::Fresh => "fresh",
            Self::Stale => "stale",
            Self::Missing => "missing",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FlashTarget {
    #[default]
    None,
    Screen,
    Sensors,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CycleState {
    #[default]
    Idle,
    ThermalPreheat,
    Filling,
    Preinfusion,
    Brew,
    Rampdown,
    Finished,
    Purge,
}

impl CycleState {
    /// Un cycle d'infusion ou de purge est en cours (pas `Finished`).
    pub const fn is_active(self) -> bool {
        matches!(
            self,
            Self::ThermalPreheat
                | Self::Filling
                | Self::Preinfusion
                | Self::Brew
                | Self::Rampdown
                | Self::Purge
        )
    }

    pub const fn text(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::ThermalPreheat => "thermal_preheat",
            Self::Filling => "filling",
            Self::Preinfusion => "preinfusion",
            Self::Brew => "brew",
            Self::Rampdown => "rampdown",
            Self::Finished => "finished",
            Self::Purge => "purge",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NetworkState {
    #[default]
    Off,
    ApProvisioning,
    StaConnecting,
    StaConnected,
    StaDisconnected,
}

impl NetworkState {
    pub const fn text(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::ApProvisioning => "ap_provisioning",
            Self::StaConnecting => "sta_connecting",
            Self::StaConnected => "sta_connected",
            Self::StaDisconnected => "sta_disconnected",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Snapshot {
    pub pressure_bar: f32,
    /// Ancien champ interne XDB401 ; alias HTTP = chaudière.
    pub temperature_c: f32,
    pub xdb401_temperature_c: f32,
    /// Domaine utilisateur : sonde moins 10,5 °C.
    pub boiler_temperature_c: f32,
    /// Estimation locale, sans offset.
    pub boiler_sensor_temperature_c: f32,
    pub boiler_ntc_a0_raw: i16,
    pub boiler_ntc_a1_raw: i16,
    /// Code ADS1115 de A2 (XDB401 analogique), lu avec la paire NTC.
    pub pressure_a2_raw: i16,
    pub pressure_a2_valid: bool,
    pub boiler_temperature_valid: bool,
    pub boiler_temperature_freshness: Freshness,
    pub boiler_temperature_age_ms: u32,
    pub flow_ml_s: f32,
    pub volume_ml: f32,
    pub flow_pulse_count: u32,
    pub flow_last_edge_age_ms: u32,

    pub weight_g: f32,
    pub scale_connected: bool,
    pub scale_present: bool,
    pub scale_age_ms: u32,

    /// Sonde SCACE (BLE, 10 Hz) : température de l'eau dans le panier.
    /// `scace_valid` exige une mesure correcte reçue depuis moins de 300 ms.
    pub scace_connected: bool,
    pub scace_valid: bool,
    pub scace_temperature_c: f32,
    pub scace_age_ms: u32,

    pub sensors_alive: bool,
    pub pressure_valid: bool,
    pub flow_valid: bool,
    pub valve_open: bool,
    pub dimmer_pct: u8,
    pub pump_pct: u8,
    pub heating_capable: bool,
    pub heating_power_capable: bool,
    /// Les capteurs acceptent le drapeau de nouvelle période.
    pub heating_window_restart_capable: bool,
    pub heater_on: bool,
    /// Consigne calculée par l'écran, pas de 0,1 %.
    pub heating_power_pct: f32,
    /// Dernier écho des capteurs.
    pub heating_power_accepted_pct: f32,
    pub brew_temperature_ready: bool,
    pub heating_requested: bool,
    pub heating_lease_remaining_ms: u16,
    pub heating_freshness: Freshness,
    pub heating_age_ms: u32,
    pub dimmer_ready: bool,
    pub dimmer_valid: bool,
    pub dimmer_error_active: bool,
    pub lockout: bool,
    /// Vanne seule, ouverte par l'action de vidange.
    pub maintenance_valve_open: bool,
    /// Les capteurs annoncent MAINTENANCE_VALVE.
    pub maintenance_valve_capable: bool,
    pub lease_remaining_ms: u16,
    pub continuous_on_ms: u16,

    pub pressure_freshness: Freshness,
    pub flow_freshness: Freshness,
    pub actuators_freshness: Freshness,
    pub pressure_age_ms: u32,
    pub flow_age_ms: u32,
    pub actuators_age_ms: u32,

    pub sensors_version_major: u8,
    pub sensors_version_minor: u8,
    pub sensors_version_patch: u8,
    pub sensors_uptime_s: u32,
    pub sensors_twai_rx_errors: u16,
    pub sensors_twai_tx_errors: u16,
    pub sensors_twai_bus_errors: u32,
    pub touch_ready: bool,
    pub touch_press_count: u32,

    pub network_state: NetworkState,
    /// Ordre réseau (octet de poids faible = premier octet), 0 = aucune
    /// adresse.
    pub ipv4_address: u32,
    /// Bascule Wi-Fi de la session (réglages) : vraie au démarrage, jamais
    /// mémorisée.
    pub wifi_enabled: bool,
    /// Sans identifiants Wi-Fi : point d'accès de provisioning, BLE coupé.
    pub provisioning: bool,
    /// Le BLE a le droit de tourner (faux en provisioning).
    pub ble_enabled: bool,
    pub internal_heap_free: u32,
    pub internal_heap_largest: u32,
    pub internal_heap_minimum: u32,
    pub time_known: bool,
    pub boot_time_syncing: bool,

    pub pressure_raw: u32,
    pub temperature_raw: u16,
    pub xdb401_temperature_raw: u16,

    pub screen_version_major: u8,
    pub screen_version_minor: u8,
    pub screen_version_patch: u8,
    pub screen_uptime_s: u32,
    pub flash_active: bool,
    pub flash_target: FlashTarget,
    pub flash_bytes_done: u32,
    pub flash_bytes_total: u32,
    pub cycle_state: CycleState,
    pub cycle_elapsed_ms: u32,
    pub cycle_phase_elapsed_ms: u32,
    pub cycle_start_weight_g: f32,
    pub cycle_weight_goal: bool,
    pub capture_cooldown: bool,
    pub capture_cooldown_remaining_ms: u32,
    /// Résumé de la dernière infusion, tenu à jour pendant sa capture HF.
    pub shot: ShotSummary,
    pub shot_start_unix_s: i64,
    pub last_shot_available: bool,
    pub last_shot_weight_g: f32,
    pub last_shot_duration_ms: u32,
    pub last_shot_flow_ml_s: f32,
    pub last_shot_unix_s: i64,
}

impl Default for Snapshot {
    fn default() -> Self {
        Self {
            pressure_bar: 0.0,
            temperature_c: 0.0,
            xdb401_temperature_c: 0.0,
            boiler_temperature_c: 0.0,
            boiler_sensor_temperature_c: 0.0,
            boiler_ntc_a0_raw: 0,
            boiler_ntc_a1_raw: 0,
            pressure_a2_raw: 0,
            pressure_a2_valid: false,
            boiler_temperature_valid: false,
            boiler_temperature_freshness: Freshness::Missing,
            boiler_temperature_age_ms: u32::MAX,
            flow_ml_s: 0.0,
            volume_ml: 0.0,
            flow_pulse_count: 0,
            flow_last_edge_age_ms: 0,
            weight_g: 0.0,
            scale_connected: false,
            scale_present: false,
            scale_age_ms: 0,
            scace_connected: false,
            scace_valid: false,
            scace_temperature_c: 0.0,
            scace_age_ms: u32::MAX,
            sensors_alive: false,
            pressure_valid: false,
            flow_valid: false,
            valve_open: false,
            dimmer_pct: 0,
            pump_pct: 0,
            heating_capable: false,
            heating_power_capable: false,
            heating_window_restart_capable: false,
            heater_on: false,
            heating_power_pct: 0.0,
            heating_power_accepted_pct: 0.0,
            brew_temperature_ready: false,
            heating_requested: false,
            heating_lease_remaining_ms: 0,
            heating_freshness: Freshness::Missing,
            heating_age_ms: u32::MAX,
            dimmer_ready: false,
            dimmer_valid: false,
            dimmer_error_active: false,
            lockout: false,
            maintenance_valve_open: false,
            maintenance_valve_capable: false,
            lease_remaining_ms: 0,
            continuous_on_ms: 0,
            pressure_freshness: Freshness::Missing,
            flow_freshness: Freshness::Missing,
            actuators_freshness: Freshness::Missing,
            pressure_age_ms: 0,
            flow_age_ms: 0,
            actuators_age_ms: 0,
            sensors_version_major: 0,
            sensors_version_minor: 0,
            sensors_version_patch: 0,
            sensors_uptime_s: 0,
            sensors_twai_rx_errors: 0,
            sensors_twai_tx_errors: 0,
            sensors_twai_bus_errors: 0,
            touch_ready: false,
            touch_press_count: 0,
            network_state: NetworkState::Off,
            ipv4_address: 0,
            wifi_enabled: true,
            provisioning: false,
            ble_enabled: true,
            internal_heap_free: 0,
            internal_heap_largest: 0,
            internal_heap_minimum: 0,
            time_known: false,
            boot_time_syncing: false,
            pressure_raw: 0,
            temperature_raw: 0,
            xdb401_temperature_raw: 0,
            screen_version_major: 0,
            screen_version_minor: 0,
            screen_version_patch: 0,
            screen_uptime_s: 0,
            flash_active: false,
            flash_target: FlashTarget::None,
            flash_bytes_done: 0,
            flash_bytes_total: 0,
            cycle_state: CycleState::Idle,
            cycle_elapsed_ms: 0,
            cycle_phase_elapsed_ms: 0,
            cycle_start_weight_g: 0.0,
            cycle_weight_goal: false,
            capture_cooldown: false,
            capture_cooldown_remaining_ms: 0,
            shot: ShotSummary::default(),
            shot_start_unix_s: 0,
            last_shot_available: false,
            last_shot_weight_g: 0.0,
            last_shot_duration_ms: 0,
            last_shot_flow_ml_s: 0.0,
            last_shot_unix_s: 0,
        }
    }
}
