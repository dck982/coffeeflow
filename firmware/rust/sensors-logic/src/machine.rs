//! Machine d'état du module capteurs : actionneurs, bail, présence, verrou
//! de 60 s, chauffe, dimmer, publications périodiques, réception FLASH et
//! validation de l'image.
//!
//! Tout le matériel passe par [`Hw`] : le firmware l'implémente avec les
//! broches, l'I2C, le TWAI et la flash ; les tests, avec un faux. La logique
//! tourne dans une seule tâche, donc sans verrou : le C++ en avait besoin
//! pour sa chauffe (réception CAN et tâche de sécurité), ici une trame et un
//! tick ne s'exécutent jamais en même temps.

use protocol::messages::{
    DimmerCommand, DimmerCommandPayload, FlashCtrlPayload, LogPayload, MaintenanceValvePayload,
    NodeIdentityPayload, ReqStatusPayload, SetHeatingPayload, SetHeatingPowerPayload, SetPayload,
    StatusActuatorsPayload, StatusFlowPayload, StatusHeatingPayload, StatusPressurePayload,
};
use protocol::version::HEATING_PROTOCOL_CONFIRMATION_TOKEN;
use protocol::{Dest, FIRMWARE_VERSION, Frame, LogCode, LogSeverity, Message, MessageType, Node};

use crate::flash_rx::{
    BeginOutcome, BlockStartOutcome, DataOutcome, EndOutcome, FlashReceiver, FlashSink,
};
use crate::flow::FlowCounter;
use crate::heating_pwm::HeatingPwm;
use crate::zero_guard::DimmerZeroGuard;

// Registres du DimmerLink. 0x10 est la seule écriture du cycle d'infusion.
// 0x00 (STATUS, bit0 = READY, bit1 = ERROR) est la source de vérité de l'état,
// pas le registre d'erreur seul, qui peut contenir des codes non documentés.
// La commande 0x03 du registre COMMAND bascule le dimmer en UART et l'écrit
// dans son EEPROM : ce firmware n'écrit que RESET (0x01) et RECALIBRATE
// (0x02).
pub const DIMMER_ADDR: u16 = 0x50;
pub const DIMMER_REG_STATUS: u8 = 0x00;
pub const DIMMER_REG_COMMAND: u8 = 0x01;
pub const DIMMER_REG_ERROR: u8 = 0x02;
pub const DIMMER_REG_LEVEL: u8 = 0x10;
/// Informatif seulement : observée à 0 en continu sur ce module, même prêt.
pub const DIMMER_REG_FREQ: u8 = 0x20;
pub const DIMMER_CMD_RESET: u8 = 0x01;
pub const DIMMER_CMD_RECALIBRATE: u8 = 0x02;

/// Relecture à vide de l'état du dimmer, pour que les drapeaux de
/// `STATUS_ACTUATORS` restent à jour même sans `SET` récent.
pub const DIMMER_HEALTH_PERIOD_US: i64 = 2000 * 1000;
/// Marge avant l'unique essai de repli `RECALIBRATE` si le module reste
/// « pas prêt » : il calibre tout seul à la mise sous tension du secteur.
pub const DIMMER_AUTO_RECALIBRATE_DELAY_US: i64 = 20 * 1000 * 1000;
/// Plancher des publications `STATUS_PRESSURE`, pour qu'une requête mal
/// formée ne sature pas le bus.
pub const PRESSURE_PERIOD_FLOOR_MS: u16 = 100;

pub const LEASE_DEFAULT_US: i64 = 500 * 1000;
pub const PRESENCE_SILENCE_BEFORE_CHECK_US: i64 = 1500 * 1000;
pub const PRESENCE_PING_INTERVAL_US: i64 = 500 * 1000;
pub const PRESENCE_CHECK_BAIL_US: i64 = 1500 * 1000;
pub const RUNTIME_LOCKOUT_US: i64 = 60 * 1000 * 1000;
pub const RUNTIME_REARM_GAP_US: i64 = 2000 * 1000;
pub const TWAI_COUNTERS_LOG_PERIOD_US: i64 = 5000 * 1000;
pub const TICK_PERIOD_US: i64 = 100 * 1000;
pub const HEATING_MAX_DURATION_MS: u16 = 30000;
pub const HEATING_POWER_LEASE_MS: u16 = 1500;
/// Image non confirmée par l'écran : retour en arrière au bout de ce délai.
pub const OTA_VALIDATION_TIMEOUT_US: i64 = 30 * 1000 * 1000;

/// Motifs de fermeture de la vanne de maintenance, `arg16` du LOG.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum MaintenanceValveClose {
    Command = 0,
    Duration = 1,
    BrewSet = 2,
    Safety = 3,
}

/// Compteurs d'erreur du contrôleur CAN, seul diagnostic de câblage sans
/// oscilloscope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TwaiCounters {
    pub rx_errors: u8,
    pub tx_errors: u8,
    pub bus_errors: u32,
}

/// Le matériel vu par la logique.
pub trait Hw: FlashSink {
    /// Microsecondes depuis le démarrage.
    fn now_us(&self) -> i64;
    /// Vanne : sortie haute = ouverte.
    fn set_valve(&mut self, open: bool);
    /// Chauffe : `true` = SSR actif (broche basse).
    fn set_heater(&mut self, on: bool);
    /// Écrit `[reg, value]` sur le dimmer. `false` si l'adresse n'acquitte pas.
    fn dimmer_write(&mut self, reg: u8, value: u8) -> bool;
    /// Lit un registre du dimmer (écriture du pointeur puis lecture en
    /// redémarrage de bus). `None` si la transaction échoue.
    fn dimmer_read(&mut self, reg: u8) -> Option<u8>;
    /// Met la trame en file d'émission. Une file pleine la perd.
    fn send(&mut self, frame: Frame);
    fn flow(&self) -> FlowCounter;
    fn twai_counters(&mut self) -> Option<TwaiCounters>;
    /// Écrit le verrou 60 s en mémoire RTC persistante.
    fn store_lockout(&mut self, active: bool);
    /// `esp_ota_mark_app_valid_cancel_rollback` : l'image courante passe
    /// `VALID`.
    fn ota_mark_valid(&mut self) -> bool;
}

/// Ce que la boucle du firmware doit faire après un message ou un tick : le
/// temps de laisser partir le LOG (50 ms), puis redémarrer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    None,
    Reboot,
    /// Invalide l'image courante puis redémarre : le bootloader revient sur
    /// l'image précédente.
    RollbackReboot,
}

#[derive(Debug, Clone, Copy)]
pub struct Config {
    /// Verrou 60 s relu de la mémoire RTC (déjà remis à zéro au power-on).
    pub lockout_active: bool,
    /// `Some(état otadata brut)` si l'image tourne en `NEW` ou
    /// `PENDING_VERIFY`.
    pub ota_pending: Option<u32>,
    /// `false` (feature `no-validate`) : la confirmation de l'écran est
    /// ignorée et la carte redémarre au bout de 30 s.
    pub validate: bool,
}

pub struct Machine<H: Hw> {
    pub hw: H,
    validate: bool,

    lockout_active: bool,

    valve_open: bool,
    maintenance_valve: bool,
    heater_on: bool,
    heating_deadline_us: i64,
    heating_power_mode: bool,
    heating_power_permille: u16,
    pwm: HeatingPwm,
    pump_pct: u8,
    zero_guard: DimmerZeroGuard,
    dimmer_present: bool,
    dimmer_ready: bool,
    dimmer_error_active: bool,
    /// 0 = pas de bail actif.
    lease_deadline_us: i64,

    /// 0 = pas de marche en cours.
    run_start_us: i64,
    /// 0 = pas actuellement à l'arrêt.
    off_since_us: i64,

    last_presence_rx_us: i64,
    presence_check_started_us: i64,
    last_own_ping_us: i64,
    presence_probe_count: u8,
    /// État de repos correct : carte seule sur la table.
    presence_lost: bool,

    ota_pending_verify: bool,
    ota_boot_state: u32,
    ota_pending_since_us: i64,
    ota_roundtrip_confirmed: bool,
    ota_screen_confirmed: bool,

    pressure_period_ms: u16,
    pressure_last_sent_us: i64,
    flow_period_ms: u16,
    flow_last_sent_us: i64,
    actuators_period_ms: u16,
    actuators_last_sent_us: i64,
    heating_period_ms: u16,
    heating_last_sent_us: i64,

    twai_last_log_us: i64,
    dimmer_health_last_us: i64,
    dimmer_recalibrate_sent: bool,

    flash: FlashReceiver,
    action: Action,
}

impl<H: Hw> Machine<H> {
    pub fn new(hw: H, cfg: Config) -> Self {
        let now = hw.now_us();
        Self {
            hw,
            validate: cfg.validate,
            lockout_active: cfg.lockout_active,
            valve_open: false,
            maintenance_valve: false,
            heater_on: false,
            heating_deadline_us: 0,
            heating_power_mode: false,
            heating_power_permille: 0,
            pwm: HeatingPwm::new(),
            pump_pct: 0,
            zero_guard: DimmerZeroGuard::new(),
            dimmer_present: false,
            dimmer_ready: false,
            dimmer_error_active: false,
            lease_deadline_us: 0,
            run_start_us: 0,
            off_since_us: 0,
            // En attente, pas encore vu de pair.
            last_presence_rx_us: now - PRESENCE_SILENCE_BEFORE_CHECK_US,
            presence_check_started_us: 0,
            last_own_ping_us: 0,
            presence_probe_count: 0,
            presence_lost: true,
            ota_pending_verify: cfg.ota_pending.is_some(),
            ota_boot_state: cfg.ota_pending.unwrap_or(0),
            ota_pending_since_us: now,
            ota_roundtrip_confirmed: false,
            ota_screen_confirmed: false,
            pressure_period_ms: 0,
            pressure_last_sent_us: 0,
            flow_period_ms: 0,
            flow_last_sent_us: 0,
            actuators_period_ms: 0,
            actuators_last_sent_us: 0,
            heating_period_ms: 0,
            heating_last_sent_us: 0,
            twai_last_log_us: 0,
            dimmer_health_last_us: 0,
            dimmer_recalibrate_sent: false,
            flash: FlashReceiver::new(),
            action: Action::None,
        }
    }

    /// Identité au démarrage : le PING versionné informe aussitôt l'écran, et
    /// le PONG reçu participe à la validation d'une image OTA.
    pub fn start(&mut self) {
        self.send_ping(Dest::Screen);
        self.send_log(LogCode::Boot, LogSeverity::Info, 0, 0);
        if self.ota_pending_verify {
            self.send_log(
                LogCode::OtaPendingVerify,
                LogSeverity::Info,
                0,
                self.ota_boot_state,
            );
        }
    }

    /// Dernier message du démarrage, une fois I2C et débitmètre en place.
    pub fn ready(&mut self) {
        self.send_log(LogCode::Ready, LogSeverity::Info, 0, 0);
    }

    /// Redémarrage demandé, à exécuter par le firmware après 50 ms.
    pub fn take_action(&mut self) -> Action {
        core::mem::replace(&mut self.action, Action::None)
    }

    pub fn lockout_active(&self) -> bool {
        self.lockout_active
    }

    // ---------------------------------------------------------------- envoi

    fn send_message(&mut self, message: Message, dest: Dest) {
        let frame = message.encode_routed(Node::Sensors, dest);
        self.hw.send(frame);
    }

    fn uptime_s(&self) -> u32 {
        (self.hw.now_us() / 1_000_000) as u32
    }

    fn send_log(&mut self, code: LogCode, severity: LogSeverity, arg16: u16, arg32: u32) {
        let payload = LogPayload {
            code: code as u8,
            severity: severity as u8,
            arg16,
            arg32,
        };
        self.send_message(Message::Log(payload), Dest::Broadcast);
    }

    fn send_pong(&mut self) {
        let identity = NodeIdentityPayload::new(Node::Sensors, FIRMWARE_VERSION, self.uptime_s());
        self.send_message(Message::Pong(identity), Dest::Screen);
    }

    fn send_ping(&mut self, dest: Dest) {
        let identity = NodeIdentityPayload::new(Node::Sensors, FIRMWARE_VERSION, self.uptime_s());
        self.send_message(Message::Ping(identity), dest);
    }

    fn send_status_actuators(&mut self) {
        let t = self.hw.now_us();
        let lease_remaining = if self.lease_deadline_us > t {
            (self.lease_deadline_us - t) / 1000
        } else {
            0
        };
        let continuous_ms = if self.run_start_us != 0 {
            (t - self.run_start_us) / 1000
        } else {
            0
        };
        let mut flags = StatusActuatorsPayload::FLAG_MAINTENANCE_VALVE_CAPABLE;
        if self.lockout_active {
            flags |= StatusActuatorsPayload::FLAG_LOCKOUT;
        }
        if self.dimmer_ready {
            flags |= StatusActuatorsPayload::FLAG_DIMMER_READY;
        }
        if self.dimmer_present {
            flags |= StatusActuatorsPayload::FLAG_DIMMER_PRESENT;
        }
        if self.dimmer_error_active {
            flags |= StatusActuatorsPayload::FLAG_DIMMER_ERROR;
        }
        if self.maintenance_valve {
            flags |= StatusActuatorsPayload::FLAG_MAINTENANCE_VALVE_OPEN;
        }
        let payload = StatusActuatorsPayload {
            valve_open: self.valve_open,
            pump_pct: self.pump_pct,
            lease_remaining_ms: lease_remaining.min(0xFFFF) as u16,
            continuous_on_ms: continuous_ms.min(0xFFFF) as u16,
            flags,
        };
        self.send_message(Message::StatusActuators(payload), Dest::Screen);
    }

    fn send_status_heating(&mut self) {
        let now = self.hw.now_us();
        let remaining = if self.heating_deadline_us > now {
            (self.heating_deadline_us - now) / 1000
        } else {
            0
        };
        let payload = StatusHeatingPayload {
            heater_on: self.heater_on,
            lease_remaining_ms: remaining as u16,
            power_capable: true,
            fine_power_capable: true,
            window_restart_capable: true,
            power_permille: if self.heating_power_mode {
                self.heating_power_permille
            } else {
                0
            },
        };
        self.send_message(Message::StatusHeating(payload), Dest::Screen);
    }

    /// Aucun capteur de pression branché : la valeur reste invalide
    /// (drapeaux à 0), publiée telle quelle si un REQSTATUS la demande. Une
    /// sonde rebranchée sur ce module se lirait ici, juste avant l'envoi.
    fn send_status_pressure(&mut self) {
        let payload = StatusPressurePayload {
            timestamp_ms: ((self.hw.now_us() / 1000) & 0xFFFF) as u16,
            ..StatusPressurePayload::default()
        };
        self.send_message(Message::StatusPressure(payload), Dest::Screen);
    }

    fn send_status_flow(&mut self) {
        let flow = self.hw.flow();
        let payload = StatusFlowPayload {
            pulse_count: flow.pulse_count,
            last_edge_ms: ((flow.last_edge_us / 1000) & 0xFFFF) as u16,
            // Bit 0 toujours à 1 : une entrée GPIO ne permet pas de détecter
            // l'absence du débitmètre.
            flags: 0x01,
        };
        self.send_message(Message::StatusFlow(payload), Dest::Screen);
    }

    fn send_flash_ack(&mut self, ctrl: FlashCtrlPayload) {
        self.send_message(Message::FlashCtrl(ctrl), Dest::Screen);
    }

    // -------------------------------------------------------------- sorties

    fn apply_valve(&mut self) {
        self.hw.set_valve(self.valve_open);
    }

    fn force_heating_off(&mut self) {
        self.heater_on = false;
        self.heating_deadline_us = 0;
        self.heating_power_mode = false;
        self.heating_power_permille = 0;
        self.pwm.reset();
        self.hw.set_heater(false);
    }

    fn end_maintenance_valve(&mut self, reason: MaintenanceValveClose) {
        if !self.maintenance_valve {
            return;
        }
        self.maintenance_valve = false;
        self.send_log(
            LogCode::MaintenanceValveClosed,
            LogSeverity::Info,
            reason as u16,
            0,
        );
    }

    /// Coupe les actionneurs immédiatement, sans toucher au verrou lui-même.
    fn force_actuators_off(&mut self) {
        self.end_maintenance_valve(MaintenanceValveClose::Safety);
        self.valve_open = false;
        self.pump_pct = 0;
        self.lease_deadline_us = 0;
        self.apply_valve();
        self.apply_dimmer(0);
    }

    // ------------------------------------------------------------ dimmer

    /// Relit STATUS puis, si besoin, le registre d'erreur, et la fréquence
    /// secteur à titre informatif. Utilisée après chaque écriture et à vide
    /// toutes les 2 s, pour que `present` / `ready` / `error` reflètent la
    /// dernière transaction réelle, pas un état supposé.
    fn read_dimmer_health(&mut self) {
        let Some(status) = self.hw.dimmer_read(DIMMER_REG_STATUS) else {
            self.dimmer_present = false;
            self.dimmer_ready = false;
            self.dimmer_error_active = false;
            self.send_log(LogCode::I2cError, LogSeverity::Error, DIMMER_ADDR, 0);
            return;
        };
        self.dimmer_present = true;
        self.dimmer_ready = status & 0x01 != 0;
        self.dimmer_error_active = status & 0x02 != 0;

        if !self.dimmer_ready {
            self.send_log(LogCode::DimmerCalibrating, LogSeverity::Warn, 0, 0);
        }
        if self.dimmer_error_active {
            let arg = self
                .hw
                .dimmer_read(DIMMER_REG_ERROR)
                .map_or(0xFFFF, u16::from);
            self.send_log(LogCode::DimmerError, LogSeverity::Error, arg, 0);
        }
        // Informatif : journalisée pour diagnostic, jamais utilisée pour
        // piloter un drapeau.
        let arg = self
            .hw
            .dimmer_read(DIMMER_REG_FREQ)
            .map_or(0xFFFF, u16::from);
        self.send_log(LogCode::DimmerMainsFreq, LogSeverity::Debug, arg, 0);
    }

    /// COMMAND = RECALIBRATE. Non fatal si le module n'est pas encore là.
    fn dimmer_recalibrate(&mut self) {
        if !self
            .hw
            .dimmer_write(DIMMER_REG_COMMAND, DIMMER_CMD_RECALIBRATE)
        {
            log::warn!("dimmer recalibrate échec");
        }
    }

    fn on_dimmer_command(&mut self, payload: DimmerCommandPayload) {
        // Une calibration ou une réinitialisation ne coexiste jamais avec
        // une demande de pompe encore active.
        self.force_actuators_off();
        let command = match payload.command {
            DimmerCommand::Reset => DIMMER_CMD_RESET,
            DimmerCommand::Recalibrate => DIMMER_CMD_RECALIBRATE,
        };
        if !self.hw.dimmer_write(DIMMER_REG_COMMAND, command) {
            log::warn!("dimmer command {command:#04x} échec");
            self.send_log(LogCode::I2cError, LogSeverity::Error, DIMMER_ADDR, 0);
        }
        self.read_dimmer_health();
        self.send_status_actuators();
    }

    /// Écrit le niveau de la pompe puis relit la santé du dimmer.
    fn apply_dimmer(&mut self, level: u8) -> bool {
        if !self.hw.dimmer_write(DIMMER_REG_LEVEL, level) {
            if self.dimmer_present {
                self.send_log(LogCode::I2cError, LogSeverity::Error, DIMMER_ADDR, 0);
            }
            self.dimmer_present = false;
            self.dimmer_ready = false;
            self.dimmer_error_active = false;
            return false;
        }
        self.read_dimmer_health();
        true
    }

    // --------------------------------------------------------- réception

    fn mark_presence(&mut self) {
        let was_lost = self.presence_lost;
        self.last_presence_rx_us = self.hw.now_us();
        self.presence_check_started_us = 0;
        self.presence_probe_count = 0;
        self.presence_lost = false;
        if was_lost {
            log::info!("présence retrouvée");
        }
    }

    /// Trame reçue du bus. On n'écoute que ce qui est adressé aux capteurs
    /// ou en broadcast, et qui vient de l'écran.
    pub fn on_frame(&mut self, frame: &Frame) {
        let Ok(id) = frame.can_id() else { return };
        if id.dest != Dest::Broadcast && id.dest != Dest::Sensors {
            return;
        }
        if id.src != Node::Screen {
            return;
        }
        // Toute trame attribuable à l'écran remet la présence à zéro. Le PING
        // ne sert donc qu'à sonder un vrai silence.
        self.mark_presence();

        let p = frame.payload();
        match id.message_type {
            // PING et PONG ne lisent pas leur charge : un PING de n'importe
            // quel DLC reçoit son PONG.
            MessageType::Ping => {
                self.mark_presence();
                self.send_pong();
            }
            MessageType::Pong => {
                self.mark_presence();
                self.ota_roundtrip_confirmed = true;
            }
            MessageType::Reset => {
                self.send_log(LogCode::RebootRequested, LogSeverity::Info, 0, 0);
                self.action = Action::Reboot;
            }
            MessageType::Stop => {
                self.force_heating_off();
                self.force_actuators_off();
                self.send_status_actuators();
                self.send_status_heating();
            }
            MessageType::Set => {
                if let Ok(payload) = SetPayload::unpack(p) {
                    self.on_set(payload);
                }
            }
            MessageType::SetHeating => {
                if let Ok(payload) = SetHeatingPayload::unpack(p) {
                    self.on_set_heating(payload);
                }
            }
            MessageType::SetHeatingPower => {
                if let Ok(payload) = SetHeatingPowerPayload::unpack(p) {
                    self.on_set_heating_power(payload);
                }
            }
            MessageType::MaintenanceValve => {
                if let Ok(payload) = MaintenanceValvePayload::unpack(p) {
                    self.on_maintenance_valve(payload);
                }
            }
            MessageType::ConfirmSensorsOta => self.on_confirm(p),
            MessageType::DimmerCommand => {
                if let Ok(payload) = DimmerCommandPayload::unpack(p) {
                    self.on_dimmer_command(payload);
                }
            }
            MessageType::ReqStatus => {
                if let Ok(payload) = ReqStatusPayload::unpack(p) {
                    self.on_reqstatus(payload);
                }
            }
            MessageType::FlashCtrl => {
                if let Ok(payload) = FlashCtrlPayload::unpack(p) {
                    self.on_flash_ctrl(payload);
                }
            }
            MessageType::FlashData => self.on_flash_data(p),
            // STATUS_* et LOG venant du bus : rien à en faire.
            _ => {}
        }
    }

    fn on_confirm(&mut self, p: &[u8]) {
        if !self.validate {
            return;
        }
        // L'écran confirme la nouvelle image : aller-retour PING/PONG vu,
        // publications des deux états demandées, pompe et vanne au repos, et
        // la chauffe ou bien arrêtée, ou bien régulée avec un bail actif
        // (jamais une impulsion diagnostique en cours).
        if p.len() == 1
            && p[0] == HEATING_PROTOCOL_CONFIRMATION_TOKEN
            && self.ota_roundtrip_confirmed
            && self.heating_period_ms != 0
            && self.actuators_period_ms != 0
            && (!self.heater_on
                || (self.heating_power_mode && self.heating_deadline_us > self.hw.now_us()))
            && !self.valve_open
            && self.pump_pct == 0
        {
            self.ota_screen_confirmed = true;
        }
    }

    fn on_set(&mut self, payload: SetPayload) {
        if self.flash.active() {
            return;
        }
        if self.lockout_active {
            self.send_log(LogCode::CommandRefusedLocked, LogSeverity::Warn, 0, 0);
            self.send_status_actuators();
            return;
        }
        if payload.pump_pct > 100 {
            return;
        }
        // SET reste l'intention d'infusion : il reprend la main sur la
        // maintenance.
        self.end_maintenance_valve(MaintenanceValveClose::BrewSet);
        // Le niveau est l'intention reçue ; la vanne s'ouvre avec la pompe.
        self.valve_open = payload.pump_pct > 0;
        self.pump_pct = payload.pump_pct;
        self.apply_valve();
        self.apply_dimmer(if self.valve_open { self.pump_pct } else { 0 });
        let ttl_us = if payload.ttl_ms == 0 {
            LEASE_DEFAULT_US
        } else {
            i64::from(payload.ttl_ms) * 1000
        };
        self.lease_deadline_us = self.hw.now_us() + ttl_us;
        self.send_status_actuators();
    }

    /// Vidange : vanne seule, pompe forcée à 0, chauffe coupée pour toute la
    /// durée. Refusée pendant une infusion ; une nouvelle ouverture repart de
    /// la durée reçue, sans cumul.
    fn on_maintenance_valve(&mut self, payload: MaintenanceValvePayload) {
        if !payload.open {
            if self.maintenance_valve {
                self.end_maintenance_valve(MaintenanceValveClose::Command);
                self.force_actuators_off();
            }
            self.send_status_actuators();
            return;
        }
        if self.flash.active() || self.presence_lost {
            self.send_status_actuators();
            return;
        }
        if self.lockout_active {
            self.send_log(LogCode::CommandRefusedLocked, LogSeverity::Warn, 0, 0);
            self.send_status_actuators();
            return;
        }
        if !self.maintenance_valve && (self.valve_open || self.pump_pct > 0) {
            self.send_status_actuators();
            return;
        }
        self.force_heating_off();
        self.send_status_heating();
        self.maintenance_valve = true;
        self.pump_pct = 0;
        self.valve_open = true;
        self.apply_dimmer(0);
        self.apply_valve();
        self.lease_deadline_us = self.hw.now_us() + i64::from(payload.duration_ms) * 1000;
        self.send_log(
            LogCode::MaintenanceValveOpened,
            LogSeverity::Info,
            0,
            u32::from(payload.duration_ms),
        );
        self.send_status_actuators();
    }

    fn on_set_heating(&mut self, payload: SetHeatingPayload) {
        if !payload.on {
            self.force_heating_off();
        } else if !self.heater_on
            && !self.heating_power_mode
            && !self.lockout_active
            && !self.presence_lost
            && !self.maintenance_valve
            && !self.flash.active()
            && payload.duration_ms > 0
            && payload.duration_ms <= HEATING_MAX_DURATION_MS
        {
            self.heater_on = true;
            self.heating_deadline_us = self.hw.now_us() + i64::from(payload.duration_ms) * 1000;
            self.hw.set_heater(true);
        }
        self.send_status_heating();
    }

    /// Avance la modulation et la sortie. `true` si la sortie a changé.
    fn apply_heating_pwm(&mut self, now: i64) -> bool {
        let on = self.pwm.tick(now);
        if on == self.heater_on {
            return false;
        }
        self.heater_on = on;
        self.hw.set_heater(on);
        true
    }

    fn on_set_heating_power(&mut self, payload: SetHeatingPowerPayload) {
        if payload.lease_ms != HEATING_POWER_LEASE_MS {
            return;
        }
        if payload.power_permille == 0 {
            if self.heating_power_mode || (self.pwm.active() && !self.heater_on) {
                let now = self.hw.now_us();
                self.pwm.set_power(now, 0);
                self.heating_power_permille = 0;
                if self.heating_power_mode {
                    self.heating_deadline_us = now + i64::from(payload.lease_ms) * 1000;
                }
                self.apply_heating_pwm(now);
            } else {
                self.force_heating_off();
            }
        } else if !self.lockout_active
            && !self.presence_lost
            && !self.maintenance_valve
            && !self.flash.active()
            && (self.heating_power_mode || !self.heater_on)
        {
            let now = self.hw.now_us();
            if payload.restart_window {
                self.pwm.restart(now, payload.power_permille);
            } else {
                self.pwm.set_power(now, payload.power_permille);
            }
            self.heating_power_mode = true;
            self.heating_power_permille = payload.power_permille;
            self.heating_deadline_us = now + i64::from(payload.lease_ms) * 1000;
            self.apply_heating_pwm(now);
        }
        self.send_status_heating();
    }

    fn on_reqstatus(&mut self, payload: ReqStatusPayload) {
        match payload.target() {
            Some(MessageType::StatusPressure) => {
                self.pressure_period_ms = if payload.period_ms == 0 {
                    0
                } else {
                    payload.period_ms.max(PRESSURE_PERIOD_FLOOR_MS)
                };
                // Publie dès le prochain tick, sans attendre une période.
                self.pressure_last_sent_us = 0;
            }
            Some(MessageType::StatusFlow) => {
                self.flow_period_ms = payload.period_ms;
                self.flow_last_sent_us = 0;
            }
            Some(MessageType::StatusActuators) => {
                self.actuators_period_ms = payload.period_ms;
                self.actuators_last_sent_us = 0;
            }
            Some(MessageType::StatusHeating) => {
                self.heating_period_ms = payload.period_ms;
                self.heating_last_sent_us = 0;
            }
            _ => {}
        }
    }

    // -------------------------------------------------------------- flash

    fn on_flash_ctrl(&mut self, ctrl: FlashCtrlPayload) {
        match ctrl {
            FlashCtrlPayload::Begin { image_size } => self.on_flash_begin(image_size),
            FlashCtrlPayload::End { image_crc32 } => self.on_flash_end(image_crc32),
            FlashCtrlPayload::Abort => {
                if self.flash.abort("ABORT reçu") {
                    self.send_log(LogCode::FlashFailed, LogSeverity::Warn, 0, 0);
                }
            }
            // BLOCK_ACK n'est émis que par le récepteur (nous) : rien à faire
            // si on le reçoit.
            FlashCtrlPayload::BlockAck { .. } => {}
            FlashCtrlPayload::BlockStart {
                block_number,
                block_crc16,
            } => match self.flash.block_start(block_number, block_crc16) {
                BlockStartOutcome::Replay {
                    block_number,
                    block_crc16,
                } => {
                    self.send_flash_ack(FlashCtrlPayload::BlockAck {
                        block_number,
                        block_crc16,
                    });
                }
                BlockStartOutcome::Idle | BlockStartOutcome::Next | BlockStartOutcome::Aborted => {}
            },
        }
    }

    fn on_flash_begin(&mut self, image_size: u32) {
        self.force_heating_off();
        self.force_actuators_off();
        self.send_status_heating();
        self.send_status_actuators();
        match self.flash.begin(&mut self.hw, image_size) {
            BeginOutcome::Started => {
                self.send_log(LogCode::FlashBegin, LogSeverity::Info, 0, image_size);
                self.send_flash_ack(FlashCtrlPayload::BlockAck {
                    block_number: 0,
                    block_crc16: 0,
                });
            }
            BeginOutcome::Refused | BeginOutcome::EraseFailed => {
                self.send_log(LogCode::FlashFailed, LogSeverity::Error, 0, image_size);
            }
        }
    }

    fn on_flash_data(&mut self, bytes: &[u8]) {
        match self.flash.data(&mut self.hw, bytes) {
            DataOutcome::Nothing => {}
            DataOutcome::WriteFailed { bytes_written } => {
                self.send_log(LogCode::FlashFailed, LogSeverity::Error, 0, bytes_written);
            }
            DataOutcome::BlockWritten {
                block_number,
                block_crc16,
                bytes_written,
            } => {
                self.send_log(LogCode::FlashProgress, LogSeverity::Debug, 0, bytes_written);
                self.send_flash_ack(FlashCtrlPayload::BlockAck {
                    block_number,
                    block_crc16,
                });
            }
        }
    }

    fn on_flash_end(&mut self, expected_crc32: u32) {
        match self.flash.end(&mut self.hw, expected_crc32) {
            EndOutcome::NotActive | EndOutcome::CommitFailed => {
                self.send_log(LogCode::FlashFailed, LogSeverity::Error, 0, 0);
            }
            EndOutcome::Mismatch { bytes_written } => {
                self.send_log(LogCode::FlashFailed, LogSeverity::Error, 0, bytes_written);
            }
            EndOutcome::Done => {
                self.send_log(LogCode::FlashDone, LogSeverity::Info, 0, 0);
                // L'outil n'attend qu'un FLASH_CTRL de sous-commande END en
                // retour pour considérer le transfert confirmé.
                self.send_flash_ack(FlashCtrlPayload::End { image_crc32: 0 });
                self.action = Action::Reboot;
            }
        }
    }

    // ---------------------------------------------------------------- ticks

    /// Une passe de la tâche de sécurité, toutes les 100 ms.
    pub fn tick_safety(&mut self) {
        self.tick_lease();
        self.tick_presence();
        self.tick_runtime_lockout();
        self.tick_twai_counters();
        self.tick_ota_validation();
        self.tick_flow();
        self.tick_actuators();
        self.tick_heating();
        self.tick_dimmer_zero_guard();
        self.tick_dimmer_health();
    }

    /// Bail : coupe les actionneurs si le SET le plus récent a expiré.
    fn tick_lease(&mut self) {
        let now = self.hw.now_us();
        if (self.heater_on || self.heating_power_mode)
            && self.heating_deadline_us != 0
            && now >= self.heating_deadline_us
        {
            if self.heating_power_mode {
                self.pwm.set_power(now, 0);
                self.heating_power_mode = false;
                self.heating_power_permille = 0;
                self.heating_deadline_us = 0;
                self.heater_on = false;
                self.hw.set_heater(false);
            } else {
                self.force_heating_off();
            }
            self.send_status_heating();
        }
        if self.lease_deadline_us == 0 {
            return;
        }
        let now = self.hw.now_us();
        if now < self.lease_deadline_us {
            return;
        }
        if !self.valve_open && self.pump_pct == 0 {
            self.lease_deadline_us = 0;
            return;
        }
        if self.maintenance_valve {
            // Fin normale d'une ouverture de maintenance, pas une perte de bail.
            self.end_maintenance_valve(MaintenanceValveClose::Duration);
            self.force_actuators_off();
            self.send_status_actuators();
            return;
        }
        self.force_actuators_off();
        self.send_log(LogCode::LeaseExpired, LogSeverity::Warn, 0, 0);
        self.send_status_actuators();
    }

    /// Après 1,5 s sans aucune trame écran, trois PING espacés de 500 ms
    /// sondent le pair ; sans réponse ni autre trafic dans ce bail, repli
    /// sécurité.
    fn tick_presence(&mut self) {
        let t = self.hw.now_us();
        if t - self.last_presence_rx_us < PRESENCE_SILENCE_BEFORE_CHECK_US {
            return;
        }
        if self.presence_check_started_us == 0 {
            self.presence_check_started_us = t;
            self.last_own_ping_us = 0;
            self.presence_probe_count = 0;
        }
        if self.presence_probe_count < 3
            && (self.presence_probe_count == 0
                || t - self.last_own_ping_us >= PRESENCE_PING_INTERVAL_US)
        {
            self.last_own_ping_us = t;
            self.send_ping(Dest::Broadcast);
            self.presence_probe_count += 1;
        }
        if !self.presence_lost && t - self.presence_check_started_us >= PRESENCE_CHECK_BAIL_US {
            self.presence_lost = true;
            self.force_heating_off();
            self.force_actuators_off();
            self.send_status_heating();
            self.send_log(LogCode::PresenceLost, LogSeverity::Warn, 0, 0);
        }
    }

    /// Plafond de 60 s de marche continue. Le compteur n'est remis à zéro que
    /// par un arrêt d'au moins 2 s : un écran qui commuterait juste avant
    /// l'échéance ne réarme rien. La vanne de maintenance n'y entre pas :
    /// pompe à 0, elle a son propre bail de 30 s.
    fn tick_runtime_lockout(&mut self) {
        let running = self.pump_pct > 0 || (self.valve_open && !self.maintenance_valve);
        let t = self.hw.now_us();

        if running {
            if self.off_since_us != 0 && (t - self.off_since_us) >= RUNTIME_REARM_GAP_US {
                self.run_start_us = t; // coupure assez longue : nouveau cycle
            } else if self.run_start_us == 0 {
                self.run_start_us = t;
            }
            self.off_since_us = 0;

            if !self.lockout_active
                && self.run_start_us != 0
                && (t - self.run_start_us) > RUNTIME_LOCKOUT_US
            {
                let activated_ms = ((t - self.run_start_us) / 1000) as u32;
                self.lockout_active = true;
                self.hw.store_lockout(true);
                self.force_heating_off();
                self.force_actuators_off();
                self.send_log(
                    LogCode::RuntimeLockoutTriggered,
                    LogSeverity::Error,
                    0,
                    activated_ms,
                );
                self.send_status_actuators();
            }
        } else if self.off_since_us == 0 {
            self.off_since_us = t;
        } else if (t - self.off_since_us) >= RUNTIME_REARM_GAP_US {
            self.run_start_us = 0;
        }
    }

    /// Compteurs d'erreur TWAI, republiés périodiquement en LOG.
    fn tick_twai_counters(&mut self) {
        let t = self.hw.now_us();
        if t - self.twai_last_log_us < TWAI_COUNTERS_LOG_PERIOD_US {
            return;
        }
        self.twai_last_log_us = t;
        let Some(c) = self.hw.twai_counters() else {
            return;
        };
        let arg16 = (u16::from(c.rx_errors) << 8) | u16::from(c.tx_errors);
        self.send_log(
            LogCode::TwaiErrorCounters,
            LogSeverity::Debug,
            arg16,
            c.bus_errors,
        );
    }

    /// Une image `NEW` ou `PENDING_VERIFY` qui ne reçoit jamais la
    /// confirmation explicite de l'écran revient en arrière d'elle-même. La
    /// présence ordinaire peut être maintenue par toute trame, mais la
    /// validation exige un aller-retour PING/PONG et des échos sûrs des
    /// actionneurs.
    fn tick_ota_validation(&mut self) {
        if !self.validate {
            // Première installation sans validation : on ne confirme jamais,
            // on redémarre au bout de 30 s, quel que soit l'état de l'image.
            if self.hw.now_us() - self.ota_pending_since_us > OTA_VALIDATION_TIMEOUT_US
                && self.action == Action::None
            {
                self.send_log(LogCode::OtaRollback, LogSeverity::Error, 0, 0);
                self.action = Action::RollbackReboot;
            }
            return;
        }
        if !self.ota_pending_verify {
            return;
        }
        if self.ota_screen_confirmed && self.hw.ota_mark_valid() {
            self.ota_pending_verify = false;
            self.send_log(LogCode::OtaValidated, LogSeverity::Info, 0, 0);
            return;
        }
        if self.hw.now_us() - self.ota_pending_since_us > OTA_VALIDATION_TIMEOUT_US
            && self.action == Action::None
        {
            self.send_log(LogCode::OtaRollback, LogSeverity::Error, 0, 0);
            self.action = Action::RollbackReboot;
        }
    }

    /// Débitmètre : lecture non bloquante du compteur incrémenté par ISR.
    fn tick_flow(&mut self) {
        if self.flow_period_ms == 0 {
            return;
        }
        let t = self.hw.now_us();
        if t - self.flow_last_sent_us < i64::from(self.flow_period_ms) * 1000 {
            return;
        }
        self.send_status_flow();
        self.flow_last_sent_us = t;
    }

    fn tick_actuators(&mut self) {
        if self.actuators_period_ms == 0 {
            return;
        }
        let t = self.hw.now_us();
        if t - self.actuators_last_sent_us < i64::from(self.actuators_period_ms) * 1000 {
            return;
        }
        self.send_status_actuators();
        self.actuators_last_sent_us = t;
    }

    fn tick_heating(&mut self) {
        let now = self.hw.now_us();
        let changed = self.heating_power_mode && self.apply_heating_pwm(now);
        if changed {
            self.send_status_heating();
        }
        if self.heating_period_ms == 0 {
            return;
        }
        let t = self.hw.now_us();
        if t - self.heating_last_sent_us < i64::from(self.heating_period_ms) * 1000 {
            return;
        }
        self.send_status_heating();
        self.heating_last_sent_us = t;
    }

    fn tick_dimmer_zero_guard(&mut self) {
        // En maintenance, la pompe doit rester à zéro : la garde la réaffirme.
        let now = self.hw.now_us();
        let valve_open = self.valve_open && !self.maintenance_valve;
        let mut guard = self.zero_guard;
        guard.tick(now, valve_open, || self.apply_dimmer(0));
        self.zero_guard = guard;
    }

    /// Lecture périodique à vide : sans elle, un dimmer jamais commandé
    /// resterait avec des drapeaux figés à leur valeur initiale.
    fn tick_dimmer_health(&mut self) {
        let t = self.hw.now_us();
        if t - self.dimmer_health_last_us < DIMMER_HEALTH_PERIOD_US {
            return;
        }
        self.dimmer_health_last_us = t;
        self.read_dimmer_health();

        // Repli, une seule fois par épisode « pas prêt » : `present` exigé,
        // pas de repli tant qu'on n'a jamais eu de vraie réponse I2C (câblage
        // ou alimentation, pas un problème de calibration).
        if self.dimmer_present
            && !self.dimmer_ready
            && !self.dimmer_recalibrate_sent
            && t > DIMMER_AUTO_RECALIBRATE_DELAY_US
        {
            self.dimmer_recalibrate_sent = true;
            self.dimmer_recalibrate();
        }
        if self.dimmer_ready {
            // Réarme pour une future coupure et reprise du secteur.
            self.dimmer_recalibrate_sent = false;
        }
    }

    /// Publie `STATUS_PRESSURE` à la période demandée.
    pub fn tick_pressure(&mut self) {
        if self.pressure_period_ms == 0 {
            return;
        }
        let t = self.hw.now_us();
        if t - self.pressure_last_sent_us < i64::from(self.pressure_period_ms) * 1000 {
            return;
        }
        self.send_status_pressure();
        self.pressure_last_sent_us = self.hw.now_us();
    }
}

/// Verrou 60 s au démarrage : `(magic, actif)` à garder en RTC. Seul un
/// démarrage à froid (ou une mémoire RTC jamais initialisée) le lève ; tout
/// autre reset (logiciel, watchdog, brownout...) conserve le verrou posé.
pub fn lockout_after_boot(power_on: bool, magic_ok: bool, active: bool) -> bool {
    if power_on || !magic_ok { false } else { active }
}
