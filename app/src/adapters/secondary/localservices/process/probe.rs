use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

use crate::adapters::secondary::http::agent;
use crate::core::application::{ProbeAnswer, ServiceProbe};
use crate::core::domain::ServiceKind;

/// How long a connection to a port may take: loopback answers in well under a millisecond.
const CONNECT_LIMIT: Duration = Duration::from_millis(200);
/// How long a service may take to answer its health page.
const ANSWER_LIMIT: Duration = Duration::from_secs(2);

/// Asks `GET /health` on the service's loopback port; a speech recognition service must
/// also name Whisper on its start page, so another program on its port is not taken for it.
pub struct HttpProbe {
    agent: ureq::Agent,
}

impl HttpProbe {
    pub fn new() -> Self {
        Self { agent: agent::fresh_connections(ANSWER_LIMIT) }
    }
}

impl Default for HttpProbe {
    fn default() -> Self {
        Self::new()
    }
}

impl ServiceProbe for HttpProbe {
    fn answer(&self, kind: ServiceKind, port: u16) -> ProbeAnswer {
        let address = SocketAddr::from(([127, 0, 0, 1], port));
        if TcpStream::connect_timeout(&address, CONNECT_LIMIT).is_err() {
            return ProbeAnswer::Closed;
        }
        let page = |path: &str| {
            self.agent
                .get(&format!("http://127.0.0.1:{port}{path}"))
                .call()
                .ok()
                .filter(|response| response.status() == 200)
                .and_then(|response| response.into_string().ok())
        };
        if page("/health").is_none() {
            return ProbeAnswer::Busy;
        }
        let whisper = matches!(kind, ServiceKind::Dictation | ServiceKind::Commands);
        if whisper && !page("/").is_some_and(|text| text.to_lowercase().contains("whisper")) {
            return ProbeAnswer::Busy;
        }
        ProbeAnswer::Ready
    }
}
