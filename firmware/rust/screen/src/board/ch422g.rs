//! Expandeur d'E/S CH422G : DISP, LCD_RST, TP_RST et CAN_SEL.
//!
//! L'adresse I2C choisit la fonction. Le registre de sortie n'a pas de
//! lecture : on garde un miroir et chaque écriture envoie l'octet complet,
//! pour qu'un bit ne soit jamais remis à zéro par un autre.

use embedded_hal_async::i2c::I2c;

const MODE_ADDR: u8 = 0x24;
const OUT_ADDR: u8 = 0x38;
const MODE_OUTPUTS: u8 = 0x01;

pub const EXIO_TP_RST: u8 = 1 << 1;
pub const EXIO_DISP: u8 = 1 << 2;
pub const EXIO_LCD_RST: u8 = 1 << 3;
pub const EXIO_CAN_SEL: u8 = 1 << 5;

pub struct Ch422g<I> {
    i2c: I,
    out: u8,
}

impl<I: I2c> Ch422g<I> {
    /// Passe les broches en sorties. DISP est haut dès la première écriture
    /// et ne retombe plus : la dalle sort de veille avant le flux RGB, jamais
    /// au milieu d'une trame. CAN_SEL est bas : le transceiver CAN reste
    /// désélectionné et les broches USB gardent leur rôle.
    pub async fn new(mut i2c: I) -> Result<Self, I::Error> {
        i2c.write(MODE_ADDR, &[MODE_OUTPUTS]).await?;
        let mut expander = Self { i2c, out: EXIO_DISP };
        expander.write().await?;
        Ok(expander)
    }

    async fn write(&mut self) -> Result<(), I::Error> {
        self.i2c.write(OUT_ADDR, &[self.out]).await
    }

    pub async fn set_bit(&mut self, bit: u8, high: bool) -> Result<(), I::Error> {
        if high {
            self.out |= bit;
        } else {
            self.out &= !bit;
        }
        self.write().await
    }

    /// Bascule les GPIO19/20 entre l'USB (bas) et le transceiver CAN (haut).
    /// Le bureau ne monte que l'USB, la machine que le CAN.
    #[allow(dead_code, reason = "appelée par la tâche CAN, qui n'existe pas encore")]
    pub async fn select_can(&mut self, can: bool) -> Result<(), I::Error> {
        self.set_bit(EXIO_CAN_SEL, can).await
    }
}
