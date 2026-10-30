//! The streaming recognizer: a voice gate wakes an English and a Russian streaming recognizer
//! at the onset of speech, and the first to hear a command reports it.

pub mod decide;
pub mod ports;
pub mod respell;
pub mod session;
pub mod trace;
pub mod words;
