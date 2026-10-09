//! Transfert depuis staging, avec temporisation de 2 ms par trame CAN.
//! Le réseau prête sa flash pendant l'envoi : NVS et OTA ne peuvent donc
//! modifier simultanément l'image. Aucun octet de l'image n'est gardé en tas.

use crate::{
    flash::{Error, Flash, Region},
    ota_proxy::{self, BLOCK_BYTES, HEADER_BYTES, Image},
};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};
use embassy_time::{Instant, Timer, with_timeout};
use protocol::{Crc32Incremental, Frame, messages::FlashCtrlPayload};

static ACKS: Channel<CriticalSectionRawMutex, FlashCtrlPayload, 8> = Channel::new();

/// Informations pour l'UI ; une mise à jour perdue ne bloque jamais l'OTA.
#[derive(Debug, Clone, Copy)]
pub enum Progress {
    Uploaded(u32),
    Sent(u32),
    Finished(bool),
}
pub static PROGRESS: Channel<CriticalSectionRawMutex, Progress, 16> = Channel::new();

pub fn on_frame(frame: Frame) {
    if let Some(ack) = ota_proxy::acknowledgement(&frame) {
        let _ = ACKS.try_send(ack);
    }
}

/// État d'une réception HTTP ; l'appelant assure l'exclusion avec brew/purge.
pub struct Upload {
    region: Region,
    size: u32,
    written: u32,
    crc: Crc32Incremental,
}
impl Upload {
    pub fn begin(flash: &mut Flash, region: Region, size: u32) -> Result<Self, Error> {
        if size == 0
            || size > ota_proxy::MAX_IMAGE_BYTES
            || region
                .size
                .checked_sub(HEADER_BYTES)
                .is_none_or(|capacity| size > capacity)
        {
            return Err(Error::Bounds);
        }
        region.erase(flash)?;
        Ok(Self {
            region,
            size,
            written: 0,
            crc: Crc32Incremental::new(),
        })
    }
    pub fn write(&mut self, flash: &mut Flash, bytes: &[u8]) -> Result<(), Error> {
        if bytes.len() > (self.size - self.written) as usize {
            return Err(Error::Bounds);
        }
        self.region
            .write(flash, HEADER_BYTES + self.written, bytes)?;
        self.crc.update(bytes);
        self.written += bytes.len() as u32;
        let _ = PROGRESS.try_send(Progress::Uploaded(self.written));
        Ok(())
    }
    /// L'en-tête valide arrive en dernier : une coupure ne publie pas un fragment.
    pub fn commit(self, flash: &mut Flash) -> Result<Image, Error> {
        if self.written != self.size {
            return Err(Error::Bounds);
        }
        let image = Image {
            size: self.size,
            crc32: self.crc.finish(),
        };
        self.region.write(flash, 0, &image.header())?;
        Ok(image)
    }
}

fn clear_acks() {
    while ACKS.try_receive().is_ok() {}
}
async fn control(p: FlashCtrlPayload) {
    crate::can::TX.send(ota_proxy::ctrl(p)).await;
}
async fn wait_ack(expected: ota_proxy::ExpectedAck) -> bool {
    let wait = ota_proxy::AckWait::new(Instant::now().as_micros(), expected);
    let deadline = Instant::from_micros(wait.deadline_us);
    loop {
        if wait.check(Instant::now().as_micros(), None) == ota_proxy::AckResult::Failed {
            return false;
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        match with_timeout(remaining, ACKS.receive()).await {
            Err(_) => return false,
            Ok(p) => match wait.check(Instant::now().as_micros(), Some(p)) {
                ota_proxy::AckResult::Accepted => return true,
                ota_proxy::AckResult::Failed => return false,
                ota_proxy::AckResult::Pending => {}
            },
        }
    }
}

/// Envoie l'image après commit. Trois essais par bloc seulement si le firmware
/// distant sait dédupliquer les blocs (version autorisant le chauffage).
pub async fn send_staged(
    flash: &mut Flash,
    region: Region,
    image: Image,
    replay_safe: bool,
) -> bool {
    let ok = transfer(flash, region, image, replay_safe).await;
    if !ok {
        control(FlashCtrlPayload::Abort).await;
    }
    let _ = PROGRESS.try_send(Progress::Finished(ok));
    ok
}

async fn transfer(flash: &mut Flash, region: Region, image: Image, replay_safe: bool) -> bool {
    clear_acks();
    control(FlashCtrlPayload::Begin {
        image_size: image.size,
    })
    .await;
    if !wait_ack(ota_proxy::ExpectedAck::Block { number: 0, crc: 0 }).await {
        return false;
    }
    let mut block = [0; BLOCK_BYTES];
    let mut sent = 0;
    let mut number = 1;
    while sent < image.size {
        let count = ((image.size - sent) as usize).min(BLOCK_BYTES);
        if region
            .read(flash, HEADER_BYTES + sent, &mut block[..count])
            .is_err()
        {
            return false;
        }
        let crc = protocol::crc16_ccitt(&block[..count]);
        let mut accepted = false;
        for _ in 0..if replay_safe { 3 } else { 1 } {
            clear_acks();
            control(FlashCtrlPayload::BlockStart {
                block_number: number,
                block_crc16: crc,
            })
            .await;
            for bytes in block[..count].chunks(8) {
                let frame = Frame::new(
                    protocol::encode_can_id(protocol::CanId {
                        message_type: protocol::MessageType::FlashData,
                        src: protocol::Node::Screen,
                        dest: protocol::Dest::Sensors,
                    }),
                    bytes,
                )
                .unwrap();
                crate::can::TX.send(frame).await;
                Timer::after_millis(2).await;
            }
            if wait_ack(ota_proxy::ExpectedAck::Block { number, crc }).await {
                accepted = true;
                break;
            }
        }
        if !accepted {
            return false;
        }
        sent += count as u32;
        number += 1;
        let _ = PROGRESS.try_send(Progress::Sent(sent));
    }
    clear_acks();
    control(FlashCtrlPayload::End {
        image_crc32: image.crc32,
    })
    .await;
    wait_ack(ota_proxy::ExpectedAck::End).await
}
