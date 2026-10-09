//! En-tête HTTP borné. Les uploads OTA sont lus en flux ; la longueur du
//! corps ne provoque aucune allocation. Une ambiguïté de framing est refusée.

/// Limite commune aux tâches serveur et aux tests du parseur.
pub const HEADER_LIMIT: usize = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Incomplete,
    TooLarge,
    Invalid,
    UnsupportedTransfer,
}

#[derive(Debug, Clone, Copy)]
pub struct Request<'a> {
    pub method: &'a str,
    pub path: &'a str,
    pub query: Option<&'a str>,
    pub content_length: usize,
    pub header_bytes: usize,
    pub authorization: Option<&'a str>,
    pub websocket_key: Option<&'a str>,
    pub websocket_upgrade: bool,
}

impl<'a> Request<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self, Error> {
        let end = bytes
            .windows(4)
            .position(|b| b == b"\r\n\r\n")
            .map(|i| i + 4);
        let Some(end) = end else {
            return Err(if bytes.len() >= HEADER_LIMIT {
                Error::TooLarge
            } else {
                Error::Incomplete
            });
        };
        if end > HEADER_LIMIT {
            return Err(Error::TooLarge);
        }
        let text = core::str::from_utf8(&bytes[..end]).map_err(|_| Error::Invalid)?;
        let mut lines = text.split("\r\n");
        let mut first = lines.next().ok_or(Error::Invalid)?.split(' ');
        let method = first.next().ok_or(Error::Invalid)?;
        let target = first.next().ok_or(Error::Invalid)?;
        let version = first.next().ok_or(Error::Invalid)?;
        if !matches!(method, "GET" | "POST")
            || !target.starts_with('/')
            || !matches!(version, "HTTP/1.0" | "HTTP/1.1")
            || first.next().is_some()
        {
            return Err(Error::Invalid);
        }
        let (path, query) = target
            .split_once('?')
            .map_or((target, None), |(p, q)| (p, Some(q)));
        let mut length = None;
        let mut authorization = None;
        let mut websocket_key = None;
        let mut upgrade = false;
        let mut connection_upgrade = false;
        let mut websocket_version = false;
        for line in lines.take_while(|line| !line.is_empty()) {
            let (name, value) = line.split_once(':').ok_or(Error::Invalid)?;
            if name.is_empty()
                || name
                    .bytes()
                    .any(|b| !b.is_ascii_alphanumeric() && b != b'-')
            {
                return Err(Error::Invalid);
            }
            let value = value.trim();
            if name.eq_ignore_ascii_case("Content-Length") {
                if length.is_some()
                    || value.is_empty()
                    || !value.bytes().all(|b| b.is_ascii_digit())
                {
                    return Err(Error::Invalid);
                }
                length = Some(value.parse().map_err(|_| Error::Invalid)?);
            } else if name.eq_ignore_ascii_case("Transfer-Encoding") {
                return Err(Error::UnsupportedTransfer);
            } else if name.eq_ignore_ascii_case("Authorization") {
                if authorization.replace(value).is_some() {
                    return Err(Error::Invalid);
                }
            } else if name.eq_ignore_ascii_case("Sec-WebSocket-Key") {
                if websocket_key.replace(value).is_some() {
                    return Err(Error::Invalid);
                }
            } else if name.eq_ignore_ascii_case("Upgrade") {
                upgrade = value.eq_ignore_ascii_case("websocket");
            } else if name.eq_ignore_ascii_case("Connection") {
                connection_upgrade = value
                    .split(',')
                    .any(|s| s.trim().eq_ignore_ascii_case("upgrade"));
            } else if name.eq_ignore_ascii_case("Sec-WebSocket-Version") {
                websocket_version = value == "13";
            }
        }
        Ok(Self {
            method,
            path,
            query,
            content_length: length.unwrap_or(0),
            header_bytes: end,
            authorization,
            websocket_key,
            websocket_upgrade: upgrade && connection_upgrade && websocket_version,
        })
    }
}
