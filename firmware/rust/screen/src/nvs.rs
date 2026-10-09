//! Propriétaire flash sur le cœur 1. Les accès du cœur 0 sont acquittés
//! après persistance ; les interruptions restent actives pendant son attente.
use crate::flash::{Flash, Layout};
use alloc::vec::Vec;
use embassy_executor::Spawner;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel, mutex::Mutex};
use embedded_storage::nor_flash::{ErrorType, NorFlash, ReadNorFlash};
use esp_nvs::platform::{Crc, software_crc32};
use esp_nvs::{Get, Key, Nvs};
use static_cell::StaticCell;

pub static FLASH: Mutex<CriticalSectionRawMutex, Option<Flash>> = Mutex::new(None);
static LAYOUT: StaticCell<Layout> = StaticCell::new();
static LAYOUT_READY: core::sync::atomic::AtomicPtr<Layout> =
    core::sync::atomic::AtomicPtr::new(core::ptr::null_mut());
pub fn layout() -> Layout {
    let pointer = LAYOUT_READY.load(core::sync::atomic::Ordering::Acquire);
    assert!(!pointer.is_null(), "stockage non initialisé");
    // Le pointeur vise un StaticCell, publié une seule fois avant les tâches.
    unsafe { *pointer }
}

pub enum Operation {
    Read {
        namespace: &'static str,
        key: &'static str,
    },
    Write {
        namespace: &'static str,
        key: &'static str,
        value: Vec<u8>,
    },
    Forget,
}
pub struct Request {
    pub operation: Operation,
    pub synchronous: bool,
}
pub struct Response {
    pub value: Option<Vec<u8>>,
    pub ok: bool,
}
pub static REQUESTS: Channel<CriticalSectionRawMutex, Request, 8> = Channel::new();
pub static SYNC_REPLY: Channel<CriticalSectionRawMutex, Response, 1> = Channel::new();
static ASYNC_REPLY: Channel<CriticalSectionRawMutex, Response, 1> = Channel::new();
static ASYNC_GATE: Mutex<CriticalSectionRawMutex, ()> = Mutex::new(());

pub async fn start(spawner: Spawner, flash: Flash, layout: Layout) {
    let layout = LAYOUT.init(layout);
    LAYOUT_READY.store(layout, core::sync::atomic::Ordering::Release);
    let mut flash = flash;
    // La métadonnée de l'image démarrée est lue avant que les requêtes puissent
    // sélectionner une autre partition ; un échec interdit l'OTA locale.
    let _ = crate::ota_local::initialize(&mut flash, *layout);
    *FLASH.lock().await = Some(flash);
    spawner.spawn(storage_task().unwrap());
    spawner.spawn(crate::ota_local::validation_task().unwrap());
    spawner.spawn(crate::ota_local::serial_task().unwrap());
}
pub async fn request(operation: Operation) -> Response {
    let _gate = ASYNC_GATE.lock().await;
    REQUESTS
        .send(Request {
            operation,
            synchronous: false,
        })
        .await;
    ASYNC_REPLY.receive().await
}

struct Borrowed<'a>(&'a mut Flash);
impl ErrorType for Borrowed<'_> {
    type Error = crate::flash::Error;
}
impl ReadNorFlash for Borrowed<'_> {
    const READ_SIZE: usize = Flash::READ_SIZE;
    fn read(&mut self, offset: u32, data: &mut [u8]) -> Result<(), Self::Error> {
        self.0.read(offset, data)
    }
    fn capacity(&self) -> usize {
        self.0.capacity()
    }
}
impl NorFlash for Borrowed<'_> {
    const WRITE_SIZE: usize = Flash::WRITE_SIZE;
    const ERASE_SIZE: usize = Flash::ERASE_SIZE;
    fn write(&mut self, offset: u32, data: &[u8]) -> Result<(), Self::Error> {
        self.0.write(offset, data)
    }
    fn erase(&mut self, from: u32, to: u32) -> Result<(), Self::Error> {
        self.0.erase(from, to)
    }
}
impl Crc for Borrowed<'_> {
    fn crc32(init: u32, bytes: &[u8]) -> u32 {
        software_crc32(init, bytes)
    }
}

#[embassy_executor::task]
async fn storage_task() {
    loop {
        let request = REQUESTS.receive().await;
        let response = {
            let mut flash = FLASH.lock().await;
            execute(flash.as_mut().unwrap(), request.operation)
        };
        if request.synchronous {
            SYNC_REPLY.send(response).await;
        } else {
            ASYNC_REPLY.send(response).await;
        }
    }
}
fn execute(flash: &mut Flash, operation: Operation) -> Response {
    let region = layout().nvs;
    let Ok(mut nvs) = Nvs::new(
        region.offset as usize,
        region.size as usize,
        Borrowed(flash),
    ) else {
        return Response {
            value: None,
            ok: false,
        };
    };
    match operation {
        Operation::Read { namespace, key } => {
            let value = nvs.get(&Key::from_str(namespace), &Key::from_str(key)).ok();
            Response { ok: true, value }
        }
        Operation::Write {
            namespace,
            key,
            value,
        } => {
            let ns = Key::from_str(namespace);
            let key = Key::from_str(key);
            let ok = nvs.set(&ns, &key, value.as_slice()).is_ok()
                && <Nvs<_> as Get<Vec<u8>>>::get(&mut nvs, &ns, &key)
                    .is_ok_and(|read| read == value);
            Response { ok, value: None }
        }
        // Effacer uniquement les identifiants préserve les blobs de configuration.
        Operation::Forget => Response {
            ok: nvs
                .delete(&Key::from_str("wifi"), &Key::from_str("credentials"))
                .is_ok(),
            value: None,
        },
    }
}
