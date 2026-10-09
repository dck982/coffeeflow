//! OTA locale en flux : écrit le slot inactif, vérifie son SHA et sa somme
//! de contrôle relus, puis change seulement la copie otadata non active.
use crate::{
    flash::{Flash, Layout, Region},
    otadata::{Otadata, State},
};
use alloc::string::String;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use embedded_storage::nor_flash::NorFlash;

static PENDING: AtomicBool = AtomicBool::new(false);
static BOOTED: AtomicU32 = AtomicU32::new(u32::MAX);
static STATE: AtomicU32 = AtomicU32::new(5);
pub fn valid() -> bool {
    STATE.load(Ordering::Acquire) == 2
}
pub fn pending() -> bool {
    PENDING.load(Ordering::Acquire)
}
pub fn initialize(flash: &mut Flash, layout: Layout) -> Result<(), &'static str> {
    let data = Otadata::read(flash.storage_mut(), layout.otadata.offset).map_err(|_| "otadata")?;
    let active = data.active().ok_or("otadata")?;
    let state = data.entries[active].state();
    let slot = layout.booted.ok_or("booted")?;
    if data.entries[active].slot() != slot {
        return Err("booted");
    }
    BOOTED.store(slot, Ordering::Release);
    STATE.store(
        match state {
            State::New => 0,
            State::PendingVerify => 1,
            State::Valid => 2,
            State::Invalid => 3,
            State::Aborted => 4,
            _ => 5,
        },
        Ordering::Release,
    );
    PENDING.store(
        matches!(state, State::New | State::PendingVerify),
        Ordering::Release,
    );
    Ok(())
}
pub fn platform() -> screen_core::http::PlatformInfo {
    let layout = crate::nvs::layout();
    let booted = BOOTED.load(Ordering::Acquire);
    let state = match STATE.load(Ordering::Acquire) {
        0 => "new",
        1 => "pending_verify",
        2 => "valid",
        3 => "invalid",
        4 => "aborted",
        _ => "unavailable",
    };
    screen_core::http::PlatformInfo {
        ota_pending_verify: pending(),
        ota_staging_size: Some(layout.staging.size),
        running_partition: (booted < 2).then(|| screen_core::http::RunningPartition {
            label: String::from(if booted == 0 { "ota_0" } else { "ota_1" }),
            state,
        }),
    }
}
pub struct Upload {
    region: Region,
    expected: u32,
    written: u32,
    erased: u32,
}
impl Upload {
    pub fn begin(flash: &mut Flash, layout: Layout, expected: u32) -> Result<Self, &'static str> {
        if pending() || layout.booted.is_none() || BOOTED.load(Ordering::Acquire) > 1 {
            return Err("pending_verification");
        }
        let region = layout.target();
        if expected < 24 + 8 + 16 + 32 || expected > region.size {
            return Err("size");
        }
        let erased = 65536u32.min(region.size);
        flash
            .erase(region.offset, region.offset + erased)
            .map_err(|_| "write_failed")?;
        Ok(Self {
            region,
            expected,
            written: 0,
            erased,
        })
    }
    pub fn write(&mut self, flash: &mut Flash, data: &[u8]) -> Result<(), &'static str> {
        if data.len() > (self.expected - self.written) as usize {
            return Err("size");
        }
        // Effacement par blocs au fil du flux, comme l'essai OTA : ne pas
        // immobiliser l'exécuteur pendant l'effacement du slot entier.
        while self.erased < self.written + data.len() as u32 {
            let end = (self.erased + 65536).min(self.region.size);
            flash
                .erase(self.region.offset + self.erased, self.region.offset + end)
                .map_err(|_| "write_failed")?;
            self.erased = end;
        }
        self.region
            .write(flash, self.written, data)
            .map_err(|_| "write_failed")?;
        self.written += data.len() as u32;
        Ok(())
    }
    pub fn finish(self, flash: &mut Flash, layout: Layout) -> Result<(), &'static str> {
        if self.written != self.expected {
            return Err("incomplete_upload");
        }
        verify(flash, self.region, self.expected)?;
        let mut data =
            Otadata::read(flash.storage_mut(), layout.otadata.offset).map_err(|_| "otadata")?;
        data.set_boot(flash.storage_mut(), 1 - layout.booted.ok_or("booted")?)
            .map_err(|_| "otadata")
    }
}

fn verify(flash: &mut Flash, region: Region, size: u32) -> Result<(), &'static str> {
    screen_net::image::verify(size, |at, bytes| {
        region.read(flash, at, bytes).map_err(|_| "write_failed")
    })
}
pub fn confirm(flash: &mut Flash, layout: Layout) -> Result<(), &'static str> {
    if !pending() {
        return if valid() {
            Ok(())
        } else {
            Err("pending_verification")
        };
    }
    let mut data =
        Otadata::read(flash.storage_mut(), layout.otadata.offset).map_err(|_| "otadata")?;
    let active = data.active().ok_or("otadata")?;
    if Some(data.entries[active].slot()) != layout.booted {
        return Err("booted");
    }
    data.mark_valid(flash.storage_mut())
        .map_err(|_| "otadata")?;
    PENDING.store(false, Ordering::Release);
    STATE.store(2, Ordering::Release);
    Ok(())
}

/// Comme le firmware archivé, une image non confirmée dispose de cinq minutes
/// avant invalidation et redémarrage. IDF ne déclenche pas ce délai seul.
#[embassy_executor::task]
pub async fn validation_task() {
    embassy_time::Timer::after_secs(300).await;
    if pending() {
        let mut guard = crate::nvs::FLASH.lock().await;
        let flash = guard.as_mut().unwrap();
        if let Ok(mut data) =
            Otadata::read(flash.storage_mut(), crate::nvs::layout().otadata.offset)
        {
            let _ = data.mark_invalid(flash.storage_mut());
        }
        esp_hal::system::software_reset();
    }
}

async fn serial_ack(payload: protocol::messages::FlashCtrlPayload) {
    // Le client historique inspecte le type ; l'identifiant reste celui du
    // firmware C++ et cette réponse n'est jamais envoyée sur le CAN.
    let frame = protocol::Message::FlashCtrl(payload)
        .encode_routed(protocol::Node::Screen, protocol::Dest::Sensors);
    crate::can::SERIAL.send(frame).await;
}

/// OTA USB : même image et mêmes vérifications que HTTP, blocs de 2048
/// octets acquittés après écriture durable. Les sessions expirent après 5 s.
#[embassy_executor::task]
pub async fn serial_task() {
    use crate::service::{self, Operation, Reply};
    use protocol::{Message, messages::FlashCtrlPayload as Ctrl};
    loop {
        let frame = crate::serial_bridge::LOCAL_OTA.receive().await;
        let Ok(Message::FlashCtrl(Ctrl::Begin { image_size })) = Message::decode(&frame) else {
            continue;
        };
        let begin = service::call(
            4,
            Operation::BeginUpload {
                query: Some(String::from("target=screen")),
                size: image_size as i64,
            },
        )
        .await;
        if !matches!(begin, Reply::Upload(Ok(_))) {
            serial_ack(Ctrl::Abort).await;
            continue;
        }
        let layout = crate::nvs::layout();
        let mut guard = crate::nvs::FLASH.lock().await;
        let flash = guard.as_mut().unwrap();
        let result = receive_serial(flash, layout, image_size).await;
        drop(guard);
        let _ = service::call(4, Operation::FinishUpload).await;
        if result.is_ok() {
            serial_ack(Ctrl::End { image_crc32: 0 }).await;
            embassy_time::Timer::after_millis(50).await;
            esp_hal::system::software_reset();
        } else {
            serial_ack(Ctrl::Abort).await;
            crate::serial_bridge::LOCAL_OTA.clear();
        }
    }
}

async fn receive_serial(
    flash: &mut Flash,
    layout: Layout,
    image_size: u32,
) -> Result<(), &'static str> {
    use crate::service::{self, Operation};
    use protocol::{Message, messages::FlashCtrlPayload as Ctrl};
    let mut upload = Upload::begin(flash, layout, image_size)?;
    serial_ack(Ctrl::BlockAck {
        block_number: 0,
        block_crc16: 0,
    })
    .await;
    let mut bytes = [0; 2048];
    let mut count = 0;
    let mut written = 0;
    let mut duplicate = false;
    let mut last_size = 0;
    let mut last_crc = 0;
    let mut expected_number = 1;
    let mut expected_crc = 0;
    let mut collecting = false;
    let mut crc = protocol::Crc32Incremental::new();
    loop {
        let frame = embassy_time::with_timeout(
            embassy_time::Duration::from_secs(5),
            crate::serial_bridge::LOCAL_OTA.receive(),
        )
        .await
        .map_err(|_| "incomplete_upload")?;
        match Message::decode(&frame).map_err(|_| "invalid_firmware")? {
            Message::FlashCtrl(Ctrl::BlockStart {
                block_number,
                block_crc16,
            }) if !collecting
                && ((block_number == expected_number && written < image_size)
                    || (expected_number > 1
                        && block_number == expected_number - 1
                        && block_crc16 == last_crc)) =>
            {
                duplicate = block_number != expected_number;
                expected_crc = block_crc16;
                count = 0;
                collecting = true;
            }
            Message::FlashData(data) if collecting => {
                let length = data.bytes().len();
                let target = if duplicate {
                    last_size
                } else {
                    ((image_size - written) as usize).min(bytes.len())
                };
                if count + length > target {
                    return Err("size");
                }
                bytes[count..count + length].copy_from_slice(data.bytes());
                count += length;
                if count == target {
                    if protocol::crc16_ccitt(&bytes[..count]) != expected_crc {
                        return Err("invalid_firmware");
                    }
                    if !duplicate {
                        upload.write(flash, &bytes[..count])?;
                        crc.update(&bytes[..count]);
                        written += count as u32;
                        last_size = count;
                        last_crc = expected_crc;
                        expected_number += 1;
                    }
                    serial_ack(Ctrl::BlockAck {
                        block_number: expected_number - 1,
                        block_crc16: expected_crc,
                    })
                    .await;
                    collecting = false;
                    count = 0;
                    service::notify(Operation::UploadProgress(written)).await;
                }
            }
            Message::FlashCtrl(Ctrl::End { image_crc32 })
                if !collecting
                    && count == 0
                    && written == image_size
                    && crc.finish() == image_crc32 =>
            {
                return upload.finish(flash, layout);
            }
            _ => return Err("invalid_firmware"),
        }
    }
}
