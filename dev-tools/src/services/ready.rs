//! When a service counts ready: something answers on its port, and for a speech recognition service the copy
//! transcribed the warm-up utterance; for the text refinement service its health check answers.

use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// How a service's readiness is checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ready {
    /// A speech recognition copy: its port answers and it transcribed the warm-up utterance.
    Whisper { port: u32, warm_up: PathBuf },
    /// The text refinement service: its health check answers.
    Llama { port: u32 },
    /// Anything answers on the port.
    Port { port: u32 },
}

impl Ready {
    /// A note for the ready line when the service is ready (empty, or how long the first transcription took),
    /// nothing when it is not.
    pub fn check(&self) -> Option<String> {
        match self {
            Self::Whisper { port, warm_up } => whisper_ready(*port, warm_up),
            Self::Llama { port } => {
                curl_ok(&format!("http://127.0.0.1:{port}/health"), "1").then(String::new)
            }
            Self::Port { port } => port_open(*port).then(String::new),
        }
    }
}

/// Whether something listens on the loopback port.
pub fn port_open(port: u32) -> bool {
    u16::try_from(port).is_ok_and(|port| {
        let address = SocketAddr::from(([127, 0, 0, 1], port));
        TcpStream::connect_timeout(&address, Duration::from_secs(5)).is_ok()
    })
}

/// The copy answered the transcription of the warm-up utterance. macOS builds a copy's Metal pipelines on its
/// first decode, often for seconds, and not reliably once per machine (design of
/// fix-flaky-restart-cue-under-metal-load): a copy counts ready only with that decode behind it, so no client's
/// first request waits for it; the note names how long it took.
fn whisper_ready(port: u32, warm_up: &Path) -> Option<String> {
    if !port_open(port) {
        return None;
    }
    let began = Instant::now();
    let mut file = std::ffi::OsString::from("file=@");
    file.push(warm_up);
    let ok = Command::new("curl")
        .args(["-sf", "-o", "/dev/null", "--max-time", "30"])
        .arg(format!("http://127.0.0.1:{port}/v1/audio/transcriptions"))
        .arg("-F")
        .arg(file)
        .args(["-F", "response_format=json"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success());
    ok.then(|| note(began.elapsed()))
}

/// The ready line's note on the first transcription.
pub fn note(took: Duration) -> String {
    format!(", first transcription {} ms", took.as_millis())
}

/// Whether a GET of the URL succeeds within the seconds given.
pub fn curl_ok(url: &str, max_time: &str) -> bool {
    Command::new("curl")
        .args(["-fsS", "-o", "/dev/null", "--max-time", max_time, url])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}
