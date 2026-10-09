//! BLE de l'essai, par étages cumulatifs : le connecteur (contrôleur), puis
//! l'hôte trouble-host et son runner, puis le scan passif en boucle comme
//! l'écran. Tout tourne sur le cœur 1, avant le Wi-Fi.

#![allow(
    clippy::large_stack_frames,
    reason = "futures d'embassy et de trouble-host : l'état vit dans l'arène de l'exécuteur"
)]

use esp_hal::{peripherals::BT, system::Cpu};
use esp_radio::ble::controller::BleConnector;

/// Initialise le contrôleur sur le cœur qui appelle (le cœur 1), avant le
/// Wi-Fi, dans l'ordre de l'exemple de coexistence d'esp-radio.
pub fn connector(bt: BT<'static>) -> BleConnector<'static> {
    BleConnector::new(
        bt,
        esp_radio::ble::Config::default().with_task_cpu(Cpu::AppCpu),
    )
    .unwrap()
}

/// Étage `ble-controller` : le connecteur reste vivant, rien ne le lit.
#[cfg(not(feature = "ble-host"))]
pub async fn hold(connector: BleConnector<'static>) {
    let _connector = connector;
    core::future::pending::<()>().await
}

#[cfg(feature = "ble-host")]
pub use host::run;

#[cfg(feature = "ble-host")]
mod host {
    use super::*;
    #[cfg(feature = "ble-scan")]
    use crate::stats::{ADV_REPORTS, SCAN_CYCLES};
    #[cfg(feature = "ble-scan")]
    use core::sync::atomic::Ordering::Relaxed;
    use crate::crumb;
    use bt_hci::controller::ExternalController;
    use embassy_futures::select::select;
    #[cfg(feature = "ble-scan")]
    use embassy_time::{Duration, Timer};
    use trouble_host::prelude::*;

    type Ctrl = ExternalController<BleConnector<'static>, 4>;

    #[cfg(feature = "ble-scan")]
    struct ScanHandler;

    #[cfg(feature = "ble-scan")]
    impl EventHandler for ScanHandler {
        fn on_adv_reports(&self, mut reports: LeAdvReportsIter<'_>) {
            crumb!();
            while reports.next().is_some() {
                ADV_REPORTS.fetch_add(1, Relaxed);
                // Réveille le cœur 0 depuis le cœur 1, comme `screen`.
                #[cfg(feature = "core0-load")]
                crate::load::notify(crate::load::Event::Adv);
            }
        }
    }

    /// L'hôte et son runner ; avec `ble-scan`, le scan tourne en plus.
    pub async fn run(connector: BleConnector<'static>) {
        let controller: Ctrl = ExternalController::new(connector);
        let mut resources: HostResources<_, DefaultPacketPool, 2, 1> = HostResources::new();
        let stack = trouble_host::new(controller, &mut resources)
            .set_random_address(Address::random([0xff, 0x8f, 0x1b, 0x05, 0xe4, 0xff]))
            .build();
        let mut runner = stack.runner();
        crumb!();
        #[cfg(feature = "ble-scan")]
        let _ = select(runner.run_with_handler(&ScanHandler), scan(&stack)).await;
        #[cfg(not(feature = "ble-scan"))]
        let _ = select(runner.run(), core::future::pending::<()>()).await;
        log::info!("[ble] hôte arrêté");
    }

    /// Scan passif 60 ms / fenêtre 30 ms comme l'écran : une session de 5 s,
    /// puis arrêt confirmé par `LeSetScanEnable(false)`, puis la suivante.
    #[cfg(feature = "ble-scan")]
    async fn scan(stack: &trouble_host::Stack<'_, Ctrl, DefaultPacketPool>) {
        let mut scanner = Scanner::new(stack.central());
        loop {
            crumb!();
            let config = ScanConfig {
                active: false,
                phys: PhySet::M1,
                interval: Duration::from_millis(60),
                window: Duration::from_millis(30),
                ..Default::default()
            };
            let started = scanner.scan(&config).await;
            crumb!();
            match started {
                Ok(session) => {
                    for _ in 0..250 {
                        Timer::after(Duration::from_millis(20)).await;
                        crumb!();
                    }
                    drop(session);
                }
                Err(err) => log::info!("[ble] scan impossible {err:?}"),
            }
            crumb!();
            // Attendre l'acquittement HCI plutôt qu'un délai arbitraire : la
            // coexistence Wi-Fi peut retarder les commandes du contrôleur.
            while stack
                .command(bt_hci::cmd::le::LeSetScanEnable::new(false, false))
                .await
                .is_err()
            {
                crumb!();
                Timer::after(Duration::from_millis(20)).await;
            }
            crumb!();
            SCAN_CYCLES.fetch_add(1, Relaxed);
            Timer::after(Duration::from_millis(20)).await;
        }
    }
}
