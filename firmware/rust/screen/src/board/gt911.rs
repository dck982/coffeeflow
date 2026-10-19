//! Tactile GT911, lu en polling (l'IRQ reste tenue basse depuis le reset,
//! pour l'adresse 0x5D).

use embedded_hal_async::i2c::I2c;

use super::{H_RES, V_RES};

pub struct Gt911<I> {
    i2c: I,
    addr: u8,
}

pub enum Touch {
    /// Pas de nouvelle mesure depuis la dernière lecture.
    Unchanged,
    Released,
    Pressed(i32, i32),
}

const PRODUCT_ID: [u8; 2] = [0x81, 0x40];
const STATUS: [u8; 2] = [0x81, 0x4E];
const POINT1: [u8; 2] = [0x81, 0x50];
// L'écran est vu en plongée et le doigt tombe un peu au-dessus de la cible :
// le C++ décale l'appui de 10 px vers le bas.
const Y_OFFSET: i32 = 10;

impl<I: I2c> Gt911<I> {
    /// Cherche le contrôleur aux deux adresses possibles.
    pub async fn probe(mut i2c: I) -> Option<Self> {
        for addr in [0x5D, 0x14] {
            let mut id = [0u8; 4];
            if i2c.write_read(addr, &PRODUCT_ID, &mut id).await.is_ok() && &id[..3] == b"911" {
                return Some(Self { i2c, addr });
            }
        }
        None
    }

    pub fn addr(&self) -> u8 {
        self.addr
    }

    pub async fn read(&mut self) -> Result<Touch, I::Error> {
        let mut status = [0u8];
        self.i2c.write_read(self.addr, &STATUS, &mut status).await?;
        if status[0] & 0x80 == 0 {
            return Ok(Touch::Unchanged);
        }
        let touch = if status[0] & 0x0F == 0 {
            Touch::Released
        } else {
            let mut p = [0u8; 4];
            self.i2c.write_read(self.addr, &POINT1, &mut p).await?;
            let x = u16::from_le_bytes([p[0], p[1]]) as i32;
            let y = u16::from_le_bytes([p[2], p[3]]) as i32 + Y_OFFSET;
            Touch::Pressed(x.min(H_RES as i32 - 1), y.min(V_RES as i32 - 1))
        };
        self.i2c
            .write(self.addr, &[STATUS[0], STATUS[1], 0])
            .await?;
        Ok(touch)
    }
}
