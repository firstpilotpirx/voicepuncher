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

impl HttpModelFetcher {
    /// Asks for `url` from byte `from` on; `None` when the part already holds the whole file.
    fn request(&self, url: &str, from: u64) -> Result<Option<ureq::Response>, FetchError> {
        let mut request = self.agent.get(url);
        if from > 0 {
            request = request.set("Range", &format!("bytes={from}-"));
        }
        match request.call() {
            Ok(response) => Ok(Some(response)),
            // the part already holds the whole file
            Err(ureq::Error::Status(416, _)) if from > 0 => Ok(None),
            Err(ureq::Error::Status(code, _)) if code >= 500 => {
                Err(FetchError::Connection(format!("HTTP {code} from {url}")))
            }
            Err(ureq::Error::Status(code, _)) => {
                Err(FetchError::Server(format!("HTTP {code} from {url}")))
            }
            Err(ureq::Error::Transport(transport)) => {
                Err(FetchError::Connection(transport.to_string()))
            }
        }
    }
}

/// Appends the body of `response` to `sink`, telling `received` the part's length after
/// every chunk; fails when the stream ends before its stated length.
fn append_body(
    response: ureq::Response,
    sink: &mut dyn PartSink,
    received: &mut dyn FnMut(u64),
    cancel: &AtomicBool,
) -> Result<(), FetchError> {
    let expected = response
        .header("Content-Length")
        .and_then(|length| length.parse::<u64>().ok());
    let start = sink.len();
    copy_into(response.into_reader(), sink, received, cancel)?;
    let got = sink.len() - start;
    match expected {
        Some(expected) if got < expected => Err(FetchError::Connection(format!(
            "the stream ended after {got} of {expected} bytes"
        ))),
        _ => Ok(()),
    }
}

/// Copies `reader` into `sink` chunk by chunk, telling `received` the part's length after
/// each, until it ends or `cancel` is set.
fn copy_into(
    mut reader: impl Read,
    sink: &mut dyn PartSink,
    received: &mut dyn FnMut(u64),
    cancel: &AtomicBool,
) -> Result<(), FetchError> {
    let mut buffer = vec![0; CHUNK];
    loop {
        if cancel.load(Ordering::SeqCst) {
            return Err(FetchError::Cancelled);
        }
        let read = reader
            .read(&mut buffer)
            .map_err(|e| FetchError::Connection(e.to_string()))?;
        if read == 0 {
            return Ok(());
        }
        sink.write(&buffer[..read]).map_err(FetchError::Disk)?;
        received(sink.len());
    }
}

impl ModelFetcher for HttpModelFetcher {
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
        let Some(response) = self.request(url, from)? else {
            return Ok(());
        };
        match response.status() {
            200 if from > 0 => sink.restart().map_err(FetchError::Disk)?,
            200 | 206 => {}
            code => return Err(FetchError::Server(format!("HTTP {code} from {url}"))),
        }
        append_body(response, sink, received, cancel)
    }
}
