//! Lecture du réfractomètre DiFluid R2 en BLE (`coffeetool tds`). La mesure
//! se fait à part de l'infusion, souvent quelques minutes après : elle
//! s'affiche, sans entrer dans une capture.
//!
//! Protocole publié par DiFluid (licence MIT), vérifié sur l'appareil par le
//! distributeur coréen : trames `DF DF func cmd len data… somme` sur la
//! caractéristique 0xAA01 du service 0x00FF, en écriture avec réponse et en
//! notification. Une notification peut porter plusieurs trames. Le R2
//! n'annonce plus rien tant qu'un autre appareil (l'application) y est
//! connecté.

use std::time::{Duration, Instant};

use anyhow::{Result, anyhow, bail};
use btleplug::api::{Central, Manager as _, Peripheral as _, ScanFilter, WriteType};
use btleplug::platform::{Manager, Peripheral};
use futures::StreamExt;
use uuid::Uuid;

const NAME_PREFIX: &str = "difluid r2";
const DATA_UUID: &str = "0000aa01-0000-1000-8000-00805f9b34fb";
const SCAN_TIMEOUT: Duration = Duration::from_secs(10);
// Un test simple dure 2,4 à 3,2 s ; une moyenne, à peu près autant par tour.
const SINGLE_TIMEOUT: Duration = Duration::from_secs(15);
const AVERAGE_ROUND_TIMEOUT: Duration = Duration::from_secs(6);

const HEADER: [u8; 2] = [0xDF, 0xDF];
const FUNC_ACTION: u8 = 0x03;
const CMD_SINGLE: u8 = 0x00;
const CMD_AVERAGE: u8 = 0x01;
const CMD_ERROR: u8 = 0xFE;

const STATUS_TEST_FINISHED: u8 = 0;
const STATUS_AVERAGE_FINISHED: u8 = 6;

pub const AVERAGE_MIN: u8 = 2;
pub const AVERAGE_MAX: u8 = 10;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub func: u8,
    pub cmd: u8,
    pub data: Vec<u8>,
}

fn checksum(bytes: &[u8]) -> u8 {
    bytes.iter().fold(0u8, |sum, b| sum.wrapping_add(*b))
}

pub fn encode(func: u8, cmd: u8, data: &[u8]) -> Vec<u8> {
    let mut frame = Vec::with_capacity(6 + data.len());
    frame.extend_from_slice(&HEADER);
    frame.extend_from_slice(&[func, cmd, data.len() as u8]);
    frame.extend_from_slice(data);
    frame.push(checksum(&frame));
    frame
}

/// Recolle les trames à travers les notifications ; un octet qui ne commence
/// pas une trame valide est sauté.
#[derive(Default)]
pub struct Splitter {
    buffer: Vec<u8>,
}

impl Splitter {
    pub fn push(&mut self, bytes: &[u8]) -> Vec<Frame> {
        self.buffer.extend_from_slice(bytes);
        let mut frames = Vec::new();
        loop {
            match self.buffer.windows(2).position(|w| w == HEADER) {
                Some(start) => {
                    self.buffer.drain(..start);
                }
                None => {
                    // Garder un éventuel premier octet d'en-tête.
                    let keep = usize::from(self.buffer.last() == Some(&HEADER[0]));
                    self.buffer.drain(..self.buffer.len() - keep);
                    return frames;
                }
            }
            if self.buffer.len() < 5 {
                return frames;
            }
            let total = 6 + usize::from(self.buffer[4]);
            if self.buffer.len() < total {
                return frames;
            }
            if checksum(&self.buffer[..total - 1]) == self.buffer[total - 1] {
                frames.push(Frame {
                    func: self.buffer[2],
                    cmd: self.buffer[3],
                    data: self.buffer[5..total - 1].to_vec(),
                });
                self.buffer.drain(..total);
            } else {
                self.buffer.drain(..1);
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Measurement {
    pub tds_pct: f64,
    pub refractive_index: f64,
    pub prism_c: f64,
    pub sample_c: f64,
    /// Nombre de tests moyennés ; 1 pour un test simple.
    pub rounds: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    Measurement(Measurement),
    Error(String),
}

fn be_i16(bytes: &[u8]) -> i16 {
    i16::from_be_bytes([bytes[0], bytes[1]])
}

fn celsius(tenths: i16, unit: u8) -> f64 {
    let value = f64::from(tenths) / 10.0;
    if unit == 1 {
        (value - 32.0) * 5.0 / 9.0
    } else {
        value
    }
}

fn error_text(class: u8, code: u8) -> String {
    match (class, code) {
        (2, 1) => "erreur de test".into(),
        (2, 2) => "calibration ratée".into(),
        (2, 3) => "pas de liquide sur le prisme".into(),
        (2, 4) => "hors plage".into(),
        _ => format!("erreur de l'appareil (classe {class}, code {code})"),
    }
}

/// Assemble les paquets d'un test, simple ou moyen, qu'il soit lancé par
/// `coffeetool` ou au bouton de l'appareil.
#[derive(Default)]
pub struct Assembler {
    temperatures: Option<(f64, f64)>,
    single: Option<(f64, f64)>,
    single_finished: bool,
    average_tds_pct: Option<f64>,
    average_round: u8,
}

impl Assembler {
    pub fn on_frame(&mut self, frame: &Frame) -> Option<Event> {
        if frame.func != FUNC_ACTION {
            return None;
        }
        if frame.cmd == CMD_ERROR {
            *self = Self::default();
            let class = frame.data.get(1).copied().unwrap_or(0);
            let code = frame.data.get(2).copied().unwrap_or(0);
            return Some(Event::Error(error_text(class, code)));
        }
        if frame.cmd != CMD_SINGLE && frame.cmd != CMD_AVERAGE {
            return None;
        }
        let data = &frame.data;
        match (data.first().copied(), data.len()) {
            (Some(0), 2..) => {
                let status = data[1];
                if frame.cmd == CMD_SINGLE && status == STATUS_TEST_FINISHED {
                    self.single_finished = true;
                }
                if frame.cmd == CMD_AVERAGE && status == STATUS_AVERAGE_FINISHED {
                    return self.finish_average();
                }
                None
            }
            (Some(1), 6..) => {
                let unit = data[5];
                self.temperatures = Some((
                    celsius(be_i16(&data[1..3]), unit),
                    celsius(be_i16(&data[3..5]), unit),
                ));
                None
            }
            (Some(2), 7..) => {
                let tds_pct = f64::from(be_i16(&data[1..3])) / 100.0;
                let raw_index = u32::from_be_bytes([data[3], data[4], data[5], data[6]]);
                self.single = Some((tds_pct, f64::from(raw_index) / 100_000.0));
                if frame.cmd == CMD_SINGLE && self.single_finished {
                    return self.finish_single();
                }
                None
            }
            // Moyenne en cours ; le champ de 4 octets qui suit ne correspond
            // pas à un indice et n'est pas lu.
            (Some(3), 3..) => {
                self.average_tds_pct = Some(f64::from(be_i16(&data[1..3])) / 100.0);
                None
            }
            (Some(4), 6..) => {
                self.average_round = data[5];
                None
            }
            _ => None,
        }
    }

    fn finish_single(&mut self) -> Option<Event> {
        let state = std::mem::take(self);
        let (tds_pct, refractive_index) = state.single?;
        let (prism_c, sample_c) = state.temperatures?;
        Some(Event::Measurement(Measurement {
            tds_pct,
            refractive_index,
            prism_c,
            sample_c,
            rounds: 1,
        }))
    }

    fn finish_average(&mut self) -> Option<Event> {
        let state = std::mem::take(self);
        let tds_pct = state.average_tds_pct?;
        let (_, refractive_index) = state.single?;
        let (prism_c, sample_c) = state.temperatures?;
        Some(Event::Measurement(Measurement {
            tds_pct,
            refractive_index,
            prism_c,
            sample_c,
            rounds: state.average_round,
        }))
    }
}

/// Masses d'une capture haute fréquence, avec les définitions de
/// `describe_hf_capture.py` pour que l'extraction tombe sur les chiffres du
/// journal : dose du profil, et poids final = maximum des poids valides après
/// l'arrêt de la pompe (premier échantillon hors infusion après l'infusion).
/// Un poids est écarté sans le bit balance, s'il est négatif, ou dans un
/// cooldown après un poids négatif (tasse retirée).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CaptureMasses {
    pub dose_g: Option<f64>,
    pub final_weight_g: Option<f64>,
}

const SCALE_VALID: u64 = 0x04;

fn mode(sample: &serde_json::Value) -> &str {
    sample
        .get("mode")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("")
}

fn infusion_mode(mode: &str) -> bool {
    matches!(mode, "infusion" | "ramp_down" | "rampdown")
}

pub fn capture_masses(capture: &serde_json::Value) -> Result<CaptureMasses> {
    let schema_ok = matches!(
        capture.get("schema").and_then(serde_json::Value::as_str),
        Some("coffeeflow.hf_capture.v1" | "coffeeflow.hf_capture.v2" | "coffeeflow.hf_capture.v3")
    );
    let Some(samples) = capture.get("samples").and_then(serde_json::Value::as_array) else {
        bail!("capture inconnue ou incompatible");
    };
    if !schema_ok {
        bail!("capture inconnue ou incompatible");
    }
    let dose_g = capture
        .pointer("/profile/dose_g")
        .and_then(serde_json::Value::as_f64)
        .filter(|d| d.is_finite() && *d > 0.0);

    let mut cooldown_disturbed = false;
    let weights: Vec<Option<f64>> = samples
        .iter()
        .map(|s| {
            let value = s.get("weight_g").and_then(serde_json::Value::as_f64);
            let flags = s
                .get("flags")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            let cooldown = mode(s) == "cooldown";
            if cooldown && value.is_some_and(|v| v < 0.0) {
                cooldown_disturbed = true;
            }
            value.filter(|v| {
                v.is_finite()
                    && *v >= 0.0
                    && flags & SCALE_VALID != 0
                    && !(cooldown && cooldown_disturbed)
            })
        })
        .collect();
    let first = samples.iter().position(|s| infusion_mode(mode(s)));
    let stop =
        first.and_then(|first| (first..samples.len()).find(|&i| !infusion_mode(mode(&samples[i]))));
    let final_weight_g = stop.and_then(|stop| {
        weights[stop..]
            .iter()
            .flatten()
            .copied()
            .fold(None, |max: Option<f64>, w| {
                Some(max.map_or(w, |m| m.max(w)))
            })
    });
    Ok(CaptureMasses {
        dose_g,
        final_weight_g,
    })
}

/// Ligne affichée pour une mesure, avec le rendement d'extraction si la dose
/// et la boisson sont données.
pub fn describe(m: &Measurement, dose_g: Option<f64>, beverage_g: Option<f64>) -> String {
    let mut line = format!("TDS {:.2} %", m.tds_pct);
    if m.rounds > 1 {
        line.push_str(&format!(
            " (moyenne de {} tests ; indice du dernier)",
            m.rounds
        ));
    }
    line.push_str(&format!(
        "  indice {:.5}  prisme {:.1} °C  échantillon {:.1} °C",
        m.refractive_index, m.prism_c, m.sample_c
    ));
    if let (Some(dose), Some(beverage)) = (dose_g, beverage_g) {
        let extraction = m.tds_pct * beverage / dose;
        line.push_str(&format!(
            "\nextraction {extraction:.1} % ({:.2} % × {beverage:.1} g / {dose:.1} g)",
            m.tds_pct
        ));
    }
    line
}

async fn find_peripheral(manager: &Manager, address: Option<&str>) -> Result<Peripheral> {
    let adapter = manager
        .adapters()
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("aucun adaptateur Bluetooth"))?;
    adapter.start_scan(ScanFilter::default()).await?;
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
                None => properties
                    .local_name
                    .is_some_and(|name| name.to_lowercase().starts_with(NAME_PREFIX)),
            };
            if matches {
                found = Some(peripheral);
                break;
            }
        }
    }
    let _ = adapter.stop_scan().await;
    found
        .ok_or_else(|| anyhow!("R2 introuvable : allumé, et déconnecté de l'application DiFluid ?"))
}

pub enum Mode {
    Single,
    Average(u8),
    /// Attend les tests lancés au bouton, jusqu'à Ctrl-C.
    Listen,
}

async fn run_async(
    mode: Mode,
    address: Option<&str>,
    on_measurement: &mut dyn FnMut(&Measurement),
) -> Result<()> {
    let manager = Manager::new().await?;
    let peripheral = find_peripheral(&manager, address).await?;
    peripheral.connect().await?;
    peripheral.discover_services().await?;
    let data_uuid = Uuid::parse_str(DATA_UUID)?;
    let characteristic = peripheral
        .characteristics()
        .into_iter()
        .find(|c| c.uuid == data_uuid)
        .ok_or_else(|| anyhow!("caractéristique 0xAA01 introuvable"))?;
    peripheral.subscribe(&characteristic).await?;
    let name = peripheral
        .properties()
        .await?
        .and_then(|p| p.local_name)
        .unwrap_or_else(|| peripheral.id().to_string());
    println!("{name} connecté");
    let mut notifications = peripheral.notifications().await?;

    let timeout = match mode {
        Mode::Single => Some(SINGLE_TIMEOUT),
        Mode::Average(rounds) => Some(AVERAGE_ROUND_TIMEOUT * u32::from(rounds)),
        Mode::Listen => None,
    };
    match mode {
        Mode::Single => {
            let command = encode(FUNC_ACTION, CMD_SINGLE, &[]);
            peripheral
                .write(&characteristic, &command, WriteType::WithResponse)
                .await?;
        }
        Mode::Average(rounds) => {
            let command = encode(FUNC_ACTION, CMD_AVERAGE, &[rounds]);
            peripheral
                .write(&characteristic, &command, WriteType::WithResponse)
                .await?;
        }
        Mode::Listen => println!("en attente d'un test lancé sur l'appareil (Ctrl-C pour finir)"),
    }

    let deadline = timeout.map(|t| tokio::time::Instant::now() + t);
    let mut splitter = Splitter::default();
    let mut assembler = Assembler::default();
    let outcome = loop {
        if crate::runtime::interrupted() {
            break Ok(());
        }
        let wait = Duration::from_millis(200);
        let next = match deadline {
            Some(deadline) if tokio::time::Instant::now() >= deadline => {
                break Err(anyhow!("pas de résultat du R2 dans le temps imparti"));
            }
            _ => tokio::time::timeout(wait, notifications.next()).await,
        };
        let notification = match next {
            Err(_) => continue,
            Ok(None) => break Err(anyhow!("liaison BLE perdue")),
            Ok(Some(notification)) => notification,
        };
        if notification.uuid != data_uuid {
            continue;
        }
        let mut done = None;
        for frame in splitter.push(&notification.value) {
            match assembler.on_frame(&frame) {
                Some(Event::Measurement(m)) => {
                    on_measurement(&m);
                    if !matches!(mode, Mode::Listen) {
                        done = Some(Ok(()));
                    }
                }
                Some(Event::Error(text)) => {
                    if matches!(mode, Mode::Listen) {
                        eprintln!("R2 : {text}");
                    } else {
                        done = Some(Err(anyhow!("R2 : {text}")));
                    }
                }
                None => {}
            }
        }
        if let Some(result) = done {
            break result;
        }
    };
    let _ = peripheral.disconnect().await;
    outcome
}

pub fn run(
    mode: Mode,
    address: Option<&str>,
    on_measurement: &mut dyn FnMut(&Measurement),
) -> Result<()> {
    if let Mode::Average(rounds) = mode
        && !(AVERAGE_MIN..=AVERAGE_MAX).contains(&rounds)
    {
        bail!("--average : {AVERAGE_MIN} à {AVERAGE_MAX} tests");
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(run_async(mode, address, on_measurement))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(text: &str) -> Vec<u8> {
        text.split_whitespace()
            .map(|b| u8::from_str_radix(b, 16).unwrap())
            .collect()
    }

    #[test]
    fn commands_match_the_published_bytes() {
        assert_eq!(encode(3, 0, &[]), hex("DF DF 03 00 00 C1"));
        assert_eq!(encode(3, 1, &[3]), hex("DF DF 03 01 01 03 C6"));
    }

    // Test simple relevé sur un R2 par le distributeur : 8,11 %, 1,34741.
    const SINGLE: [&str; 4] = [
        "DF DF 03 00 03 00 0B 00 CF",
        "DF DF 03 00 03 00 00 00 C4",
        "DF DF 03 00 06 01 00 BB 00 B3 00 36",
        "DF DF 03 00 07 02 03 2B 00 02 0E 55 5D",
    ];

    #[test]
    fn single_test_gives_one_measurement() {
        let mut splitter = Splitter::default();
        let mut assembler = Assembler::default();
        let mut events = Vec::new();
        for line in SINGLE {
            for frame in splitter.push(&hex(line)) {
                events.extend(assembler.on_frame(&frame));
            }
        }
        assert_eq!(
            events,
            vec![Event::Measurement(Measurement {
                tds_pct: 8.11,
                refractive_index: 1.34741,
                prism_c: 18.7,
                sample_c: 17.9,
                rounds: 1,
            })]
        );
    }

    #[test]
    fn frames_are_rejoined_across_notifications() {
        // Tout d'un bloc, puis coupé au milieu d'une trame.
        let all: Vec<u8> = SINGLE.iter().flat_map(|l| hex(l)).collect();
        let mut splitter = Splitter::default();
        assert_eq!(splitter.push(&all).len(), 4);
        let mut splitter = Splitter::default();
        let (a, b) = all.split_at(17);
        let mut frames = splitter.push(a);
        frames.extend(splitter.push(b));
        assert_eq!(frames.len(), 4);
    }

    #[test]
    fn a_corrupt_frame_is_skipped() {
        let mut bytes = hex("DF DF 03 00 03 00 0B 00 CE"); // somme fausse
        bytes.extend(hex("DF DF 03 00 03 00 00 00 C4"));
        let frames = Splitter::default().push(&bytes);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].data, vec![0, 0, 0]);
    }

    #[test]
    fn no_liquid_is_reported() {
        let frames = Splitter::default().push(&hex("DF DF 03 FE 03 00 02 03 C7"));
        assert_eq!(
            Assembler::default().on_frame(&frames[0]),
            Some(Event::Error("pas de liquide sur le prisme".into()))
        );
    }

    #[test]
    fn negative_concentration_is_signed() {
        let frame = Frame {
            func: 3,
            cmd: 0,
            data: hex("02 FF 91 00 02 08 D0"),
        };
        let mut assembler = Assembler::default();
        assembler.on_frame(&Frame {
            func: 3,
            cmd: 0,
            data: hex("01 00 FA 00 FA 00"),
        });
        assembler.on_frame(&Frame {
            func: 3,
            cmd: 0,
            data: hex("00 00 00"),
        });
        let Some(Event::Measurement(m)) = assembler.on_frame(&frame) else {
            panic!("pas de mesure");
        };
        assert_eq!(m.tds_pct, -1.11);
    }

    #[test]
    fn average_ends_on_its_status_with_the_running_average() {
        let mut assembler = Assembler::default();
        let frames = [
            "00 05",
            "01 00 FA 00 F0 00",
            "02 03 20 00 02 0E 50",
            "03 03 16 00 0C 5E 5C",
            "04 00 FA 00 F0 02 02",
            "00 06",
        ];
        let mut events = Vec::new();
        for data in frames {
            events.extend(assembler.on_frame(&Frame {
                func: 3,
                cmd: 1,
                data: hex(data),
            }));
        }
        assert_eq!(
            events,
            vec![Event::Measurement(Measurement {
                tds_pct: 7.90,
                refractive_index: 1.34736,
                prism_c: 25.0,
                sample_c: 24.0,
                rounds: 2,
            })]
        );
    }

    #[test]
    fn fahrenheit_is_converted() {
        assert!((celsius(791, 1) - 26.1667).abs() < 1e-3);
    }

    fn sample(mode: &str, weight_g: f64, flags: u64) -> serde_json::Value {
        serde_json::json!({ "mode": mode, "weight_g": weight_g, "flags": flags })
    }

    #[test]
    fn capture_gives_the_profile_dose_and_the_final_weight() {
        let capture = serde_json::json!({
            "schema": "coffeeflow.hf_capture.v3",
            "profile": { "dose_g": 7.5 },
            "samples": [
                sample("filling", 0.0, 0x04),
                sample("infusion", 10.0, 0x04),
                sample("infusion", 19.2, 0x04),
                sample("cooldown", 20.1, 0x04),
                sample("cooldown", 25.0, 0x00), // balance invalide
                sample("cooldown", 20.7, 0x04),
                sample("cooldown", 20.6, 0x04),
            ],
        });
        assert_eq!(
            capture_masses(&capture).unwrap(),
            CaptureMasses {
                dose_g: Some(7.5),
                final_weight_g: Some(20.7),
            }
        );
    }

    #[test]
    fn a_removed_cup_ends_the_final_weight() {
        let capture = serde_json::json!({
            "schema": "coffeeflow.hf_capture.v3",
            "samples": [
                sample("infusion", 19.0, 0x04),
                sample("cooldown", 20.2, 0x04),
                sample("cooldown", -110.0, 0x04),
                sample("cooldown", 30.0, 0x04),
            ],
        });
        let masses = capture_masses(&capture).unwrap();
        assert_eq!(masses.dose_g, None);
        assert_eq!(masses.final_weight_g, Some(20.2));
    }

    #[test]
    fn an_unknown_file_is_refused() {
        let other = serde_json::json!({ "schema": "coffeeflow.config", "samples": [] });
        assert!(capture_masses(&other).is_err());
    }

    #[test]
    fn extraction_uses_the_given_masses() {
        let m = Measurement {
            tds_pct: 7.75,
            refractive_index: 1.34676,
            prism_c: 25.1,
            sample_c: 25.1,
            rounds: 1,
        };
        assert!(describe(&m, Some(7.5), Some(20.7)).contains("extraction 21.4 %"));
        assert!(!describe(&m, None, Some(20.7)).contains("extraction"));
    }
}
