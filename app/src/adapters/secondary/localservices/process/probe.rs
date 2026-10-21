use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

use crate::core::application::{ProbeAnswer, ServiceProbe};
use crate::core::domain::{ServiceKind, ServiceSpec};

/// How long a connection to a port may take: loopback answers in well under a millisecond.
const CONNECT_LIMIT: Duration = Duration::from_millis(200);
/// How long a service may take to answer its health page.
const ANSWER_LIMIT: Duration = Duration::from_secs(2);

/// Asks `GET /health` on the service's loopback port; a speech recognition service must
/// also name Whisper on its start page, so another program on its port is not taken for it.
pub struct HttpProbe;

impl ServiceProbe for HttpProbe {
    fn answer(&self, spec: &ServiceSpec) -> ProbeAnswer {
        let address = SocketAddr::from(([127, 0, 0, 1], spec.port));
        if TcpStream::connect_timeout(&address, CONNECT_LIMIT).is_err() {
            return ProbeAnswer::Closed;
        }
        let agent = ureq::AgentBuilder::new()
            .timeout(ANSWER_LIMIT)
            .max_idle_connections(0)
            .build();
        let page = |path: &str| {
            agent
                .get(&format!("http://127.0.0.1:{}{path}", spec.port))
                .call()
                .ok()
                .filter(|response| response.status() == 200)
                .and_then(|response| response.into_string().ok())
        };
        if page("/health").is_none() {
            return ProbeAnswer::Busy;
        }
        let whisper = matches!(spec.kind, ServiceKind::Dictation | ServiceKind::Commands);
        if whisper && !page("/").is_some_and(|text| text.to_lowercase().contains("whisper")) {
            return ProbeAnswer::Busy;
        }
        ProbeAnswer::Ready
    }
}
