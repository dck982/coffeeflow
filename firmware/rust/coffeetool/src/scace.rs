//! Lecture de la sonde SCACE en BLE depuis le Mac (`telemetry --record
//! --scace`). L'écran ne la donne pas dans `/telemetry` : la lecture directe
//! par le Mac est la référence que l'enregistrement joint à chaque
//! échantillon. Le Core2 accepte l'écran et le Mac à la fois.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use anyhow::{Result, anyhow};
use btleplug::api::{Central, Manager as _, Peripheral as _, ScanFilter};
use btleplug::platform::{Manager, Peripheral};
use futures::StreamExt;
use protocol::scace::{FRAME_UUID, Frame, NO_TEMPERATURE, SERVICE_UUID, Status};
use uuid::Uuid;

use crate::record::ScaceSource;

/// Une température plus vieille que ceci est donnée comme absente.
const STALE: Duration = Duration::from_millis(300);
const RECONNECT_DELAY: Duration = Duration::from_secs(2);
const SCAN_TIMEOUT: Duration = Duration::from_secs(10);

/// Dernière température de la sonde, écrite par le fil BLE et lue par la
/// boucle d'enregistrement.
#[derive(Default)]
pub struct LatestTemperature {
    inner: Mutex<Option<(Option<f64>, Instant)>>,
}

impl LatestTemperature {
    pub fn on_data_at(&self, data: &[u8], now: Instant) {
        let Some(frame) = Frame::decode(data) else {
            return;
        };
        let celsius = (frame.status == Status::Ok && frame.centi_c != NO_TEMPERATURE)
            .then(|| f64::from(frame.centi_c) / 100.0);
        *self.inner.lock().expect("verrou SCACE") = Some((celsius, now));
    }

    pub fn on_data(&self, data: &[u8]) {
        self.on_data_at(data, Instant::now());
    }

    pub fn read_at(&self, now: Instant) -> (Option<f64>, Option<u64>) {
        let Some((celsius, received)) = *self.inner.lock().expect("verrou SCACE") else {
            return (None, None);
        };
        let age = now.saturating_duration_since(received);
        let age_ms = (age.as_secs_f64() * 1000.0).round() as u64;
        ((age <= STALE).then_some(celsius).flatten(), Some(age_ms))
    }
}

impl ScaceSource for LatestTemperature {
    fn read(&self) -> (Option<f64>, Option<u64>) {
        self.read_at(Instant::now())
    }
}

async fn find_peripheral(manager: &Manager, address: Option<&str>) -> Result<Option<Peripheral>> {
    let adapter = manager
        .adapters()
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("aucun adaptateur Bluetooth"))?;
    let service = Uuid::parse_str(SERVICE_UUID)?;
    let filter = match address {
        Some(_) => ScanFilter::default(),
        None => ScanFilter {
            services: vec![service],
        },
    };
    adapter.start_scan(filter).await?;
    let deadline = Instant::now() + SCAN_TIMEOUT;
    let mut found = None;
    while found.is_none() && Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(300)).await;
        for peripheral in adapter.peripherals().await? {
            let Some(properties) = peripheral.properties().await? else {
                continue;
            };
            let matches = match address {
                Some(wanted) => {
                    properties.address.to_string().eq_ignore_ascii_case(wanted)
                        || peripheral.id().to_string().eq_ignore_ascii_case(wanted)
                }
                None => properties.services.contains(&service),
            };
            if matches {
                found = Some(peripheral);
                break;
            }
        }
    }
    let _ = adapter.stop_scan().await;
    Ok(found)
}

async fn follow_once(latest: &LatestTemperature, address: Option<&str>) -> Result<()> {
    let manager = Manager::new().await?;
    let Some(peripheral) = find_peripheral(&manager, address).await? else {
        return Ok(());
    };
    peripheral.connect().await?;
    peripheral.discover_services().await?;
    let frame_uuid = Uuid::parse_str(FRAME_UUID)?;
    let characteristic = peripheral
        .characteristics()
        .into_iter()
        .find(|c| c.uuid == frame_uuid)
        .ok_or_else(|| anyhow!("caractéristique de trame introuvable"))?;
    peripheral.subscribe(&characteristic).await?;
    let name = peripheral
        .properties()
        .await?
        .and_then(|p| p.local_name)
        .unwrap_or_else(|| peripheral.id().to_string());
    println!("SCACE connectée ({name})");
    let mut notifications = peripheral.notifications().await?;
    while let Some(notification) = notifications.next().await {
        if notification.uuid == frame_uuid {
            latest.on_data(&notification.value);
        }
    }
    let _ = peripheral.disconnect().await;
    Ok(())
}

/// Garde la sonde connectée dans un fil de fond, jusqu'à la fin du
/// programme.
pub fn follow_in_background(latest: std::sync::Arc<LatestTemperature>, address: Option<String>) {
    std::thread::Builder::new()
        .name("scace-ble".into())
        .spawn(move || {
            let runtime = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(runtime) => runtime,
                Err(error) => {
                    eprintln!("SCACE, runtime : {error}");
                    return;
                }
            };
            runtime.block_on(async {
                loop {
                    if let Err(error) = follow_once(&latest, address.as_deref()).await {
                        println!("SCACE, liaison BLE : {error}");
                    }
                    println!("SCACE perdue, reconnexion…");
                    tokio::time::sleep(RECONNECT_DELAY).await;
                }
            });
        })
        .expect("fil BLE");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(centi_c: i16, status: Status) -> [u8; 12] {
        Frame {
            seq: 1,
            ms: 2,
            a0: 3,
            a1: 4,
            centi_c,
            status,
        }
        .encode()
    }

    #[test]
    fn nothing_before_the_first_frame() {
        assert_eq!(LatestTemperature::default().read(), (None, None));
    }

    #[test]
    fn fresh_reading_has_a_temperature_and_an_age() {
        let latest = LatestTemperature::default();
        let t0 = Instant::now();
        latest.on_data_at(&frame(9150, Status::Ok), t0);
        assert_eq!(
            latest.read_at(t0 + Duration::from_millis(40)),
            (Some(91.5), Some(40))
        );
    }

    #[test]
    fn stale_reading_loses_the_temperature_but_keeps_the_age() {
        let latest = LatestTemperature::default();
        let t0 = Instant::now();
        latest.on_data_at(&frame(9150, Status::Ok), t0);
        assert_eq!(
            latest.read_at(t0 + Duration::from_millis(301)),
            (None, Some(301))
        );
        assert_eq!(
            latest.read_at(t0 + Duration::from_millis(300)).0,
            Some(91.5)
        );
    }

    #[test]
    fn frames_without_measurement_give_no_temperature() {
        let latest = LatestTemperature::default();
        let t0 = Instant::now();
        latest.on_data_at(&frame(NO_TEMPERATURE, Status::Open), t0);
        assert_eq!(latest.read_at(t0), (None, Some(0)));
        latest.on_data_at(&frame(2000, Status::NoMeasurement), t0);
        assert_eq!(latest.read_at(t0).0, None);
        // Une trame illisible ne remplace rien.
        latest.on_data_at(&frame(9000, Status::Ok), t0);
        latest.on_data_at(&[1, 2, 3], t0);
        assert_eq!(latest.read_at(t0).0, Some(90.0));
    }
}
