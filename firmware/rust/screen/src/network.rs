//! Radio et TCP sur le cœur 1, avec événements vers le propriétaire du cœur.
//! Les réponses lentes sont fermées ; jamais une trame WebSocket entrante
//! n'est injectée sur le bus CAN.
#![allow(
    clippy::large_stack_frames,
    reason = "tampons TCP et futures stockés dans l'arène Embassy"
)]
use crate::{
    nvs,
    service::{self, Operation, Reply},
};
use alloc::{format, string::ToString, vec::Vec};
use core::{
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    sync::atomic::{AtomicBool, AtomicU32, Ordering},
};
use embassy_executor::Spawner;
use embassy_futures::select::{Either, select};
use embassy_net::{Ipv4Cidr, Runner, Stack, StackResources, StaticConfigV4, tcp::TcpSocket};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};
use embassy_time::{Duration, Instant, Timer, with_timeout};
use embedded_io_async::Write;
use esp_hal::{peripherals::WIFI, rng::Rng};
use esp_radio::wifi::{
    AuthenticationMethodConfig, Config, ControllerConfig, Interface, WifiController,
    ap::AccessPointConfig, sta::StationConfig,
};
use screen_core::{
    core::{FlashStart, RadioRequest, WifiRequest},
    http::{self, FirmwareTarget, HttpResponse},
    snapshot::NetworkState,
};
use screen_net::{credentials::Credentials, http::Request, websocket};
use static_cell::StaticCell;
mod secrets {
    include!(concat!(env!("OUT_DIR"), "/secrets.rs"));
}

/// Même nom public que le firmware IDF : deux derniers octets du MAC AP.
pub fn provisioning_ssid() -> alloc::string::String {
    let mac = esp_hal::efuse::base_mac_address();
    let bytes = mac.as_bytes();
    let suffix = u16::from_be_bytes([bytes[4], bytes[5]]).wrapping_add(1);
    format!("CoffeeFlow-{suffix:04X}")
}
static SCAN: Channel<CriticalSectionRawMutex, usize, 4> = Channel::new();
static SCAN_REPLY: [embassy_sync::signal::Signal<
    CriticalSectionRawMutex,
    Option<alloc::string::String>,
>; 4] = [const { embassy_sync::signal::Signal::new() }; 4];
const ADDRESS: Ipv4Addr = Ipv4Addr::new(192, 168, 4, 1);
static DESIRED: AtomicU32 = AtomicU32::new(1);
/// Diagnostic : la station a une adresse.
pub static STA_UP: AtomicBool = AtomicBool::new(false);
static PROVISIONING: AtomicBool = AtomicBool::new(false);
static FORGET: AtomicBool = AtomicBool::new(false);
static WALL_GENERATION: AtomicU32 = AtomicU32::new(0);
static WALL_SECONDS: AtomicU32 = AtomicU32::new(0);
static WALL_MONO_S: AtomicU32 = AtomicU32::new(0);
static WS_ACTIVE: AtomicU32 = AtomicU32::new(0);
static WS_OVERFLOW: AtomicU32 = AtomicU32::new(0);
static WS: [Channel<CriticalSectionRawMutex, protocol::Frame, 32>; 4] =
    [const { Channel::new() }; 4];
pub fn configure(request: RadioRequest) {
    DESIRED.store(
        match request.wifi {
            WifiRequest::Off => 0,
            WifiRequest::Sta => 1,
            WifiRequest::Provisioning => 2,
        },
        Ordering::Release,
    );
}
pub fn forget_network() {
    FORGET.store(true, Ordering::Release);
}
pub fn wall_time() -> Option<i64> {
    loop {
        crate::crumb!();
        let generation = WALL_GENERATION.load(Ordering::Acquire);
        if generation & 1 != 0 {
            core::hint::spin_loop();
            continue;
        }
        let seconds = WALL_SECONDS.load(Ordering::Acquire);
        let mono = WALL_MONO_S.load(Ordering::Acquire);
        if WALL_GENERATION.load(Ordering::Acquire) != generation {
            continue;
        }
        return (seconds != 0).then(|| {
            i64::from(seconds) + i64::from((Instant::now().as_secs() as u32).wrapping_sub(mono))
        });
    }
}
pub fn mirror(frame: protocol::Frame) {
    let mask = WS_ACTIVE.load(Ordering::Acquire);
    for (worker, queue) in WS.iter().enumerate() {
        if mask & (1 << worker) != 0 && queue.try_send(frame).is_err() {
            WS_OVERFLOW.fetch_or(1 << worker, Ordering::Release);
        }
    }
}
fn ap_config() -> Option<Config> {
    let password = secrets::PROVISIONING_AP_PASSWORD;
    if password == "CHANGE_ME" || !(8..=63).contains(&password.len()) {
        return None;
    }
    Some(Config::AccessPoint(
        AccessPointConfig::default()
            .with_ssid(provisioning_ssid().as_str().try_into().ok()?)
            .with_authentication(AuthenticationMethodConfig::Wpa2Personal(
                password.try_into().ok()?,
            ))
            .with_max_connections(2),
    ))
}
fn sta_config(credentials: &Credentials) -> Option<Config> {
    Some(Config::Station(
        StationConfig::default()
            .with_ssid(credentials.ssid.as_str().try_into().ok()?)
            .with_authentication(AuthenticationMethodConfig::Wpa2Personal(
                credentials.password.as_str().try_into().ok()?,
            )),
    ))
}
/// Appeler après `nvs::start`, sur le cœur 1, avant d'annoncer la radio prête.
pub async fn start(spawner: Spawner, wifi: WIFI<'static>) {
    let stored = nvs::request(nvs::Operation::Read {
        namespace: "wifi",
        key: "credentials",
    })
    .await;
    let credentials = stored.value.and_then(|blob| Credentials::from_blob(&blob));
    let provisioning = credentials.is_none();
    PROVISIONING.store(provisioning, Ordering::Release);
    service::notify(Operation::Credentials(!provisioning)).await;
    service::notify(Operation::BootTimeSyncing(!provisioning)).await;
    let config = credentials.as_ref().and_then(sta_config).or_else(ap_config);
    // Diagnostic : `DIAG_NO_WIFI=1` au build laisse le Wi-Fi éteint.
    let config = if option_env!("DIAG_NO_WIFI").is_some() { None } else { config };
    let Some(config) = config else {
        service::notify(Operation::Network {
            state: NetworkState::Off,
            ipv4: 0,
        })
        .await;
        service::notify(Operation::BootTimeSyncing(false)).await;
        return;
    };
    let Ok(controller) = WifiController::new(
        wifi,
        ControllerConfig::default().with_initial_config(config),
    ) else {
        // L'UI ne doit pas rester dans l'attente d'une heure que ce pilote
        // n'obtiendra jamais. Le BLE et la logique peuvent continuer seuls.
        service::notify(Operation::Network {
            state: NetworkState::Off,
            ipv4: 0,
        })
        .await;
        service::notify(Operation::BootTimeSyncing(false)).await;
        return;
    };
    // esp-radio met la station sans économie d'énergie (WIFI_PS_NONE), alors
    // qu'IDF, donc coffeeflow C++, part en modem sleep minimal. Avec le BLE,
    // la coexistence a besoin de ce sommeil pour alterner les créneaux radio.
    let mut controller = controller;
    let _ = controller.set_power_saving(esp_radio::wifi::PowerSaveMode::Minimum);
    let rng = Rng::new();
    let seed = u64::from(rng.random()) << 32 | u64::from(rng.random());
    static STA_RESOURCES: StaticCell<StackResources<8>> = StaticCell::new();
    static AP_RESOURCES: StaticCell<StackResources<8>> = StaticCell::new();
    let (sta, sta_runner) = embassy_net::new(
        Interface::station(),
        embassy_net::Config::dhcpv4(Default::default()),
        STA_RESOURCES.init(StackResources::new()),
        seed,
    );
    let (ap, ap_runner) = embassy_net::new(
        Interface::access_point(),
        embassy_net::Config::ipv4_static(StaticConfigV4 {
            address: Ipv4Cidr::new(ADDRESS, 24),
            gateway: Some(ADDRESS),
            dns_servers: Default::default(),
        }),
        AP_RESOURCES.init(StackResources::new()),
        seed ^ 1,
    );
    spawner.spawn(net_task(sta_runner).unwrap());
    spawner.spawn(net_task(ap_runner).unwrap());
    spawner.spawn(connection(controller, credentials, sta).unwrap());
    spawner.spawn(dhcp(ap).unwrap());
    spawner.spawn(sntp(sta).unwrap());
    if !provisioning {
        spawner.spawn(boot_sync(sta).unwrap());
    }
    for worker in 0..4 {
        spawner.spawn(server(worker, sta, ap).unwrap());
    }
}
#[embassy_executor::task(pool_size = 2)]
async fn net_task(mut runner: Runner<'static, Interface>) {
    runner.run().await
}
#[embassy_executor::task]
async fn connection(
    mut controller: WifiController<'static>,
    mut credentials: Option<Credentials>,
    stack: Stack<'static>,
) {
    let mut applied = if credentials.is_some() { 1 } else { 2 };
    loop {
        crate::crumb!();
        if FORGET.swap(false, Ordering::AcqRel) && nvs::request(nvs::Operation::Forget).await.ok {
            credentials = None;
            PROVISIONING.store(true, Ordering::Release);
            service::notify(Operation::Credentials(false)).await;
            DESIRED.store(2, Ordering::Release);
        }
        if let Ok(worker) = SCAN.try_receive() {
            let result = if applied == 2 {
                // esp-radio refuse le scan en AP seul. Une station non connectée
                // n'est activée que durant ce scan opportuniste, puis supprimée.
                if let Some(Config::AccessPoint(ap)) = ap_config() {
                    let station =
                        StationConfig::default().with_ssid("provisioning-scan".try_into().unwrap());
                    let _ = controller.set_config(&Config::AccessPointStation(station, ap));
                    let scanned = with_timeout(
                        Duration::from_secs(10),
                        controller
                            .scan_async(&esp_radio::wifi::scan::ScanConfig::default().with_max(20)),
                    )
                    .await;
                    if let Some(ap) = ap_config() {
                        let _ = controller.set_config(&ap);
                    }
                    scanned.ok().and_then(Result::ok).map(|networks| {
                        let mut list = alloc::string::String::new();
                        for network in networks {
                            list.push_str(network.ssid.as_str());
                            list.push('\n');
                        }
                        list
                    })
                } else {
                    None
                }
            } else {
                None
            };
            SCAN_REPLY[worker].signal(result);
        }
        let desired = if credentials.is_none() {
            2
        } else {
            DESIRED.load(Ordering::Acquire)
        };
        if desired != applied {
            if desired == 0 {
                // Le contrôleur n'expose pas d'arrêt public ; l'appel IDF ne
                // détruit pas le contrôleur et ne coupe pas la radio BLE.
                unsafe {
                    esp_wifi_sys_esp32s3::include::esp_wifi_stop();
                }
            } else if let Some(config) = if desired == 2 {
                ap_config()
            } else {
                credentials.as_ref().and_then(sta_config)
            } {
                if controller.set_config(&config).is_err() {
                    Timer::after_secs(1).await;
                    continue;
                }
                unsafe {
                    esp_wifi_sys_esp32s3::include::esp_wifi_start();
                }
            }
            applied = desired;
        }
        let state = if applied == 0 {
            NetworkState::Off
        } else if applied == 2 {
            NetworkState::ApProvisioning
        } else if stack.is_config_up() {
            NetworkState::StaConnected
        } else {
            NetworkState::StaConnecting
        };
        let up = state == NetworkState::StaConnected;
        if up != STA_UP.swap(up, Ordering::AcqRel) {
            esp_println::println!(
                "[diag] Wi-Fi {} à {} ms",
                if up { "associé" } else { "perdu" },
                embassy_time::Instant::now().as_millis()
            );
        }
        let ipv4 = if state == NetworkState::StaConnected {
            stack
                .config_v4()
                .map_or(0, |c| u32::from_le_bytes(c.address.address().octets()))
        } else {
            0
        };
        service::notify(Operation::Network { state, ipv4 }).await;
        if applied == 1 && !controller.is_connected() {
            let _ = with_timeout(Duration::from_secs(5), controller.connect_async()).await;
        }
        Timer::after_millis(500).await;
    }
}
#[embassy_executor::task]
async fn dhcp(stack: Stack<'static>) {
    use edge_dhcp::{
        io::{self, DEFAULT_SERVER_PORT},
        server::{Server, ServerOptions},
    };
    use edge_nal::UdpBind;
    use edge_nal_embassy::{Udp, UdpBuffers};
    let mut bytes = [0; 1500];
    let mut gateways = [Ipv4Addr::UNSPECIFIED];
    let buffers = UdpBuffers::<1, 1024, 1024, 4>::new();
    let udp = Udp::new(stack, &buffers);
    let Ok(mut socket) = udp
        .bind(SocketAddr::V4(SocketAddrV4::new(
            Ipv4Addr::UNSPECIFIED,
            DEFAULT_SERVER_PORT,
        )))
        .await
    else {
        return;
    };
    let mut server = Server::<_, 8>::new_with_et(ADDRESS);
    loop {
        crate::crumb!();
        let _ = io::server::run(
            &mut server,
            &ServerOptions::new(ADDRESS, Some(&mut gateways)),
            &mut socket,
            &mut bytes,
        )
        .await;
        Timer::after_secs(1).await;
    }
}
#[embassy_executor::task]
async fn boot_sync(stack: Stack<'static>) {
    // Bornes du firmware archivé : 5 s pour association/DHCP, puis 10 s
    // pour une première heure connue. L'échec laisse l'heure facultative.
    if with_timeout(Duration::from_secs(5), stack.wait_config_up())
        .await
        .is_ok()
    {
        let _ = with_timeout(Duration::from_secs(10), async {
            while WALL_SECONDS.load(Ordering::Acquire) == 0 {
                Timer::after_millis(50).await;
            }
        })
        .await;
    }
    service::notify(Operation::BootTimeSyncing(false)).await;
}
#[embassy_executor::task]
async fn sntp(stack: Stack<'static>) {
    use embassy_net::udp::{PacketMetadata, UdpSocket};
    let mut rx_meta = [PacketMetadata::EMPTY; 1];
    let mut tx_meta = [PacketMetadata::EMPTY; 1];
    let mut rx = [0; 512];
    let mut tx = [0; 512];
    let mut socket = UdpSocket::new(stack, &mut rx_meta, &mut rx, &mut tx_meta, &mut tx);
    if socket.bind(49152).is_err() {
        return;
    }
    loop {
        crate::crumb!();
        stack.wait_config_up().await;
        let cookie = (Instant::now().as_micros()).to_be_bytes();
        let address = match stack
            .dns_query("pool.ntp.org", embassy_net::dns::DnsQueryType::A)
            .await
        {
            Ok(addresses) => match addresses.first() {
                Some(embassy_net::IpAddress::Ipv4(address)) => *address,
                _ => {
                    Timer::after_secs(5).await;
                    continue;
                }
            },
            Err(_) => {
                Timer::after_secs(5).await;
                continue;
            }
        };
        if socket
            .send_to(&screen_net::sntp::request(cookie), (address, 123))
            .await
            .is_ok()
        {
            let mut response = [0; 48];
            if let Ok(Ok((len, from))) =
                with_timeout(Duration::from_secs(10), socket.recv_from(&mut response)).await
                && from.endpoint.addr == embassy_net::IpAddress::Ipv4(address)
                && from.endpoint.port == 123
                && let Some(seconds) = screen_net::sntp::response(&response[..len], cookie)
            {
                WALL_GENERATION.fetch_add(1, Ordering::AcqRel);
                WALL_MONO_S.store(Instant::now().as_secs() as u32, Ordering::Release);
                WALL_SECONDS.store(seconds as u32, Ordering::Release);
                WALL_GENERATION.fetch_add(1, Ordering::Release);
            }
        }
        Timer::after_secs(if WALL_SECONDS.load(Ordering::Acquire) == 0 {
            5
        } else {
            3600
        })
        .await;
    }
}

#[embassy_executor::task(pool_size = 4)]
async fn server(worker: usize, sta: Stack<'static>, ap: Stack<'static>) {
    // Fenêtre TCP agrandie : les paquets d'OTA ne sont pas arrêtés à chaque
    // secteur écrit ; les interruptions continuent sur le cœur radio.
    let mut rx = crate::heap::psram_buffer::<8192>();
    let mut tx = crate::heap::psram_buffer::<2048>();
    loop {
        crate::crumb!();
        let stack = if PROVISIONING.load(Ordering::Acquire) {
            ap
        } else {
            sta
        };
        let mut socket = TcpSocket::new(stack, &mut rx[..], &mut tx[..]);
        socket.set_timeout(Some(Duration::from_secs(10)));
        if with_timeout(Duration::from_secs(1), socket.accept(80))
            .await
            .is_ok_and(|v| v.is_ok())
        {
            let _ = serve(worker, &mut socket).await;
        }
        socket.close();
        let _ = with_timeout(Duration::from_secs(1), socket.flush()).await;
        socket.abort();
    }
}
struct Broken;
async fn send(
    socket: &mut TcpSocket<'_>,
    response: HttpResponse,
    kind: &str,
) -> Result<(), Broken> {
    let head = format!(
        "HTTP/1.1 {}\r\nContent-Type: {}\r\nCache-Control: no-store\r\nConnection: close\r\nContent-Length: {}\r\n\r\n",
        response.status,
        kind,
        response.body.len()
    );
    socket
        .write_all(head.as_bytes())
        .await
        .map_err(|_| Broken)?;
    socket
        .write_all(response.body.as_bytes())
        .await
        .map_err(|_| Broken)
}
fn response(status: &'static str, body: &str) -> HttpResponse {
    HttpResponse {
        status,
        body: body.to_string(),
    }
}
async fn call_http(worker: usize, operation: Operation) -> HttpResponse {
    match service::call(worker, operation).await {
        Reply::Http(response) => response,
        _ => http::error_response("500 Internal Server Error", "internal", None),
    }
}
async fn serve(worker: usize, socket: &mut TcpSocket<'_>) -> Result<(), Broken> {
    let mut head = crate::heap::psram_buffer::<4096>();
    let mut used = 0;
    let request = loop {
        crate::crumb!();
        match Request::parse(&head[..used]) {
            Ok(request) => break request,
            Err(screen_net::http::Error::Incomplete) if used < head.len() => {}
            _ => {
                return send(
                    socket,
                    http::error_response("400 Bad Request", "invalid_request", None),
                    "application/json",
                )
                .await;
            }
        }
        let len = socket.read(&mut head[used..]).await.map_err(|_| Broken)?;
        if len == 0 {
            return Err(Broken);
        }
        used += len;
    };
    let body_prefix = &head[request.header_bytes..used];
    if PROVISIONING.load(Ordering::Acquire) {
        if request.method == "GET" && request.path == "/" {
            return send(socket,response("200 OK",r#"<!doctype html><html lang=fr><meta name=viewport content="width=device-width"><title>coffeeflow</title><h1>Configurer le réseau</h1><button type=button onclick="fetch('/scan').then(r=>r.text()).then(t=>document.getElementById('networks').textContent=t)">Voir les reseaux</button><pre id=networks></pre><form method=post action=/provision><label>SSID <input name=ssid maxlength=32 required></label><br><label>Mot de passe <input name=password type=password minlength=8 maxlength=63 required></label><br><button>Connecter</button></form></html>"#),"text/html; charset=utf-8").await;
        }
        if request.method == "GET" && request.path == "/scan" {
            SCAN.send(worker).await;
            let response = match SCAN_REPLY[worker].wait().await {
                Some(list) => HttpResponse {
                    status: "200 OK",
                    body: list,
                },
                None => response("500 Internal Server Error", "scan unavailable"),
            };
            return send(socket, response, "text/plain; charset=utf-8").await;
        }
        if request.method == "POST" && request.path == "/provision" && request.content_length <= 512
        {
            let body = read_body(socket, body_prefix, request.content_length).await?;
            let Some(credentials) = Credentials::from_form(&body) else {
                return send(
                    socket,
                    response("400 Bad Request", "Identifiants invalides"),
                    "text/plain; charset=utf-8",
                )
                .await;
            };
            if !nvs::request(nvs::Operation::Write {
                namespace: "wifi",
                key: "credentials",
                value: credentials.to_blob().to_vec(),
            })
            .await
            .ok
            {
                return send(
                    socket,
                    response("500 Internal Server Error", "Erreur stockage"),
                    "text/plain",
                )
                .await;
            }
            send(
                socket,
                response("200 OK", "Identifiants enregistrés. Redémarrage."),
                "text/plain; charset=utf-8",
            )
            .await?;
            reboot_after_response(socket).await;
        }
        return send(
            socket,
            response("404 Not Found", "Introuvable"),
            "text/plain",
        )
        .await;
    }
    let token = secrets::HTTP_BEARER_TOKEN;
    let authorized = token != "CHANGE_ME"
        && !token.is_empty()
        && request.authorization.is_some_and(|value| {
            let expected = format!("Bearer {token}");
            value.len() == expected.len()
                && value
                    .bytes()
                    .zip(expected.bytes())
                    .fold(0u8, |a, (b, c)| a | (b ^ c))
                    == 0
        });
    if !authorized {
        service::notify(Operation::Unauthorized).await;
        return send(
            socket,
            http::error_response("401 Unauthorized", "unauthorized", None),
            "application/json",
        )
        .await;
    }
    if request.method == "GET" && request.path == "/ws" {
        return websocket_session(worker, socket, &request).await;
    }
    if request.method == "POST" && request.path == "/firmware" {
        return firmware(worker, socket, &request, body_prefix).await;
    }
    if request.method == "POST" && request.path == "/firmware/confirm" {
        let ready = matches!(
            service::call(worker, Operation::Confirm).await,
            Reply::Ready(true)
        );
        let confirmed = if ready {
            let mut guard = nvs::FLASH.lock().await;
            crate::ota_local::confirm(guard.as_mut().unwrap(), nvs::layout()).is_ok()
        } else {
            false
        };
        send(
            socket,
            http::firmware_confirm_response(confirmed),
            "application/json",
        )
        .await?;
        if confirmed {
            reboot_after_response(socket).await;
        }
        return Ok(());
    }
    if request.content_length > http::MAX_BODY {
        return send(
            socket,
            http::error_response("400 Bad Request", "invalid_json", None),
            "application/json",
        )
        .await;
    }
    let operation = match (request.method, request.path) {
        ("GET", "/telemetry") => Operation::Telemetry,
        ("GET", "/config") => Operation::Config,
        ("GET", "/hf-capture") => Operation::Hf(request.query.map(ToString::to_string)),
        ("POST", "/config") => {
            Operation::PostConfig(read_body(socket, body_prefix, request.content_length).await?)
        }
        ("POST", "/action") => {
            Operation::Action(read_body(socket, body_prefix, request.content_length).await?)
        }
        _ => {
            return send(
                socket,
                http::error_response("404 Not Found", "not_found", None),
                "application/json",
            )
            .await;
        }
    };
    send(
        socket,
        call_http(worker, operation).await,
        "application/json",
    )
    .await
}
async fn read_body(
    socket: &mut TcpSocket<'_>,
    prefix: &[u8],
    length: usize,
) -> Result<Vec<u8>, Broken> {
    let mut body = alloc::vec![0;length];
    let mut done = prefix.len().min(length);
    body[..done].copy_from_slice(&prefix[..done]);
    while done < length {
        let got = socket.read(&mut body[done..]).await.map_err(|_| Broken)?;
        if got == 0 {
            return Err(Broken);
        }
        done += got;
    }
    Ok(body)
}
async fn reboot_after_response(socket: &mut TcpSocket<'_>) -> ! {
    let _ = socket.flush().await;
    Timer::after_millis(250).await;
    esp_hal::system::software_reset()
}

async fn websocket_session(
    worker: usize,
    socket: &mut TcpSocket<'_>,
    request: &Request<'_>,
) -> Result<(), Broken> {
    let Some(key) = request
        .websocket_key
        .filter(|_| request.websocket_upgrade)
        .and_then(websocket::accept_key)
    else {
        return send(
            socket,
            http::error_response("400 Bad Request", "invalid_websocket", None),
            "application/json",
        )
        .await;
    };
    let active = WS_ACTIVE.load(Ordering::Acquire);
    if active.count_ones() >= 3 {
        return send(
            socket,
            http::error_response("503 Service Unavailable", "busy", None),
            "application/json",
        )
        .await;
    }
    // Les tâches sont toutes dans le même exécuteur ; aucun await entre
    // contrôle et ajout ne laisse une quatrième session entrer.
    WS_ACTIVE.fetch_or(1 << worker, Ordering::AcqRel);
    WS_OVERFLOW.fetch_and(!(1 << worker), Ordering::AcqRel);
    WS[worker].clear();
    let result=async {
        socket.write_all(format!("HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {key}\r\n\r\n").as_bytes()).await.map_err(|_|Broken)?;
        let mut incoming=[0;144]; let mut received=0;
        loop {
            crate::crumb!();
            if WS_OVERFLOW.load(Ordering::Acquire)&(1<<worker)!=0 {return Err(Broken);}
            match select(WS[worker].receive(),socket.read(&mut incoming[received..])).await {
                Either::First(frame)=>{
                    let mut bytes=[0;protocol::framing::MAX_PDU_SIZE];let length=protocol::framing::encode_pdu(&frame,&mut bytes).map_err(|_|Broken)?;
                    let (head,used)=websocket::binary_header(length);
                    with_timeout(Duration::from_secs(1),async {socket.write_all(&head[..used]).await?;socket.write_all(&bytes[..length]).await}).await.map_err(|_|Broken)?.map_err(|_|Broken)?;
                }
                Either::Second(Ok(0) | Err(_)) => return Ok(()),
                Either::Second(Ok(count)) => {
                    received += count;
                    loop {
                        crate::crumb!();
                        match websocket::ClientFrame::parse(&incoming[..received]) {
                            Ok(frame) => {
                                if frame.opcode == 8 { return Ok(()); }
                                if frame.opcode == 9 {
                                    let head = [0x8a, frame.length as u8];
                                    socket.write_all(&head).await.map_err(|_| Broken)?;
                                    socket.write_all(&frame.payload[..frame.length]).await.map_err(|_| Broken)?;
                                }
                                // Les messages texte/binaires sont consommés puis ignorés.
                                incoming.copy_within(frame.consumed..received, 0);
                                received -= frame.consumed;
                                if received == 0 { break; }
                            }
                            Err(websocket::ClientError::Incomplete) => break,
                            Err(websocket::ClientError::Invalid) => return Err(Broken),
                        }
                    }
                }
            }
        }
    }.await;
    WS_ACTIVE.fetch_and(!(1 << worker), Ordering::AcqRel);
    WS[worker].clear();
    result
}

async fn firmware(
    worker: usize,
    socket: &mut TcpSocket<'_>,
    request: &Request<'_>,
    prefix: &[u8],
) -> Result<(), Broken> {
    if request.content_length > u32::MAX as usize {
        return send(
            socket,
            http::error_response("400 Bad Request", "invalid_firmware", Some("size")),
            "application/json",
        )
        .await;
    }
    let begin = service::call(
        worker,
        Operation::BeginUpload {
            query: request.query.map(ToString::to_string),
            size: request.content_length as i64,
        },
    )
    .await;
    let Reply::Upload(begin) = begin else {
        return Err(Broken);
    };
    let (target, start) = match begin {
        Ok(ok) => ok,
        Err(response) => return send(socket, response, "application/json").await,
    };
    let outcome = async {
        if start == FlashStart::AwaitingStop {
            let mut ready = false;
            for _ in 0..20 {
                Timer::after_millis(25).await;
                if matches!(
                    service::call(worker, Operation::PollUpload).await,
                    Reply::Ready(true)
                ) {
                    ready = true;
                    break;
                }
            }
            if !ready {
                return Err("busy");
            }
        }
        let replay_safe = matches!(
            service::call(worker, Operation::SensorsReplaySafe).await,
            Reply::Ready(true)
        );
        let layout = nvs::layout();
        let mut guard = nvs::FLASH.lock().await;
        let flash = guard.as_mut().unwrap();
        enum Upload {
            Screen(crate::ota_local::Upload),
            Sensors(crate::proxy_service::Upload),
        }
        let mut upload = match target {
            FirmwareTarget::Screen => Upload::Screen(crate::ota_local::Upload::begin(
                flash,
                layout,
                request.content_length as u32,
            )?),
            FirmwareTarget::Sensors
                if request.content_length > crate::ota_proxy::MAX_IMAGE_BYTES as usize
                    || request.content_length + crate::ota_proxy::HEADER_BYTES as usize
                        > layout.staging.size as usize =>
            {
                return Err("size");
            }
            FirmwareTarget::Sensors => Upload::Sensors(
                crate::proxy_service::Upload::begin(
                    flash,
                    layout.staging,
                    request.content_length as u32,
                )
                .map_err(|_| "write_failed")?,
            ),
        };
        let mut written = 0;
        let mut bytes = [0; 2048];
        loop {
            crate::crumb!();
            let data = if written == 0 && !prefix.is_empty() {
                &prefix[..prefix.len().min(request.content_length)]
            } else {
                if written >= request.content_length {
                    break;
                }
                let limit = (request.content_length - written).min(bytes.len());
                let len = socket
                    .read(&mut bytes[..limit])
                    .await
                    .map_err(|_| "incomplete_upload")?;
                if len == 0 {
                    return Err("incomplete_upload");
                }
                &bytes[..len]
            };
            match &mut upload {
                Upload::Screen(upload) => upload.write(flash, data)?,
                Upload::Sensors(upload) => upload.write(flash, data).map_err(|_| "write_failed")?,
            }
            written += data.len();
            service::notify(Operation::UploadProgress(written as u32)).await;
            if written >= request.content_length {
                break;
            }
        }
        match upload {
            Upload::Screen(upload) => upload.finish(flash, layout)?,
            Upload::Sensors(upload) => {
                let image = upload.commit(flash).map_err(|_| "queue_failed")?;
                // La réponse part avant le transfert CAN. La pompe est déjà
                // arrêtée et le cœur reste en flash jusqu'au résultat final.
                send(
                    socket,
                    response("202 Accepted", http::FIRMWARE_SENSORS_QUEUED),
                    "application/json",
                )
                .await
                .map_err(|_| "incomplete_upload")?;
                let _ = socket.flush().await;
                let _ =
                    crate::proxy_service::send_staged(flash, layout.staging, image, replay_safe)
                        .await;
            }
        }
        Ok(())
    }
    .await;
    let _ = service::call(worker, Operation::FinishUpload).await;
    match outcome {
        Ok(()) if target == FirmwareTarget::Screen => {
            send(
                socket,
                response("202 Accepted", http::FIRMWARE_SCREEN_ACCEPTED),
                "application/json",
            )
            .await?;
            reboot_after_response(socket).await;
        }
        Ok(()) => Ok(()),
        Err(error) => {
            send(
                socket,
                http::error_response(
                    if error == "busy" {
                        "409 Conflict"
                    } else if error == "write_failed" || error == "queue_failed" {
                        "500 Internal Server Error"
                    } else {
                        "400 Bad Request"
                    },
                    error,
                    if error == "size" { Some("size") } else { None },
                ),
                "application/json",
            )
            .await
        }
    }
}
