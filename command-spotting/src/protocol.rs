//! Messages between the app and the service over the WebSocket connection.

use serde::{Deserialize, Serialize};

pub use crate::matcher::Placement;

/// Text frame from the app: the phrases to spot from now on.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PhraseSelection {
    pub phrases: Vec<String>,
}

/// The version of this protocol: the app says it in its first message, `hello`.
pub const PROTOCOL_VERSION: u32 = 2;

/// A text frame from the app.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AppMessage {
    /// The first message of every connection: the protocol version the app speaks.
    Hello {
        version: u32,
    },
    Select(PhraseSelection),
    /// Asks for `synced` with this id once every event for the audio sent before it is out.
    Sync {
        id: u64,
    },
}

/// Text frame to the app. Times are seconds since the first sample of the connection.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum ServerEvent {
    SpeechStart {
        t: f32,
    },
    SpeechEnd {
        t: f32,
    },
    Phrase {
        phrase: String,
        start: f32,
        end: f32,
        /// Present when a confirmation heard the phrase.
        #[serde(skip_serializing_if = "Option::is_none")]
        timing: Option<Timing>,
    },
    /// Commands said without a pause around them, to carry out in order without the
    /// app's pause rule: `start..end` spans their words.
    Command {
        commands: Vec<String>,
        start: f32,
        end: f32,
        placement: Placement,
        #[serde(skip_serializing_if = "Option::is_none")]
        timing: Option<Timing>,
    },
    /// The speech segment `start..end` got no answer from the speech recognition service:
    /// a command said in it is lost.
    ConfirmationFailed {
        start: f32,
        end: f32,
    },
    /// A confirmation has run for `waited_ms`, past its attempts: the service hangs, and
    /// the app is to move to another copy.
    Stalled {
        waited_ms: u64,
    },
    /// `samples` of audio did not fit the backlog and were dropped: the stream's clock is
    /// broken, and the app is to connect again.
    Dropped {
        samples: u64,
    },
    /// Every event for the audio sent before the sync with this id is out.
    Synced {
        id: u64,
    },
}

/// The confirmation that heard a phrase or command: how far the stream had come when it was
/// asked, in seconds, and how long the speech recognition service took to answer it.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Timing {
    pub asked_at: f32,
    pub took_ms: u64,
}

const SAMPLE_BYTES: usize = 4;

/// Binary frame from the app: 16 kHz mono `f32` little-endian samples; a frame whose length
/// is not a whole number of samples is refused, naming its length.
pub fn samples_from_bytes(bytes: &[u8]) -> Result<Vec<f32>, String> {
    if bytes.len() % SAMPLE_BYTES != 0 {
        return Err(format!(
            "an audio frame of {} bytes holds no whole number of {SAMPLE_BYTES}-byte samples",
            bytes.len()
        ));
    }
    Ok(bytes
        .chunks_exact(SAMPLE_BYTES)
        .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .collect())
}

#[cfg(test)]
mod tests;
