//! Deux transports de trames CAN, un seul décodeur : l'USB série cadre en
//! COBS, le WebSocket transporte le PDU nu (un message = une trame). Le reste
//! de l'outil ne manipule que des [`Frame`].

use std::collections::VecDeque;
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, anyhow, bail};
use protocol::{Frame, MAX_FRAMED_SIZE, StreamDecoder, decode_pdu, encode_framed};

/// Port série de l'écran (pont USB-série vers CAN de l'image `factory`).
pub const DEFAULT_USB_PORT: &str = "/dev/cu.usbmodem5B790235091";

pub trait Transport {
    fn send(&mut self, frame: &Frame) -> Result<()>;
    /// Trame reçue avec son horodatage Unix, ou `None` si `timeout` s'écoule
    /// sans trame.
    fn recv(&mut self, timeout: Duration) -> Result<Option<(f64, Frame)>>;
}

pub fn unix_now() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0.0, |d| d.as_secs_f64())
}

/// USB CDC, cadrage COBS + 0x00.
pub struct SerialTransport {
    port: Box<dyn serialport::SerialPort>,
    decoder: StreamDecoder,
    pending: VecDeque<Frame>,
}

impl SerialTransport {
    pub fn open(path: &str) -> Result<Self> {
        // CDC ACM ignore le débit : la valeur est cosmétique, un flash ne se
        // dimensionne pas sur ce débit.
        let port = serialport::new(path, 115_200)
            .timeout(Duration::from_millis(50))
            .open()
            .with_context(|| format!("ouverture de {path}"))?;
        Ok(Self {
            port,
            decoder: StreamDecoder::new(),
            pending: VecDeque::new(),
        })
    }
}

impl Transport for SerialTransport {
    fn send(&mut self, frame: &Frame) -> Result<()> {
        let mut out = [0u8; MAX_FRAMED_SIZE];
        let n = encode_framed(frame, &mut out).ok_or_else(|| anyhow!("trame hors gabarit"))?;
        self.port.write_all(&out[..n])?;
        self.port.flush()?;
        Ok(())
    }

    fn recv(&mut self, timeout: Duration) -> Result<Option<(f64, Frame)>> {
        let deadline = Instant::now() + timeout;
        let mut chunk = [0u8; 256];
        loop {
            if let Some(frame) = self.pending.pop_front() {
                return Ok(Some((unix_now(), frame)));
            }
            match self.port.read(&mut chunk) {
                Ok(0) => {}
                Ok(n) => {
                    for &byte in &chunk[..n] {
                        if let Some(frame) = self.decoder.push_byte(byte) {
                            self.pending.push_back(frame);
                        }
                    }
                    continue;
                }
                Err(e)
                    if matches!(
                        e.kind(),
                        std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                    ) => {}
                Err(e) => return Err(e).context("lecture du port série"),
            }
            if Instant::now() >= deadline {
                return Ok(None);
            }
        }
    }
}

/// Miroir du trafic CAN par WebSocket. Le `/ws` de l'écran est en lecture
/// seule : il ferme la session au premier message reçu du client.
pub struct WebSocketTransport {
    socket: tungstenite::WebSocket<TcpStream>,
}

impl WebSocketTransport {
    /// `host` : adresse de l'écran, avec ou sans port (80 par défaut).
    pub fn connect(host: &str, token: &str) -> Result<Self> {
        use tungstenite::client::IntoClientRequest;
        let host = host
            .trim_start_matches("http://")
            .trim_end_matches('/')
            .to_string();
        let address = if host.contains(':') {
            host.clone()
        } else {
            format!("{host}:80")
        };
        let target = address
            .to_socket_addrs()
            .with_context(|| format!("adresse {address}"))?
            .next()
            .ok_or_else(|| anyhow!("adresse {address} introuvable"))?;
        let stream = TcpStream::connect_timeout(&target, Duration::from_secs(5))
            .with_context(|| format!("connexion à {address}"))?;
        stream.set_read_timeout(Some(Duration::from_secs(5)))?;
        stream.set_nodelay(true)?;
        let mut request = format!("ws://{address}/ws").into_client_request()?;
        request
            .headers_mut()
            .insert("Authorization", format!("Bearer {token}").parse()?);
        let (socket, _response) = tungstenite::client(request, stream).map_err(|e| match e {
            tungstenite::HandshakeError::Failure(tungstenite::Error::Http(response)) => {
                anyhow!("WebSocket refusé : HTTP {}", response.status().as_u16())
            }
            other => anyhow!("poignée de main WebSocket : {other}"),
        })?;
        Ok(Self { socket })
    }
}

impl Transport for WebSocketTransport {
    fn send(&mut self, _frame: &Frame) -> Result<()> {
        bail!("le WebSocket CoffeeFlow est un miroir CAN en lecture seule (utiliser --usb)")
    }

    fn recv(&mut self, timeout: Duration) -> Result<Option<(f64, Frame)>> {
        use tungstenite::Message;
        let deadline = Instant::now() + timeout;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Ok(None);
            }
            self.socket
                .get_ref()
                .set_read_timeout(Some(remaining.max(Duration::from_millis(1))))?;
            match self.socket.read() {
                // Trafic CAN attendu en binaire uniquement ; un PDU invalide
                // s'ignore.
                Ok(Message::Binary(data)) => {
                    if let Ok(frame) = decode_pdu(&data) {
                        return Ok(Some((unix_now(), frame)));
                    }
                }
                Ok(Message::Close(_)) => bail!("connexion WebSocket fermée par l'écran"),
                Ok(_) => {}
                Err(tungstenite::Error::Io(e))
                    if matches!(
                        e.kind(),
                        std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                    ) => {}
                Err(tungstenite::Error::ConnectionClosed | tungstenite::Error::AlreadyClosed) => {
                    bail!("connexion WebSocket fermée par l'écran")
                }
                Err(e) => return Err(e).context("lecture WebSocket"),
            }
        }
    }
}

#[cfg(test)]
pub mod fake {
    use super::*;

    /// Transport simulé : garde ce qui est envoyé, rend ce qu'on lui a mis
    /// en file. `on_send` permet à un récepteur simulé de répondre.
    #[derive(Default)]
    pub struct FakeTransport {
        pub sent: Vec<Frame>,
        pub incoming: VecDeque<(f64, Frame)>,
    }

    impl Transport for FakeTransport {
        fn send(&mut self, frame: &Frame) -> Result<()> {
            self.sent.push(*frame);
            Ok(())
        }

        fn recv(&mut self, _timeout: Duration) -> Result<Option<(f64, Frame)>> {
            Ok(self.incoming.pop_front())
        }
    }
}
