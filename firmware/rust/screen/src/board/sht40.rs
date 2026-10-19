//! SHT40 du boîtier de l'écran, à 0x44 sur le bus partagé avec l'ADS1115 :
//! température et humidité en haute précision, une fois par seconde. Ce
//! module ne rend que des codes bruts ; la conversion en °C et en % appartient
//! à `screen-core`.

use core::sync::atomic::{AtomicBool, AtomicU32, Ordering::Relaxed};

use embassy_time::{Duration, Instant, Timer};
use embedded_hal_async::i2c::I2c;
use log::info;

const ADDRESS: u8 = 0x44;
/// Mesure haute précision, résultat disponible après 10 ms au plus.
const CMD_MEASURE_HIGH: u8 = 0xFD;
const MEASURE_TIME: Duration = Duration::from_millis(10);
const PERIOD: Duration = Duration::from_millis(1000);

#[derive(Debug)]
pub enum Error<E> {
    I2c(E),
    /// Le CRC-8 d'une des deux paires ne correspond pas.
    Crc,
}

/// CRC-8 du SHT4x : polynôme 0x31, valeur initiale 0xFF.
fn crc8(data: [u8; 2]) -> u8 {
    let mut crc = 0xFF_u8;
    for byte in data {
        crc ^= byte;
        for _ in 0..8 {
            crc = if crc & 0x80 != 0 {
                (crc << 1) ^ 0x31
            } else {
                crc << 1
            };
        }
    }
    crc
}

/// Lance une mesure, rend le bus pendant sa durée, puis lit les codes bruts
/// (température, humidité).
async fn read<I: I2c>(i2c: &mut I) -> Result<(u16, u16), Error<I::Error>> {
    i2c.write(ADDRESS, &[CMD_MEASURE_HIGH])
        .await
        .map_err(Error::I2c)?;
    Timer::after(MEASURE_TIME).await;
    let mut frame = [0u8; 6];
    i2c.read(ADDRESS, &mut frame).await.map_err(Error::I2c)?;
    if crc8([frame[0], frame[1]]) != frame[2] || crc8([frame[3], frame[4]]) != frame[5] {
        return Err(Error::Crc);
    }
    Ok((
        u16::from_be_bytes([frame[0], frame[1]]),
        u16::from_be_bytes([frame[3], frame[4]]),
    ))
}

/// Dernière lecture réussie, en codes bruts : température dans les 16 bits
/// hauts, humidité dans les 16 bits bas. `OK` à faux : la dernière tentative
/// a échoué (NACK ou CRC) et les codes sont périmés.
pub static RAW: AtomicU32 = AtomicU32::new(0);
pub static OK: AtomicBool = AtomicBool::new(false);
/// Compte chaque tentative terminée, succès comme erreur ; le cœur lit les
/// codes après ce marqueur, comme pour l'ADS1115.
pub static SAMPLES: AtomicU32 = AtomicU32::new(0);
pub static ERRORS: AtomicU32 = AtomicU32::new(0);

/// Code brut de température et d'humidité de la dernière lecture réussie.
pub fn raw() -> (u16, u16) {
    let packed = RAW.load(Relaxed);
    ((packed >> 16) as u16, packed as u16)
}

/// Mesure toutes les secondes. Un capteur absent ou en erreur ne bloque rien :
/// la tâche réessaie à la période suivante.
pub async fn run<I: I2c>(mut i2c: I) -> ! {
    let mut next = Instant::now();
    let mut first = true;
    loop {
        match read(&mut i2c).await {
            Ok((t_raw, rh_raw)) => {
                RAW.store((u32::from(t_raw) << 16) | u32::from(rh_raw), Relaxed);
                OK.store(true, Relaxed);
                if first {
                    first = false;
                    info!("SHT40 à 0x{ADDRESS:02x}: t={t_raw} rh={rh_raw}");
                }
            }
            Err(_) => {
                OK.store(false, Relaxed);
                ERRORS.fetch_add(1, Relaxed);
            }
        }
        SAMPLES.fetch_add(1, Relaxed);
        next += PERIOD;
        // Une mesure en retard ne déclenche pas de rafale de rattrapage.
        if next < Instant::now() {
            next = Instant::now();
        }
        Timer::at(next).await;
    }
}
