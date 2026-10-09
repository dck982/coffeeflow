//! Enregistrement et relecture de trames. Le fichier est du JSON Lines, une
//! trame par ligne :
//!
//! ```text
//! {"t": <secondes depuis la première trame>, "id": <entier>, "data": "<hex>"}
//! ```
//!
//! Le `t` relatif permet de rejouer un shot à son rythme d'origine quelle que
//! soit la date de l'enregistrement ; le format est celui de l'ancien
//! `coffeetool record`, que les fichiers déjà enregistrés partagent.

use std::io::Write;
use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use protocol::Frame;
use serde::Deserialize;

use crate::runtime::Runtime;
use crate::transport::Transport;

#[derive(Deserialize)]
struct Entry {
    t: f64,
    id: u16,
    data: String,
}

fn hex(data: &[u8]) -> String {
    data.iter().map(|b| format!("{b:02x}")).collect()
}

fn unhex(text: &str) -> Result<Vec<u8>> {
    if !text.len().is_multiple_of(2) || !text.is_ascii() {
        return Err(anyhow!("données hexadécimales invalides : {text:?}"));
    }
    (0..text.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&text[i..i + 2], 16)
                .map_err(|_| anyhow!("données hexadécimales invalides : {text:?}"))
        })
        .collect()
}

/// La ligne d'un enregistrement, avec les espaces de `json.dumps`.
pub fn entry_line(t: f64, frame: &Frame) -> String {
    let t = serde_json::Number::from_f64(t).map_or_else(|| "0.0".to_string(), |n| n.to_string());
    format!(
        "{{\"t\": {t}, \"id\": {}, \"data\": \"{}\"}}",
        frame.id,
        hex(frame.payload())
    )
}

/// Enregistre jusqu'à ce que `stop` réponde vrai (Ctrl-C). `on_frame` voit
/// chaque trame, pour l'afficher.
pub fn record(
    transport: &mut dyn Transport,
    path: &Path,
    mut on_frame: impl FnMut(f64, &Frame),
    stop: &dyn Fn() -> bool,
) -> Result<()> {
    let mut file =
        std::fs::File::create(path).with_context(|| format!("création de {}", path.display()))?;
    let mut start: Option<f64> = None;
    while !stop() {
        let Some((ts, frame)) = transport.recv(Duration::from_secs(1))? else {
            continue;
        };
        let origin = *start.get_or_insert(ts);
        writeln!(file, "{}", entry_line(ts - origin, &frame))?;
        file.flush()?;
        on_frame(ts, &frame);
    }
    Ok(())
}

/// Relit un fichier enregistré, sans matériel : `(t relatif, trame)`.
pub fn replay(path: &Path) -> Result<Vec<(f64, Frame)>> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("lecture de {}", path.display()))?;
    let mut out = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let entry: Entry = serde_json::from_str(line)
            .with_context(|| format!("{} ligne {}", path.display(), index + 1))?;
        let data = unhex(&entry.data)?;
        let frame = Frame::new(entry.id, &data)
            .ok_or_else(|| anyhow!("{} ligne {} : plus de 8 octets", path.display(), index + 1))?;
        out.push((entry.t, frame));
    }
    Ok(out)
}

/// Rejoue un enregistrement sur un vrai transport, au rythme d'origine par
/// défaut.
pub fn replay_to_transport(
    path: &Path,
    transport: &mut dyn Transport,
    realtime: bool,
    runtime: &dyn Runtime,
) -> Result<()> {
    let mut previous = 0.0;
    for (t, frame) in replay(path)? {
        if realtime {
            runtime.sleep(t - previous);
            previous = t;
        }
        transport.send(&frame)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::fake::FakeRuntime;
    use crate::transport::fake::FakeTransport;
    use protocol::{Dest, MessageType, Node};

    #[test]
    fn replay_roundtrip_of_a_python_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("shot.jsonl");
        let id = protocol::encode_can_id(protocol::CanId {
            message_type: MessageType::StatusFlow,
            dest: Dest::Screen,
            src: Node::Sensors,
        });
        // Octet pour octet ce qu'écrivait `recorder.py`.
        std::fs::write(
            &path,
            format!("{{\"t\": 0.0, \"id\": {id}, \"data\": \"01020304050600\"}}\n\n"),
        )
        .unwrap();
        let entries = replay(&path).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].0, 0.0);
        assert_eq!(
            entries[0].1,
            Frame::new(id, &[1, 2, 3, 4, 5, 6, 0]).unwrap()
        );
    }

    #[test]
    fn record_writes_the_python_line_format() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.jsonl");
        let mut transport = FakeTransport::default();
        transport
            .incoming
            .push_back((100.0, Frame::new(0x2A1, &[0xde, 0xad]).unwrap()));
        transport
            .incoming
            .push_back((100.25, Frame::new(0x2A2, &[]).unwrap()));
        let seen = std::cell::Cell::new(0);
        let mut shown = 0;
        record(&mut transport, &path, |_, _| shown += 1, &|| {
            seen.set(seen.get() + 1);
            seen.get() > 2
        })
        .unwrap();
        assert_eq!(shown, 2);
        let text = std::fs::read_to_string(&path).unwrap();
        assert_eq!(
            text,
            "{\"t\": 0.0, \"id\": 673, \"data\": \"dead\"}\n{\"t\": 0.25, \"id\": 674, \"data\": \"\"}\n"
        );
        // Et `replay` relit ce que `record` écrit.
        let back = replay(&path).unwrap();
        assert_eq!(back.len(), 2);
        assert_eq!(back[1].0, 0.25);
        assert_eq!(back[0].1.payload(), &[0xde, 0xad]);
    }

    #[test]
    fn replay_to_transport_sends_in_order_and_keeps_the_rhythm() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("shot.jsonl");
        std::fs::write(
            &path,
            "{\"t\": 0.0, \"id\": 1, \"data\": \"00\"}\n{\"t\": 0.01, \"id\": 2, \"data\": \"01\"}\n",
        )
        .unwrap();
        let mut transport = FakeTransport::default();
        let runtime = FakeRuntime::new();
        replay_to_transport(&path, &mut transport, false, &runtime).unwrap();
        assert_eq!(
            transport.sent,
            vec![Frame::new(1, &[0]).unwrap(), Frame::new(2, &[1]).unwrap()]
        );
        assert_eq!(runtime.now.get(), 0.0);

        let mut transport = FakeTransport::default();
        replay_to_transport(&path, &mut transport, true, &runtime).unwrap();
        assert!((runtime.now.get() - 0.01).abs() < 1e-9);
    }

    #[test]
    fn bad_lines_are_reported() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad.jsonl");
        std::fs::write(&path, "{\"t\": 0, \"id\": 1, \"data\": \"zz\"}\n").unwrap();
        assert!(replay(&path).is_err());
    }
}
