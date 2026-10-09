//! Station Wi-Fi sur le réseau de la maison, DHCP par embassy-net. Rien ici
//! ne journalise le SSID, le mot de passe ni le BSSID : seuls la raison d'un
//! échec, le RSSI et l'adresse obtenue le sont.

#![allow(
    clippy::large_stack_frames,
    reason = "futures d'embassy : clippy compte l'état de la future, qui vit dans l'arène de l'exécuteur"
)]

use core::sync::atomic::Ordering::Relaxed;

use embassy_executor::Spawner;
use embassy_net::{Runner, Stack, StackResources};
use embassy_time::{Duration, Timer};
use esp_hal::peripherals::WIFI;
use esp_hal::rng::Rng;
use esp_radio::wifi::sta::StationConfig;
use esp_radio::wifi::{
    AuthenticationMethodConfig, Config, ConnectionError, ControllerConfig, Interface,
    WifiController,
};
use log::info;
use static_cell::StaticCell;

use crate::crumb;
use crate::stats::{ADDRESS, ASSOCIATED, DISCONNECTED, FAILED};

/// Événement Wi-Fi vers le cœur 0 (réveil inter-cœurs de `core0-load`).
fn wake_core0() {
    #[cfg(feature = "core0-load")]
    crate::load::notify(crate::load::Event::Wifi);
}

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
    crumb!();
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
    crumb!();
    // Comme l'écran : avec le BLE, la coexistence a besoin du sommeil du modem.
    #[cfg(feature = "ps-min")]
    let controller = {
        let mut controller = controller;
        let result = controller.set_power_saving(esp_radio::wifi::PowerSaveMode::Minimum);
        info!("wifi: économie d'énergie minimale, {result:?}");
        controller
    };
    let rng = Rng::new();
    let seed = u64::from(rng.random()) << 32 | u64::from(rng.random());
    static RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();
    let (stack, runner) = embassy_net::new(
        Interface::station(),
        embassy_net::Config::dhcpv4(Default::default()),
        RESOURCES.init(StackResources::new()),
        seed,
    );
    spawner.spawn(connection(controller).unwrap());
    spawner.spawn(net_task(runner).unwrap());
    spawner.spawn(address(stack).unwrap());
}

#[embassy_executor::task]
async fn connection(mut controller: WifiController<'static>) {
    loop {
        crumb!();
        match controller.connect_async().await {
            Ok(_) => {
                crumb!();
                ASSOCIATED.fetch_add(1, Relaxed);
                wake_core0();
                info!("[wifi] associé");
                match controller.wait_for_disconnect_async().await {
                    Ok(info) => {
                        info!("[wifi] déconnecté raison={:?} rssi={}", info.reason, info.rssi)
                    }
                    Err(err) => info!("[wifi] déconnecté erreur={err:?}"),
                }
                DISCONNECTED.fetch_add(1, Relaxed);
                wake_core0();
                ADDRESS.store(0, Relaxed);
            }
            Err(ConnectionError::Failed(info)) => {
                FAILED.fetch_add(1, Relaxed);
                wake_core0();
                info!("[wifi] échec raison={:?} rssi={}", info.reason, info.rssi)
            }
            Err(err) => {
                FAILED.fetch_add(1, Relaxed);
                wake_core0();
                info!("[wifi] échec erreur={err:?}")
            }
        }
        Timer::after(Duration::from_secs(5)).await;
    }
}

#[embassy_executor::task]
async fn net_task(mut runner: Runner<'static, Interface>) {
    runner.run().await
}

/// Affiche l'adresse à chaque obtention, y compris après une reconnexion.
#[embassy_executor::task]
async fn address(stack: Stack<'static>) {
    loop {
        crumb!();
        stack.wait_config_up().await;
        if let Some(config) = stack.config_v4() {
            ADDRESS.store(u32::from(config.address.address()), Relaxed);
            wake_core0();
            info!("[wifi] ip={}", config.address.address());
        }
        stack.wait_config_down().await;
        ADDRESS.store(0, Relaxed);
        info!("[wifi] ip perdue");
    }
}
