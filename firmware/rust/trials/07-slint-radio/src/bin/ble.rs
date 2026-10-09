// Les deux liens GATT de l'essai 04b, repris de l'essai 05 : la balance
// Acaia Lunar (pesées notifiées, heartbeat toutes les 2,5 s) et la sonde
// SCACE (une trame notifiée à 10 Hz). La température de la sonde s'affiche
// (`probe_celsius`).

#![allow(
    clippy::large_stack_frames,
    reason = "futures d'embassy et de trouble-host : clippy compte l'état de la future, qui vit dans l'arène de l'exécuteur ; trames réelles mesurées (emit-stack-sizes) de 3,1 Ko au plus"
)]

use core::cell::RefCell;
use core::sync::atomic::{AtomicU32, Ordering::Relaxed};

use bt_hci::controller::ExternalController;
use bt_hci::param::Status;
use critical_section::Mutex;
use embassy_futures::select::{Either, Either3, select, select3};
use embassy_time::{Duration, Instant, Timer, with_timeout};
use esp_hal::peripherals::BT;
use esp_hal::system::Cpu;
use esp_radio::ble::controller::BleConnector;
use log::info;
use trouble_host::prelude::*;

/// Démarre le contrôleur BLE sur le cœur qui appelle (le cœur 1), avant le
/// Wi-Fi, dans l'ordre de l'exemple de coexistence d'esp-radio.
pub fn connector(bt: BT<'static>) -> BleConnector<'static> {
    let config = esp_radio::ble::Config::default().with_task_cpu(Cpu::AppCpu);
    BleConnector::new(bt, config).unwrap()
}

/// Compteurs du rapport précédent, pour les débits.
pub struct ReportState {
    adv: u32,
    notifs: [u32; 2],
}

impl ReportState {
    pub const fn new() -> Self {
        Self {
            adv: 0,
            notifs: [0; 2],
        }
    }
}

/// Lignes BLE du rapport toutes les 10 s, espacées de 20 ms.
pub async fn report(state: &mut ReportState) {
    Timer::after(Duration::from_millis(20)).await;
    let adv = ADV_REPORTS.load(Relaxed);
    info!(
        "ble {} adv={}/s",
        ["init", "scan", "liens"][BLE_PHASE.load(Relaxed) as usize],
        adv.wrapping_sub(state.adv) / 10,
    );
    state.adv = adv;
    let last_notifs = &mut state.notifs;

    let seen = critical_section::with(|cs| SEEN.borrow(cs).take());
    for (kind, dev) in DEVICE_NAMES.iter().zip(seen) {
        if let Some(dev) = dev {
            Timer::after(Duration::from_millis(20)).await;
            info!(
                "{kind} vue {} {} {}dBm",
                core::str::from_utf8(dev.name()).unwrap_or("?"),
                Mac(dev.addr.addr.into_inner()),
                dev.rssi,
            );
        }
    }

    // Une ligne par appareil : état du lien, connexions, échecs avant
    // abonnement, pertes après abonnement, notifications par seconde,
    // intervalle de connexion, puis la donnée elle-même.
    for dev in [SCALE, PROBE] {
        Timer::after(Duration::from_millis(20)).await;
        let notifs = NOTIFS[dev].load(Relaxed);
        let rate = notifs.wrapping_sub(last_notifs[dev]) / 10;
        last_notifs[dev] = notifs;
        let link = LinkLine {
            state: LINK_STATE_NAMES[LINK_STATE[dev].load(Relaxed) as usize],
            connects: CONNECTS[dev].load(Relaxed),
            fails: FAILS[dev].load(Relaxed),
            drops: DROPS[dev].load(Relaxed),
            rate,
            interval_us: INTERVAL_US[dev].load(Relaxed),
        };
        if dev == SCALE {
            info!(
                "balance {link} poids={:.1}g pesees={} hb={}",
                f32::from_bits(WEIGHT_BITS.load(Relaxed)),
                WEIGHTS.load(Relaxed),
                HEARTBEATS.load(Relaxed),
            );
        } else {
            info!(
                "scace {link} t={} etat={} manq={} inval={}",
                Centi(PROBE_CENTI_C.load(Relaxed) as i16),
                PROBE_STATUS.load(Relaxed),
                PROBE_LOST.load(Relaxed),
                PROBE_INVALID.load(Relaxed),
            );
        }
    }

    let errors = critical_section::with(|cs| LAST_ERROR.borrow(cs).take());
    for (kind, err) in DEVICE_NAMES.iter().zip(errors) {
        if let Some(err) = err {
            Timer::after(Duration::from_millis(20)).await;
            info!("{kind} erreur {err}");
        }
    }
}

// Côté BLE, écrit par le cœur 1 et lu par le rapport du cœur 0. Les
// tableaux sont indexés par appareil : SCALE puis PROBE.
const SCALE: usize = 0;
const PROBE: usize = 1;
const DEVICE_NAMES: [&str; 2] = ["balance", "scace"];

const PHASE_SCAN: u32 = 1;
const PHASE_LINKS: u32 = 2;
static BLE_PHASE: AtomicU32 = AtomicU32::new(0);
static ADV_REPORTS: AtomicU32 = AtomicU32::new(0);
// Adresses retenues au scan, puis annonce à journaliser une fois.
static TARGETS: Mutex<RefCell<[Option<Address>; 2]>> = Mutex::new(RefCell::new([None, None]));
static SEEN: Mutex<RefCell<[Option<Seen>; 2]>> = Mutex::new(RefCell::new([None, None]));

const LINK_DOWN: u32 = 0;
const LINK_CONNECTING: u32 = 1;
const LINK_DISCOVERING: u32 = 2;
const LINK_SUBSCRIBED: u32 = 3;
const LINK_STATE_NAMES: [&str; 4] = ["coupe", "connexion", "decouverte", "abonne"];
static LINK_STATE: [AtomicU32; 2] = [const { AtomicU32::new(LINK_DOWN) }; 2];
static CONNECTS: [AtomicU32; 2] = [const { AtomicU32::new(0) }; 2];
// Échecs avant l'abonnement (connexion, MTU, découverte), et liens perdus
// après.
static FAILS: [AtomicU32; 2] = [const { AtomicU32::new(0) }; 2];
static DROPS: [AtomicU32; 2] = [const { AtomicU32::new(0) }; 2];
static NOTIFS: [AtomicU32; 2] = [const { AtomicU32::new(0) }; 2];
static INTERVAL_US: [AtomicU32; 2] = [const { AtomicU32::new(0) }; 2];
static LAST_ERROR: Mutex<RefCell<[Option<ErrText>; 2]>> =
    Mutex::new(RefCell::new([const { None }; 2]));

static WEIGHT_BITS: AtomicU32 = AtomicU32::new(f32::NAN.to_bits());
static WEIGHTS: AtomicU32 = AtomicU32::new(0);
static HEARTBEATS: AtomicU32 = AtomicU32::new(0);
static PROBE_CENTI_C: AtomicU32 = AtomicU32::new(SCACE_NO_TEMPERATURE as u16 as u32);
static PROBE_STATUS: AtomicU32 = AtomicU32::new(0);
static PROBE_LOST: AtomicU32 = AtomicU32::new(0);
static PROBE_INVALID: AtomicU32 = AtomicU32::new(0);
// Instant de la dernière trame valide, en ms depuis le démarrage.
static PROBE_LAST_MS: AtomicU32 = AtomicU32::new(0);

/// Une trame plus vieille que ça ne s'affiche plus : la sonde en envoie 10
/// par seconde, et une par seconde quand elle n'a pas de mesure.
const PROBE_STALE_MS: u32 = 1000;
const SCACE_STATUS_OK: u32 = 1;

/// Dernière température de la sonde, en °C, pour l'affichage. `None` sans
/// lien, sans mesure (sonde ouverte ou en court-circuit), ou si la dernière
/// trame a plus d'une seconde.
pub fn probe_celsius() -> Option<f32> {
    let centi = PROBE_CENTI_C.load(Relaxed) as u16 as i16;
    let age_ms = (Instant::now().as_millis() as u32).wrapping_sub(PROBE_LAST_MS.load(Relaxed));
    let fresh = LINK_STATE[PROBE].load(Relaxed) == LINK_SUBSCRIBED
        && PROBE_STATUS.load(Relaxed) == SCACE_STATUS_OK
        && centi != SCACE_NO_TEMPERATURE
        && age_ms < PROBE_STALE_MS;
    fresh.then(|| centi as f32 / 100.0)
}

/// Champs BLE de `/stats` : état de chaque lien et compteurs cumulés.
pub fn stats_json(out: &mut impl core::fmt::Write) -> core::fmt::Result {
    for dev in [SCALE, PROBE] {
        write!(
            out,
            ",\"{}\":{{\"etat\":\"{}\",\"connexions\":{},\"echecs\":{},\"pertes\":{},\"notifs\":{}",
            DEVICE_NAMES[dev],
            LINK_STATE_NAMES[LINK_STATE[dev].load(Relaxed) as usize],
            CONNECTS[dev].load(Relaxed),
            FAILS[dev].load(Relaxed),
            DROPS[dev].load(Relaxed),
            NOTIFS[dev].load(Relaxed),
        )?;
        if dev == SCALE {
            let weight = f32::from_bits(WEIGHT_BITS.load(Relaxed));
            write!(out, ",\"pesees\":{},\"heartbeats\":{},\"poids_g\":", WEIGHTS.load(Relaxed), HEARTBEATS.load(Relaxed))?;
            if weight.is_nan() {
                out.write_str("null}")?;
            } else {
                write!(out, "{weight:.1}}}")?;
            }
        } else {
            write!(
                out,
                ",\"manquees\":{},\"invalides\":{},\"temperature\":\"{}\"}}",
                PROBE_LOST.load(Relaxed),
                PROBE_INVALID.load(Relaxed),
                Centi(PROBE_CENTI_C.load(Relaxed) as i16),
            )?;
        }
    }
    Ok(())
}

/// Résumé d'un lien BLE pour le rapport.
struct LinkLine {
    state: &'static str,
    connects: u32,
    fails: u32,
    drops: u32,
    rate: u32,
    interval_us: u32,
}

impl core::fmt::Display for LinkLine {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{} conn={} ech={} pertes={} notif={}/s int={}ms",
            self.state,
            self.connects,
            self.fails,
            self.drops,
            self.rate,
            self.interval_us / 1000,
        )
    }
}

/// Centièmes de °C, ou `-` sans température.
struct Centi(i16);

impl core::fmt::Display for Centi {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if self.0 == SCACE_NO_TEMPERATURE {
            return f.write_str("-");
        }
        let abs = self.0.unsigned_abs();
        let sign = if self.0 < 0 { "-" } else { "" };
        write!(f, "{sign}{}.{:02}C", abs / 100, abs % 100)
    }
}

/// Adresse BLE, octets reçus en petit-boutiste, affichée dans l'ordre usuel.
struct Mac([u8; 6]);

impl core::fmt::Display for Mac {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let a = self.0;
        write!(
            f,
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            a[5], a[4], a[3], a[2], a[1], a[0]
        )
    }
}

/// Première annonce reçue d'un appareil reconnu.
#[derive(Clone, Copy)]
struct Seen {
    name: [u8; 20],
    name_len: u8,
    addr: Address,
    rssi: i8,
}

impl Seen {
    fn name(&self) -> &[u8] {
        &self.name[..self.name_len as usize]
    }
}

/// Mêmes critères de nom que coffeeflow : `ACAIA` ou `LUNAR` sans tenir
/// compte de la casse pour la balance (ble_scale.cpp), `SCACE` exact pour
/// la sonde (scace_ble.hpp).
fn classify(name: &[u8]) -> Option<usize> {
    let contains = |needle: &[u8]| {
        name.windows(needle.len())
            .any(|w| w.eq_ignore_ascii_case(needle))
    };
    if contains(b"ACAIA") || contains(b"LUNAR") {
        Some(SCALE)
    } else if name == b"SCACE" {
        Some(PROBE)
    } else {
        None
    }
}

/// Reçoit les annonces et réponses de scan dans le contexte de l'hôte, sur
/// le cœur 1. Retient l'adresse de chaque appareil reconnu à sa première
/// annonce : les liens s'y connectent ensuite sans scanner.
struct ScanHandler;

impl EventHandler for ScanHandler {
    fn on_adv_reports(&self, mut reports: LeAdvReportsIter<'_>) {
        while let Some(Ok(report)) = reports.next() {
            ADV_REPORTS.fetch_add(1, Relaxed);
            let name = AdStructure::decode(report.data).find_map(|ad| match ad {
                Ok(AdStructure::CompleteLocalName(n) | AdStructure::ShortenedLocalName(n)) => {
                    Some(n)
                }
                _ => None,
            });
            let Some(name) = name else { continue };
            let Some(dev) = classify(name) else { continue };
            let addr = Address {
                kind: report.addr_kind,
                addr: report.addr,
            };
            critical_section::with(|cs| {
                let mut targets = TARGETS.borrow_ref_mut(cs);
                if targets[dev].is_some() {
                    return;
                }
                targets[dev] = Some(addr);
                let mut seen = Seen {
                    name: [0; 20],
                    name_len: name.len().min(20) as u8,
                    addr,
                    rssi: report.rssi,
                };
                seen.name[..seen.name_len as usize]
                    .copy_from_slice(&name[..seen.name_len as usize]);
                SEEN.borrow_ref_mut(cs)[dev] = Some(seen);
            });
        }
    }
}

const CONNECTIONS_MAX: usize = 2;
const L2CAP_CHANNELS_MAX: usize = 1;

type Ctrl = ExternalController<BleConnector<'static>, 4>;
type BleStack<'a> = trouble_host::Stack<'a, Ctrl, DefaultPacketPool>;
type Client<'a> = GattClient<'a, Ctrl, DefaultPacketPool, 4>;

/// Texte d'erreur formaté sur le cœur 1, journalisé par le cœur 0.
type ErrText = heapless::String<64>;

fn err_text(stage: &str, err: impl core::fmt::Debug) -> ErrText {
    use core::fmt::Write;
    let mut text = ErrText::new();
    let _ = write!(text, "{stage}: {err:?}");
    text
}

/// Scan actif continu (fenêtre = intervalle), sur le modèle de l'exemple
/// `ble/scanner` d'esp-hal, jusqu'à avoir vu les deux appareils. Puis un
/// lien par appareil, en parallèle. Tourne dans l'exécuteur du cœur 1.
pub async fn run(connector: BleConnector<'static>) {
    let controller: Ctrl = ExternalController::new(connector);

    // Adresse aléatoire fixe, comme l'exemple : aucun appairage.
    let address = Address::random([0xff, 0x8f, 0x1b, 0x05, 0xe4, 0xff]);
    let mut resources: HostResources<_, DefaultPacketPool, CONNECTIONS_MAX, L2CAP_CHANNELS_MAX> =
        HostResources::new();
    let stack = trouble_host::new(controller, &mut resources)
        .set_random_address(address)
        .build();
    let mut runner = stack.runner();

    let _ = embassy_futures::join::join(runner.run_with_handler(&ScanHandler), async {
        scan_until_both(&stack).await;
        embassy_futures::join::join(link(&stack, SCALE), link(&stack, PROBE)).await;
    })
    .await;
}

async fn scan_until_both(stack: &BleStack<'_>) {
    let mut scanner = Scanner::new(stack.central());
    let config = ScanConfig {
        active: true,
        phys: PhySet::M1,
        interval: Duration::from_secs(1),
        window: Duration::from_secs(1),
        ..Default::default()
    };
    {
        let _session = scanner.scan(&config).await.unwrap();
        BLE_PHASE.store(PHASE_SCAN, Relaxed);
        while critical_section::with(|cs| TARGETS.borrow_ref(cs).iter().any(Option::is_none)) {
            Timer::after(Duration::from_millis(100)).await;
        }
    }
    // Lâcher la session demande l'arrêt du scan, que l'hôte envoie de son
    // côté : on lui laisse le temps avant la première connexion.
    Timer::after(Duration::from_millis(200)).await;
    BLE_PHASE.store(PHASE_LINKS, Relaxed);
}

/// Un lien : connexion par adresse (le contrôleur cherche l'appareil
/// lui-même, sans scan de l'hôte), découverte, abonnement, puis réception
/// jusqu'à la déconnexion. Recommence indéfiniment. Les deux liens partagent
/// l'initiateur du contrôleur : trouble-host sérialise leurs connexions.
async fn link(stack: &BleStack<'_>, dev: usize) {
    let target = critical_section::with(|cs| TARGETS.borrow_ref(cs)[dev]).unwrap();
    let accept = [target];
    let config = ConnectConfig {
        scan_config: ScanConfig {
            active: false,
            filter_accept_list: &accept,
            phys: PhySet::M1,
            interval: Duration::from_millis(60),
            window: Duration::from_millis(30),
            ..Default::default()
        },
        // Paramètres initiaux par défaut de NimBLE, ceux de coffeeflow.
        connect_params: RequestedConnParams {
            min_connection_interval: Duration::from_millis(30),
            max_connection_interval: Duration::from_millis(50),
            max_latency: 0,
            min_event_length: Duration::from_secs(0),
            max_event_length: Duration::from_secs(0),
            supervision_timeout: Duration::from_millis(2560),
        },
    };

    loop {
        LINK_STATE[dev].store(LINK_CONNECTING, Relaxed);
        let mut central = stack.central();
        let conn = match with_timeout(Duration::from_secs(10), central.connect(&config)).await {
            Ok(Ok(conn)) => conn,
            Ok(Err(e)) => {
                link_failed(dev, err_text("connexion", e));
                Timer::after(Duration::from_secs(2)).await;
                continue;
            }
            Err(_) => {
                link_failed(dev, err_text("connexion", "aucune reponse en 10 s"));
                continue;
            }
        };
        CONNECTS[dev].fetch_add(1, Relaxed);
        INTERVAL_US[dev].store(conn.params().conn_interval.as_micros() as u32, Relaxed);
        LINK_STATE[dev].store(LINK_DISCOVERING, Relaxed);

        let err = match Client::new(stack, &conn).await {
            Err(e) => err_text("mtu", e),
            Ok(client) => {
                let session = async {
                    if dev == SCALE {
                        scale_session(&client).await
                    } else {
                        probe_session(&client).await
                    }
                };
                match select3(client.task(), session, conn_events(&conn, dev)).await {
                    Either3::First(r) => err_text("gatt", r),
                    Either3::Second(text) => text,
                    Either3::Third(reason) => err_text("deconnexion", reason),
                }
            }
        };
        if LINK_STATE[dev].load(Relaxed) == LINK_SUBSCRIBED {
            DROPS[dev].fetch_add(1, Relaxed);
            set_error(dev, err);
        } else {
            link_failed(dev, err);
        }
        LINK_STATE[dev].store(LINK_DOWN, Relaxed);
        if conn.is_connected() {
            conn.disconnect();
            let _ = with_timeout(Duration::from_secs(5), conn_events(&conn, dev)).await;
        }
        Timer::after(Duration::from_secs(1)).await;
    }
}

fn link_failed(dev: usize, err: ErrText) {
    FAILS[dev].fetch_add(1, Relaxed);
    set_error(dev, err);
}

fn set_error(dev: usize, err: ErrText) {
    critical_section::with(|cs| LAST_ERROR.borrow_ref_mut(cs)[dev] = Some(err));
}

/// Vide la file d'événements de la connexion (deux places) et rend la cause
/// de la déconnexion. Retient l'intervalle accepté par l'appareil.
async fn conn_events(conn: &Connection<'_, DefaultPacketPool>, dev: usize) -> Status {
    loop {
        match conn.next().await {
            ConnectionEvent::Disconnected { reason } => return reason,
            ConnectionEvent::ConnectionParamsUpdated { conn_interval, .. } => {
                INTERVAL_US[dev].store(conn_interval.as_micros() as u32, Relaxed);
            }
            _ => {}
        }
    }
}

// Acaia Lunar (ble_scale.cpp). Service et caractéristique legacy, confirmés
// sur ACAIAL-C1EF4C9 ; repli sur le service moderne des Lunar récentes, à
// canaux d'écriture et de notification distincts. UUID 128 bits dans
// l'ordre de l'air.
const ACAIA_LEGACY_SERVICE: Uuid = Uuid::new_short(0x1820);
const ACAIA_LEGACY_CHAR: Uuid = Uuid::new_short(0x2a80);
const ACAIA_SERVICE: Uuid = Uuid::new_long([
    0x55, 0xe4, 0x05, 0xd2, 0xaf, 0x9f, 0xa9, 0x8f, 0xe5, 0x4a, 0x7d, 0xfe, 0x43, 0x53, 0x53, 0x49,
]);
const ACAIA_WRITE: Uuid = Uuid::new_long([
    0xb3, 0x9b, 0x72, 0x34, 0xbe, 0xec, 0xd4, 0xa8, 0xf4, 0x43, 0x41, 0x88, 0x43, 0x53, 0x53, 0x49,
]);
const ACAIA_NOTIFY: Uuid = Uuid::new_long([
    0x16, 0x96, 0x24, 0x47, 0xc6, 0x23, 0x61, 0xba, 0xd9, 0x4b, 0x4d, 0x1e, 0x43, 0x53, 0x53, 0x49,
]);
const ACAIA_HEARTBEAT: Duration = Duration::from_millis(2500);

/// Poignée d'écriture et caractéristique notifiée de la balance.
async fn scale_discover(client: &Client<'_>) -> Result<(u16, Characteristic<[u8]>), ErrText> {
    let legacy = client
        .services_by_uuid(&ACAIA_LEGACY_SERVICE)
        .await
        .map_err(|e| err_text("service", e))?;
    if let Some(service) = legacy.first() {
        let chr: Characteristic<[u8]> = client
            .characteristic_by_uuid(service, &ACAIA_LEGACY_CHAR)
            .await
            .map_err(|e| err_text("caracteristique", e))?;
        return Ok((chr.handle, chr));
    }
    let modern = client
        .services_by_uuid(&ACAIA_SERVICE)
        .await
        .map_err(|e| err_text("service moderne", e))?;
    let service = modern
        .first()
        .ok_or_else(|| err_text("service", "absent"))?;
    let write: Characteristic<[u8]> = client
        .characteristic_by_uuid(service, &ACAIA_WRITE)
        .await
        .map_err(|e| err_text("ecriture", e))?;
    let notify: Characteristic<[u8]> = client
        .characteristic_by_uuid(service, &ACAIA_NOTIFY)
        .await
        .map_err(|e| err_text("notification", e))?;
    Ok((write.handle, notify))
}

/// Trame Acaia : en-tête EF DD, commande, longueur facultative, charge,
/// puis les sommes des octets de rang pair et impair après la commande.
async fn acaia_send(
    client: &Client<'_>,
    handle: u16,
    cmd: u8,
    payload: &[u8],
    length_prefix: bool,
) -> Result<(), ErrText> {
    let mut frame = [0u8; 24];
    let mut n = 0;
    for b in [0xef, 0xdd, cmd] {
        frame[n] = b;
        n += 1;
    }
    if length_prefix {
        frame[n] = payload.len() as u8 + 1;
        n += 1;
    }
    frame[n..n + payload.len()].copy_from_slice(payload);
    n += payload.len();
    let (mut even, mut odd) = (0u8, 0u8);
    for (i, &b) in frame[3..n].iter().enumerate() {
        if i % 2 == 0 {
            even = even.wrapping_add(b);
        } else {
            odd = odd.wrapping_add(b);
        }
    }
    frame[n] = even;
    frame[n + 1] = odd;
    n += 2;
    client
        .write_handle_without_response(handle, &frame[..n])
        .await
        .map_err(|e| err_text("ecriture", e))
}

/// Identification, heartbeat et demande de notifications, envoyés ensemble
/// à l'abonnement puis toutes les 2,5 s, comme coffeeflow.
async fn acaia_heartbeat(client: &Client<'_>, handle: u16) -> Result<(), ErrText> {
    const ID: [u8; 15] = *b"012345678901234";
    const REQUEST: [u8; 8] = [0x00, 0x01, 0x01, 0x02, 0x02, 0x05, 0x03, 0x04];
    acaia_send(client, handle, 0x0b, &ID, false).await?;
    acaia_send(client, handle, 0x00, &[0], true).await?;
    acaia_send(client, handle, 0x0c, &REQUEST, true).await?;
    HEARTBEATS.fetch_add(1, Relaxed);
    Ok(())
}

async fn scale_session(client: &Client<'_>) -> ErrText {
    let (write_handle, notify) = match scale_discover(client).await {
        Ok(found) => found,
        Err(e) => return e,
    };
    let mut listener = match client.subscribe(&notify, false).await {
        Ok(listener) => listener,
        Err(e) => return err_text("abonnement", e),
    };
    LINK_STATE[SCALE].store(LINK_SUBSCRIBED, Relaxed);

    let mut rx = AcaiaRx::new();
    let mut next_heartbeat = Instant::now();
    loop {
        match select(listener.next(), Timer::at(next_heartbeat)).await {
            Either::First(notification) => {
                NOTIFS[SCALE].fetch_add(1, Relaxed);
                if rx.consume(notification.as_ref()) {
                    return err_text("balance", "deconnexion demandee (0x20)");
                }
            }
            Either::Second(()) => {
                if let Err(e) = acaia_heartbeat(client, write_handle).await {
                    return e;
                }
                next_heartbeat += ACAIA_HEARTBEAT;
            }
        }
    }
}

/// Recompose les trames Acaia réparties sur plusieurs notifications
/// (consume_rx de ble_scale.cpp).
struct AcaiaRx {
    buf: [u8; 128],
    len: usize,
}

impl AcaiaRx {
    const fn new() -> Self {
        Self {
            buf: [0; 128],
            len: 0,
        }
    }

    /// Vrai si la balance demande la déconnexion.
    fn consume(&mut self, data: &[u8]) -> bool {
        if data.len() > self.buf.len() {
            return false;
        }
        if self.len + data.len() > self.buf.len() {
            self.len = 0;
        }
        self.buf[self.len..self.len + data.len()].copy_from_slice(data);
        self.len += data.len();
        while self.len >= 3 {
            if self.buf[0] != 0xef || self.buf[1] != 0xdd {
                self.drop_front(1);
                continue;
            }
            if self.buf[2] == 0x20 {
                self.len = 0;
                return true;
            }
            if self.len < 6 {
                return false;
            }
            let payload_len = self.buf[3] as usize;
            let frame_len = payload_len + 5;
            if payload_len == 0 || frame_len > self.buf.len() {
                self.len = 0;
                return false;
            }
            if self.len < frame_len {
                return false;
            }
            if self.buf[2] == 12 {
                acaia_status(&self.buf[4..4 + payload_len]);
            }
            self.drop_front(frame_len);
        }
        false
    }

    fn drop_front(&mut self, n: usize) {
        self.buf.copy_within(n..self.len, 0);
        self.len -= n;
    }
}

/// Sous-messages d'une trame d'état : seule la pesée (type 5) est retenue.
fn acaia_status(payload: &[u8]) {
    let mut i = 0;
    while i < payload.len() {
        let kind = payload[i];
        i += 1;
        match kind {
            5 if i + 6 <= payload.len() => {
                WEIGHT_BITS.store(acaia_weight(&payload[i..i + 6]).to_bits(), Relaxed);
                WEIGHTS.fetch_add(1, Relaxed);
                i += 6;
            }
            7 if i + 3 <= payload.len() => i += 3,
            8 if i < payload.len() => i += 1,
            _ => break,
        }
    }
}

fn acaia_weight(data: &[u8]) -> f32 {
    let raw = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as f32;
    let weight = match data[4] {
        1 => raw / 10.0,
        2 => raw / 100.0,
        3 => raw / 1000.0,
        4 => raw / 10000.0,
        _ => raw,
    };
    if data[5] & 0x02 != 0 { -weight } else { weight }
}

// Sonde SCACE (common/scace_ble.hpp) : une trame de 12 octets à 10 Hz.
// UUID dans l'ordre de l'air.
const SCACE_SERVICE: Uuid = Uuid::new_long([
    0xfe, 0x01, 0x12, 0x4b, 0xb2, 0x61, 0x2e, 0xa2, 0xd8, 0x46, 0xa7, 0x7f, 0x0a, 0x12, 0x17, 0x50,
]);
const SCACE_FRAME: Uuid = Uuid::new_long([
    0x38, 0x9a, 0xa7, 0xae, 0x17, 0x6f, 0x21, 0xb2, 0x9a, 0x4c, 0x71, 0x11, 0x74, 0xf5, 0xfe, 0x84,
]);
const SCACE_FRAME_SIZE: usize = 12;
const SCACE_STATUS_MAX: u8 = 3;
const SCACE_NO_TEMPERATURE: i16 = i16::MIN;

async fn probe_session(client: &Client<'_>) -> ErrText {
    let services = match client.services_by_uuid(&SCACE_SERVICE).await {
        Ok(services) => services,
        Err(e) => return err_text("service", e),
    };
    let Some(service) = services.first() else {
        return err_text("service", "absent");
    };
    let frame: Characteristic<[u8]> = match client.characteristic_by_uuid(service, &SCACE_FRAME).await {
        Ok(chr) => chr,
        Err(e) => return err_text("caracteristique", e),
    };
    let mut listener = match client.subscribe(&frame, false).await {
        Ok(listener) => listener,
        Err(e) => return err_text("abonnement", e),
    };
    LINK_STATE[PROBE].store(LINK_SUBSCRIBED, Relaxed);

    // Les trames manquées se lisent dans les numéros de séquence. La file de
    // notifications de trouble-host n'a qu'une place : une trame non lue à
    // temps est écrasée par la suivante, et compte comme manquée.
    let mut last_seq: Option<u16> = None;
    loop {
        let notification = listener.next().await;
        NOTIFS[PROBE].fetch_add(1, Relaxed);
        let data = notification.as_ref();
        if data.len() < SCACE_FRAME_SIZE || data[10] > SCACE_STATUS_MAX {
            PROBE_INVALID.fetch_add(1, Relaxed);
            continue;
        }
        let seq = u16::from_le_bytes([data[0], data[1]]);
        if let Some(last) = last_seq {
            // Un saut énorme est un redémarrage de la sonde, pas une perte.
            let gap = seq.wrapping_sub(last).wrapping_sub(1);
            if gap < 1000 {
                PROBE_LOST.fetch_add(gap as u32, Relaxed);
            }
        }
        last_seq = Some(seq);
        PROBE_CENTI_C.store(i16::from_le_bytes([data[8], data[9]]) as u16 as u32, Relaxed);
        PROBE_STATUS.store(data[10] as u32, Relaxed);
        PROBE_LAST_MS.store(Instant::now().as_millis() as u32, Relaxed);
    }
}
