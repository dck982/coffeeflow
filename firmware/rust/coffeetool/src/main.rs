//! `coffeetool` : l'outil Mac de coffeeflow. Il remplace l'ancien
//! `coffeetool` Python et les scripts `confirm_fw.sh`, `record_probe.py`,
//! `record_heating.py`, `save_config.py`, `restore_config.py`,
//! `set_heating.py`, `set_scace.py`, `download_hf_capture.py` et `purge.py`.
//!
//! Le transport par défaut est HTTP (`COFFEEFLOW_IP`, `COFFEEFLOW_HTTP_TOKEN`
//! ou `--ip`, `--token`). `--usb[=PORT]` passe par le port série, pour les
//! seules commandes qui ont un sens sur le bus CAN.

mod decoder;
mod flash;
mod http;
mod paths;
mod profile;
mod purge;
mod record;
mod recorder;
mod runtime;
mod scace;
mod settings;
mod transport;

use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use clap::{Args, Parser, Subcommand, ValueEnum};
use protocol::messages::{NodeIdentityPayload, ReqStatusPayload, SetPayload};
use protocol::{Dest, Message, MessageType, Node, Version};

use crate::http::{Connection, JsonClient};
use crate::runtime::{Interrupted, SystemRuntime};
use crate::transport::{DEFAULT_USB_PORT, SerialTransport, Transport, WebSocketTransport};

#[derive(Parser)]
#[command(name = "coffeetool", version, about = "Outil Mac de coffeeflow")]
struct Cli {
    /// Adresse de l'écran (sinon COFFEEFLOW_IP)
    #[arg(long, global = true)]
    ip: Option<String>,
    /// Jeton HTTP (sinon COFFEEFLOW_HTTP_TOKEN)
    #[arg(long, global = true)]
    token: Option<String>,
    #[command(subcommand)]
    command: Command,
}

/// `--usb[=PORT]` : le signe égal est obligatoire pour que `--usb image.bin`
/// ne prenne pas le fichier pour un port.
#[derive(Args, Clone)]
struct Usb {
    /// Passe par le port série (pont USB-série vers CAN) au lieu de HTTP
    #[arg(
        long,
        value_name = "PORT",
        num_args = 0..=1,
        require_equals = true,
        default_missing_value = DEFAULT_USB_PORT
    )]
    usb: Option<String>,
}

#[derive(Subcommand)]
enum Command {
    /// Flashe une image : POST /firmware, ou FLASH_CTRL/FLASH_DATA avec --usb
    Flash {
        image: PathBuf,
        #[arg(long, value_enum)]
        target: Target,
        #[command(flatten)]
        usb: Usb,
    },
    /// Confirme le firmware en essai : POST /firmware/confirm
    Confirm,
    /// Décode le flux CAN en direct (WebSocket /ws, ou série avec --usb)
    Monitor {
        /// Enregistre aussi le flux dans ce fichier (JSON Lines, relu par replay)
        #[arg(long)]
        out: Option<PathBuf>,
        #[command(flatten)]
        usb: Usb,
    },
    /// Envoie un message CAN à la main
    Send {
        message: SendMessage,
        #[arg(long, value_enum, default_value = "screen")]
        src: NodeArg,
        #[arg(long, value_enum, default_value = "broadcast")]
        dest: DestArg,
        /// SET : niveau de pompe, 0 à 100
        #[arg(long, default_value_t = 0, value_parser = clap::value_parser!(u8).range(0..=100))]
        pump: u8,
        /// SET : 0 = défaut (500 ms)
        #[arg(long, default_value_t = 0)]
        ttl_ms: u16,
        /// REQSTATUS : message demandé
        #[arg(long, value_enum, default_value = "status_pressure")]
        target: StatusTarget,
        /// REQSTATUS : 0 = arrêt
        #[arg(long, default_value_t = 0)]
        period_ms: u16,
        #[command(flatten)]
        usb: Usb,
    },
    /// Relit un fichier enregistré (le décode, ou le rejoue sur le bus avec --send / --usb)
    Replay {
        file: PathBuf,
        /// Envoie les trames au lieu de les afficher (--usb l'implique)
        #[arg(long)]
        send: bool,
        /// Sans respecter le rythme d'origine
        #[arg(long)]
        fast: bool,
        #[command(flatten)]
        usb: Usb,
    },
    /// GET /telemetry, ou son enregistrement dans captures/ avec --record
    Telemetry {
        #[arg(long)]
        record: bool,
        /// Fréquence d'interrogation en Hz (défaut : 5)
        #[arg(long, requires = "record", default_value_t = record::DEFAULT_RATE_HZ)]
        rate: f64,
        /// Durée en secondes (défaut : jusqu'à Ctrl-C)
        #[arg(long, requires = "record")]
        duration: Option<f64>,
        /// Lit aussi la sonde SCACE en BLE depuis le Mac
        #[arg(long, requires = "record")]
        scace: bool,
        /// Adresse BLE de la sonde (sinon recherche par service)
        #[arg(long, requires = "scace")]
        scace_address: Option<String>,
        /// Fichier de sortie (défaut : captures/probe-<date>.json)
        #[arg(long, requires = "record")]
        output: Option<PathBuf>,
    },
    /// Sauvegarde ou restaure la configuration
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    /// Active ou désactive un réglage de /config
    Set {
        #[arg(value_parser = clap::builder::PossibleValuesParser::new(
            settings::SETTINGS.iter().map(|s| s.name)))]
        setting: String,
        #[arg(value_enum)]
        state: OnOff,
    },
    /// Liste, crée, renomme, modifie, supprime ou choisit les profils d'infusion
    Profile {
        #[command(subcommand)]
        action: profile::ProfileAction,
    },
    /// Télécharge la dernière capture haute fréquence dans captures/
    DownloadHf,
    /// Purge Wi-Fi de la durée donnée (60 s au plus, exacte si multiple de 5)
    Purge { seconds: f64 },
}

#[derive(Subcommand)]
enum ConfigAction {
    /// GET /config, écrit dans configs/ (ou dans le fichier donné)
    Save { file: Option<PathBuf> },
    /// POST /config avec le contenu du fichier
    Restore { file: PathBuf },
}

#[derive(Clone, Copy, ValueEnum)]
enum Target {
    Screen,
    Sensors,
}

#[derive(Clone, Copy, ValueEnum)]
enum OnOff {
    On,
    Off,
}

#[derive(Clone, Copy, ValueEnum)]
enum SendMessage {
    Ping,
    Pong,
    Stop,
    Reset,
    Set,
    Reqstatus,
}

#[derive(Clone, Copy, ValueEnum)]
enum NodeArg {
    Screen,
    Sensors,
}

#[derive(Clone, Copy, ValueEnum)]
enum DestArg {
    Broadcast,
    Screen,
    Sensors,
}

#[derive(Clone, Copy, ValueEnum)]
enum StatusTarget {
    #[value(name = "status_pressure")]
    Pressure,
    #[value(name = "status_flow")]
    Flow,
    #[value(name = "status_actuators")]
    Actuators,
    #[value(name = "status_heating")]
    Heating,
}

impl Target {
    fn name(self) -> &'static str {
        match self {
            Self::Screen => "screen",
            Self::Sensors => "sensors",
        }
    }
}

/// Transport de trames CAN : série avec --usb, WebSocket sinon.
fn open_transport(connection: &Connection, usb: &Usb) -> Result<Box<dyn Transport>> {
    if let Some(port) = &usb.usb {
        return Ok(Box::new(SerialTransport::open(port)?));
    }
    Ok(Box::new(WebSocketTransport::connect(
        &connection.host()?,
        connection.token()?,
    )?))
}

fn cmd_monitor(connection: &Connection, out: Option<PathBuf>, usb: &Usb) -> Result<()> {
    runtime::install_ctrlc();
    let mut transport = open_transport(connection, usb)?;
    let print = |ts: f64, frame: &protocol::Frame| {
        println!("{}", decoder::decode_frame(ts, frame));
    };
    match out {
        Some(path) => {
            println!(
                "enregistrement vers {} — Ctrl-C pour arrêter",
                path.display()
            );
            recorder::record(transport.as_mut(), &path, print, &runtime::interrupted)?;
            println!("\narrêté");
        }
        None => {
            while !runtime::interrupted() {
                if let Some((ts, frame)) = transport.recv(Duration::from_secs(1))? {
                    print(ts, &frame);
                }
            }
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn cmd_send(
    connection: &Connection,
    usb: &Usb,
    message: SendMessage,
    src: NodeArg,
    dest: DestArg,
    pump: u8,
    ttl_ms: u16,
    target: StatusTarget,
    period_ms: u16,
) -> Result<()> {
    let src = match src {
        NodeArg::Screen => Node::Screen,
        NodeArg::Sensors => Node::Sensors,
    };
    let dest = match dest {
        DestArg::Broadcast => Dest::Broadcast,
        DestArg::Screen => Dest::Screen,
        DestArg::Sensors => Dest::Sensors,
    };
    // Identité factice des PING/PONG envoyés à la main, comme l'ancien outil.
    let identity = NodeIdentityPayload::new(src, Version::new(0, 1, 0), 0);
    let (name, message) = match message {
        SendMessage::Ping => ("ping", Message::Ping(identity)),
        SendMessage::Pong => ("pong", Message::Pong(identity)),
        SendMessage::Stop => ("stop", Message::Stop),
        SendMessage::Reset => ("reset", Message::Reset),
        SendMessage::Set => (
            "set",
            Message::Set(SetPayload {
                pump_pct: pump,
                ttl_ms,
            }),
        ),
        SendMessage::Reqstatus => {
            let target = match target {
                StatusTarget::Pressure => MessageType::StatusPressure,
                StatusTarget::Flow => MessageType::StatusFlow,
                StatusTarget::Actuators => MessageType::StatusActuators,
                StatusTarget::Heating => MessageType::StatusHeating,
            };
            (
                "reqstatus",
                Message::ReqStatus(ReqStatusPayload::new(target, period_ms)),
            )
        }
    };
    let frame = message.encode_routed(src, dest);
    let mut transport = open_transport(connection, usb)?;
    transport.send(&frame)?;
    let data: String = frame.payload().iter().map(|b| format!("{b:02x}")).collect();
    println!("envoyé {name} id={:#05x} data={data}", frame.id);
    Ok(())
}

fn cmd_replay(
    connection: &Connection,
    file: PathBuf,
    send: bool,
    fast: bool,
    usb: &Usb,
) -> Result<()> {
    if send || usb.usb.is_some() {
        let mut transport = open_transport(connection, usb)?;
        recorder::replay_to_transport(&file, transport.as_mut(), !fast, &SystemRuntime::new())?;
        println!("relecture envoyée");
        return Ok(());
    }
    for (t, frame) in recorder::replay(&file)? {
        println!("{}", decoder::decode_frame(t, &frame));
    }
    Ok(())
}

fn cmd_flash(connection: &Connection, image: PathBuf, target: Target, usb: &Usb) -> Result<bool> {
    let data = std::fs::read(&image).with_context(|| format!("lecture de {}", image.display()))?;
    if usb.usb.is_none() {
        let client = connection.client()?;
        let path = format!("/firmware?target={}", target.name());
        let result = client.raw(
            "POST",
            &path,
            Some((&data, "application/octet-stream")),
            Duration::from_secs(120),
        );
        return match result {
            Ok((status, body)) if (200..300).contains(&status) => {
                println!("{body}");
                println!("upload terminé");
                Ok(true)
            }
            Ok((status, body)) => {
                eprintln!("échec: HTTP {status}: {body}");
                Ok(false)
            }
            Err(error) => {
                eprintln!("échec: {error:#}");
                Ok(false)
            }
        };
    }
    // Les capteurs seuls se flashent par le bus : l'écran est le pont.
    if matches!(target, Target::Screen) {
        bail!("l'écran se flashe par HTTP ; --usb ne flashe que --target sensors");
    }
    let mut transport = open_transport(connection, usb)?;
    let options = flash::FlashOptions::default();
    let outcome = flash::flash(transport.as_mut(), &data, &options, |p| {
        let pct = 100.0 * p.bytes_sent as f64 / p.image_size as f64;
        print!("\rbloc {}/{} ({pct:5.1}%)", p.block_number, p.block_count);
        let _ = std::io::stdout().flush();
    });
    match outcome {
        Ok(()) => {
            println!("\nfait");
            Ok(true)
        }
        Err(error) => {
            println!();
            eprintln!("échec: {error}");
            Ok(false)
        }
    }
}

fn cmd_confirm(connection: &Connection) -> Result<bool> {
    let client = connection.client()?;
    let (status, body) = client.raw("POST", "/firmware/confirm", None, Duration::from_secs(20))?;
    if (200..300).contains(&status) {
        println!("{body}");
        Ok(true)
    } else {
        eprintln!("erreur: POST /firmware/confirm: HTTP {status}: {body}");
        Ok(false)
    }
}

#[allow(clippy::too_many_arguments)]
fn cmd_telemetry(
    connection: &Connection,
    record: bool,
    rate: f64,
    duration: Option<f64>,
    scace: bool,
    scace_address: Option<String>,
    output: Option<PathBuf>,
) -> Result<bool> {
    let mut client = connection.client()?;
    if !record {
        let telemetry = client.call("GET", "/telemetry", None)?;
        println!("{}", serde_json::to_string_pretty(&telemetry)?);
        return Ok(true);
    }
    runtime::install_ctrlc();
    let output = output.unwrap_or_else(|| {
        paths::captures_dir().join(format!("probe-{}.json", paths::long_stamp()))
    });
    let latest = scace.then(|| {
        let latest = Arc::new(scace::LatestTemperature::default());
        scace::follow_in_background(Arc::clone(&latest), scace_address);
        latest
    });
    let options = record::RecordOptions {
        rate_hz: rate,
        duration_s: duration,
    };
    let capture = record::record_telemetry(
        &output,
        &mut client,
        &SystemRuntime::new(),
        &options,
        latest.as_deref().map(|l| l as &dyn record::ScaceSource),
        &mut |line| {
            println!("{line}");
            let _ = std::io::stdout().flush();
        },
    )
    .map_err(|e| anyhow::anyhow!("erreur du relevé : {e:#}"))?;
    let count = capture["samples"].as_array().map_or(0, Vec::len);
    let reason = capture["stop_reason"].as_str().unwrap_or("?");
    println!(
        "relevé écrit dans {} ({count} mesures, {reason})",
        output.display()
    );
    if reason == "error" {
        eprintln!(
            "erreur : {}",
            capture["error"].as_str().unwrap_or("inconnue")
        );
        return Ok(false);
    }
    // Ctrl-C est la façon normale d'arrêter cet enregistrement.
    Ok(true)
}

fn run(cli: Cli) -> Result<bool> {
    let connection = Connection::from_args_and_env(cli.ip, cli.token);
    match cli.command {
        Command::Flash { image, target, usb } => {
            return cmd_flash(&connection, image, target, &usb);
        }
        Command::Confirm => return cmd_confirm(&connection),
        Command::Monitor { out, usb } => cmd_monitor(&connection, out, &usb)?,
        Command::Send {
            message,
            src,
            dest,
            pump,
            ttl_ms,
            target,
            period_ms,
            usb,
        } => cmd_send(
            &connection,
            &usb,
            message,
            src,
            dest,
            pump,
            ttl_ms,
            target,
            period_ms,
        )?,
        Command::Replay {
            file,
            send,
            fast,
            usb,
        } => {
            cmd_replay(&connection, file, send, fast, &usb)?;
        }
        Command::Telemetry {
            record,
            rate,
            duration,
            scace,
            scace_address,
            output,
        } => {
            return cmd_telemetry(
                &connection,
                record,
                rate,
                duration,
                scace,
                scace_address,
                output,
            );
        }
        Command::Config { action } => match action {
            ConfigAction::Save { file } => {
                let mut client = connection.client()?;
                let path = settings::save_config(&mut client, file, &paths::configs_dir())?;
                println!("configuration écrite dans {}", path.display());
            }
            ConfigAction::Restore { file } => {
                let config = settings::read_config(&file)?;
                let mut client = connection.client()?;
                let version = settings::restore_config(&mut client, &config)?;
                println!("configuration restaurée (version {version})");
            }
        },
        Command::Set { setting, state } => {
            let enabled = matches!(state, OnOff::On);
            let setting = settings::setting(&setting).expect("validé par clap");
            let mut client = connection.client()?;
            settings::set_bool(setting, enabled, &mut client)?;
            println!("{}.{}={enabled} confirmé", setting.section, setting.key);
        }
        Command::Profile { action } => {
            let mut client = connection.client()?;
            print!("{}", profile::run(&action, &mut client)?);
        }
        Command::DownloadHf => {
            let mut client = connection.client()?;
            let path = settings::download_hf(&mut client, &paths::captures_dir())?;
            println!("capture écrite dans {}", path.display());
        }
        Command::Purge { seconds } => {
            runtime::install_ctrlc();
            let mut client = connection.client()?;
            purge::purge(seconds, &mut client, &SystemRuntime::new())?;
        }
    }
    Ok(true)
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(error) if error.downcast_ref::<Interrupted>().is_some() => ExitCode::from(130),
        Err(error) => {
            eprintln!("erreur: {error:#}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
        Cli::try_parse_from(std::iter::once("coffeetool").chain(args.iter().copied()))
    }

    #[test]
    fn cli_definition_is_consistent() {
        Cli::command().debug_assert();
    }

    #[test]
    fn usb_takes_a_port_only_with_an_equals_sign() {
        let cli = parse(&["flash", "--usb", "image.bin", "--target", "sensors"]).unwrap();
        let Command::Flash { image, usb, .. } = cli.command else {
            panic!("flash attendu");
        };
        assert_eq!(image, PathBuf::from("image.bin"));
        assert_eq!(usb.usb.as_deref(), Some(DEFAULT_USB_PORT));

        let cli = parse(&["monitor", "--usb=/dev/cu.test"]).unwrap();
        let Command::Monitor { usb, .. } = cli.command else {
            panic!("monitor attendu");
        };
        assert_eq!(usb.usb.as_deref(), Some("/dev/cu.test"));
    }

    #[test]
    fn usb_is_refused_by_http_only_commands() {
        for args in [
            &["confirm", "--usb"][..],
            &["telemetry", "--usb"],
            &["purge", "5", "--usb"],
            &["set", "heating", "on", "--usb"],
            &["download-hf", "--usb"],
            &["config", "save", "--usb"],
            &["profile", "list", "--usb"],
        ] {
            assert!(parse(args).is_err(), "{args:?}");
        }
    }

    #[test]
    fn record_options_need_record() {
        assert!(parse(&["telemetry", "--rate", "2"]).is_err());
        assert!(parse(&["telemetry", "--scace"]).is_err());
        assert!(
            parse(&[
                "telemetry",
                "--record",
                "--rate",
                "2",
                "--duration",
                "30",
                "--scace"
            ])
            .is_ok()
        );
    }

    #[test]
    fn set_accepts_only_known_settings() {
        assert!(parse(&["set", "heating", "on"]).is_ok());
        assert!(parse(&["set", "scace", "off"]).is_ok());
        assert!(parse(&["set", "pump", "on"]).is_err());
        assert!(parse(&["set", "heating", "maybe"]).is_err());
    }

    #[test]
    fn profile_subcommands_parse_their_options() {
        assert!(parse(&["profile", "list"]).is_ok());
        assert!(parse(&["profile", "add"]).is_err());
        let cli = parse(&[
            "profile",
            "add",
            "lungo",
            "--from",
            "single espresso",
            "--dose",
            "0",
            "--basket",
            "",
        ])
        .unwrap();
        let Command::Profile { action } = cli.command else {
            panic!("profile attendu");
        };
        assert_eq!(
            action,
            profile::ProfileAction::Add {
                name: "lungo".into(),
                from: Some("single espresso".into()),
                dose: Some(0.0),
                basket: Some(String::new()),
            }
        );
        assert!(parse(&["profile", "rename", "a"]).is_err());
        assert!(parse(&["profile", "rename", "a", "b"]).is_ok());
        assert!(parse(&["profile", "set", "a", "--dose", "9"]).is_ok());
        assert!(parse(&["profile", "rm", "a"]).is_ok());
        assert!(parse(&["profile", "use", "a"]).is_ok());
    }

    #[test]
    fn flash_needs_an_explicit_target() {
        assert!(parse(&["flash", "image.bin"]).is_err());
        assert!(parse(&["flash", "image.bin", "--target", "screen"]).is_ok());
        assert!(parse(&["flash", "image.bin", "--target", "other"]).is_err());
    }
}
