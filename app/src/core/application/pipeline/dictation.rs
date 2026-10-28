//! The running dictation as one value, from its start cue to its close.

use crate::core::application::pipeline::archiving::DictationTrace;
use crate::core::application::DictationStart;
use crate::core::domain::{
    without_leading_command, without_spoken_pastes, without_trailing_command, FailingPart,
    FailureNotice, VoiceCommand,
};

use super::failure_notices::graver;
use super::{LiveSessionResult, LiveTranscriptionSession};

/// A dictation from its start cue to its close: what transcribes it while it records, the
/// command words said in one breath with it, the notice it ends with and its archive trace.
pub(super) struct Dictation {
    /// Its chunks transcribed while it records; none when it is transcribed on stop.
    pub(super) live: Option<LiveTranscriptionSession>,
    pub(super) command_words: CommandWords,
    /// The failure notice the dictation ends with instead of hiding the overlay.
    pub(super) pending_notice: Option<FailureNotice>,
    /// The dictation as its archive entry will hold it; none without an archive.
    pub(super) trace: Option<DictationTrace>,
}

/// Command words said in one breath with a dictation, so they may reach its transcript.
#[derive(Debug, Default, Clone, Copy)]
pub(super) struct CommandWords {
    /// The command that started the dictation, said in one breath with its first words.
    pub(super) opening: Option<VoiceCommand>,
    /// A command said without a pause ended the dictation.
    pub(super) closing: bool,
    /// The "paste text" commands carried out while the dictation ran.
    pub(super) pastes: usize,
}

impl Dictation {
    /// A dictation that starts now; traced when `archived`.
    pub(super) fn new(start: DictationStart, archived: bool) -> Self {
        Self {
            live: None,
            command_words: CommandWords::default(),
            pending_notice: None,
            trace: archived.then(|| DictationTrace::new(start)),
        }
    }

    /// Keeps `notice` for the end of the dictation, unless a graver one is already pending.
    pub(super) fn note_failure(&mut self, notice: FailureNotice) {
        log::warn!("failure notice: {}", notice.message());
        self.pending_notice = Some(graver(self.pending_notice.take(), notice));
    }

    /// Notes the chunks speech recognition lost: the whole dictation when none had text.
    pub(super) fn note_lost_chunks(&mut self, result: &LiveSessionResult) {
        if result.lost_chunks == 0 {
            return;
        }
        let part = result
            .lost_to
            .clone()
            .unwrap_or(FailingPart::SpeechRecognition);
        self.note_failure(if result.texts.is_empty() {
            FailureNotice::DictationLost(part)
        } else {
            FailureNotice::PartOfDictationLost(part)
        });
    }

    /// `text` without the command words said in one breath with the dictation, and without
    /// the "paste text" commands carried out while it ran.
    pub(super) fn without_command_words(&mut self, text: &str) -> String {
        let words = std::mem::take(&mut self.command_words);
        let text = without_spoken_pastes(text, words.pastes);
        let text = match words.opening {
            Some(command) => without_leading_command(command, &text),
            None => text,
        };
        if words.closing {
            without_trailing_command(&text)
        } else {
            text
        }
    }
}
