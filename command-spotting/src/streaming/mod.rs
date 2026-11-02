//! The streaming recognizer: a voice gate wakes an English and a Russian streaming recognizer
//! at the onset of speech, and the first to hear a command reports it.

pub mod decide;
pub mod engine;
pub mod ports;
pub mod respell;
pub mod session;
pub mod sherpa;
pub mod trace;
pub mod words;

use std::sync::Arc;

/// Called on a recognizer's thread right after it decoded, so the session takes its result
/// at once instead of at its next poll.
pub type Wake = Arc<dyn Fn() + Send + Sync>;
