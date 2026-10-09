//! Radio et flash possédées par le cœur 1. Le démarrage est acquitté avant
//! de lancer le LCD : les blobs radio ne doivent pas bloquer sa recopie.
use embassy_executor::Spawner;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use esp_hal::{peripherals::{CPU_CTRL, FROM_CPU_INTR1, WIFI, BT}, system::Stack};
use esp_radio::ble::controller::BleConnector;
use screen::{ble, flash::{Flash, Layout}, network, nvs};
use static_cell::StaticCell;

static READY: Signal<CriticalSectionRawMutex, ()> = Signal::new();
const STACK_BYTES: usize = 16384;

pub fn start(cpu_ctrl: CPU_CTRL<'static>, intr1: FROM_CPU_INTR1<'static>,
    wifi: WIFI<'static>, bt: BT<'static>, flash: Flash, layout: Layout) {
    static STACK: StaticCell<Stack<STACK_BYTES>> = StaticCell::new();
    static EXECUTOR: StaticCell<esp_rtos::embassy::Executor> = StaticCell::new();
    let stack = STACK.init_with(Stack::new);
    esp_rtos::start_second_core(cpu_ctrl, intr1, stack, move || {
        let executor = EXECUTOR.init(esp_rtos::embassy::Executor::new());
        executor.run(|spawner| { spawner.spawn(boot(spawner,wifi,bt,flash,layout).unwrap()); });
    });
}

pub async fn wait_ready() { READY.wait().await }

#[allow(clippy::large_stack_frames, reason = "état des futures dans l'arène de l'exécuteur")]
#[embassy_executor::task]
async fn boot(spawner: Spawner, wifi: WIFI<'static>, bt: BT<'static>, flash: Flash, layout: Layout) {
    let connector = ble::connector(bt);
    nvs::start(spawner,flash,layout).await;
    network::start(spawner,wifi).await;
    spawner.spawn(ble_task(connector).unwrap());
    READY.signal(());
}

#[allow(clippy::large_stack_frames, reason = "état de l'hôte GATT dans l'arène de l'exécuteur")]
#[embassy_executor::task]
async fn ble_task(connector: BleConnector<'static>) {
    ble::run(connector).await;
    // Une erreur fatale de l'hôte ne laisse pas une interface connectée
    // fictive ; le bootloader conserve sa politique de rollback.
    embassy_time::Timer::after_millis(50).await;
    esp_hal::system::software_reset();
}
