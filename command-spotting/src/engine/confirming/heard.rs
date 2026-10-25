//! The event an answer heard makes of the confirmation it answers.

use crate::confirm::Heard;
use crate::matcher::PhraseMatcher;
use crate::protocol::ServerEvent;

use super::super::spotted::Round;
use super::super::Session;

use super::place::HeardClip;
use super::{head_command, spotted_command, JobContext};

impl Session<'_> {
    /// The event `heard` makes of the confirmation asked with `context` when the stream had
    /// come to `asked_at`, read with `reading`.
    pub(super) fn apply_heard(
        &mut self,
        reading: &PhraseMatcher,
        heard: Heard,
        context: JobContext,
        asked_at: usize,
    ) -> Option<ServerEvent> {
        match context {
            JobContext::Head { utterance, from, early } => {
                let clip = HeardClip { heard: &heard, from, end: Some(asked_at) };
                self.apply_head(reading, clip, utterance, early)
            }
            spotted @ JobContext::Spotted { .. } => {
                self.apply_spotted(reading, &heard, &spotted, asked_at)
            }
            JobContext::Window { utterance, from } => {
                self.apply_window(reading, heard, utterance, from)
            }
            JobContext::Segment { utterance, segment } => {
                self.apply_segment(reading, &heard, utterance, &segment)
            }
            JobContext::Unheld { spotted, from } => {
                self.apply_unheld(reading, &heard, &spotted, from)
            }
            // Its answer waits for the segment it may stand for.
            JobContext::HalfPause { .. } => None,
            JobContext::Tail { utterance, from } => {
                let whole = HeardClip { heard: &heard, from, end: None };
                self.place_anywhere_or_at_end(reading, whole, utterance)
            }
            JobContext::Rehear { bounds, origin, phrase, .. } => {
                self.apply_rehear(reading, &heard, (bounds, origin), phrase)
            }
        }
    }

    /// The head command `clip` heard; "start" heard alone early may be cut off before the
    /// word after it, as the first spotted check may hear it: "start stop" starts nothing.
    fn apply_head(
        &mut self,
        reading: &PhraseMatcher,
        clip: HeardClip<'_>,
        utterance: u64,
        early: bool,
    ) -> Option<ServerEvent> {
        if early {
            self.place(clip, utterance, |w| {
                spotted_command(reading, w, true, Round::First)
            })
        } else {
            self.place(clip, utterance, |w| head_command(reading, w))
        }
    }

    /// The command a check of a spotted phrase heard.
    fn apply_spotted(
        &mut self,
        reading: &PhraseMatcher,
        heard: &Heard,
        context: &JobContext,
        asked_at: usize,
    ) -> Option<ServerEvent> {
        let &JobContext::Spotted { utterance, from, at_head, round, said_end } = context else {
            return None;
        };
        let clip = HeardClip { heard, from, end: Some(asked_at) };
        let found = self.place(clip, utterance, |w| {
            spotted_command(reading, w, at_head, round)
        });
        // The first check may come before the command ends: "start" alone at the head, or the
        // second "restart" of "restart restart" cut off by the clip.
        if found.is_none() && round == Round::First {
            self.jobs.found_nothing(said_end, asked_at, utterance);
        }
        found
    }

    /// A command anywhere in a window heard while the utterance ran; the window is kept as the
    /// utterance's last.
    fn apply_window(
        &mut self,
        reading: &PhraseMatcher,
        heard: Heard,
        utterance: u64,
        from: usize,
    ) -> Option<ServerEvent> {
        let whole = HeardClip { heard: &heard, from, end: None };
        let found = self.place(whole, utterance, |w| reading.anywhere(w));
        self.mark(utterance, |r| r.last_window = Some((heard, from)));
        found
    }
}
