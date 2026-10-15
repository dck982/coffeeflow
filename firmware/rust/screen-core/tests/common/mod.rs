//! Banc d'essai du cœur : un faux module capteurs qui répond aux SET et
//! publie ses STATUS, et une horloge qui avance par pas de 50 ms.

#![allow(dead_code)]

use protocol::messages::{
    NodeIdentityPayload, StatusActuatorsPayload, StatusFlowPayload, StatusHeatingPayload,
};
use protocol::{Dest, Frame, Message, Node, Version};
use screen_core::config::{Config, MemoryConfigStore};
use screen_core::core::{Clock, Core, Output};
use screen_core::ntc;

pub fn from_sensors(message: Message) -> Frame {
    message.encode_routed(Node::Sensors, Dest::Screen)
}

/// Code A1 qui donne `user_c` (domaine utilisateur) avec A0 = 28000.
pub fn ntc_a1_for(user_c: f32) -> i16 {
    let mut best = (f32::MAX, 0i16);
    for a1 in 5000..27999i16 {
        let reading = ntc::boiler_temperature(28000, a1, true);
        if !reading.valid {
            continue;
        }
        let error = (reading.temperature_c - user_c).abs();
        if error < best.0 {
            best = (error, a1);
        }
    }
    best.1
}

pub struct Rig {
    pub core: Core<MemoryConfigStore>,
    pub now_us: i64,
    pub outputs: Vec<Output>,
    /// Les capteurs publient leurs STATUS.
    pub sensors_online: bool,
    pub boiler_user_c: f32,
    pub pressure_bar: f32,
    pub pump_pct: u8,
    pub valve_open: bool,
    pub heater_on: bool,
    pub lockout: bool,
    pub dimmer_ready: bool,
    pub power_permille: u16,
    pub sensors_version: Version,
    pub pulses: u32,
    pub wall_unix_s: Option<i64>,
    ntc_a1: i16,
    /// Comme le module capteurs, l'horodatage ne bouge qu'avec une impulsion.
    published_pulses: u32,
    last_edge_ms: u16,
    ticks: u64,
}

impl Rig {
    pub fn new() -> Self {
        let core = Core::new(MemoryConfigStore::default(), Clock::new(1_000_000));
        let mut rig = Self {
            core,
            now_us: 1_000_000,
            outputs: Vec::new(),
            sensors_online: true,
            boiler_user_c: 90.0,
            pressure_bar: 0.3,
            pump_pct: 0,
            valve_open: false,
            heater_on: false,
            lockout: false,
            dimmer_ready: true,
            power_permille: 0,
            sensors_version: Version::new(0, 4, 0),
            pulses: 0,
            wall_unix_s: None,
            ntc_a1: ntc_a1_for(90.0),
            published_pulses: 0,
            last_edge_ms: 1000,
            ticks: 0,
        };
        rig.collect();
        rig
    }

    pub fn clock(&self) -> Clock {
        Clock {
            mono_us: self.now_us,
            wall_unix_s: self.wall_unix_s,
        }
    }

    /// Récupère les sorties du cœur, en faisant répondre le faux module.
    pub fn collect(&mut self) -> Vec<Output> {
        let outputs = self.core.drain_outputs();
        for output in &outputs {
            if let Output::Can(frame) = output
                && let Ok(message) = Message::decode(frame)
            {
                match message {
                    Message::Set(set) => {
                        self.pump_pct = set.pump_pct;
                        self.valve_open = set.pump_pct > 0;
                    }
                    Message::Stop => {
                        self.pump_pct = 0;
                        self.valve_open = false;
                    }
                    Message::SetHeatingPower(power) => self.power_permille = power.power_permille,
                    _ => {}
                }
            }
        }
        self.outputs.extend(outputs.iter().cloned());
        outputs
    }

    pub fn take_outputs(&mut self) -> Vec<Output> {
        self.collect();
        core::mem::take(&mut self.outputs)
    }

    pub fn rx(&mut self, message: Message) {
        let clock = self.clock();
        self.core.on_can_frame(clock, &from_sensors(message));
        self.collect();
    }

    pub fn pong(&mut self) {
        self.rx(Message::Pong(NodeIdentityPayload::new(
            Node::Sensors,
            self.sensors_version,
            1000,
        )));
    }

    pub fn publish_actuators(&mut self) {
        let mut flags = StatusActuatorsPayload::FLAG_DIMMER_PRESENT;
        if self.dimmer_ready {
            flags |= StatusActuatorsPayload::FLAG_DIMMER_READY;
        }
        if self.lockout {
            flags |= StatusActuatorsPayload::FLAG_LOCKOUT;
        }
        self.rx(Message::StatusActuators(StatusActuatorsPayload {
            valve_open: self.valve_open,
            pump_pct: self.pump_pct,
            lease_remaining_ms: 400,
            continuous_on_ms: 0,
            flags,
        }));
    }

    pub fn publish_heating(&mut self) {
        self.rx(Message::StatusHeating(StatusHeatingPayload {
            heater_on: self.heater_on,
            lease_remaining_ms: 1000,
            power_capable: true,
            fine_power_capable: true,
            window_restart_capable: true,
            power_permille: self.power_permille,
        }));
    }

    pub fn publish_flow(&mut self) {
        if self.pulses != self.published_pulses {
            self.published_pulses = self.pulses;
            self.last_edge_ms = (self.now_us / 1000) as u16;
        }
        self.rx(Message::StatusFlow(StatusFlowPayload {
            pulse_count: self.pulses,
            last_edge_ms: self.last_edge_ms,
            flags: 1,
        }));
    }

    pub fn measure(&mut self) {
        let clock = self.clock();
        let a1 = self.ntc_a1;
        self.core.on_boiler_ntc_reading(clock, 28000, a1, true);
        let raw = (self.pressure_bar / 0.00075 + 3167.3) as i16;
        self.core.on_pressure_a2_reading(clock, raw, true);
    }

    /// Avance de `ms` par pas de 50 ms ; tous les 100 ms, les mesures locales
    /// et les STATUS des capteurs arrivent.
    pub fn advance(&mut self, ms: i64) {
        self.ntc_a1 = ntc_a1_for(self.boiler_user_c);
        let steps = ms / 50;
        for _ in 0..steps {
            self.now_us += 50_000;
            self.ticks += 1;
            if self.ticks.is_multiple_of(2) {
                self.measure();
                if self.sensors_online {
                    self.publish_actuators();
                    self.publish_heating();
                    self.publish_flow();
                }
            }
            let clock = self.clock();
            self.core.tick(clock);
            self.collect();
        }
    }

    /// Prépare une machine prête à infuser (capteurs vivants, chaudière à la
    /// consigne depuis plusieurs secondes).
    pub fn ready(&mut self) {
        self.pong();
        self.core.set_credentials_present(true);
        self.advance(5000);
        self.collect();
    }

    pub fn ready_config() -> Config {
        Config {
            brew_preheat_time_s: 0.0,
            filling_time_s: 1,
            preinfusion_time_s: 0,
            target_time_s: 5,
            ..Config::default()
        }
    }

    pub fn snapshot(&mut self) -> screen_core::snapshot::Snapshot {
        let clock = self.clock();
        self.core.snapshot(clock)
    }

    pub fn can_frames(&mut self) -> Vec<Message> {
        self.collect();
        self.outputs
            .iter()
            .filter_map(|o| match o {
                Output::Can(f) => Message::decode(f).ok(),
                _ => None,
            })
            .collect()
    }
}
