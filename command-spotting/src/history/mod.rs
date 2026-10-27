//! The command history: one entry per utterance in which either model heard a command, with
//! its audio, both models' results and the outcome.

mod entry;
mod outcome;
mod recorder;
mod writer;

pub use entry::{write_wav, Entry, Hearing, Reported, SpottedPhrase, AUDIO_FILE, ENTRY_FILE};
pub use outcome::{outcome, Outcome};
pub use recorder::Recorder;
pub use writer::{keep, CommandHistory, HistoryCfg};
