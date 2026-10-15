//! Le simulateur emploie la même UI et le même présentateur que la carte.
extern crate alloc;
slint::include_modules!();
#[path = "../../screen/src/ui/presenter.rs"]
mod presenter;
mod rig;
use presenter::{Presenter, UiRequest};
use screen_core::{
    config::Config,
    guided::{self, Hint, Screen},
    snapshot::{CycleState, Freshness, Snapshot},
};
use slint::{
    ComponentHandle,
    platform::software_renderer::{MinimalSoftwareWindow, RepaintBufferType, Rgb565Pixel},
    platform::{Platform, WindowAdapter},
};
use std::{cell::RefCell, rc::Rc, time::Duration};

const SCENARIOS: &[&str] = &[
    "home",
    "home-time",
    "brew",
    "done",
    "purge",
    "settings",
    "settings1",
    "settings2",
    "settings3",
    "diagnostic",
    "keypad-weight",
    "keypad-time",
    "preinfusion",
    "rampdown",
    "dimmer",
    "heating",
    "valve",
    "forget",
    "boot",
    "ota",
    "lock",
    "bus",
    "provisioning",
    "standby",
    "choice",
    "choice-heating",
    "guided-connect",
    "guided-rinse-temp",
    "guided-rinse-ready",
    "guided-purge",
    "guided-wait",
    "guided-early-removal",
    "guided-prepare",
    "guided-start",
    "guided-start-other-cup",
    "guided-refused",
];

/// Remplit l'état du guide comme le cœur le ferait à l'écran donné.
fn guided_view(s: &mut Snapshot, screen: Screen, hint: Hint, enabled: bool) {
    s.guided = guided::View {
        active: true,
        screen,
        hint,
        button_enabled: enabled,
        rinse_min_c: 87.0,
        ..guided::View::default()
    };
}
fn fixture(name: &str) -> Result<(Presenter, Snapshot, Config), String> {
    let c = Config::default();
    let mut p = Presenter::default();
    let mut s = Snapshot {
        screen_uptime_s: 10,
        time_known: true,
        heating_power_capable: true,
        heating_freshness: Freshness::Fresh,
        sensors_alive: true,
        scale_present: true,
        scale_connected: true,
        weight_g: 0.2,
        pressure_bar: 0.3,
        pressure_valid: true,
        pressure_freshness: Freshness::Fresh,
        boiler_temperature_c: 90.0,
        boiler_sensor_temperature_c: 104.0,
        boiler_temperature_valid: true,
        boiler_temperature_freshness: Freshness::Fresh,
        brew_temperature_ready: true,
        dimmer_ready: true,
        dimmer_valid: true,
        screen_version_major: protocol::FIRMWARE_VERSION.major,
        screen_version_minor: protocol::FIRMWARE_VERSION.minor,
        screen_version_patch: protocol::FIRMWARE_VERSION.patch,
        sensors_version_major: protocol::FIRMWARE_VERSION.major,
        sensors_version_minor: protocol::FIRMWARE_VERSION.minor,
        sensors_version_patch: protocol::FIRMWARE_VERSION.patch,
        internal_heap_free: 59 * 1024,
        internal_heap_largest: 31 * 1024,
        maintenance_valve_capable: true,
        ..Snapshot::default()
    };
    match name {
        "home" => {}
        "home-time" => s.scale_present = false,
        "brew" | "done" => {
            s.cycle_state = if name == "done" {
                CycleState::Finished
            } else {
                CycleState::Brew
            };
            s.shot.active = true;
            s.shot.axis_s = 38.0;
            s.shot.elapsed_s = 24.0;
            s.shot.segment_start_s = [0.0, 2.5, 5.0, 9.0, 10.0, f32::NAN];
            s.shot.segment_end_s = [2.5, 5.0, 9.0, 10.0, 24.0, f32::NAN];
            s.shot.cup_weight_g = 28.4;
            s.shot.infusion_s = 14.0;
            s.shot.cup_mean_c = 89.2;
            s.shot.start_temperature_c = 90.0;
            s.shot.min_temperature_c = 87.5;
            s.shot.infusion_cup_g_s = 1.8;
            s.shot.cup_rate_g_s = 1.9;
            s.shot.pump_stop_s = if name == "done" { 24.0 } else { f32::NAN };
            s.pressure_bar = 9.0;
            s.heating_power_pct = 65.0;
            s.dimmer_pct = 100;
            s.flow_ml_s = 1.85;
        }
        "purge" => {
            s.cycle_state = CycleState::Purge;
            s.cycle_elapsed_ms = 7500;
            s.flow_ml_s = 4.5;
        }
        "settings" | "settings1" | "settings2" | "settings3" => {
            p.page = 3;
            p.settings_page = match name {
                "settings1" => 1,
                "settings2" => 2,
                "settings3" => 3,
                _ => 0,
            };
        }
        "diagnostic" => p.page = 4,
        "keypad-weight" | "keypad-time" => {
            s.scale_present = name == "keypad-weight";
            p.command(5, "", 1, &s, &c);
        }
        "preinfusion" | "rampdown" => {
            p.settings_page = if name == "preinfusion" { 1 } else { 2 };
            p.command(13, if name == "preinfusion" { "1" } else { "0" }, 1, &s, &c);
        }
        "dimmer" | "valve" | "heating" => {
            p.command(
                16,
                match name {
                    "dimmer" => "3",
                    "valve" => "4",
                    _ => "5",
                },
                1,
                &s,
                &c,
            );
        }
        "forget" => {
            p.page = 3;
            p.settings_page = 3;
            p.command(13, "0", 1, &s, &c);
        }
        "boot" => s.screen_uptime_s = 0,
        "ota" => {
            s.flash_active = true;
            s.flash_target = screen_core::snapshot::FlashTarget::Screen;
            s.flash_bytes_done = 460_000;
            s.flash_bytes_total = 1_000_000;
        }
        "lock" => s.lockout = true,
        "bus" => s.sensors_alive = false,
        "provisioning" => {
            s.provisioning = true;
            s.ble_enabled = false;
        }
        "standby" => {}
        "choice" | "choice-heating" => {
            s.boiler_temperature_c = c.brew_temperature_c;
            if name == "choice-heating" {
                s.boiler_temperature_c = 68.5;
                s.brew_temperature_ready = false;
            }
            p.command(21, "", 1, &s, &c);
        }
        "guided-connect" => {
            s.scale_present = false;
            guided_view(&mut s, Screen::Rinse, Hint::ConnectScale, false);
        }
        "guided-rinse-temp" => {
            s.boiler_temperature_c = 61.4;
            guided_view(&mut s, Screen::Rinse, Hint::WaitRinseTemp, false);
        }
        "guided-rinse-ready" => {
            s.weight_g = 41.3;
            guided_view(&mut s, Screen::Rinse, Hint::PressNextRinse, true);
        }
        "guided-purge" => {
            s.weight_g = 11.2;
            s.cycle_state = CycleState::Purge;
            s.cycle_elapsed_ms = 2300;
            guided_view(&mut s, Screen::Rinse, Hint::PressNextRinse, true);
            s.guided.purging = true;
            s.guided.purge_elapsed_ms = 2300;
        }
        "guided-wait" => {
            s.boiler_temperature_c = 88.3;
            s.weight_g = 24.1;
            guided_view(&mut s, Screen::Wait, Hint::WaitSetpoint, false);
            s.guided.since_rinse_s = 134;
        }
        "guided-early-removal" => {
            s.weight_g = 0.1;
            guided_view(&mut s, Screen::Wait, Hint::CupRemovedEarly, false);
            s.guided.since_rinse_s = 95;
        }
        "guided-prepare" => {
            s.weight_g = 0.1;
            guided_view(&mut s, Screen::Wait, Hint::Prepare, false);
        }
        "guided-start" => {
            s.weight_g = 41.6;
            guided_view(&mut s, Screen::Wait, Hint::PressStart, true);
            s.guided.cup_recognized = true;
        }
        "guided-start-other-cup" => {
            s.weight_g = 87.9;
            guided_view(&mut s, Screen::Wait, Hint::PressStart, true);
        }
        "guided-refused" => {
            s.weight_g = 0.1;
            guided_view(&mut s, Screen::Wait, Hint::PressStart, true);
            s.guided.refusal = Some(screen_core::core::ActionStatus::DimmerNotReady);
        }
        _ => return Err(format!("scénario inconnu : {name}")),
    }
    Ok((p, s, c))
}
struct Headless(Rc<MinimalSoftwareWindow>);
impl Platform for Headless {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
        Ok(self.0.clone())
    }
}
fn snapshots(dir: &std::path::Path, names: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(dir)?;
    let window = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
    window.set_size(slint::PhysicalSize::new(800, 480));
    slint::platform::set_platform(Box::new(Headless(window.clone())))?;
    let app = AppWindow::new()?;
    app.show()?;
    for name in names {
        let (mut p, s, c) = fixture(name)?;
        for g in 0..4 {
            p.update(
                &app,
                if *name == "standby" {
                    1_900_000_000 + g * 62_500
                } else {
                    1_000_000 + g * 62_500
                },
                &s,
                &c,
                "coffeeflow-setup",
                "09:16",
            );
        }
        let mut pixels = vec![Rgb565Pixel(0); 800 * 480];
        window.request_redraw();
        window.draw_if_needed(|r| {
            r.render(&mut pixels, 800);
        });
        let mut rgb = Vec::with_capacity(pixels.len() * 3);
        for p in pixels {
            let r = ((p.0 >> 11) & 31) as u8;
            let g = ((p.0 >> 5) & 63) as u8;
            let b = (p.0 & 31) as u8;
            rgb.extend_from_slice(&[
                (r << 3) | (r >> 2),
                (g << 2) | (g >> 4),
                (b << 3) | (b >> 2),
            ]);
        }
        let file = std::fs::File::create(dir.join(format!("{name}.png")))?;
        let mut enc = png::Encoder::new(std::io::BufWriter::new(file), 800, 480);
        enc.set_color(png::ColorType::Rgb);
        enc.set_depth(png::BitDepth::Eight);
        enc.write_header()?.write_image_data(&rgb)?;
    }
    Ok(())
}
fn live() -> Result<(), Box<dyn std::error::Error>> {
    let app = AppWindow::new()?;
    let rig = Rc::new(RefCell::new(rig::Rig::new()));
    let presenter = Rc::new(RefCell::new(Presenter::default()));
    let r = rig.clone();
    let p = presenter.clone();
    app.on_command(move |id, payload| {
        let mut rig = r.borrow_mut();
        let s = rig.snapshot();
        let c = rig.core.config();
        let now = rig.now_us;
        let mut p = p.borrow_mut();
        if let Some(UiRequest::Core(input)) = p.command(id, &payload, now as u64, &s, &c) {
            let clock = rig.clock();
            p.feedback(rig.core.handle(clock, input));
        }
    });
    let timer = slint::Timer::default();
    let weak = app.as_weak();
    timer.start(
        slint::TimerMode::Repeated,
        Duration::from_millis(50),
        move || {
            let Some(app) = weak.upgrade() else { return };
            let mut rig = rig.borrow_mut();
            rig.advance();
            let s = rig.snapshot();
            let clock = presenter::clock_text(rig.clock().wall_unix_s);
            presenter.borrow_mut().update(
                &app,
                rig.now_us as u64,
                &s,
                &rig.core.config(),
                "coffeeflow-setup",
                &clock,
            );
        },
    );
    app.run()?;
    Ok(())
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("--snapshots") => snapshots(
            std::path::Path::new(args.get(1).map_or("snapshots", String::as_str)),
            SCENARIOS,
        ),
        Some("--snapshot") => {
            let name = args.get(1).ok_or("scénario manquant")?;
            let dir = args.get(2).ok_or("répertoire manquant")?;
            snapshots(std::path::Path::new(dir), &[name])
        }
        Some("--list") => {
            println!("{}", SCENARIOS.join("\n"));
            Ok(())
        }
        None => live(),
        _ => Err(
            "usage : screen-sim [--snapshots dossier | --snapshot scénario dossier | --list]"
                .into(),
        ),
    }
}

#[cfg(test)]
mod render_tests {
    use super::*;
    #[test]
    fn all_destinations_render_at_panel_resolution() {
        let dir = std::env::temp_dir().join(format!("coffeeflow-ui-test-{}", std::process::id()));
        snapshots(&dir, SCENARIOS).unwrap();
        for name in SCENARIOS {
            let file = std::fs::File::open(dir.join(format!("{name}.png"))).unwrap();
            let reader = png::Decoder::new(std::io::BufReader::new(file))
                .read_info()
                .unwrap();
            assert_eq!((reader.info().width, reader.info().height), (800, 480));
        }
        assert_ne!(
            std::fs::read(dir.join("home.png")).unwrap(),
            std::fs::read(dir.join("home-time.png")).unwrap()
        );
        assert_ne!(
            std::fs::read(dir.join("keypad-weight.png")).unwrap(),
            std::fs::read(dir.join("keypad-time.png")).unwrap()
        );
        assert_ne!(
            std::fs::read(dir.join("choice.png")).unwrap(),
            std::fs::read(dir.join("choice-heating.png")).unwrap()
        );
        assert_ne!(
            std::fs::read(dir.join("guided-rinse-ready.png")).unwrap(),
            std::fs::read(dir.join("guided-purge.png")).unwrap()
        );
        assert_ne!(
            std::fs::read(dir.join("guided-start.png")).unwrap(),
            std::fs::read(dir.join("guided-start-other-cup.png")).unwrap()
        );
        std::fs::remove_dir_all(dir).unwrap();

        // Le présentateur rafraîchit les tuiles pendant qu'un doigt est posé :
        // l'appui sur « chauffage » doit tout de même aboutir au relâchement.
        // La plateforme Slint n'est installée qu'une fois par processus, d'où
        // ce contrôle dans le même test que les captures.
        use slint::platform::{PointerEventButton, WindowEvent};
        let app = AppWindow::new().unwrap();
        let (mut p, s, c) = fixture("diagnostic").unwrap();
        for g in 0..8 {
            p.update(&app, 1_000_000 + g * 62_500, &s, &c, "", "09:16");
        }
        let hit = Rc::new(RefCell::new(None));
        let sink = hit.clone();
        app.on_command(move |k, a| *sink.borrow_mut() = Some((k, a.to_string())));
        let position = slint::LogicalPosition::new(640.0, 260.0);
        let button = PointerEventButton::Left;
        app.window()
            .dispatch_event(WindowEvent::PointerPressed { position, button });
        for g in 0..4 {
            p.update(&app, 2_000_000 + g * 62_500, &s, &c, "", "09:16");
        }
        app.window()
            .dispatch_event(WindowEvent::PointerReleased { position, button });
        assert_eq!(*hit.borrow(), Some((16, "5".into())));

        // Balance perdue tasse retirée après une infusion au poids : la purge
        // ne doit pas montrer, même une image, le poids négatif de l'arrêt.
        let (mut p, mut s, c) = fixture("home").unwrap();
        s.cycle_weight_goal = true;
        s.cycle_start_weight_g = 0.4;
        s.weight_g = -310.2;
        for g in 0..4 {
            p.update(&app, 3_000_000 + g * 62_500, &s, &c, "", "09:16");
        }
        assert_eq!(app.get_purge_time(), "-310,6 g");
        s.cycle_state = CycleState::Purge;
        p.update(&app, 3_200_000, &s, &c, "", "09:16");
        assert_eq!((app.get_page(), app.get_purge_time().as_str()), (2, "0 s"));
    }
}
