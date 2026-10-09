//! Passerelle de requêtes : le réseau ne prend jamais un verrou sur le cœur.
//! Un propriétaire sur le cœur 0 appelle `process` à chaque tour de rendu.
use crate::nvs;
use alloc::{string::String, vec::Vec};
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel, signal::Signal,
};
use screen_core::{
    config::{ConfigStore, Slot},
    core::{Clock, Core, FlashStart},
    http::{self, HttpResponse, PlatformInfo},
};

pub struct CoreStore;
impl CoreStore {
    pub const fn new() -> Self {
        Self
    }
}
impl Default for CoreStore {
    fn default() -> Self {
        Self::new()
    }
}
impl CoreStore {
    fn exchange(operation: nvs::Operation) -> nvs::Response {
        let mut request = nvs::Request {
            operation,
            synchronous: true,
        };
        loop {
            crate::crumb!();
            match nvs::REQUESTS.try_send(request) {
                Ok(()) => break,
                Err(embassy_sync::channel::TrySendError::Full(back)) => request = back,
            }
            core::hint::spin_loop();
        }
        // Ne pas rendre un faux échec avec une écriture durable encore en vol :
        // la RAM doit rester cohérente avec NVS, même si le flash est lent.
        loop {
            crate::crumb!();
            if let Ok(response) = nvs::SYNC_REPLY.try_receive() {
                return response;
            }
            core::hint::spin_loop();
        }
    }
}
impl ConfigStore for CoreStore {
    fn read_slot(&mut self, slot: Slot) -> Option<Vec<u8>> {
        Self::exchange(nvs::Operation::Read {
            namespace: "ui",
            key: slot.key(),
        })
        .value
    }
    fn write_slot(&mut self, slot: Slot, blob: &[u8]) -> bool {
        Self::exchange(nvs::Operation::Write {
            namespace: "ui",
            key: slot.key(),
            value: blob.to_vec(),
        })
        .ok
    }
}

pub enum Operation {
    Telemetry,
    Config,
    PostConfig(Vec<u8>),
    Action(Vec<u8>),
    Hf(Option<String>),
    BeginUpload {
        query: Option<String>,
        size: i64,
    },
    PollUpload,
    SensorsReplaySafe,
    FinishUpload,
    UploadProgress(u32),
    Confirm,
    Unauthorized,
    Network {
        state: screen_core::snapshot::NetworkState,
        ipv4: u32,
    },
    Credentials(bool),
    BootTimeSyncing(bool),
}
struct Request {
    worker: usize,
    operation: Operation,
}
pub enum Reply {
    Http(HttpResponse),
    Upload(Result<(http::FirmwareTarget, FlashStart), HttpResponse>),
    Ready(bool),
}
static REQUESTS: Channel<CriticalSectionRawMutex, Request, 16> = Channel::new();
static REPLIES: [Signal<CriticalSectionRawMutex, Reply>; 5] = [const { Signal::new() }; 5];
pub async fn call(worker: usize, operation: Operation) -> Reply {
    REQUESTS.send(Request { worker, operation }).await;
    REPLIES[worker].wait().await
}
pub async fn notify(operation: Operation) {
    REQUESTS
        .send(Request {
            worker: usize::MAX,
            operation,
        })
        .await;
}

/// Vide les requêtes en attente ; le réseau reçoit des documents produits
/// depuis l'état courant, pas un cache qui pourrait autoriser une infusion.
pub fn process(core: &mut Core<CoreStore>, clock: Clock) {
    while let Ok(request) = REQUESTS.try_receive() {
        let reply = match request.operation {
            Operation::Telemetry => Reply::Http(http::handle_get_telemetry(
                core,
                clock,
                &crate::ota_local::platform(),
            )),
            Operation::Config => Reply::Http(http::handle_get_config(core)),
            Operation::PostConfig(body) => {
                Reply::Http(http::handle_post_config(core, clock, &body))
            }
            Operation::Action(body) => Reply::Http(http::handle_post_action(core, clock, &body)),
            Operation::Hf(query) => Reply::Http(http::hf_capture_document(core, query.as_deref())),
            Operation::BeginUpload { query, size } => Reply::Upload(http::begin_firmware_upload(
                core,
                clock,
                query.as_deref(),
                size,
                crate::ota_local::pending(),
            )),
            Operation::SensorsReplaySafe => {
                let s = core.snapshot(clock);
                Reply::Ready(protocol::heating_protocol_supported(
                    s.sensors_version_major,
                    s.sensors_version_minor,
                    s.sensors_version_patch,
                ))
            }
            Operation::PollUpload => Reply::Ready(core.poll_flash_stop(clock)),
            Operation::UploadProgress(done) => {
                core.update_flash_progress(done);
                continue;
            }
            Operation::FinishUpload => {
                core.cancel_pending_flash();
                core.finish_flash();
                Reply::Ready(true)
            }
            Operation::Confirm => {
                let s = core.snapshot(clock);
                Reply::Ready(
                    crate::ota_local::valid()
                        || (crate::ota_local::pending()
                            && s.sensors_alive
                            && core.peer_roundtrip_confirmed()
                            && s.actuators_freshness == screen_core::snapshot::Freshness::Fresh
                            && s.network_state
                                == screen_core::snapshot::NetworkState::StaConnected),
                )
            }
            Operation::Unauthorized => {
                core.note_http_auth_refused();
                Reply::Ready(false)
            }
            Operation::Network { state, ipv4 } => {
                let previous = core.snapshot(clock).network_state;
                if previous != state {
                    let event = match state {
                        screen_core::snapshot::NetworkState::StaConnected => {
                            Some(screen_core::events::EventKind::WifiConnected)
                        }
                        screen_core::snapshot::NetworkState::ApProvisioning => {
                            Some(screen_core::events::EventKind::WifiApStarted)
                        }
                        _ if previous == screen_core::snapshot::NetworkState::StaConnected => {
                            Some(screen_core::events::EventKind::WifiDisconnected)
                        }
                        _ => None,
                    };
                    if let Some(event) = event {
                        core.note_event(clock, event);
                    }
                }
                core.update_network_status(state, ipv4);
                continue;
            }
            Operation::BootTimeSyncing(syncing) => {
                core.set_boot_time_syncing(syncing);
                continue;
            }
            Operation::Credentials(present) => {
                core.set_credentials_present(present);
                continue;
            }
        };
        if request.worker < REPLIES.len() {
            REPLIES[request.worker].signal(reply);
        }
    }
}
pub fn platform() -> PlatformInfo {
    crate::ota_local::platform()
}
