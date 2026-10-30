//! The two engines the service can listen with, behind one face for the server: the keyword
//! spotter confirmed by the speech recognition service, or the streaming recognizers.

use std::time::Duration;

use anyhow::Result;

use crate::engine::{self, Engine};
use crate::keywords::UnknownPhrases;
use crate::protocol::{PhraseSelection, ServerEvent};
use crate::streaming::engine::StreamingEngine;
use crate::streaming::session;
use crate::worker::Wake;

/// The engine the service was started with.
pub enum Service {
    KeywordSpotting(Engine),
    Streaming(StreamingEngine),
}

impl From<Engine> for Service {
    fn from(engine: Engine) -> Self {
        Self::KeywordSpotting(engine)
    }
}

impl Service {
    /// The session of connection `conn`; a confirmation's answer, or a recognizer's decode,
    /// calls `wake`.
    pub fn session_waking(&self, conn: u64, wake: Option<Wake>) -> Result<Listening<'_>> {
        Ok(match self {
            Self::KeywordSpotting(engine) => {
                Listening::KeywordSpotting(Box::new(engine.session_waking(conn, wake)?))
            }
            Self::Streaming(engine) => {
                Listening::Streaming(Box::new(engine.session_waking(conn, wake)?))
            }
        })
    }
}

/// One connection's session in the engine of its service.
pub enum Listening<'a> {
    KeywordSpotting(Box<engine::Session<'a>>),
    Streaming(Box<session::Session<'a>>),
}

impl Listening<'_> {
    pub fn accept(&mut self, samples: &[f32]) -> Vec<ServerEvent> {
        match self {
            Self::KeywordSpotting(s) => s.accept(samples),
            Self::Streaming(s) => s.accept(samples),
        }
    }

    pub fn poll(&mut self) -> Vec<ServerEvent> {
        match self {
            Self::KeywordSpotting(s) => s.poll(),
            Self::Streaming(s) => s.poll(),
        }
    }

    pub fn select(&mut self, selection: &PhraseSelection) -> Result<(), UnknownPhrases> {
        match self {
            Self::KeywordSpotting(s) => s.select(selection),
            Self::Streaming(s) => s.select(selection),
        }
    }

    pub fn sync(&mut self, id: u64) -> Vec<ServerEvent> {
        match self {
            Self::KeywordSpotting(s) => s.sync(id),
            Self::Streaming(s) => s.sync(id),
        }
    }

    pub fn stalled(&self) -> Option<Duration> {
        match self {
            Self::KeywordSpotting(s) => s.stalled(),
            Self::Streaming(s) => s.stalled(),
        }
    }

    pub fn pending_confirmations(&self) -> usize {
        match self {
            Self::KeywordSpotting(s) => s.pending_confirmations(),
            Self::Streaming(s) => s.pending_confirmations(),
        }
    }
}
