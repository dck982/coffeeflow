//! Client HTTP de l'écran : adresse, jeton et appels JSON. Les commandes ne
//! voient que [`JsonClient`], pour que leurs tests tournent sans réseau.

use std::time::Duration;

use anyhow::{Result, anyhow, bail};
use serde_json::Value;

/// Un appel JSON : méthode, chemin (avec requête éventuelle) et corps.
pub trait JsonClient {
    fn call(&mut self, method: &str, path: &str, body: Option<&Value>) -> Result<Value>;
}

/// Adresse et jeton, de la ligne de commande ou de l'environnement.
#[derive(Debug, Clone, Default)]
pub struct Connection {
    pub ip: Option<String>,
    pub token: Option<String>,
}

impl Connection {
    /// `--ip` / `--token`, sinon `COFFEEFLOW_IP` / `COFFEEFLOW_HTTP_TOKEN`.
    pub fn from_args_and_env(ip: Option<String>, token: Option<String>) -> Self {
        Self::resolve(ip, token, |name| std::env::var(name).ok())
    }

    pub fn resolve(
        ip: Option<String>,
        token: Option<String>,
        env: impl Fn(&str) -> Option<String>,
    ) -> Self {
        let pick = |arg: Option<String>, name: &str| {
            arg.or_else(|| env(name)).filter(|value| !value.is_empty())
        };
        Self {
            ip: pick(ip, "COFFEEFLOW_IP"),
            token: pick(token, "COFFEEFLOW_HTTP_TOKEN"),
        }
    }

    /// Adresse de l'écran, sans schéma ni `/` final (`192.168.2.196[:port]`).
    pub fn host(&self) -> Result<String> {
        let ip = self
            .ip
            .as_deref()
            .ok_or_else(|| anyhow!("COFFEEFLOW_IP manquante (variable d'environnement ou --ip)"))?;
        Ok(ip
            .trim_start_matches("http://")
            .trim_end_matches('/')
            .to_string())
    }

    pub fn base_url(&self) -> Result<String> {
        Ok(format!("http://{}", self.host()?))
    }

    pub fn token(&self) -> Result<&str> {
        self.token.as_deref().ok_or_else(|| {
            anyhow!("COFFEEFLOW_HTTP_TOKEN manquante (variable d'environnement ou --token)")
        })
    }

    /// Exige l'adresse et le jeton avant tout accès réseau, en nommant ce qui
    /// manque.
    pub fn client(&self) -> Result<HttpClient> {
        Ok(HttpClient::new(&self.base_url()?, self.token()?))
    }
}

pub struct HttpClient {
    agent: ureq::Agent,
    base: String,
    token: String,
}

/// Délai d'une requête : la télémétrie s'interroge à 5 Hz et ne doit pas
/// bloquer la boucle ; les gros JSON (`/config`, `/hf-capture`) ont plus de
/// temps.
fn timeout_for(path: &str) -> Duration {
    if path.starts_with("/telemetry") {
        Duration::from_secs(2)
    } else if path.starts_with("/config") || path.starts_with("/hf-capture") {
        Duration::from_secs(20)
    } else {
        Duration::from_secs(5)
    }
}

impl HttpClient {
    pub fn new(base: &str, token: &str) -> Self {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            // Le statut est examiné ici, pour joindre le corps à l'erreur.
            .http_status_as_error(false)
            .build()
            .into();
        Self {
            agent,
            base: base.trim_end_matches('/').to_string(),
            token: token.to_string(),
        }
    }

    /// Requête brute : `(statut, corps)`. `timeout` borne la requête entière.
    pub fn raw(
        &self,
        method: &str,
        path: &str,
        body: Option<(&[u8], &str)>,
        timeout: Duration,
    ) -> Result<(u16, String)> {
        let url = format!("{}{path}", self.base);
        let authorization = format!("Bearer {}", self.token);
        let transport = |e: ureq::Error| anyhow!("{method} {path}: {e}");
        let mut response = match (method, body) {
            ("GET", _) => self
                .agent
                .get(&url)
                .config()
                .timeout_global(Some(timeout))
                .build()
                .header("Authorization", &authorization)
                .header("Accept", "application/json")
                .call(),
            ("POST", Some((data, content_type))) => self
                .agent
                .post(&url)
                .config()
                .timeout_global(Some(timeout))
                .build()
                .header("Authorization", &authorization)
                .header("Accept", "application/json")
                .header("Content-Type", content_type)
                .send(data),
            ("POST", None) => self
                .agent
                .post(&url)
                .config()
                .timeout_global(Some(timeout))
                .build()
                .header("Authorization", &authorization)
                .header("Accept", "application/json")
                .send_empty(),
            _ => bail!("méthode {method} non gérée"),
        }
        .map_err(transport)?;
        let status = response.status().as_u16();
        let text = response
            .body_mut()
            .with_config()
            .limit(256 * 1024 * 1024)
            .lossy_utf8(true)
            .read_to_string()
            .map_err(transport)?;
        Ok((status, text))
    }
}

impl JsonClient for HttpClient {
    fn call(&mut self, method: &str, path: &str, body: Option<&Value>) -> Result<Value> {
        let data = body.map(|value| value.to_string().into_bytes());
        let (status, text) = self.raw(
            method,
            path,
            data.as_deref().map(|d| (d, "application/json")),
            timeout_for(path),
        )?;
        if !(200..300).contains(&status) {
            bail!("{method} {path}: HTTP {status}: {text}");
        }
        let payload: Value = serde_json::from_str(&text)
            .map_err(|e| anyhow!("{method} {path}: réponse JSON invalide : {e}"))?;
        if !payload.is_object() {
            bail!("{method} {path}: réponse JSON inattendue");
        }
        Ok(payload)
    }
}

#[cfg(test)]
pub mod fake {
    use super::*;

    type Handler = Box<dyn FnMut(&str, &str, Option<&Value>) -> Result<Value>>;

    /// Client simulé : garde les appels `(méthode, chemin, corps)`.
    pub struct FakeClient {
        pub calls: Vec<(String, String, Option<Value>)>,
        handler: Handler,
    }

    impl FakeClient {
        pub fn new(
            handler: impl FnMut(&str, &str, Option<&Value>) -> Result<Value> + 'static,
        ) -> Self {
            Self {
                calls: Vec::new(),
                handler: Box::new(handler),
            }
        }
    }

    impl JsonClient for FakeClient {
        fn call(&mut self, method: &str, path: &str, body: Option<&Value>) -> Result<Value> {
            self.calls.push((method.into(), path.into(), body.cloned()));
            (self.handler)(method, path, body)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    /// Petit serveur local : lit une requête, la renvoie au test, répond.
    fn serve_once(status: &str, body: &str) -> (String, std::thread::JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap().to_string();
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = Vec::new();
            let mut buffer = [0u8; 4096];
            loop {
                let n = stream.read(&mut buffer).unwrap();
                request.extend_from_slice(&buffer[..n]);
                let text = String::from_utf8_lossy(&request).to_string();
                if let Some(head_end) = text.find("\r\n\r\n") {
                    let length = text[..head_end]
                        .lines()
                        .find_map(|l| {
                            l.to_ascii_lowercase()
                                .strip_prefix("content-length: ")
                                .map(|v| v.trim().parse::<usize>().unwrap())
                        })
                        .unwrap_or(0);
                    if request.len() >= head_end + 4 + length {
                        break;
                    }
                }
                if n == 0 {
                    break;
                }
            }
            stream.write_all(response.as_bytes()).unwrap();
            String::from_utf8_lossy(&request).to_string()
        });
        (address, handle)
    }

    #[test]
    fn missing_settings_are_named() {
        let none = |_: &str| None;
        let connection = Connection::resolve(None, None, none);
        let error = connection.client().err().unwrap().to_string();
        assert!(error.contains("COFFEEFLOW_IP"), "{error}");
        let connection = Connection::resolve(Some("1.2.3.4".into()), None, none);
        let error = connection.client().err().unwrap().to_string();
        assert!(error.contains("COFFEEFLOW_HTTP_TOKEN"), "{error}");
        assert!(!error.contains("COFFEEFLOW_IP"), "{error}");
    }

    #[test]
    fn arguments_override_the_environment() {
        let env = |name: &str| Some(format!("env-{name}"));
        let c = Connection::resolve(Some("10.0.0.1".into()), None, env);
        assert_eq!(c.ip.as_deref(), Some("10.0.0.1"));
        assert_eq!(c.token.as_deref(), Some("env-COFFEEFLOW_HTTP_TOKEN"));
        assert_eq!(c.base_url().unwrap(), "http://10.0.0.1");
        let c = Connection::resolve(Some("http://10.0.0.1/".into()), Some("t".into()), env);
        assert_eq!(c.base_url().unwrap(), "http://10.0.0.1");
    }

    #[test]
    fn sends_the_bearer_token_and_the_json_body() {
        let (address, server) = serve_once("200 OK", "{\"version\": 3}");
        let mut client = HttpClient::new(&format!("http://{address}"), "secret");
        let body = serde_json::json!({"version": 2});
        let answer = client.call("POST", "/config", Some(&body)).unwrap();
        assert_eq!(answer["version"], 3);
        let request = server.join().unwrap();
        assert!(request.starts_with("POST /config HTTP/1.1"), "{request}");
        assert!(
            request.contains("authorization: Bearer secret"),
            "{request}"
        );
        assert!(
            request.contains("content-type: application/json"),
            "{request}"
        );
        assert!(request.ends_with("{\"version\":2}"), "{request}");
    }

    #[test]
    fn http_errors_carry_status_and_body() {
        let (address, server) = serve_once("409 Conflict", "{\"error\":\"busy\"}");
        let mut client = HttpClient::new(&format!("http://{address}"), "t");
        let error = client
            .call("GET", "/telemetry", None)
            .unwrap_err()
            .to_string();
        server.join().unwrap();
        assert_eq!(error, "GET /telemetry: HTTP 409: {\"error\":\"busy\"}");
    }

    #[test]
    fn non_object_json_is_rejected() {
        let (address, server) = serve_once("200 OK", "[1]");
        let mut client = HttpClient::new(&format!("http://{address}"), "t");
        let error = client.call("GET", "/config", None).unwrap_err().to_string();
        server.join().unwrap();
        assert_eq!(error, "GET /config: réponse JSON inattendue");
    }
}
