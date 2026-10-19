use std::time::SystemTime;

use anyhow::Result;

use crate::core::domain::AudioBuffer16k;

/// Keeps every dictation that reached speech recognition for later evaluation.
#[cfg_attr(test, mockall::automock)]
pub trait DictationArchive: Send {
    fn keep(&mut self, entry: &ArchiveEntry) -> Result<()>;
}

/// How a dictation was started.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DictationStart {
    HotkeyHeld,
    HotkeyToggled,
    Voice,
    RecordingFile,
}

/// Whether the pasted text is the refinement service's answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefinementOutcome {
    Refined,
    /// Text refinement is turned off: the raw transcript is pasted.
    Off,
    /// Refinement failed or its answer was rejected: the raw transcript is pasted.
    FellBack,
}

/// How a dictation that reached speech recognition ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DictationOutcome {
    Pasted,
    LeftInClipboard,
    PasteFailed,
    SpeechRecognitionFailed,
    NoText,
    OnlyCommandWords,
}

/// The time one stage after the recording stopped took.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StageTiming {
    pub stage: &'static str,
    pub ms: u64,
}

/// What the archive keeps of one dictation.
#[derive(Debug, Clone)]
pub struct ArchiveEntry {
    pub started_at: SystemTime,
    pub start: DictationStart,
    pub audio: AudioBuffer16k,
    /// The text speech recognition returned for each chunk, in the order spoken.
    pub chunk_texts: Vec<String>,
    /// The stitched transcript that went to text refinement.
    pub raw_transcript: Option<String>,
    /// The text to paste: pasted, left in the clipboard or lost to a failed paste.
    pub pasted_text: Option<String>,
    pub refinement: Option<RefinementOutcome>,
    pub outcome: DictationOutcome,
    /// Chunks speech recognition failed on.
    pub lost_chunks: usize,
    pub timings: Vec<StageTiming>,
}
