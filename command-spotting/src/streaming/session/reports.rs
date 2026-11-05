//! What one utterance reported so far, and whether a finding goes out or stays in.

use super::super::decide::{doubles, Finding};
use super::decoder::Update;

/// What an utterance reports: its last report, what waits to be decided, what it leaves to
/// the next utterance, and what each recognizer heard of it.
#[derive(Default)]
pub(super) struct Reports {
    /// The last finding, by whichever recognizer: commands ending before its end are decided.
    pub(super) last: Option<Finding>,
    /// The commands of every report that went out.
    pub(super) sent: Vec<Vec<String>>,
    /// Finished updates held back at the close: a recognizer that heard only the end of a
    /// command word ("start" of a quiet "restart") waits for the other to finish, and every
    /// one waits a frame for the next utterance to open over this one's tail.
    pub(super) held: Vec<(usize, Update)>,
    /// Where the next utterance's audio starts: the words from there on are its to report.
    pub(super) handover: Option<f32>,
}

impl Reports {
    /// Whether `found` goes out: it does unless it doubles the last finding. A report by the
    /// other recognizer is never found again: both hear the same speech, and the words of
    /// the commands reported are found in each recognizer's text before it decides
    /// (`decide`), however the two stamp them. A finding that stays in still ends what is
    /// decided, as the words before it were decided already.
    pub(super) fn goes_out(&mut self, found: &Finding) -> bool {
        let doubled = doubles(self.last.as_ref(), found);
        self.last = Some(found.clone());
        if !doubled {
            self.sent.push(found.commands.clone());
        }
        !doubled
    }
}
