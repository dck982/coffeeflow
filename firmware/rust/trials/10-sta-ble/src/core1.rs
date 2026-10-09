//! Radio sur le cœur 1 : le contrôleur BLE d'abord, puis le Wi-Fi station
//! avec les identifiants relus en NVS, comme dans l'écran.
#![allow(
    clippy::large_stack_frames,
    reason = "constructeurs Embassy, futures de trouble-host sur une pile interne de 16 Ko"
)]

use embassy_executor::Spawner;
use esp_hal::{
    peripherals::{BT, CPU_CTRL, FROM_CPU_INTR1, WIFI},
    system::Stack,
};
use static_cell::StaticCell;

#[cfg(not(feature = "no-wifi"))]
use crate::{nvs, sta};
#[cfg(feature = "ble-controller")]
use crate::ble;
use crate::crumb;

const STACK_BYTES: usize = 16384;

pub fn start(
    cpu_ctrl: CPU_CTRL<'static>,
    intr1: FROM_CPU_INTR1<'static>,
    wifi: WIFI<'static>,
    bt: BT<'static>,
    flash: esp_hal::peripherals::FLASH<'static>,
) {
    static STACK: StaticCell<Stack<STACK_BYTES>> = StaticCell::new();
    static EXECUTOR: StaticCell<esp_rtos::embassy::Executor> = StaticCell::new();
    let stack = STACK.init_with(Stack::new);
    crate::stack_watermark::paint_core1(stack.mem.as_mut_ptr().cast(), STACK_BYTES);
    esp_rtos::start_second_core(cpu_ctrl, intr1, stack, move || {
        let executor = EXECUTOR.init(esp_rtos::embassy::Executor::new());
        executor.run(|spawner| {
            spawner.spawn(boot(spawner, wifi, bt, flash).unwrap());
        });
    });
}

#[embassy_executor::task]
async fn boot(
    spawner: Spawner,
    wifi: WIFI<'static>,
    bt: BT<'static>,
    flash: esp_hal::peripherals::FLASH<'static>,
) {
    crumb!();
    #[cfg(feature = "ble-controller")]
    let connector = ble::connector(bt);
    #[cfg(not(feature = "ble-controller"))]
    let _ = bt;
    crumb!();
    #[cfg(not(feature = "no-wifi"))]
    {
        // Lecture seule, comme `screen` (propriétaire de la flash sur le
        // cœur 1) : l'autre cœur n'est pas arrêté pendant la lecture.
        let storage = unsafe { esp_storage::FlashStorage::new(flash).multicore_ignore() };
        if let Some((ssid, password)) = nvs::read_credentials(storage) {
            sta::start(spawner, wifi, &ssid, &password);
        }
    }
    #[cfg(feature = "no-wifi")]
    let _ = (wifi, flash);
    crumb!();
    #[cfg(feature = "ble-controller")]
    spawner.spawn(ble_task(connector).unwrap());
    let _ = spawner;
}

#[cfg(feature = "ble-controller")]
#[embassy_executor::task]
async fn ble_task(connector: esp_radio::ble::controller::BleConnector<'static>) {
    #[cfg(feature = "ble-host")]
    ble::run(connector).await;
    #[cfg(not(feature = "ble-host"))]
    ble::hold(connector).await;
}
