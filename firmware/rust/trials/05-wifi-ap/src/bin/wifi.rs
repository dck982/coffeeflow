// Point d'accès Wi-Fi de l'essai, sur le modèle de l'exemple
// `wifi/embassy_access_point` d'esp-hal : WPA2 avec un SSID et un mot de
// passe propres à l'essai (aucune lecture de NVS), adresse fixe, serveur
// DHCP, et un serveur HTTP qui sert la page de charge et `/stats`. Tout
// tourne dans l'exécuteur du cœur 1, et rien ici ne journalise : les
// erreurs sont comptées, et le rapport du cœur 0 les affiche.

use core::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use core::sync::atomic::{AtomicU32, Ordering::Relaxed};

use embassy_executor::Spawner;
use embassy_net::tcp::TcpSocket;
use embassy_net::{Ipv4Cidr, Runner, Stack, StackResources, StaticConfigV4};
use embassy_time::{Duration, Timer, with_timeout};
use embedded_io_async::Write;
use esp_hal::peripherals::WIFI;
use esp_hal::rng::Rng;
use esp_radio::wifi::ap::AccessPointConfig;
use esp_radio::wifi::event::{EventInfo, MessageResult};
use esp_radio::wifi::{
    AuthenticationMethodConfig, Config, ControllerConfig, Interface, WifiController,
};
use static_cell::StaticCell;

pub const SSID: &str = "coffeeflow-essai-05";
pub const PASSWORD: &str = "essai-rust-05";
const ADDRESS: Ipv4Addr = Ipv4Addr::new(192, 168, 2, 1);
const HTTP_PORT: u16 = 80;

/// Connexions HTTP servies en parallèle. Safari en ouvre plusieurs : la page,
/// puis `/stats` en keep-alive, et parfois une connexion d'avance.
const HTTP_SOCKETS: usize = 4;
/// Sockets du stack : HTTP, le serveur DHCP, et le socket DNS qu'embassy-net
/// réserve à sa création (feature `dns`, activée par edge-nal-embassy).
const SOCKETS: usize = HTTP_SOCKETS + 2;

const AP_INIT: u32 = 0;
const AP_STARTED: u32 = 1;
const AP_FAILED: u32 = 2;
pub const AP_STATE_NAMES: [&str; 3] = ["init", "ap", "echec"];
pub static AP_STATE: AtomicU32 = AtomicU32::new(AP_INIT);
/// Téléphones associés au point d'accès.
pub static STATIONS: AtomicU32 = AtomicU32::new(0);
/// Événements de station perdus par l'abonnement : le compte peut alors
/// être faux.
pub static STATION_EVENTS_LOST: AtomicU32 = AtomicU32::new(0);
/// Requêtes servies, toutes adresses confondues.
pub static HTTP_REQUESTS: AtomicU32 = AtomicU32::new(0);
/// Connexions HTTP interrompues par une erreur (lecture, écriture, délai,
/// requête trop longue ou illisible).
pub static HTTP_ERRORS: AtomicU32 = AtomicU32::new(0);
pub static DHCP_ERRORS: AtomicU32 = AtomicU32::new(0);

/// Crée le contrôleur sur le cœur qui appelle, ce qui y épingle la tâche
/// Wi-Fi et y active ses interruptions, puis lance les tâches réseau.
pub fn start(spawner: Spawner, wifi: WIFI<'static>) {
    let ap = Config::AccessPoint(
        AccessPointConfig::default()
            .with_ssid(SSID.try_into().unwrap())
            .with_authentication(AuthenticationMethodConfig::Wpa2Personal(
                PASSWORD.try_into().unwrap(),
            ))
            .with_max_connections(4),
    );
    let controller = match WifiController::new(
        wifi,
        ControllerConfig::default().with_initial_config(ap),
    ) {
        Ok(controller) => controller,
        Err(_) => {
            AP_STATE.store(AP_FAILED, Relaxed);
            return;
        }
    };
    AP_STATE.store(AP_STARTED, Relaxed);

    let config = embassy_net::Config::ipv4_static(StaticConfigV4 {
        address: Ipv4Cidr::new(ADDRESS, 24),
        gateway: Some(ADDRESS),
        dns_servers: Default::default(),
    });
    let rng = Rng::new();
    let seed = (rng.random() as u64) << 32 | rng.random() as u64;
    static RESOURCES: StaticCell<StackResources<SOCKETS>> = StaticCell::new();
    let (stack, runner) = embassy_net::new(
        Interface::access_point(),
        config,
        RESOURCES.init(StackResources::new()),
        seed,
    );

    spawner.spawn(net_task(runner).unwrap());
    spawner.spawn(ap_events(controller).unwrap());
    spawner.spawn(dhcp(stack).unwrap());
    for _ in 0..HTTP_SOCKETS {
        spawner.spawn(http(stack).unwrap());
    }
}

#[embassy_executor::task]
async fn net_task(mut runner: Runner<'static, Interface>) {
    runner.run().await
}

/// Compte les téléphones associés. Garde aussi le contrôleur en vie : le
/// lâcher arrêterait le Wi-Fi.
///
/// L'abonnement dure autant que la tâche. La méthode d'attente du
/// contrôleur en crée un à chaque appel, et perd les événements reçus entre
/// deux appels. Les stations sont suivies par AID, dans un masque :
/// une réassociation sans déconnexion ne compte pas deux fois.
#[embassy_executor::task]
async fn ap_events(controller: WifiController<'static>) {
    let mut events = controller.subscribe().unwrap();
    let mut aids: u64 = 0;
    loop {
        match events.next_event().await {
            MessageResult::Message(EventInfo::AccessPointStationConnected { aid, .. }) => {
                aids |= 1u64 << (aid % 64);
            }
            MessageResult::Message(EventInfo::AccessPointStationDisconnected { aid, .. }) => {
                aids &= !(1u64 << (aid % 64));
            }
            MessageResult::Message(_) => continue,
            MessageResult::Lagged(n) => {
                STATION_EVENTS_LOST.fetch_add(n as u32, Relaxed);
                continue;
            }
        }
        STATIONS.store(aids.count_ones(), Relaxed);
    }
}

/// Serveur DHCP d'edge-dhcp, comme l'exemple : le téléphone reçoit une
/// adresse en 192.168.2.0/24 et le point d'accès comme passerelle.
#[embassy_executor::task]
async fn dhcp(stack: Stack<'static>) {
    use edge_dhcp::io::{self, DEFAULT_SERVER_PORT};
    use edge_dhcp::server::{Server, ServerOptions};
    use edge_nal::UdpBind;
    use edge_nal_embassy::{Udp, UdpBuffers};

    let mut buf = [0u8; 1500];
    let mut gateways = [Ipv4Addr::UNSPECIFIED];
    let buffers = UdpBuffers::<1, 1024, 1024, 4>::new();
    let udp = Udp::new(stack, &buffers);
    let mut socket = udp
        .bind(SocketAddr::V4(SocketAddrV4::new(
            Ipv4Addr::UNSPECIFIED,
            DEFAULT_SERVER_PORT,
        )))
        .await
        .unwrap();
    let mut server = Server::<_, 8>::new_with_et(ADDRESS);
    loop {
        if io::server::run(
            &mut server,
            &ServerOptions::new(ADDRESS, Some(&mut gateways)),
            &mut socket,
            &mut buf,
        )
        .await
        .is_err()
        {
            DHCP_ERRORS.fetch_add(1, Relaxed);
        }
        Timer::after(Duration::from_millis(500)).await;
    }
}

/// Une connexion HTTP à la fois par tâche, en keep-alive : la page appelle
/// `/stats` à 5 Hz sur la même connexion.
#[embassy_executor::task(pool_size = HTTP_SOCKETS)]
async fn http(stack: Stack<'static>) {
    let mut rx = [0u8; 1024];
    let mut tx = [0u8; 2048];
    loop {
        let mut socket = TcpSocket::new(stack, &mut rx, &mut tx);
        socket.set_timeout(Some(Duration::from_secs(10)));
        if socket.accept(HTTP_PORT).await.is_err() {
            HTTP_ERRORS.fetch_add(1, Relaxed);
            Timer::after(Duration::from_millis(100)).await;
            continue;
        }
        if serve(&mut socket).await.is_err() {
            HTTP_ERRORS.fetch_add(1, Relaxed);
        }
        socket.close();
        let _ = with_timeout(Duration::from_secs(1), socket.flush()).await;
        socket.abort();
        let _ = with_timeout(Duration::from_secs(1), socket.flush()).await;
    }
}

/// Erreur de connexion : la tâche ferme et revient à l'écoute.
struct Broken;

/// Sert les requêtes d'une connexion jusqu'à sa fermeture par le client.
async fn serve(socket: &mut TcpSocket<'_>) -> Result<(), Broken> {
    let mut req = [0u8; 1024];
    let mut len = 0;
    loop {
        // Les requêtes GET n'ont pas de corps : l'en-tête finit la requête.
        let end = loop {
            if let Some(i) = req[..len].windows(4).position(|w| w == b"\r\n\r\n") {
                break i + 4;
            }
            if len == req.len() {
                return Err(Broken);
            }
            match socket.read(&mut req[len..]).await {
                Ok(0) if len == 0 => return Ok(()),
                Ok(0) | Err(_) => return Err(Broken),
                Ok(n) => len += n,
            }
        };
        let head = &req[..end];
        let close = contains_ignore_case(head, b"connection: close");
        respond(socket, path(head)).await?;
        HTTP_REQUESTS.fetch_add(1, Relaxed);
        if close {
            return Ok(());
        }
        req.copy_within(end..len, 0);
        len -= end;
    }
}

fn path(head: &[u8]) -> &[u8] {
    let mut words = head.split(|&b| b == b' ');
    match (words.next(), words.next()) {
        (Some(b"GET"), Some(path)) => path,
        _ => b"",
    }
}

fn contains_ignore_case(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|w| w.eq_ignore_ascii_case(needle))
}

async fn respond(socket: &mut TcpSocket<'_>, path: &[u8]) -> Result<(), Broken> {
    let mut stats = heapless::String::<768>::new();
    let (status, kind, body): (&str, &str, &[u8]) = match path {
        b"/" => ("200 OK", "text/html; charset=utf-8", PAGE.as_bytes()),
        b"/stats" => {
            crate::stats_json(&mut stats).map_err(|_| Broken)?;
            ("200 OK", "application/json", stats.as_bytes())
        }
        _ => ("404 Not Found", "text/plain", b"introuvable"),
    };
    let mut head = heapless::String::<160>::new();
    core::fmt::Write::write_fmt(
        &mut head,
        format_args!(
            "HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\n\
             Cache-Control: no-store\r\n\r\n",
            body.len()
        ),
    )
    .map_err(|_| Broken)?;
    socket.write_all(head.as_bytes()).await.map_err(|_| Broken)?;
    socket.write_all(body).await.map_err(|_| Broken)?;
    socket.flush().await.map_err(|_| Broken)
}

/// Page de charge : appelle `/stats` à 5 Hz, ou sans pause, avec un délai de
/// 1 s par requête. Affiche la dernière réponse, les requêtes, erreurs et
/// délais dépassés, la latence moyenne et maximale, et ce qui a changé côté
/// affichage depuis l'ouverture de la page.
const PAGE: &str = r#"<!doctype html>
<html lang="fr"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>Essai 05</title>
<style>
body{font:15px -apple-system,sans-serif;margin:12px}
table{border-collapse:collapse}td{padding:2px 10px 2px 0}
td:nth-child(2){text-align:right;font-variant-numeric:tabular-nums}
button{font-size:16px;padding:8px 14px;margin:8px 8px 8px 0}
pre{font-size:12px;white-space:pre-wrap;word-break:break-all}
.bad{color:#c00;font-weight:bold}
</style></head><body>
<h3>Essai 05 &mdash; Wi-Fi</h3>
<button id="mode">Sans pause</button><button id="reset">Remise &agrave; z&eacute;ro</button>
<table>
<tr><td>Mode</td><td id="m">5 Hz</td></tr>
<tr><td>Dur&eacute;e</td><td id="d"></td></tr>
<tr><td>Requ&ecirc;tes</td><td id="n"></td></tr>
<tr><td>Requ&ecirc;tes/s</td><td id="r"></td></tr>
<tr><td>Erreurs</td><td id="e"></td></tr>
<tr><td>D&eacute;lais d&eacute;pass&eacute;s (1 s)</td><td id="o"></td></tr>
<tr><td>Latence moy.</td><td id="a"></td></tr>
<tr><td>Latence max.</td><td id="x"></td></tr>
<tr><td>Retards de recopie (page)</td><td id="rl"></td></tr>
<tr><td>Recalages (page)</td><td id="rc"></td></tr>
</table>
<pre id="j"></pre>
<script>
let fast=false,s;
const $=i=>document.getElementById(i);
function reset(){s={t0:performance.now(),n:0,e:0,o:0,sum:0,max:0,first:null};}
reset();
$('mode').onclick=()=>{fast=!fast;$('mode').textContent=fast?'5 Hz':'Sans pause';$('m').textContent=fast?'sans pause':'5 Hz';reset();};
$('reset').onclick=reset;
function show(j){
 const sec=(performance.now()-s.t0)/1000;
 $('d').textContent=sec.toFixed(0)+' s';
 $('n').textContent=s.n;
 $('r').textContent=(s.n/sec).toFixed(1);
 $('e').textContent=s.e;$('e').className=s.e?'bad':'';
 $('o').textContent=s.o;$('o').className=s.o?'bad':'';
 $('a').textContent=s.n?(s.sum/s.n).toFixed(0)+' ms':'';
 $('x').textContent=s.max.toFixed(0)+' ms';
 if(j){
  if(!s.first)s.first=j;
  const rl=j.retards-s.first.retards,rc=j.recalages-s.first.recalages;
  $('rl').textContent=rl;$('rl').className=rl?'bad':'';
  $('rc').textContent=rc;$('rc').className=rc?'bad':'';
  $('j').textContent=JSON.stringify(j,null,1);
 }
}
async function tick(){
 const t=performance.now(),c=new AbortController(),k=setTimeout(()=>c.abort(),1000);
 let j=null;
 try{
  const r=await fetch('/stats',{cache:'no-store',signal:c.signal});
  j=await r.json();
  const dt=performance.now()-t;
  s.n++;s.sum+=dt;s.max=Math.max(s.max,dt);
 }catch(x){if(x.name==='AbortError')s.o++;else s.e++;}
 clearTimeout(k);
 show(j);
 const wait=fast?0:Math.max(0,200-(performance.now()-t));
 setTimeout(tick,wait);
}
tick();
</script></body></html>
"#;
