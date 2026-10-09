//! Station Wi-Fi sur le réseau de la maison, avec les identifiants lus dans
//! le NVS de coffeeflow, sur le modèle de l'exemple `wifi/embassy_dhcp`
//! d'esp-hal. Une page minimale sur le port 80 prouve qu'on répond sur le
//! réseau local.
//!
//! Rien ici ne journalise le SSID, le mot de passe ni le BSSID : ni
//! `ConnectedInfo` ni `DisconnectedInfo`, qui les portent, ne sont affichés.
//! Seuls la raison d'un échec, le RSSI et l'adresse obtenue le sont.

#![allow(
    clippy::large_stack_frames,
    reason = "futures d'embassy : clippy compte l'état de la future, qui vit dans l'arène de l'exécuteur"
)]

use embassy_executor::Spawner;
use embassy_net::tcp::TcpSocket;
use embassy_net::{Runner, Stack, StackResources};
use embassy_time::{Duration, Timer, with_timeout};
use embedded_io_async::Write;
use esp_hal::peripherals::WIFI;
use esp_hal::rng::Rng;
use esp_radio::wifi::sta::StationConfig;
use esp_radio::wifi::{
    AuthenticationMethodConfig, Config, ConnectionError, ControllerConfig, Interface,
    WifiController,
};
use log::info;
use static_cell::StaticCell;

pub fn start(spawner: Spawner, wifi: WIFI<'static>, ssid: &str, password: &str) {
    let (Ok(ssid), Ok(password)) = (ssid.try_into(), password.try_into()) else {
        info!("wifi: identifiants hors des limites d'esp-radio");
        return;
    };
    let station = Config::Station(
        StationConfig::default()
            .with_ssid(ssid)
            .with_authentication(AuthenticationMethodConfig::Wpa2Personal(password)),
    );
    let controller = match WifiController::new(
        wifi,
        ControllerConfig::default().with_initial_config(station),
    ) {
        Ok(controller) => controller,
        Err(err) => {
            info!("wifi: contrôleur {err:?}");
            return;
        }
    };
    let rng = Rng::new();
    let seed = (rng.random() as u64) << 32 | rng.random() as u64;
    static RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();
    let (stack, runner) = embassy_net::new(
        Interface::station(),
        embassy_net::Config::dhcpv4(Default::default()),
        RESOURCES.init(StackResources::new()),
        seed,
    );
    spawner.spawn(connection(controller).unwrap());
    spawner.spawn(net_task(runner).unwrap());
    spawner.spawn(http(stack).unwrap());
}

#[embassy_executor::task]
async fn connection(mut controller: WifiController<'static>) {
    loop {
        match controller.connect_async().await {
            Ok(_) => {
                info!("wifi: associé");
                match controller.wait_for_disconnect_async().await {
                    Ok(info) => info!("wifi: déconnecté, {:?}, rssi {}", info.reason, info.rssi),
                    Err(err) => info!("wifi: déconnecté, {err:?}"),
                }
            }
            Err(ConnectionError::Failed(info)) => {
                info!("wifi: échec, {:?}, rssi {}", info.reason, info.rssi)
            }
            Err(err) => info!("wifi: échec, {err:?}"),
        }
        Timer::after(Duration::from_secs(5)).await;
    }
}

#[embassy_executor::task]
async fn net_task(mut runner: Runner<'static, Interface>) {
    runner.run().await
}

/// Une connexion à la fois : lit l'en-tête, répond, ferme.
#[embassy_executor::task]
async fn http(stack: Stack<'static>) {
    stack.wait_config_up().await;
    if let Some(config) = stack.config_v4() {
        info!("wifi: adresse {}, page sur http://{}/", config.address, config.address.address());
    }
    let mut rx = [0u8; 1024];
    let mut tx = [0u8; 512];
    let mut served = 0u32;
    loop {
        let mut socket = TcpSocket::new(stack, &mut rx, &mut tx);
        socket.set_timeout(Some(Duration::from_secs(10)));
        if socket.accept(80).await.is_err() {
            Timer::after(Duration::from_millis(100)).await;
            continue;
        }
        let mut req = [0u8; 1024];
        let mut len = 0;
        while len < req.len() && !req[..len].windows(4).any(|w| w == b"\r\n\r\n") {
            match socket.read(&mut req[len..]).await {
                Ok(0) | Err(_) => break,
                Ok(n) => len += n,
            }
        }
        const BODY: &str = "essai 08c : Rust sur le Wi-Fi de la maison, identifiants lus dans le NVS d'IDF\n";
        let mut head = heapless::String::<128>::new();
        let _ = core::fmt::write(
            &mut head,
            format_args!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/plain; charset=utf-8\r\n\
                 Content-Length: {}\r\nConnection: close\r\n\r\n",
                BODY.len()
            ),
        );
        let _ = socket.write_all(head.as_bytes()).await;
        let _ = socket.write_all(BODY.as_bytes()).await;
        let _ = socket.flush().await;
        socket.close();
        let _ = with_timeout(Duration::from_secs(1), socket.flush()).await;
        socket.abort();
        served += 1;
        info!("http: {served} requête(s) servie(s)");
    }
}
