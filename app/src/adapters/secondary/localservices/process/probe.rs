use std::net::{SocketAddr, TcpStream};
use std::time::{Duration, Instant};

use crate::adapters::secondary::http::agent;
use crate::core::application::{ProbeAnswer, ServiceProbe};
use crate::core::domain::ServiceKind;

/// How long a connection to a port may take: loopback answers in well under a millisecond.
const CONNECT_LIMIT: Duration = Duration::from_millis(200);
/// How long a service may take to answer its health page.
const ANSWER_LIMIT: Duration = Duration::from_secs(2);
/// How long a copy may take to transcribe its warm-up: its first decode builds its Metal
/// pipelines, seconds on a busy machine.
const WARM_UP_LIMIT: Duration = Duration::from_secs(30);
/// A short utterance ("hello there", 16 kHz mono), the same one the service scripts send.
const WARM_UP_WAV: &[u8] = include_bytes!("../../../../../assets/warm-up.wav");
const WARM_UP_BOUNDARY: &str = "voicepuncher-warm-up-5c81e2";

/// Asks `GET /health` on the service's loopback port; a speech recognition service must
/// also name Whisper on its start page, so another program on its port is not taken for it.
pub struct HttpProbe {
    agent: ureq::Agent,
    warm_up_agent: ureq::Agent,
}

impl HttpProbe {
    pub fn new() -> Self {
        Self {
            agent: agent::fresh_connections(ANSWER_LIMIT),
            warm_up_agent: agent::fresh_connections(WARM_UP_LIMIT),
        }
    }
}

/// The warm-up utterance as a transcription request's multipart body.
fn warm_up_body() -> Vec<u8> {
    let mut body = format!(
        "--{WARM_UP_BOUNDARY}\r\nContent-Disposition: form-data; name=\"response_format\"\r\n\r\njson\r\n\
         --{WARM_UP_BOUNDARY}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"warm-up.wav\"\r\n\
         Content-Type: audio/wav\r\n\r\n"
    )
    .into_bytes();
    body.extend_from_slice(WARM_UP_WAV);
    body.extend_from_slice(format!("\r\n--{WARM_UP_BOUNDARY}--\r\n").as_bytes());
    body
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

    fn warm_up(&self, kind: ServiceKind, port: u16) -> Option<Duration> {
        if !matches!(kind, ServiceKind::Dictation | ServiceKind::Commands) {
            return Some(Duration::ZERO);
        }
        let began = Instant::now();
        self.warm_up_agent
            .post(&format!("http://127.0.0.1:{port}/v1/audio/transcriptions"))
            .set(
                "Content-Type",
                &format!("multipart/form-data; boundary={WARM_UP_BOUNDARY}"),
            )
            .send_bytes(&warm_up_body())
            .ok()
            .filter(|response| response.status() == 200)
            .map(|_| began.elapsed().max(Duration::from_millis(1)))
    }
}
