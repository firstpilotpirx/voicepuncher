//! Spotted triggers no utterance held: confirmed on their own clip.

use crate::confirm::Heard;
use crate::matcher::{PhraseMatcher, Placement};
use crate::orderer::Spotted;
use crate::protocol::ServerEvent;
use crate::worker::JobKind;

use super::super::{samples_in, seconds, Session, PRE_ROLL_SECS};

use super::JobContext;

impl Session<'_> {
    /// A phrase the keyword spotter found but no speech segment took: the voice activity
    /// detector missed the utterance. It counts when the speech recognition service hears
    /// only the command around it.
    pub(in crate::engine) fn confirm_unheld(&mut self) {
        if self.worker.is_none() {
            return;
        }
        for spotted in self.orderer.unheld(seconds(self.detector.samples_seen)) {
            let around = samples_in(PRE_ROLL_SECS / 2.0);
            let from = samples_in(spotted.start)
                .saturating_sub(around)
                .max(self.utterances.last_segment_end);
            let to = samples_in(spotted.end) + around;
            self.ask(
                JobKind::Unheld,
                from,
                to,
                JobContext::Unheld { spotted, from },
            );
        }
    }

    pub(super) fn apply_unheld(
        &self,
        reading: &PhraseMatcher,
        heard: &Heard,
        spotted: &Spotted,
        from: usize,
    ) -> Option<ServerEvent> {
        let phrase = reading.phrase_in(&heard.text)?;
        let (start, end) = match (heard.words.first(), heard.words.last()) {
            (Some(first), Some(last)) => (
                seconds(from + samples_in(first.start)),
                seconds(from + samples_in(last.end)),
            ),
            _ => (spotted.start, spotted.end),
        };
        log::info!("conn={} missed utterance → {phrase:?}", self.conn);
        Some(ServerEvent::Command {
            commands: vec![phrase],
            start,
            end,
            placement: Placement::Tail,
            timing: None,
        })
    }
}
