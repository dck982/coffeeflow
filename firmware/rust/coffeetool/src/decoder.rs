//! Décodeur de trames : une trame CAN donne une ligne de texte horodatée,
//! sans matériel. Le texte est celui de l'ancien `decoder.py`, que les
//! enregistrements et les habitudes de lecture supposent identique.

use protocol::messages::{FlashCtrlPayload, LogPayload};
use protocol::{Frame, LogCode, LogSeverity, Message, MessageType, split_can_id};

use crate::paths::format_timestamp;

fn node_name(value: u8) -> String {
    match value {
        1 => "écran".into(),
        2 => "capteurs".into(),
        other => format!("?{other}"),
    }
}

fn dest_name(value: u8) -> String {
    match value {
        0 => "broadcast".into(),
        1 => "écran".into(),
        2 => "capteurs".into(),
        other => format!("?{other}"),
    }
}

pub fn type_name(t: MessageType) -> &'static str {
    match t {
        MessageType::Stop => "STOP",
        MessageType::Set => "SET",
        MessageType::Reset => "RESET",
        MessageType::DimmerCommand => "DIMMER_COMMAND",
        MessageType::SetHeating => "SET_HEATING",
        MessageType::ConfirmSensorsOta => "CONFIRM_SENSORS_OTA",
        MessageType::SetHeatingPower => "SET_HEATING_POWER",
        MessageType::MaintenanceValve => "MAINTENANCE_VALVE",
        MessageType::Ping => "PING",
        MessageType::Pong => "PONG",
        MessageType::ReqStatus => "REQSTATUS",
        MessageType::StatusPressure => "STATUS_PRESSURE",
        MessageType::StatusFlow => "STATUS_FLOW",
        MessageType::StatusActuators => "STATUS_ACTUATORS",
        MessageType::StatusHeating => "STATUS_HEATING",
        MessageType::Log => "LOG",
        MessageType::FlashCtrl => "FLASH_CTRL",
        MessageType::FlashData => "FLASH_DATA",
    }
}

fn boiler_adc_stage(stage: u16) -> String {
    match stage {
        0 => "bus indisponible".into(),
        1 => "ajout du périphérique".into(),
        2 => "configuration A0".into(),
        3 => "statut A0".into(),
        4 => "résultat A0".into(),
        5 => "configuration A1".into(),
        6 => "statut A1".into(),
        7 => "résultat A1".into(),
        8 => "configuration A2".into(),
        9 => "statut A2".into(),
        10 => "résultat A2".into(),
        other => format!("étape {other}"),
    }
}

fn hex(data: &[u8]) -> String {
    data.iter().map(|b| format!("{b:02x}")).collect()
}

/// `format(value, "#0<width>b")` de Python : préfixe `0b` compris dans la
/// largeur.
fn bin_flags(value: u8, width: usize) -> String {
    format!("0b{value:0w$b}", w = width.saturating_sub(2))
}

fn format_log(p: &LogPayload) -> String {
    let name = match LogCode::from_u8(p.code) {
        Some(code) => code.name().to_string(),
        None => format!("UNKNOWN({})", p.code),
    };
    let severity = match LogSeverity::from_u8(p.severity) {
        Some(s) => s.name().to_string(),
        None => format!("unknown({})", p.severity),
    };
    match name.as_str() {
        "BOILER_ADC_NOT_FOUND" => format!(
            "{name} [{severity}] adresses=0x48–0x4b dernier_esp_err={:#x}",
            p.arg32
        ),
        "BOILER_ADC_I2C_ERROR" => format!(
            "{name} [{severity}] adresse={:#04x} étape={} esp_err={:#x}",
            p.arg16 & 0xff,
            boiler_adc_stage(p.arg16 >> 8),
            p.arg32
        ),
        "BOILER_ADC_CONVERSION_TIMEOUT" => format!(
            "{name} [{severity}] adresse={:#04x} canal=A{}",
            p.arg16 & 0xff,
            p.arg16 >> 8
        ),
        "BOILER_NTC_INVALID_READING" => {
            let a0 = (p.arg32 >> 16) as u16 as i16;
            let a1 = (p.arg32 & 0xffff) as u16 as i16;
            format!(
                "{name} [{severity}] adresse={:#04x} A0_brut={a0} A1_brut={a1}",
                p.arg16
            )
        }
        "OTA_PENDING_VERIFY" => {
            let state = match p.arg32 {
                0 => "NEW".to_string(),
                1 => "PENDING_VERIFY".to_string(),
                other => format!("{other:#x}"),
            };
            format!("{name} [{severity}] etat={state}")
        }
        "BOILER_ADC_RECOVERED" => format!(
            "{name} [{severity}] adresse={:#04x} interruption={}ms",
            p.arg16, p.arg32
        ),
        _ => {
            let mut extra = Vec::new();
            if p.arg16 != 0 {
                extra.push(format!("arg16={}", p.arg16));
            }
            if p.arg32 != 0 {
                extra.push(format!("arg32={}", p.arg32));
            }
            let extra = if extra.is_empty() {
                String::new()
            } else {
                format!(" {}", extra.join(" "))
            };
            format!("{name} [{severity}]{extra}")
        }
    }
}

fn format_flash_ctrl(p: &FlashCtrlPayload) -> String {
    match *p {
        FlashCtrlPayload::Begin { image_size } => format!("BEGIN taille={image_size}"),
        FlashCtrlPayload::BlockAck {
            block_number,
            block_crc16,
        } => format!("BLOCK_ACK bloc={block_number} crc16={block_crc16:#06x}"),
        FlashCtrlPayload::BlockStart {
            block_number,
            block_crc16,
        } => format!("BLOCK_START bloc={block_number} crc16={block_crc16:#06x}"),
        FlashCtrlPayload::End { image_crc32 } => format!("END crc32={image_crc32:#010x}"),
        FlashCtrlPayload::Abort => "ABORT".to_string(),
    }
}

fn raw_data(data: &[u8]) -> String {
    if data.is_empty() {
        String::new()
    } else {
        format!("data={}", hex(data))
    }
}

fn format_payload(message_type: MessageType, frame: &Frame) -> String {
    let data = frame.payload();
    // Ces types n'ont pas de lecture dans l'ancien décodeur : octets bruts.
    if matches!(
        message_type,
        MessageType::Stop
            | MessageType::Reset
            | MessageType::FlashData
            | MessageType::DimmerCommand
            | MessageType::MaintenanceValve
    ) {
        return raw_data(data);
    }
    let message = match Message::decode(frame) {
        Ok(m) => m,
        Err(e) => return format!("(charge utile invalide: {e})"),
    };
    match message {
        Message::Log(p) => format_log(&p),
        Message::PingLegacy => "(PING ancien, sans identité)".to_string(),
        Message::Ping(p) | Message::Pong(p) => format!(
            "node={} v{}.{}.{} uptime={}s",
            node_name(p.node),
            p.version.major,
            p.version.minor,
            p.version.patch,
            p.uptime_s
        ),
        Message::Set(p) => format!(
            "pompe={}% ttl={}ms",
            p.pump_pct,
            if p.ttl_ms == 0 { 500 } else { p.ttl_ms }
        ),
        Message::SetHeating(p) => format!(
            "chauffage={} duree={}ms",
            if p.on { "on" } else { "off" },
            p.duration_ms
        ),
        Message::SetHeatingPower(p) => format!(
            "puissance_chaudiere={:.1}% bail={}ms{}",
            f64::from(p.power_permille) / 10.0,
            p.lease_ms,
            if p.restart_window {
                " nouvelle_periode"
            } else {
                ""
            }
        ),
        Message::ConfirmSensorsOta(p) => {
            format!("version_protocole_chauffage={}", p.protocol_patch)
        }
        Message::ReqStatus(p) => {
            let target = match p.target() {
                Some(t) => type_name(t).to_string(),
                None => format!("{:#x}", p.target_type),
            };
            if p.period_ms != 0 {
                format!("cible={target} periode={}ms", p.period_ms)
            } else {
                format!("cible={target} arrêt")
            }
        }
        Message::StatusPressure(p) => format!(
            "pression_brute={} temp_brute={} t={}ms flags={}",
            p.pressure_raw,
            p.temperature_raw,
            p.timestamp_ms,
            bin_flags(p.flags, 4)
        ),
        Message::StatusFlow(p) => format!(
            "impulsions={} dernier_front={}ms flags={}",
            p.pulse_count,
            p.last_edge_ms,
            bin_flags(p.flags, 4)
        ),
        Message::StatusActuators(p) => format!(
            "vanne={} pompe={}% bail_restant={}ms marche_continue={}ms flags={}",
            if p.valve_open { "ouverte" } else { "fermee" },
            p.pump_pct,
            p.lease_remaining_ms,
            p.continuous_on_ms,
            bin_flags(p.flags, 5)
        ),
        Message::StatusHeating(p) => format!(
            "chauffage={} bail_restant={}ms puissance={:.1}% capable={}{}",
            if p.heater_on { "on" } else { "off" },
            p.lease_remaining_ms,
            f64::from(p.power_permille) / 10.0,
            if p.fine_power_capable {
                "fin"
            } else {
                "diagnostic"
            },
            if p.window_restart_capable {
                " nouvelle_periode"
            } else {
                ""
            }
        ),
        Message::FlashCtrl(p) => format_flash_ctrl(&p),
        // Déjà traités plus haut.
        Message::Stop
        | Message::Reset
        | Message::FlashData(_)
        | Message::DimmerCommand(_)
        | Message::MaintenanceValve(_) => raw_data(data),
    }
}

/// Une ligne : `[HH:MM:SS.mmm] src→dest          TYPE              détail`.
pub fn decode_frame(ts: f64, frame: &Frame) -> String {
    let (type_bits, dest_bits, src_bits) = split_can_id(frame.id);
    let route = format!("{}→{}", node_name(src_bits), dest_name(dest_bits));
    let (type_name, detail) = match MessageType::from_u8(type_bits) {
        Some(t) => (type_name(t).to_string(), format_payload(t, frame)),
        None => (format!("0x{:03x}(inconnu)", frame.id), hex(frame.payload())),
    };
    let mut line = format!("[{}] {route:<18} {type_name:<17}", format_timestamp(ts));
    if !detail.is_empty() {
        line.push(' ');
        line.push_str(&detail);
    }
    line
}

#[cfg(test)]
mod tests {
    use super::*;
    use protocol::messages::{NodeIdentityPayload, SetHeatingPayload, StatusHeatingPayload};
    use protocol::{Dest, Node, Version};

    fn log(code: u8, severity: u8, arg16: u16, arg32: u32) -> String {
        let frame = Message::Log(LogPayload {
            code,
            severity,
            arg16,
            arg32,
        })
        .encode_routed(Node::Sensors, Dest::Broadcast);
        decode_frame(0.0, &frame)
    }

    #[test]
    fn pong() {
        let frame = Message::Pong(NodeIdentityPayload::new(
            Node::Sensors,
            Version::new(0, 1, 0),
            42,
        ))
        .encode_routed(Node::Sensors, Dest::Screen);
        let line = decode_frame(0.0, &frame);
        assert!(line.contains("PONG"), "{line}");
        assert!(line.contains("v0.1.0"), "{line}");
        assert!(line.contains("uptime=42s"), "{line}");
        assert!(line.contains("node=capteurs"), "{line}");
        assert!(line.contains("capteurs→écran"), "{line}");
    }

    #[test]
    fn log_uses_generated_code_names() {
        let line = log(6, 3, 0, 60000);
        assert!(line.contains("RUNTIME_LOCKOUT_TRIGGERED"), "{line}");
        assert!(line.contains("[error]"), "{line}");
        assert!(line.contains("arg32=60000"), "{line}");
    }

    #[test]
    fn unknown_log_code_and_severity() {
        let line = log(250, 9, 0, 0);
        assert!(line.contains("UNKNOWN(250) [unknown(9)]"), "{line}");
    }

    #[test]
    fn boiler_adc_diagnostics() {
        assert!(log(40, 3, 0, 0x107).contains("dernier_esp_err=0x107"));
        let i2c = log(41, 3, (3 << 8) | 0x48, 0x107);
        assert!(i2c.contains("BOILER_ADC_I2C_ERROR"), "{i2c}");
        assert!(i2c.contains("adresse=0x48"), "{i2c}");
        assert!(i2c.contains("statut A0"), "{i2c}");
        assert!(i2c.contains("esp_err=0x107"), "{i2c}");
        assert!(log(42, 3, (1 << 8) | 0x48, 0).contains("canal=A1"));
        let invalid = log(43, 3, 0x48, (26485 << 16) | 1050);
        assert!(invalid.contains("A0_brut=26485"), "{invalid}");
        assert!(invalid.contains("A1_brut=1050"), "{invalid}");
        let negative = log(43, 3, 0x48, (0xFFFFu32 << 16) | 0x8000);
        assert!(negative.contains("A0_brut=-1 A1_brut=-32768"), "{negative}");
        assert!(log(44, 3, 0x48, 1000).contains("interruption=1000ms"));
    }

    #[test]
    fn ota_pending_verify_state() {
        let new = log(20, 1, 0, 0);
        assert!(new.contains("OTA_PENDING_VERIFY"), "{new}");
        assert!(new.contains("etat=NEW"), "{new}");
        assert!(log(20, 1, 0, 1).contains("etat=PENDING_VERIFY"));
        assert!(log(20, 1, 0, 7).contains("etat=0x7"));
    }

    #[test]
    fn unknown_type_does_not_crash() {
        let frame = Frame::new(0x7FF, &[0xff; 8]).unwrap();
        let line = decode_frame(0.0, &frame);
        assert!(line.contains("0x7ff(inconnu)"), "{line}");
        assert!(line.ends_with(" ffffffffffffffff"), "{line}");
        assert!(line.contains("?3→?3") || line.contains("?7"), "{line}");
    }

    #[test]
    fn heating_and_confirmation_frames() {
        let short = Frame::new(0x46A, &StatusHeatingPayload::default().pack()[..4]).unwrap();
        let heating = decode_frame(0.0, &short);
        assert!(heating.contains("STATUS_HEATING"), "{heating}");
        assert!(heating.contains("chauffage=off"), "{heating}");
        assert!(heating.contains("capable=diagnostic"), "{heating}");

        let confirmation = decode_frame(0.0, &Frame::new(0x0B1, &[0x3f]).unwrap());
        assert!(
            confirmation.contains("CONFIRM_SENSORS_OTA"),
            "{confirmation}"
        );
        assert!(
            confirmation.contains("version_protocole_chauffage=63"),
            "{confirmation}"
        );

        let request = decode_frame(0.0, &Frame::new(0x211, &[0x23, 0xfa, 0x00]).unwrap());
        assert!(request.contains("cible=STATUS_HEATING"), "{request}");
        assert!(request.contains("periode=250ms"), "{request}");
    }

    #[test]
    fn text_layout_matches_the_python_decoder() {
        let frame = Message::SetHeating(SetHeatingPayload {
            on: true,
            duration_ms: 2000,
        })
        .encode();
        let line = decode_frame(0.0, &frame);
        // Après l'heure : route sur 18 colonnes, type sur 17, détail.
        let body = line.split_once("] ").unwrap().1;
        assert_eq!(
            body,
            "écran→capteurs     SET_HEATING       chauffage=on duree=2000ms"
        );
    }

    #[test]
    fn flags_use_python_binary_format() {
        assert_eq!(bin_flags(0, 4), "0b00");
        assert_eq!(bin_flags(1, 4), "0b01");
        assert_eq!(bin_flags(0b110, 5), "0b110");
        assert_eq!(bin_flags(0b101, 4), "0b101");
    }

    #[test]
    fn invalid_payload_is_reported() {
        let line = decode_frame(0.0, &Frame::new(0x01 << 5 | 0x08 | 1, &[1]).unwrap());
        assert!(line.contains("charge utile invalide"), "{line}");
    }

    #[test]
    fn legacy_ping_and_flash() {
        let line = decode_frame(0.0, &Frame::new(0x08 << 5 | 1, &[]).unwrap());
        assert!(line.contains("(PING ancien, sans identité)"), "{line}");
        let begin = Message::FlashCtrl(FlashCtrlPayload::Begin { image_size: 1024 }).encode();
        assert!(decode_frame(0.0, &begin).contains("BEGIN taille=1024"));
        let end = Message::FlashCtrl(FlashCtrlPayload::End {
            image_crc32: 0xDEADBEEF,
        })
        .encode();
        assert!(decode_frame(0.0, &end).contains("END crc32=0xdeadbeef"));
    }
}
