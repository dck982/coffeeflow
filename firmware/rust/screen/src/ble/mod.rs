//! Deux liens GATT sur le cœur 1. Le cœur décide du scan et de la cadence
//! des pesées ; ce service ne connaît ni l'UI, ni les sorties de la machine.
//! Les notifications vont au cœur 0 par une file bornée, sans attente radio.
#![allow(
    clippy::large_stack_frames,
    reason = "état des futures dans l'arène de l'exécuteur"
)]

mod control;
mod gatt;
use gatt::*;

use bt_hci::{controller::ExternalController, param::Status};
use core::cell::RefCell;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering::Relaxed};
use embassy_futures::{
    join::join3,
    select::{Either, Either3, select, select3},
};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};
use embassy_time::{Duration, Instant, Timer, with_timeout};
use esp_hal::{peripherals::BT, system::Cpu};
use esp_radio::ble::controller::BleConnector;
use screen_core::{acaia, ble_policy, core::BleCommand, scace};
use trouble_host::prelude::*;

const SCALE: usize = 0;
const PROBE: usize = 1;
const CONNECTIONS_MAX: usize = 2;
type Ctrl = ExternalController<BleConnector<'static>, 4>;
type BleStack<'a> = trouble_host::Stack<'a, Ctrl, DefaultPacketPool>;
type Client<'a> = GattClient<'a, Ctrl, DefaultPacketPool, 4>;
type ErrText = heapless::String<64>;

/// Entrées à consommer par la boucle du cœur, dans l'ordre de réception.
pub enum EventKind {
    ScaleConnected(bool),
    Weight(f32),
    ScaceConnected(bool),
    ScaceSample(scace::Sample),
}
/// Horodatage capturé avant la mise en file : un rendu lent ne rajeunit
/// jamais une ancienne pesée ou une température SCACE.
pub struct Event {
    pub at_us: i64,
    pub kind: EventKind,
}
pub static EVENTS: Channel<CriticalSectionRawMutex, Event, 32> = Channel::new();
pub static LOST_EVENTS: AtomicU32 = AtomicU32::new(0);
static COMMANDS: Channel<CriticalSectionRawMutex, (), 8> = Channel::new();
static ENABLED: AtomicBool = AtomicBool::new(false);
static PROBE_ENABLED: AtomicBool = AtomicBool::new(false);
static CYCLE_ACTIVE: AtomicBool = AtomicBool::new(false);
static PUBLISH_ALL: AtomicBool = AtomicBool::new(false);
static SCANNING: AtomicBool = AtomicBool::new(false);
static CONNECTING: AtomicU32 = AtomicU32::new(0);
static CONNECTED: [AtomicBool; 2] = [const { AtomicBool::new(false) }; 2];
static TARGETS: critical_section::Mutex<RefCell<[Option<Address>; 2]>> =
    critical_section::Mutex::new(RefCell::new([None, None]));

/// Pose l'état voulu à chaque pas du cœur : provisioning coupe les deux liens.
pub fn configure(enabled: bool, probe_enabled: bool, cycle_active: bool) {
    ENABLED.store(enabled, Relaxed);
    PROBE_ENABLED.store(probe_enabled, Relaxed);
    CYCLE_ACTIVE.store(cycle_active, Relaxed);
}

/// File dédiée à la balance : tare et changement de cadence ne bloquent pas CAN.
pub fn command(command: BleCommand) -> bool {
    match command {
        BleCommand::SetActive(active) => {
            PUBLISH_ALL.store(active, Relaxed);
            true
        }
        BleCommand::DisconnectScace => {
            PROBE_ENABLED.store(false, Relaxed);
            true
        }
        BleCommand::Tare => CONNECTED[SCALE].load(Relaxed) && COMMANDS.try_send(()).is_ok(),
    }
}

fn publish(kind: EventKind) {
    let control = matches!(
        kind,
        EventKind::ScaleConnected(_) | EventKind::ScaceConnected(_)
    );
    let event = Event {
        at_us: Instant::now().as_micros() as i64,
        kind,
    };
    if let Err(embassy_sync::channel::TrySendError::Full(event)) = EVENTS.try_send(event) {
        LOST_EVENTS.fetch_add(1, Relaxed);
        // Une mesure perdue vieillit naturellement. Une déconnexion perdue
        // laisserait l'état de lien faux indéfiniment : elle remplace l'aînée.
        if control {
            let _ = EVENTS.try_receive();
            let _ = EVENTS.try_send(event);
        }
    }
}

/// Initialise le contrôleur avant l'affichage, sur le cœur qui porte la radio.
pub fn connector(bt: BT<'static>) -> BleConnector<'static> {
    BleConnector::new(
        bt,
        esp_radio::ble::Config::default().with_task_cpu(Cpu::AppCpu),
    )
    .unwrap()
}

fn err_text(stage: &str, err: impl core::fmt::Debug) -> ErrText {
    use core::fmt::Write;
    let mut out = ErrText::new();
    let _ = write!(out, "{stage}: {err:?}");
    out
}

struct ScanHandler;
impl EventHandler for ScanHandler {
    fn on_adv_reports(&self, mut reports: LeAdvReportsIter<'_>) {
        while let Some(Ok(report)) = reports.next() {
            let mut name = None;
            let mut uuid16 = heapless::Vec::<u16, 16>::new();
            let mut uuid128 = heapless::Vec::<[u8; 16], 8>::new();
            for ad in AdStructure::decode(report.data).flatten() {
                match ad {
                    AdStructure::CompleteLocalName(n) | AdStructure::ShortenedLocalName(n) => {
                        name = Some(n)
                    }
                    AdStructure::CompleteServiceUuids16(uuids)
                    | AdStructure::IncompleteServiceUuids16(uuids) => {
                        for uuid in uuids {
                            let _ = uuid16.push(u16::from_le_bytes(*uuid));
                        }
                    }
                    AdStructure::CompleteServiceUuids128(uuids)
                    | AdStructure::IncompleteServiceUuids128(uuids) => {
                        for uuid in uuids {
                            let _ = uuid128.push(*uuid);
                        }
                    }
                    _ => {}
                }
            }
            let adv = acaia::Advertisement {
                name,
                uuids16: &uuid16,
                uuids128_le: &uuid128,
            };
            let dev = if acaia::is_acaia_advertisement(&adv) {
                Some(SCALE)
            } else if scace::matches(&uuid128, name) {
                Some(PROBE)
            } else {
                None
            };
            let Some(dev) = dev else { continue };
            if !wanted(dev) {
                continue;
            }
            critical_section::with(|cs| {
                TARGETS.borrow_ref_mut(cs)[dev].get_or_insert(Address {
                    kind: report.addr_kind,
                    addr: report.addr,
                });
            });
        }
    }
}

fn wanted(dev: usize) -> bool {
    ENABLED.load(Relaxed) && (dev == SCALE || PROBE_ENABLED.load(Relaxed))
}

fn scan_inputs() -> ble_policy::Inputs {
    ble_policy::Inputs {
        scale_wanted: wanted(SCALE) && !CONNECTED[SCALE].load(Relaxed),
        probe_wanted: wanted(PROBE) && !CONNECTED[PROBE].load(Relaxed),
        connecting: CONNECTING.load(Relaxed) != 0,
        cycle_active: CYCLE_ACTIVE.load(Relaxed),
    }
}

/// L'hôte, le scan et les deux connexions progressent ensemble ; les connexions
/// sont sérialisées par le compteur, sans scan de l'hôte pendant l'initiation.
pub async fn run(connector: BleConnector<'static>) {
    let controller: Ctrl = ExternalController::new(connector);
    let mut resources: HostResources<_, DefaultPacketPool, CONNECTIONS_MAX, 1> =
        HostResources::new();
    let stack = trouble_host::new(controller, &mut resources)
        .set_random_address(Address::random([0xff, 0x8f, 0x1b, 0x05, 0xe4, 0xff]))
        .build();
    let mut runner = stack.runner();
    // Une erreur fatale de l'hôte annule les services : join attendrait leur
    // boucle infinie et laisserait les liens annoncés connectés pour toujours.
    let _ = select(
        runner.run_with_handler(&ScanHandler),
        join3(scan(&stack), link(&stack, SCALE), link(&stack, PROBE)),
    )
    .await;
    SCANNING.store(false, Relaxed);
    CONNECTING.store(0, Relaxed);
    CONNECTED[SCALE].store(false, Relaxed);
    CONNECTED[PROBE].store(false, Relaxed);
    critical_section::with(|cs| *TARGETS.borrow_ref_mut(cs) = [None, None]);
    while COMMANDS.try_receive().is_ok() {}
    publish(EventKind::ScaleConnected(false));
    publish(EventKind::ScaceConnected(false));
}

async fn scan(stack: &BleStack<'_>) {
    let mut scanner = Scanner::new(stack.central());
    let mut last_probe_us = 0;
    loop {
        let inputs = scan_inputs();
        let policy = ble_policy::decide(&inputs);
        let now = Instant::now().as_micros() as i64;
        let due = policy != ble_policy::Scan::ProbeWindow
            || ble_policy::probe_window_due(now, last_probe_us);
        if let (Some(params), true) = (policy.params(), due) {
            let config = ScanConfig {
                active: params.active,
                phys: PhySet::M1,
                interval: Duration::from_millis(params.interval_ms as u64),
                window: Duration::from_millis(params.window_ms as u64),
                ..Default::default()
            };
            // Le drapeau couvre aussi les commandes HCI de démarrage : aucune
            // autre future de cet exécuteur ne peut s'intercaler avant lui.
            if !control::scan_can_start(&scan_inputs(), ENABLED.load(Relaxed)) {
                // Sans attente, la boucle garderait l'exécuteur du cœur 1 et
                // aucune autre future ne pourrait faire changer la condition.
                Timer::after(Duration::from_millis(20)).await;
                continue;
            }
            SCANNING.store(true, Relaxed);
            if let Ok(session) = scanner.scan(&config).await {
                if policy == ble_policy::Scan::ProbeWindow {
                    last_probe_us = now;
                }
                while control::scan_continues(
                    policy,
                    &scan_inputs(),
                    ENABLED.load(Relaxed),
                    Instant::now().as_micros() as i64,
                    now,
                ) {
                    Timer::after(Duration::from_millis(20)).await;
                    let found = critical_section::with(|cs| {
                        TARGETS
                            .borrow_ref(cs)
                            .iter()
                            .enumerate()
                            .any(|(dev, target)| {
                                wanted(dev) && !CONNECTED[dev].load(Relaxed) && target.is_some()
                            })
                    });
                    if found {
                        break;
                    }
                    if CONNECTED[SCALE].load(Relaxed) && policy == ble_policy::Scan::Continuous {
                        break;
                    }
                }
                drop(session);
            }
            // Même un démarrage HCI échoué peut avoir ouvert le scan avant
            // son annulation : confirmer l'arrêt dans les deux chemins.
            {
                // Attendre l'acquittement HCI, plutôt qu'un délai arbitraire :
                // la coexistence Wi-Fi peut retarder les commandes du contrôleur.
                while stack
                    .command(bt_hci::cmd::le::LeSetScanEnable::new(false, false))
                    .await
                    .is_err()
                {
                    Timer::after(Duration::from_millis(20)).await;
                }
            }
            SCANNING.store(false, Relaxed);
        }
        Timer::after(Duration::from_millis(20)).await;
    }
}

async fn link(stack: &BleStack<'_>, dev: usize) {
    loop {
        let target = critical_section::with(|cs| TARGETS.borrow_ref(cs)[dev]);
        if !wanted(dev)
            || target.is_none()
            || CONNECTING.compare_exchange(0, 1, Relaxed, Relaxed).is_err()
        {
            Timer::after(Duration::from_millis(50)).await;
            continue;
        }
        while SCANNING.load(Relaxed) {
            Timer::after(Duration::from_millis(10)).await;
        }
        if !wanted(dev) {
            CONNECTING.store(0, Relaxed);
            critical_section::with(|cs| TARGETS.borrow_ref_mut(cs)[dev] = None);
            continue;
        }
        let accept = [target.unwrap()];
        let config = ConnectConfig {
            scan_config: ScanConfig {
                active: false,
                filter_accept_list: &accept,
                phys: PhySet::M1,
                interval: Duration::from_millis(60),
                window: Duration::from_millis(30),
                ..Default::default()
            },
            connect_params: RequestedConnParams {
                min_connection_interval: Duration::from_millis(30),
                max_connection_interval: Duration::from_millis(50),
                max_latency: 0,
                min_event_length: Duration::from_secs(0),
                max_event_length: Duration::from_secs(0),
                supervision_timeout: Duration::from_millis(2560),
            },
        };
        let mut central = stack.central();
        let result = select(
            with_timeout(Duration::from_secs(10), central.connect(&config)),
            wait_unwanted(dev),
        )
        .await;
        CONNECTING.store(0, Relaxed);
        if let Either::First(Ok(Ok(conn))) = result {
            let gatt = async {
                if let Ok(client) = Client::new(stack, &conn).await {
                    let session = async {
                        if dev == SCALE {
                            scale_session(&client).await
                        } else {
                            probe_session(&client).await
                        }
                    };
                    let _ = select3(client.task(), session, conn_events(&conn)).await;
                }
            };
            // Le provisioning doit interrompre aussi MTU, découverte et CCCD,
            // pas seulement la boucle de notifications déjà établie.
            let _ = select(gatt, wait_unwanted(dev)).await;
            if dev == SCALE {
                while COMMANDS.try_receive().is_ok() {}
            }
            CONNECTED[dev].store(false, Relaxed);
            publish(if dev == SCALE {
                EventKind::ScaleConnected(false)
            } else {
                EventKind::ScaceConnected(false)
            });
            if conn.is_connected() {
                conn.disconnect();
                let _ = with_timeout(Duration::from_secs(5), conn_events(&conn)).await;
            }
        }
        critical_section::with(|cs| TARGETS.borrow_ref_mut(cs)[dev] = None);
        Timer::after(Duration::from_secs(1)).await;
    }
}

async fn wait_unwanted(dev: usize) {
    while wanted(dev) {
        Timer::after(Duration::from_millis(20)).await;
    }
}

async fn conn_events(conn: &Connection<'_, DefaultPacketPool>) -> Status {
    loop {
        if let ConnectionEvent::Disconnected { reason } = conn.next().await {
            return reason;
        }
    }
}

async fn scale_session(client: &Client<'_>) -> ErrText {
    let (handle, notify) = match scale_discover(client).await {
        Ok(value) => value,
        Err(e) => return e,
    };
    let mut listener = match client.subscribe(&notify, false).await {
        Ok(value) => value,
        Err(e) => return err_text("abonnement", e),
    };
    for frame in acaia::subscribe_frames() {
        if let Err(e) = client.write_handle_without_response(handle, &frame).await {
            return err_text("écriture", e);
        }
    }
    CONNECTED[SCALE].store(true, Relaxed);
    publish(EventKind::ScaleConnected(true));
    let mut session = acaia::Session::new();
    let mut next_heartbeat_poll = Instant::now() + Duration::from_millis(50);
    loop {
        if !wanted(SCALE) {
            return err_text("radio", "arrêt demandé");
        }
        match select3(
            listener.next(),
            COMMANDS.receive(),
            Timer::at(next_heartbeat_poll),
        )
        .await
        {
            Either3::First(notification) => {
                session.set_active(PUBLISH_ALL.load(Relaxed));
                for event in session
                    .on_notification(notification.as_ref(), Instant::now().as_micros() as i64)
                {
                    match event {
                        acaia::RxEvent::Weight(value) => publish(EventKind::Weight(value)),
                        acaia::RxEvent::Disconnected => {
                            return err_text("balance", "déconnexion demandée");
                        }
                    }
                }
            }
            Either3::Second(()) => {
                if let Err(e) = client
                    .write_handle_without_response(handle, &acaia::tare_frame())
                    .await
                {
                    return err_text("tare", e);
                }
            }
            Either3::Third(()) => {}
        }
        // Même des notifications à 100 Hz ne repoussent jamais le maintien.
        if control::heartbeat_poll_due(Instant::now().as_micros(), next_heartbeat_poll.as_micros())
        {
            next_heartbeat_poll = Instant::now() + Duration::from_millis(50);
            for frame in session.heartbeat(Instant::now().as_micros() as i64, true) {
                if let Err(e) = client.write_handle_without_response(handle, &frame).await {
                    return err_text("maintien", e);
                }
            }
        }
    }
}

async fn probe_session(client: &Client<'_>) -> ErrText {
    let services = match client.services_by_uuid(&SCACE_SERVICE).await {
        Ok(value) => value,
        Err(e) => return err_text("service", e),
    };
    let Some(service) = services.first() else {
        return err_text("service", "absent");
    };
    let frame: Characteristic<[u8]> =
        match client.characteristic_by_uuid(service, &SCACE_FRAME).await {
            Ok(value) => value,
            Err(e) => return err_text("caractéristique", e),
        };
    let mut listener = match client.subscribe(&frame, false).await {
        Ok(value) => value,
        Err(e) => return err_text("abonnement", e),
    };
    CONNECTED[PROBE].store(true, Relaxed);
    publish(EventKind::ScaceConnected(true));
    loop {
        if !wanted(PROBE) {
            return err_text("sonde", "arrêt demandé");
        }
        match select(listener.next(), Timer::after(Duration::from_millis(50))).await {
            Either::First(notification) => {
                if let Some(sample) = scace::parse_notification(notification.as_ref()) {
                    publish(EventKind::ScaceSample(sample));
                }
            }
            Either::Second(()) => {
                if !wanted(PROBE) {
                    return err_text("sonde", "arrêt demandé");
                }
            }
        }
    }
}
