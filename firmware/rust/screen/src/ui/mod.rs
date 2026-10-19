//! UI Slint : plateforme minimale (une fenêtre, le temps d'embassy), boucle de
//! rendu ligne par ligne et routage du toucher. Elle tourne sur le cœur 0,
//! celui de la recopie d'affichage : les interruptions de recopie préemptent le
//! rendu, et Slint n'est pas partagé entre cœurs.

slint::include_modules!();

pub mod presenter;

use alloc::boxed::Box;
use alloc::collections::VecDeque;
use alloc::rc::Rc;
use alloc::vec::Vec;
use core::cell::RefCell;
use core::sync::atomic::{AtomicU32, Ordering::Relaxed};

use embassy_time::{Duration, Instant, Timer};
use slint::ComponentHandle;
use slint::platform::software_renderer::{MinimalSoftwareWindow, RepaintBufferType, Rgb565Pixel};
use slint::platform::{Platform, PointerEventButton, WindowAdapter, WindowEvent};

use crate::board::gt911::{Gt911, Touch};
use crate::board::{Device, H_RES, V_RES};
use crate::display::render::LineToFb;

// Mesures du rendu, lues par le rapport.
/// Rendus effectués, le plus long, et le dernier (en µs).
pub static RENDERS: AtomicU32 = AtomicU32::new(0);
pub static RENDER_MAX_US: AtomicU32 = AtomicU32::new(0);
pub static RENDER_LAST_US: AtomicU32 = AtomicU32::new(0);
pub static TOUCH_PRESSES: AtomicU32 = AtomicU32::new(0);
pub static TOUCH_ERRORS: AtomicU32 = AtomicU32::new(0);

/// Plateforme Slint minimale : une seule fenêtre, et le temps d'embassy.
struct EmbassyPlatform {
    window: Rc<MinimalSoftwareWindow>,
}

impl Platform for EmbassyPlatform {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
        Ok(self.window.clone())
    }

    fn duration_since_start(&self) -> core::time::Duration {
        core::time::Duration::from_micros(Instant::now().as_micros())
    }
}

pub struct Ui {
    window: Rc<MinimalSoftwareWindow>,
    // Garde la fenêtre vivante ; l'UI réelle lui parlera par ses propriétés.
    pub app: AppWindow,
    pub presenter: presenter::Presenter,
    commands: Rc<RefCell<VecDeque<(i32, slint::SharedString)>>>,
}

impl Ui {
    /// Les callbacks déposent des gestes ; seul l'assemblage appelle le cœur.
    pub fn take_requests(
        &mut self,
        now_us: u64,
        snapshot: &screen_core::snapshot::Snapshot,
        config: &screen_core::config::Config,
    ) -> Vec<presenter::UiRequest> {
        let mut requests = Vec::new();
        while let Some((id, payload)) = self.commands.borrow_mut().pop_front() {
            if let Some(request) = self
                .presenter
                .command(id, &payload, now_us, snapshot, config)
            {
                requests.push(request);
            }
        }
        requests
    }

    /// Affiche les priorités avant le prochain rendu, puis une tranche de valeurs.
    pub fn update(
        &mut self,
        now_us: u64,
        snapshot: &screen_core::snapshot::Snapshot,
        config: &screen_core::config::Config,
        provisioning_ssid: &str,
        clock_text: &str,
    ) {
        self.presenter.update(
            &self.app,
            now_us,
            snapshot,
            config,
            provisioning_ssid,
            clock_text,
        );
    }

    pub fn feedback(&mut self, reply: screen_core::core::Reply) {
        self.presenter.feedback(reply);
    }
}

/// Pose la plateforme et affiche la fenêtre.
pub fn init() -> Ui {
    // Le rendu ligne par ligne réécrit le framebuffer affiché : seule la
    // zone modifiée est à redessiner, comme en ReusedBuffer.
    let window = MinimalSoftwareWindow::new(RepaintBufferType::ReusedBuffer);
    window.set_size(slint::PhysicalSize::new(H_RES as u32, V_RES as u32));
    slint::platform::set_platform(Box::new(EmbassyPlatform {
        window: window.clone(),
    }))
    .expect("plateforme Slint déjà posée");
    let app = AppWindow::new().unwrap();
    app.show().unwrap();
    let commands = Rc::new(RefCell::new(VecDeque::new()));
    let queue = commands.clone();
    app.on_command(move |id, payload| queue.borrow_mut().push_back((id, payload)));
    Ui {
        window,
        app,
        presenter: presenter::Presenter::default(),
        commands,
    }
}

/// Boucle de rendu : minuteurs et animations de Slint, tactile toutes les
/// 20 ms, puis rendu de ce qui a changé. Chaque tour se termine par une pause
/// de 10 ms, même après un rendu long : les autres tâches du cœur 0 doivent
/// pouvoir passer. C'est aussi ici que le watchdog RTC est nourri : si le
/// cœur 0 se fige, la carte redémarre.
#[allow(
    clippy::large_stack_frames,
    reason = "futures d'embassy : clippy compte l'état de la future, qui vit dans l'arène de l'exécuteur ou la future de main, pas sur la pile"
)]
pub async fn run(
    mut ui: Ui,
    fb: *mut Rgb565Pixel,
    mut touch: Option<Gt911<Device>>,
    mut runtime: crate::runtime::Runtime,
) -> ! {
    let mut pressed: Option<slint::LogicalPosition> = None;
    let mut iteration = 0u32;
    loop {
        crate::watchdog::feed();
        slint::platform::update_timers_and_animations();

        iteration = iteration.wrapping_add(1);
        if let (Some(touch), true) = (&mut touch, iteration.is_multiple_of(2)) {
            match touch.read().await {
                Ok(Touch::Unchanged) => {}
                Ok(Touch::Pressed(x, y)) => {
                    let position = slint::LogicalPosition::new(x as f32, y as f32);
                    let event = match pressed.replace(position) {
                        Some(_) => WindowEvent::PointerMoved { position },
                        None => {
                            TOUCH_PRESSES.fetch_add(1, Relaxed);
                            runtime.touch_press();
                            ui.presenter.note_touch(Instant::now().as_micros());
                            WindowEvent::PointerPressed {
                                position,
                                button: PointerEventButton::Left,
                            }
                        }
                    };
                    ui.window.dispatch_event(event);
                }
                Ok(Touch::Released) => {
                    runtime.purge_release();
                    if let Some(position) = pressed.take() {
                        ui.window.dispatch_event(WindowEvent::PointerReleased {
                            position,
                            button: PointerEventButton::Left,
                        });
                        ui.window.dispatch_event(WindowEvent::PointerExited);
                    }
                }
                Err(_) => {
                    TOUCH_ERRORS.fetch_add(1, Relaxed);
                    // Une erreur pendant un appui ne doit pas retenir une
                    // purge. Retirer aussi l'appui Slint évite un bouton figé.
                    runtime.purge_release();
                    if let Some(position) = pressed.take() {
                        ui.window.dispatch_event(WindowEvent::PointerReleased {
                            position,
                            button: PointerEventButton::Left,
                        });
                        ui.window.dispatch_event(WindowEvent::PointerExited);
                    }
                }
            }
        }

        runtime.poll(&mut ui);
        ui.window.draw_if_needed(|renderer| {
            // Différence saturée : voir la recopie d'affichage, le SYSTIMER lu
            // depuis les deux cœurs n'est pas strictement monotone.
            let t0 = esp_hal::time::Instant::now()
                .duration_since_epoch()
                .as_micros();
            renderer.render_by_line(LineToFb(fb));
            let us = esp_hal::time::Instant::now()
                .duration_since_epoch()
                .as_micros()
                .saturating_sub(t0) as u32;
            RENDERS.fetch_add(1, Relaxed);
            RENDER_LAST_US.store(us, Relaxed);
            RENDER_MAX_US.fetch_max(us, Relaxed);
        });

        Timer::after(Duration::from_millis(10)).await;
    }
}
