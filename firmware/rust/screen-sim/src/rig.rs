//! Faux module interne : les sorties CAN du vrai cœur commandent le banc.
//! Aucune action UI n'est simulée en contournant les contrôles de sécurité.
use protocol::messages::{
    NodeIdentityPayload, StatusActuatorsPayload, StatusEnclosurePayload, StatusFlowPayload,
    StatusHeatingPayload,
};
use protocol::{Dest, Message, Node, Version};
use screen_core::{
    config::MemoryConfigStore,
    core::{Clock, Core, Output},
    snapshot::Snapshot,
};

pub struct Rig {
    pub core: Core<MemoryConfigStore>,
    pub now_us: i64,
    pump: u8,
    power: u16,
    pulses: u32,
    weight: f32,
    ntc_a1: i16,
}
impl Rig {
    pub fn new() -> Self {
        let mut best = (f32::MAX, 0);
        for a1 in 5000..27999 {
            let r = screen_core::ntc::boiler_temperature(28000, a1, true);
            let err = (r.temperature_c - 90.0).abs();
            if r.valid && err < best.0 {
                best = (err, a1);
            }
        }
        let core = Core::new(MemoryConfigStore::default(), Clock::new(1_000_000));
        let mut rig = Self {
            core,
            now_us: 1_000_000,
            pump: 0,
            power: 0,
            pulses: 0,
            weight: 0.0,
            ntc_a1: best.1,
        };
        rig.core.set_credentials_present(true);
        rig.core.update_scale_connection(rig.clock(), true);
        rig.core.set_scale_writable(true);
        rig.rx(Message::Pong(NodeIdentityPayload::new(
            Node::Sensors,
            Version::new(0, 4, 0),
            1000,
        )));
        for _ in 0..120 {
            rig.advance();
        }
        rig
    }
    pub fn clock(&self) -> Clock {
        Clock::with_wall(self.now_us, 1_791_552_000 + self.now_us / 1_000_000)
    }
    fn rx(&mut self, msg: Message) {
        self.core.on_can_frame(
            self.clock(),
            &msg.encode_routed(Node::Sensors, Dest::Screen),
        );
    }
    pub fn advance(&mut self) {
        self.now_us += 50_000;
        for output in self.core.drain_outputs() {
            match output {
                Output::Can(f) => match Message::decode(&f) {
                    Ok(Message::Set(s)) => self.pump = s.pump_pct,
                    Ok(Message::Stop) => self.pump = 0,
                    Ok(Message::SetHeatingPower(p)) => self.power = p.power_permille,
                    Ok(Message::Ping(_)) => self.rx(Message::Pong(NodeIdentityPayload::new(
                        Node::Sensors,
                        Version::new(0, 4, 0),
                        1000,
                    ))),
                    _ => {}
                },
                Output::Ble(screen_core::core::BleCommand::Tare) => self.weight = 0.0,
                _ => {}
            }
        }
        if self.pump > 0 {
            self.weight += 0.075;
            self.pulses += 8;
        }
        self.rx(Message::StatusActuators(StatusActuatorsPayload {
            valve_open: self.pump > 0,
            pump_pct: self.pump,
            lease_remaining_ms: 400,
            continuous_on_ms: 0,
            flags: StatusActuatorsPayload::FLAG_DIMMER_PRESENT
                | StatusActuatorsPayload::FLAG_DIMMER_READY,
        }));
        self.rx(Message::StatusHeating(StatusHeatingPayload {
            heater_on: self.power > 0,
            lease_remaining_ms: 1000,
            power_capable: true,
            fine_power_capable: true,
            window_restart_capable: true,
            power_permille: self.power,
        }));
        self.rx(Message::StatusFlow(StatusFlowPayload {
            pulse_count: self.pulses,
            last_edge_ms: (self.now_us / 1000) as u16,
            flags: 1,
        }));
        // 41,6 °C côté capteurs ; SHT40 à 28,4 °C et 52 %.
        self.rx(Message::StatusEnclosure(StatusEnclosurePayload {
            temperature_raw: 666,
            timestamp_ms: (self.now_us / 1000) as u16,
            flags: StatusEnclosurePayload::FLAG_VALID,
        }));
        self.core
            .on_enclosure_reading(self.clock(), 27_224, 31_281, true);
        self.core
            .on_boiler_ntc_reading(self.clock(), 28000, self.ntc_a1, true);
        self.core.on_pressure_a2_reading(
            self.clock(),
            if self.pump > 0 { 15_167 } else { 3567 },
            true,
        );
        self.core.update_scale_weight(self.clock(), self.weight);
        self.core.tick(self.clock());
    }
    pub fn snapshot(&mut self) -> Snapshot {
        self.core.snapshot(self.clock())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn simulated_machine_can_run_a_real_core_brew() {
        let mut rig = Rig::new();
        let s = rig.snapshot();
        assert!(
            s.sensors_alive
                && s.scale_present
                && s.brew_temperature_ready
                && s.heating_power_capable
        );
        let reply = rig.core.handle(
            rig.clock(),
            screen_core::core::Input::Action {
                command: screen_core::core::ActionCommand::new(
                    screen_core::core::Action::StartBrew,
                ),
                source: screen_core::core::ActionSource::Ui,
            },
        );
        assert!(matches!(
            reply,
            screen_core::core::Reply::Action(screen_core::core::ActionResult {
                status: screen_core::core::ActionStatus::Ok,
                ..
            })
        ));
        for _ in 0..100 {
            rig.advance();
        }
        assert!(rig.snapshot().cycle_state.is_active());
    }
}
