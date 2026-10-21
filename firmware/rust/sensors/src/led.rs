//! LED d'état de la carte sensors (D2 sur GPIO2, allumée au niveau haut).
//!
//! - Au démarrage, fixe pendant au moins 1 s : la carte a démarré.
//! - Défaut constaté au démarrage : clignotement continu jusqu'au prochain
//!   redémarrage, 4 Hz pour les puces de la carte, 1 Hz pour le dimmer.
//! - Sinon, éclair de 50 ms à chaque trame CAN reçue : éteinte, le bus est
//!   muet ; un écran présent la fait scintiller.

use esp_hal::gpio::Output;

const BOOT_HOLD_US: i64 = 1_000_000;
const ACTIVITY_US: i64 = 50_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BootFault {
    /// Le TMP102 (0x48) ou l'ADS1115 (0x49) soudés sur la carte ne répondent
    /// pas : XIAO mal enfiché, bus I2C en défaut. Masque le défaut dimmer.
    Board,
    /// Le DimmerLink (0x50) ne répond pas : la pompe ne démarrera pas.
    Dimmer,
}

impl BootFault {
    /// Demi-période du clignotement.
    fn half_period_us(self) -> i64 {
        match self {
            Self::Board => 125_000,
            Self::Dimmer => 500_000,
        }
    }
}

pub struct Led {
    pin: Output<'static>,
    fault: Option<BootFault>,
    hold_until_us: i64,
    activity_until_us: i64,
}

impl Led {
    /// `pin` est déjà allumée depuis le début de `main`.
    pub fn new(pin: Output<'static>, fault: Option<BootFault>, boot_us: i64) -> Self {
        Self {
            pin,
            fault,
            hold_until_us: boot_us + BOOT_HOLD_US,
            activity_until_us: 0,
        }
    }

    pub fn on_frame(&mut self, now_us: i64) {
        self.activity_until_us = now_us + ACTIVITY_US;
    }

    pub fn update(&mut self, now_us: i64) {
        let on = if now_us < self.hold_until_us {
            true
        } else if let Some(fault) = self.fault {
            // Éteinte d'abord, pour séparer le clignotement de l'allumage fixe.
            (now_us - self.hold_until_us) / fault.half_period_us() % 2 == 1
        } else {
            now_us < self.activity_until_us
        };
        if on {
            self.pin.set_high();
        } else {
            self.pin.set_low();
        }
    }
}
