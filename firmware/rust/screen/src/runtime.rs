//! Un propriétaire du cœur sur l'exécuteur 0. Les services du cœur 1
//! échangent des messages, jamais des références mutables vers la machine.
use crate::{
    board::ads1115,
    ui::{Ui, presenter::UiRequest},
};
use core::sync::atomic::Ordering::Relaxed;
use screen::{ble, can, network, proxy_service, service};
use screen_core::{
    core::{Action, ActionCommand, ActionSource, Clock, Core, Input, Output},
    snapshot::Snapshot,
};

pub struct Runtime {
    core: Core<service::CoreStore>,
    next_tick_us: u64,
    ads_samples: u32,
}

fn clock() -> Clock {
    Clock {
        mono_us: embassy_time::Instant::now().as_micros() as i64,
        wall_unix_s: network::wall_time(),
    }
}

impl Runtime {
    /// Le stockage du cœur 1 est déjà lancé ; les lectures synchrones du
    /// démarrage peuvent donc attendre NVS sans masquer les interruptions.
    #[allow(
        clippy::large_stack_frames,
        reason = "construction du cœur estimée à 11 Ko ; la pile du cœur 0 est bornée à au moins 16 Ko par le linker"
    )]
    pub fn new(touch_ready: bool) -> Self {
        let mut core = Core::new(service::CoreStore::new(), clock());
        core.set_touch_ready(touch_ready);
        Self {
            core,
            next_tick_us: 0,
            ads_samples: 0,
        }
    }

    pub fn touch_press(&mut self) {
        self.core.note_touch_press();
    }

    /// Une transition Slint peut détruire le bouton maintenu avant son
    /// PointerReleased. Le relâchement physique garde donc sa propre voie.
    pub fn purge_release(&mut self) {
        self.core.perform_action(
            clock(),
            &ActionCommand::new(Action::PurgeRelease),
            ActionSource::Ui,
        );
    }

    fn outputs(&mut self) {
        for output in self.core.drain_outputs() {
            match output {
                Output::Can(frame) => {
                    let _ = can::TX.try_send(frame);
                }
                Output::Ble(command) => {
                    let _ = ble::command(command);
                }
                Output::Radio(request) => network::configure(request),
                Output::ForgetNetwork => network::forget_network(),
                // Le task proxy reçoit déjà les ACK depuis can::task.
                Output::SensorsFlashCtrl(_) => {}
            }
        }
    }

    /// Appelé avant chaque rendu : les requêtes HTTP restent au service du
    /// même état que l'UI, et le pas de chauffe ne dépend pas de la fréquence UI.
    #[allow(
        clippy::large_stack_frames,
        reason = "instantanés et configuration temporaires, estimés à 3 Ko sur une pile d’au moins 16 Ko"
    )]
    pub fn poll(&mut self, ui: &mut Ui) {
        let clock = clock();
        let now_us = clock.mono_us as u64;
        while let Ok(frame) = can::RX.try_receive() {
            self.core.on_can_frame(clock, &frame);
        }
        while let Ok(event) = ble::EVENTS.try_receive() {
            let event_clock = Clock {
                mono_us: event.at_us,
                wall_unix_s: clock.wall_unix_s,
            };
            let input = match event.kind {
                ble::EventKind::ScaleConnected(connected) => Input::ScaleConnection(connected),
                ble::EventKind::Weight(weight) => Input::ScaleWeight(weight),
                ble::EventKind::ScaceConnected(connected) => Input::ScaceConnection(connected),
                ble::EventKind::ScaceSample(sample) => Input::ScaceSample(sample),
            };
            self.core.handle(event_clock, input);
        }
        let samples = ads1115::SAMPLES.load(Relaxed);
        if samples != self.ads_samples {
            self.ads_samples = samples;
            self.core.on_boiler_ntc_reading(
                clock,
                ads1115::raw(0),
                ads1115::raw(1),
                ads1115::NTC_OK.load(Relaxed),
            );
            self.core
                .on_pressure_a2_reading(clock, ads1115::raw(2), ads1115::A2_OK.load(Relaxed));
        }
        while let Ok(progress) = proxy_service::PROGRESS.try_receive() {
            match progress {
                proxy_service::Progress::Uploaded(bytes) | proxy_service::Progress::Sent(bytes) => {
                    self.core.update_flash_progress(bytes)
                }
                // Le réseau libère d'abord le propriétaire flash, puis acquitte
                // FinishUpload. Lever l'exclusion ici permettrait à une écriture
                // NVS de bloquer le cœur avant cette dernière requête réseau.
                proxy_service::Progress::Finished(_) => {}
            }
        }
        if now_us >= self.next_tick_us {
            self.core.tick(clock);
            // esp-alloc expose l'usage global mais pas le plus grand bloc
            // libre : 0 annonce cette mesure indisponible au lieu de l'inventer.
            let heap = esp_alloc::HEAP.stats();
            self.core.set_heap_stats(
                esp_alloc::HEAP.free() as u32,
                0,
                (heap.size - heap.max_usage) as u32,
            );
            self.next_tick_us = now_us + 50_000;
        }
        service::process(&mut self.core, clock);
        let snapshot = self.core.snapshot(clock);
        let config = self.core.config();
        for request in ui.take_requests(now_us, &snapshot, &config) {
            match request {
                UiRequest::Core(input) => ui.feedback(self.core.handle(clock, input)),
                UiRequest::RestartLcd => crate::display::request_restart(),
            }
        }
        self.outputs();
        let snapshot: Snapshot = self.core.snapshot(clock);
        let request = self.core.radio_request();
        network::configure(request);
        ble::configure(
            request.ble,
            self.core.config().scace_enabled,
            snapshot.cycle_state.is_active(),
        );
        ui.update(
            now_us,
            &snapshot,
            &self.core.config(),
            &network::provisioning_ssid(),
            &crate::ui::presenter::clock_text(clock.wall_unix_s),
        );
    }
}
