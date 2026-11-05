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
    /// The commands of every finding of each recognizer, reported or not.
    pub(super) heard: Vec<(usize, Vec<String>)>,
    /// Finished updates held back at the close: a recognizer that heard only the end of a
    /// command word ("start" of a quiet "restart") waits for the other to finish, and every
    /// one waits a frame for the next utterance to open over this one's tail.
    pub(super) held: Vec<(usize, Update)>,
    /// Where the next utterance's audio starts: the words from there on are its to report.
    pub(super) handover: Option<f32>,
}

impl Reports {
    /// Whether `found`, heard by recognizer `r`, goes out: it neither doubles the last finding
    /// nor repeats a report by the other recognizer; a finding that stays in still ends what is
    /// decided, as the words before it were decided already.
    pub(super) fn goes_out(&mut self, r: usize, found: &Finding) -> bool {
        let repeated = self.repeats(r, found);
        let doubled = doubles(self.last.as_ref(), found) || repeated;
        self.last = Some(found.clone());
        if !doubled {
            self.sent.push(found.commands.clone());
        }
        !doubled
    }

    /// Whether `found`, heard by recognizer `r`, repeats a report: both recognizers hear the
    /// same speech, so the n-th time one hears a command is the utterance's n-th, however
    /// the two stamp it, and it is reported once the utterance reported it fewer times than
    /// `r` heard it, `found` counted.
    fn repeats(&mut self, r: usize, found: &Finding) -> bool {
        self.heard.push((r, found.commands.clone()));
        let heard = self
            .heard
            .iter()
            .filter(|(by, commands)| *by == r && *commands == found.commands)
            .count();
        let sent = self
            .sent
            .iter()
            .filter(|commands| **commands == found.commands)
            .count();
        sent >= heard
    }
}
