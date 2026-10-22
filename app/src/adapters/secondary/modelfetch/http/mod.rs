//! Model files fetched over HTTPS: a Range request resumes a part, a whole-file answer to it
//! starts the part over, redirects to the hosts' CDNs are followed with the range kept.

use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::adapters::secondary::http::agent;
use crate::core::application::{FetchError, ModelFetcher, PartSink};

const CHUNK: usize = 8 * 1024;

/// Fetches with one agent; no overall time limit, so a slow link is no failure.
pub struct HttpModelFetcher {
    agent: ureq::Agent,
}

impl HttpModelFetcher {
    pub fn new() -> Self {
        Self { agent: agent::downloads() }
    }
}

impl Default for HttpModelFetcher {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelFetcher for HttpModelFetcher {
    #[expect(clippy::too_many_lines, reason = "size baseline")]
    fn fetch(
        &self,
        url: &str,
        sink: &mut dyn PartSink,
        received: &mut dyn FnMut(u64),
        cancel: &AtomicBool,
    ) -> Result<(), FetchError> {
        if cancel.load(Ordering::SeqCst) {
            return Err(FetchError::Cancelled);
        }
        let from = sink.len();
        let mut request = self.agent.get(url);
        if from > 0 {
            request = request.set("Range", &format!("bytes={from}-"));
        }
        let response = match request.call() {
            Ok(response) => response,
            // the part already holds the whole file
            Err(ureq::Error::Status(416, _)) if from > 0 => return Ok(()),
            Err(ureq::Error::Status(code, _)) if code >= 500 => {
                return Err(FetchError::Connection(format!("HTTP {code} from {url}")))
            }
            Err(ureq::Error::Status(code, _)) => {
                return Err(FetchError::Server(format!("HTTP {code} from {url}")))
            }
            Err(ureq::Error::Transport(transport)) => {
                return Err(FetchError::Connection(transport.to_string()))
            }
        };
        match response.status() {
            206 => {}
            200 if from > 0 => sink.restart().map_err(FetchError::Disk)?,
            200 => {}
            code => return Err(FetchError::Server(format!("HTTP {code} from {url}"))),
        }
        let expected = response
            .header("Content-Length")
            .and_then(|length| length.parse::<u64>().ok());
        let start = sink.len();
        let mut reader = response.into_reader();
        let mut buffer = vec![0; CHUNK];
        loop {
            if cancel.load(Ordering::SeqCst) {
                return Err(FetchError::Cancelled);
            }
            let read = reader
                .read(&mut buffer)
                .map_err(|e| FetchError::Connection(e.to_string()))?;
            if read == 0 {
                break;
            }
            sink.write(&buffer[..read]).map_err(FetchError::Disk)?;
            received(sink.len());
        }
        let got = sink.len() - start;
        match expected {
            Some(expected) if got < expected => Err(FetchError::Connection(format!(
                "the stream ended after {got} of {expected} bytes"
            ))),
            _ => Ok(()),
        }
    }
}
