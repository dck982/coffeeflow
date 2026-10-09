//! Radio et flash possédées par le cœur 1. Le démarrage est acquitté avant
//! de lancer le LCD : les blobs radio ne doivent pas bloquer sa recopie.
// La macro Embassy génère aussi un constructeur de future hors des attributs
// de la fonction. Sa future GATT est estimée à 9 Ko ; la pile interne réserve
// 16 Ko, puis l'état vit dans l'arène statique de l'exécuteur.
#![allow(
    clippy::large_stack_frames,
    reason = "constructeurs Embassy, future GATT de 9 Ko sur une pile interne de 16 Ko"
)]
use embassy_executor::Spawner;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use esp_hal::{
    peripherals::{BT, CPU_CTRL, FROM_CPU_INTR1, WIFI},
    system::Stack,
};
use esp_radio::ble::controller::BleConnector;
use screen::{
    ble,
    flash::{Flash, Layout},
    network, nvs,
};
use static_cell::StaticCell;

static READY: Signal<CriticalSectionRawMutex, ()> = Signal::new();
// Mesuré sur carte : 8,7 Ko au plus haut, NVS corrigé et radio active.
const STACK_BYTES: usize = 16384;

pub fn start(
    cpu_ctrl: CPU_CTRL<'static>,
    intr1: FROM_CPU_INTR1<'static>,
    wifi: WIFI<'static>,
    bt: BT<'static>,
    flash: Flash,
    layout: Layout,
) {
    static STACK: StaticCell<Stack<STACK_BYTES>> = StaticCell::new();
    static EXECUTOR: StaticCell<esp_rtos::embassy::Executor> = StaticCell::new();
    let stack = STACK.init_with(Stack::new);
    crate::stack_watermark::paint_core1(stack.mem.as_mut_ptr().cast(), STACK_BYTES);
    esp_rtos::start_second_core(cpu_ctrl, intr1, stack, move || {
        let executor = EXECUTOR.init(esp_rtos::embassy::Executor::new());
        executor.run(|spawner| {
            spawner.spawn(boot(spawner, wifi, bt, flash, layout).unwrap());
        });
    });
}

pub async fn wait_ready() {
    READY.wait().await
}

#[embassy_executor::task]
async fn boot(
    spawner: Spawner,
    wifi: WIFI<'static>,
    bt: BT<'static>,
    flash: Flash,
    layout: Layout,
) {
    // Diagnostic : `DIAG_NO_BLE=1` au build laisse le BLE éteint.
    let connector = option_env!("DIAG_NO_BLE").is_none().then(|| ble::connector(bt));
    nvs::start(spawner, flash, layout).await;
    network::start(spawner, wifi).await;
    if let Some(connector) = connector {
        spawner.spawn(ble_task(connector).unwrap());
    }
    READY.signal(());
}

#[embassy_executor::task]
async fn ble_task(connector: BleConnector<'static>) {
    // Diagnostic : `DIAG_BLE_AFTER_STA=1` attend l'association Wi-Fi (60 s au
    // plus) avant de lancer l'hôte BLE et son scan.
    if option_env!("DIAG_BLE_AFTER_STA").is_some() {
        let start = embassy_time::Instant::now();
        while !network::STA_UP.load(core::sync::atomic::Ordering::Acquire)
            && start.elapsed() < embassy_time::Duration::from_secs(60)
        {
            embassy_time::Timer::after_millis(200).await;
        }
        esp_println::println!(
            "[diag] BLE lancé à {} ms",
            embassy_time::Instant::now().as_millis()
        );
    }
    ble::run(connector).await;
    // Une erreur fatale de l'hôte ne laisse pas une interface connectée
    // fictive ; le bootloader conserve sa politique de rollback.
    embassy_time::Timer::after_millis(50).await;
    esp_hal::system::software_reset();
}
