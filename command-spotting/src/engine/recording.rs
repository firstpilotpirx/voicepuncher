//! What a session hands its recorder for the command history, and the entries it closes.

use std::ops::Range;

use crate::confirm::Heard;
use crate::keywords;
use crate::orderer::Spotted;
use crate::protocol::ServerEvent;
use crate::worker::JobKind;

use super::{seconds, Engine, Session};

impl Engine {
    /// Whether the engine keeps a command history.
    pub fn keeps_history(&self) -> bool {
        self.history.is_some()
    }

    /// Waits until every command history entry closed so far is written or given up.
    pub fn flush_history(&self) {
        if let Some(history) = self.history.as_ref() {
            history.flush();
        }
    }
}

impl Session<'_> {
    /// `events` as they leave the session, recorded; the entries now due, or every entry
    /// with `all`, go to the command history.
    pub(super) fn recorded(&mut self, events: Vec<ServerEvent>, all: bool) -> Vec<ServerEvent> {
        let now = seconds(self.detector.samples_seen);
        let (Some(recorder), Some(history)) =
            (self.recorder.as_mut(), self.engine.history.as_ref())
        else {
            return events;
        };
        recorder.went_out(&events);
        for entry in recorder.due(now, all, &self.recent) {
            history.keep(entry);
        }
        events
    }

    /// The spotter spotted `spotted` as `tokens`; a trigger counts as the phrase it serves.
    pub(super) fn record_spotted(&mut self, spotted: &Spotted, tokens: &str) {
        if let Some(recorder) = self.recorder.as_mut() {
            let phrase = keywords::served_by(&spotted.phrase).unwrap_or(&spotted.phrase);
            recorder.spotted(phrase, tokens, spotted.start, spotted.end);
        }
    }

    pub(super) fn record_asked(&mut self, n: u64, kind: JobKind, span: &Range<usize>) {
        if let Some(recorder) = self.recorder.as_mut() {
            let kind = format!("{kind:?}").to_lowercase();
            recorder.asked(n, &kind, seconds(span.start), seconds(span.end));
        }
    }

    pub(super) fn record_answered(&mut self, n: u64, heard: Option<&Heard>) {
        if let Some(recorder) = self.recorder.as_mut() {
            recorder.answered(n, heard.map(|h| h.text.as_str()));
        }
    }
}

impl Drop for Session<'_> {
    /// The segments gone out still waiting are kept as they are.
    fn drop(&mut self) {
        self.recorded(Vec::new(), true);
    }
}
