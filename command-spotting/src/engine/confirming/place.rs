//! Commands placed in an utterance by the times of their words.

use crate::confirm::{Heard, TimedWord};
use crate::matcher::{Found, PhraseMatcher};
use crate::protocol::ServerEvent;

use super::super::{samples_in, seconds, Session, TAIL_SLACK_SECS};

/// What a confirmation heard, and where its clip began in the stream: at sample `from`.
#[derive(Clone, Copy)]
pub(super) struct HeardClip<'a> {
    pub(super) heard: &'a Heard,
    pub(super) from: usize,
}

impl HeardClip<'_> {
    /// The stream sample at which `word` starts or ends, by `time`.
    fn at(&self, word: usize, time: fn(&TimedWord) -> f32) -> usize {
        self.from + samples_in(time(&self.heard.words[word]))
    }

    /// The stream sample in the middle of `word`.
    fn middle(&self, word: usize) -> usize {
        usize::midpoint(self.at(word, |w| w.start), self.at(word, |w| w.end))
    }
}

impl Session<'_> {
    /// The commands `find` sees in what `clip` heard, as an event placed in the stream; of a
    /// chain the utterance already reported in part, only the commands after what it reported
    /// ("start" after a check heard "cancel enter"), none when nothing is left.
    pub(super) fn place(
        &mut self,
        clip: HeardClip<'_>,
        utterance: u64,
        find: impl Fn(&[&str]) -> Option<Found>,
    ) -> Option<ServerEvent> {
        let words = clip.heard.word_texts();
        let found = find(&words)?;
        let kept = self.first_unreported(clip, &found, utterance)?;
        let start = clip.at(found.spans[kept].0, |w| w.start);
        let end = clip.at(found.last, |w| w.end);
        if let Some(running) = self.utterances.running.get_mut(&utterance) {
            running.reported_until = end;
        }
        let commands = found.commands[kept..].to_vec();
        log::info!("conn={} {:?} {:?}", self.conn, found.placement, commands);
        Some(ServerEvent::Command {
            commands,
            start: seconds(start),
            end: seconds(end),
            placement: found.placement,
            timing: None,
        })
    }

    /// The first of `found`'s commands after what the utterance already reported; none when
    /// it reported them all.
    fn first_unreported(
        &self,
        clip: HeardClip<'_>,
        found: &Found,
        utterance: u64,
    ) -> Option<usize> {
        let reported_until = self
            .utterances
            .running
            .get(&utterance)
            .map_or(0, |r| r.reported_until);
        // Word times differ by about 0.1 s between two clips of the same speech: a command
        // lies after the report when the middle of its first word does; a doubled command
        // whose first word was reported alone ("start" heard before "start start") is not.
        if clip.at(found.first, |w| w.start) < reported_until {
            found
                .spans
                .iter()
                .position(|&(first, _)| clip.middle(first) > reported_until)
        } else {
            Some(0)
        }
    }

    /// A doubled or chained command anywhere in what `clip` heard, else one that ends it.
    pub(super) fn place_anywhere_or_at_end(
        &mut self,
        reading: &PhraseMatcher,
        clip: HeardClip<'_>,
        utterance: u64,
    ) -> Option<ServerEvent> {
        self.place(clip, utterance, |w| reading.anywhere(w))
            .or_else(|| self.place(clip, utterance, |w| reading.at_end(w)))
    }

    /// A command that ended the last window heard while the utterance ran, when it also
    /// ends the utterance: the closing window, cut elsewhere, may hear its words wrong
    /// ("at ten enter" as "at 10am").
    pub(super) fn tail_heard_while_running(
        &mut self,
        reading: &PhraseMatcher,
        last_window: Option<(Heard, usize)>,
        end: usize,
        utterance: u64,
    ) -> Option<ServerEvent> {
        let (heard, from) = last_window?;
        let clip = HeardClip { heard: &heard, from };
        let event = self.place(clip, utterance, |w| reading.at_end(w))?;
        match event {
            ServerEvent::Command { end: said_end, .. }
                if said_end >= seconds(end.saturating_sub(samples_in(TAIL_SLACK_SECS))) =>
            {
                Some(event)
            }
            _ => None,
        }
    }
}
