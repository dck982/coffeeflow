//! Le matériel du module capteurs, derrière le trait `Hw` de la logique.

use core::sync::atomic::Ordering::Relaxed;

use esp_hal::Blocking;
use esp_hal::gpio::{Level, Output};
use esp_hal::i2c::master::I2c;
use log::warn;
use protocol::Frame;
use sensors_logic::flash_rx::{FlashSink, HEAD_LEN};
use sensors_logic::machine::{DIMMER_ADDR, DIMMER_REG_COMMAND};
use sensors_logic::{FlowCounter, Hw, TwaiCounters};

use crate::can;
use crate::clock;
use crate::flash::FlashOta;
use crate::flow;
use crate::lockout;

/// Niveaux de HEATER_CMD : HIGH ouvre Q3, qui ouvre Q2 et envoie le 5 V au
/// SSR chaudière par J8.
pub const HEATER_ACTIVE: Level = Level::High;
pub const HEATER_INACTIVE: Level = Level::Low;

/// TMP102 de la carte (ADD0 à GND) et son registre de température.
pub const TMP102_ADDR: u8 = 0x48;
pub const TMP102_REG_TEMPERATURE: u8 = 0x00;
/// ADS1115 de la carte (ADDR au +3V3).
pub const ADS1115_ADDR: u8 = 0x49;
pub const ADS1115_REG_CONVERSION: u8 = 0x00;
pub const ADS1115_REG_CONFIG: u8 = 0x01;
/// Conversion continue de AIN1 (sonde de pression) contre GND, ±4,096 V
/// (125 µV par code), 16 SPS comme l'A2 de l'écran : 62,5 ms d'intégration
/// lissent l'ondulation de la pompe vibrante. Comparateur coupé.
/// OS 1, MUX 101, PGA 001, MODE 0, DR 010, COMP 0 0 0, QUE 11.
const ADS1115_CONFIG_AIN1: u16 = 0xD243;

pub struct Board {
    pub valve: Output<'static>,
    pub heater: Output<'static>,
    pub i2c: I2c<'static, Blocking>,
    pub ota: FlashOta,
    /// L'ADS1115 a reçu sa configuration depuis sa dernière erreur : après
    /// une coupure, il serait revenu en mode coup par coup sur AIN0.
    pub ads_configured: bool,
}

impl FlashSink for Board {
    fn slot_size(&self) -> Option<u32> {
        self.ota.slot_size()
    }
    fn erase_slot(&mut self) -> bool {
        self.ota.erase_slot()
    }
    fn write(&mut self, offset: u32, data: &[u8]) -> bool {
        self.ota.write(offset, data)
    }
    fn commit(&mut self, head: &[u8; HEAD_LEN], image_size: u32) -> bool {
        self.ota.commit(head, image_size)
    }
}

impl Hw for Board {
    fn now_us(&self) -> i64 {
        clock::now_us()
    }

    fn set_valve(&mut self, open: bool) {
        // HIGH = vanne ouverte.
        self.valve
            .set_level(if open { Level::High } else { Level::Low });
    }

    fn set_heater(&mut self, on: bool) {
        self.heater
            .set_level(if on { HEATER_ACTIVE } else { HEATER_INACTIVE });
    }

    fn dimmer_write(&mut self, reg: u8, value: u8) -> bool {
        // La commande 0x03 du registre COMMAND bascule le dimmer en UART et
        // l'écrit dans son EEPROM : aucun chemin du code ne doit l'émettre, et
        // cette dernière barrière la refuse même si l'un d'eux se trompait.
        if reg == DIMMER_REG_COMMAND && !matches!(value, 0x01 | 0x02) {
            warn!("commande dimmer {value:#04x} refusée");
            return false;
        }
        self.i2c.write(DIMMER_ADDR as u8, &[reg, value]).is_ok()
    }

    fn dimmer_read(&mut self, reg: u8) -> Option<u8> {
        let mut value = [0u8];
        self.i2c
            .write_read(DIMMER_ADDR as u8, &[reg], &mut value)
            .ok()?;
        Some(value[0])
    }

    fn send(&mut self, frame: Frame) {
        if can::TX.try_send(frame).is_err() {
            warn!("file d'émission CAN pleine, trame perdue");
        }
    }

    fn flow(&self) -> FlowCounter {
        flow::read()
    }

    fn twai_counters(&mut self) -> Option<TwaiCounters> {
        Some(TwaiCounters {
            rx_errors: can::RX_ERRORS.load(Relaxed),
            tx_errors: can::TX_ERRORS.load(Relaxed),
            // Le contrôleur n'expose pas ce compteur ; le pilote IDF le tenait
            // par interruption.
            bus_errors: 0,
        })
    }

    fn enclosure_temperature_raw(&mut self) -> Option<i16> {
        let mut value = [0u8; 2];
        self.i2c
            .write_read(TMP102_ADDR, &[TMP102_REG_TEMPERATURE], &mut value)
            .ok()?;
        // 12 bits cadrés à gauche, en 1/16 °C : le décalage garde le signe.
        Some(i16::from_be_bytes(value) >> 4)
    }

    fn pressure_raw(&mut self) -> Option<i16> {
        if !self.ads_configured {
            let [hi, lo] = ADS1115_CONFIG_AIN1.to_be_bytes();
            if self
                .i2c
                .write(ADS1115_ADDR, &[ADS1115_REG_CONFIG, hi, lo])
                .is_err()
            {
                return None;
            }
            self.ads_configured = true;
        }
        let mut value = [0u8; 2];
        if self
            .i2c
            .write_read(ADS1115_ADDR, &[ADS1115_REG_CONVERSION], &mut value)
            .is_err()
        {
            self.ads_configured = false;
            return None;
        }
        Some(i16::from_be_bytes(value))
    }

    fn store_lockout(&mut self, active: bool) {
        if active {
            lockout::set();
        }
    }

    fn ota_mark_valid(&mut self) -> bool {
        self.ota.mark_valid()
    }
}
