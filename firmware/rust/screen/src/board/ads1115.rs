//! ADS1115 de la chaudière : A0 et A1 pour le pont NTC, A2 pour la sonde de
//! pression analogique (XDB401, 0,4-2,4 V). Même séquence que l'écran en C++ :
//! une conversion unique par canal, lue par scrutation. Ce module ne rend que
//! des codes bruts ; la conversion en °C et en bar appartient à `screen-core`.

use core::sync::atomic::{AtomicBool, AtomicU32, Ordering::Relaxed};

use embassy_time::{Duration, Instant, Timer};
use embedded_hal_async::i2c::I2c;
use log::info;

const REG_CONVERSION: u8 = 0x00;
const REG_CONFIG: u8 = 0x01;

/// Canaux monopolaires AIN0, AIN1, AIN2 contre GND, tels que codés dans les
/// bits MUX (14:12) du registre de configuration.
const MUX_A0: u16 = 4;
const MUX_A1: u16 = 5;
const MUX_A2: u16 = 6;

// OS=1 (lance une conversion), PGA ±4,096 V (0x0200), mode unique (0x0100),
// comparateur désactivé (0x0003). Le pont NTC est lu à 128 SPS. La pression
// (A2) l'est à 16 SPS : la conversion intègre 62,5 ms, soit environ trois
// impulsions de la pompe vibrante (50 Hz), dont l'ondulation, échantillonnée
// à 128 SPS, ajoutait jusqu'à ±0,2 bar.
const CONFIG_BASE: u16 = 0x8000 | 0x0200 | 0x0100 | 0x0003;
const RATE_128_SPS: u16 = 0x0080;
const RATE_16_SPS: u16 = 0x0020;

/// Période d'un cycle A0, A1, A2 quand tout va bien.
const CYCLE: Duration = Duration::from_millis(100);
/// Essais de lecture d'une conversion, toutes les 10 ms : 150 ms couvrent les
/// 62,5 ms de A2 avec de la marge.
const POLL_ATTEMPTS: u32 = 15;
const POLL_EVERY: Duration = Duration::from_millis(10);
/// Reprise après erreur : 200, 400, 800 puis 1000 ms.
const FIRST_RETRY_MS: u64 = 200;
const MAX_RETRY_MS: u64 = 1000;

#[derive(Debug)]
pub enum Error<E> {
    I2c(E),
    /// Le bit OS n'est jamais passé à 1.
    Timeout,
}

pub struct Ads1115<I> {
    i2c: I,
    addr: u8,
}

impl<I: I2c> Ads1115<I> {
    /// ADDR peut être câblé à GND, VDD, SDA ou SCL : on essaie les quatre
    /// adresses, qu'aucun autre composant du bus n'utilise.
    pub async fn probe(mut i2c: I) -> Result<Self, I> {
        for addr in 0x48..=0x4B {
            let mut config = [0u8; 2];
            if i2c
                .write_read(addr, &[REG_CONFIG], &mut config)
                .await
                .is_ok()
            {
                return Ok(Self { i2c, addr });
            }
        }
        Err(i2c)
    }

    pub fn addr(&self) -> u8 {
        self.addr
    }

    /// Lance une conversion sur `mux`, attend la fin et rend le code brut
    /// signé.
    async fn read(&mut self, mux: u16) -> Result<i16, Error<I::Error>> {
        let rate = if mux == MUX_A2 {
            RATE_16_SPS
        } else {
            RATE_128_SPS
        };
        let config = CONFIG_BASE | (mux << 12) | rate;
        let [hi, lo] = config.to_be_bytes();
        self.i2c
            .write(self.addr, &[REG_CONFIG, hi, lo])
            .await
            .map_err(Error::I2c)?;
        for _ in 0..POLL_ATTEMPTS {
            Timer::after(POLL_EVERY).await;
            let mut status = [0u8; 2];
            self.i2c
                .write_read(self.addr, &[REG_CONFIG], &mut status)
                .await
                .map_err(Error::I2c)?;
            if status[0] & 0x80 != 0 {
                let mut code = [0u8; 2];
                self.i2c
                    .write_read(self.addr, &[REG_CONVERSION], &mut code)
                    .await
                    .map_err(Error::I2c)?;
                return Ok(i16::from_be_bytes(code));
            }
        }
        Err(Error::Timeout)
    }
}

/// Dernier cycle lu, en codes bruts. `A2_OK` à faux : A2 a échoué et sa
/// valeur est périmée, sans invalider la paire NTC.
pub static RAW: [AtomicU32; 3] = [const { AtomicU32::new(0) }; 3];
pub static A2_OK: AtomicBool = AtomicBool::new(false);
/// Validité de la dernière paire ; une erreur doit invalider la chaudière
/// sans republier les anciens codes comme une mesure neuve.
pub static NTC_OK: AtomicBool = AtomicBool::new(false);
/// Compte chaque tentative terminée, succès comme erreur. Tout vit sur le
/// même exécuteur : le cœur lit les codes après ce marqueur sans interruption
/// par la tâche ADS (il ne rend pas la main pendant la lecture).
pub static SAMPLES: AtomicU32 = AtomicU32::new(0);
/// Cycles complets, cycles en erreur, et adresse trouvée (0 : absent).
pub static CYCLES: AtomicU32 = AtomicU32::new(0);
pub static ERRORS: AtomicU32 = AtomicU32::new(0);
pub static ADDRESS: AtomicU32 = AtomicU32::new(0);

/// Code brut d'un canal (0 = A0, 1 = A1, 2 = A2).
pub fn raw(channel: usize) -> i16 {
    RAW[channel].load(Relaxed) as u16 as i16
}

/// Lit A0, A1 puis A2 toutes les 100 ms. Une sonde absente ou en erreur ne
/// bloque rien : la tâche réessaie avec une attente croissante.
///
/// Les codes et leur validité sont publiés avec `SAMPLES` ; le propriétaire
/// du cœur les prend une fois par tentative terminée, sans rajeunir les anciens.
#[allow(
    clippy::large_stack_frames,
    reason = "état de la future ADS stocké dans l’arène de la tâche, pas une allocation de pile à chaque lecture"
)]
pub async fn run<I: I2c>(i2c: I) -> ! {
    let mut adc = {
        let mut i2c = i2c;
        loop {
            screen::crumb!();
            match Ads1115::probe(i2c).await {
                Ok(adc) => break adc,
                Err(back) => {
                    i2c = back;
                    ERRORS.fetch_add(1, Relaxed);
                    Timer::after(Duration::from_millis(MAX_RETRY_MS)).await;
                }
            }
        }
    };
    ADDRESS.store(adc.addr() as u32, Relaxed);
    info!("ADS1115 à 0x{:02x}", adc.addr());

    let mut next = Instant::now();
    let mut failures = 0u32;
    loop {
        screen::crumb!();
        let a0 = adc.read(MUX_A0).await;
        let a1 = match a0 {
            Ok(_) => adc.read(MUX_A1).await,
            Err(_) => Err(Error::Timeout),
        };
        let ntc_ok = a0.is_ok() && a1.is_ok();
        // A2 n'est tenté que si la paire a réussi, comme en C++ : après une
        // erreur de bus, le prochain essai attend de toute façon.
        let a2 = if ntc_ok {
            Some(adc.read(MUX_A2).await)
        } else {
            None
        };
        let a2_ok = matches!(a2, Some(Ok(_)));

        if let (Ok(a0), Ok(a1)) = (&a0, &a1) {
            RAW[0].store(*a0 as u16 as u32, Relaxed);
            RAW[1].store(*a1 as u16 as u32, Relaxed);
        }
        if let Some(Ok(a2)) = &a2 {
            RAW[2].store(*a2 as u16 as u32, Relaxed);
        }
        A2_OK.store(a2_ok, Relaxed);
        NTC_OK.store(ntc_ok, Relaxed);
        SAMPLES.fetch_add(1, Relaxed);

        if ntc_ok && a2_ok {
            failures = 0;
            CYCLES.fetch_add(1, Relaxed);
            next += CYCLE;
            Timer::at(next).await;
        } else {
            ERRORS.fetch_add(1, Relaxed);
            let retry = (FIRST_RETRY_MS << failures.min(3)).min(MAX_RETRY_MS);
            failures = (failures + 1).min(4);
            Timer::after(Duration::from_millis(retry)).await;
            next = Instant::now();
        }
    }
}
