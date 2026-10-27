//! Commands reported without confirmation: the keyword spotter's phrase goes out at once, and
//! the speech recognition service's hearing of it follows as a verdict, not as a report.

use std::collections::HashMap;
use std::ops::RangeInclusive;
use std::sync::Arc;

use crate::confirm::Heard;
use crate::matcher::{PhraseMatcher, Placement};
use crate::orderer::{Segment, Spotted};
use crate::protocol::{ServerEvent, Verdict};
use crate::worker::Answer;

use super::super::spotted::Round;
use super::super::{samples_in, Session};
use super::{AskedJob, ClosedSegment, JobContext};

/// How far past the end of its segment a doubled command reported at once may end and still
/// stand for the segment's phrase: as the event orderer places phrases.
const SEGMENT_SLACK_SECS: f32 = 0.5;

/// A hearing whose answer gives a verdict: a segment's job, or the check of the doubled
/// command spotted ending at a sample.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Awaited {
    Job(u64),
    Doubled(usize),
}

/// Commands reported without confirmation, at `start..end` in seconds.
struct Report {
    commands: Vec<String>,
    start: f32,
    end: f32,
}

impl Report {
    /// Whether the reported commands fill `segment` alone, but for the slack the keyword
    /// spotter's timing leaves at either end.
    fn fills(&self, segment: Segment) -> bool {
        self.start - segment.start <= SEGMENT_SLACK_SECS
            && segment.end - self.end <= SEGMENT_SLACK_SECS
    }
}

/// What a hearing awaited for a verdict stands for.
enum Standing {
    /// A short segment went out before the keyword spotter spotted anything in it: its
    /// hearing confirms as usual unless a phrase follows.
    Unreported,
    /// An unreported segment's hearing answered before the keyword spotter's phrase of it
    /// came: what it heard, read with the phrases of its time, gives the verdict on that phrase.
    Answered(Option<Heard>, Arc<PhraseMatcher>),
    Reported(Report),
    /// A doubled command reported at once stands for its phrase: its hearing is dropped.
    Covered,
}

/// Whether commands are reported without confirmation, and the hearings whose verdicts are
/// awaited.
#[derive(Default)]
pub(in crate::engine) struct Unconfirmed {
    pub(in crate::engine) on: bool,
    awaited: HashMap<Awaited, Standing>,
    /// The job of the last short segment that went out at once.
    last_segment: Option<u64>,
    /// Where the last doubled command reported at once ended, in samples.
    doubled_end: Option<usize>,
}

impl Unconfirmed {
    /// What the hearing of a short segment whose doubled command would end within `reach`
    /// stands for, with the keyword spotter's phrases in `events`.
    fn segment_standing(
        &mut self,
        reach: &RangeInclusive<usize>,
        events: &[ServerEvent],
    ) -> Standing {
        let doubled = (self.awaited.keys())
            .find(|a| matches!(a, Awaited::Doubled(end) if reach.contains(end)))
            .copied();
        // The segment's hearing gives the verdict on a doubled command in it whose check is
        // not answered yet: a check answered after its segment is dropped.
        if let Some(standing) = doubled.and_then(|key| self.awaited.remove(&key)) {
            return standing;
        }
        if self.doubled_end.is_some_and(|end| reach.contains(&end)) {
            return Standing::Covered;
        }
        first_report(events).map_or(Standing::Unreported, Standing::Reported)
    }
}

impl Session<'_> {
    /// Reports the doubled command `spotted` completes at once, when commands go out without
    /// confirmation; its check gives the verdict.
    pub(in crate::engine) fn report_doubled(&mut self, spotted: &Spotted) -> Option<ServerEvent> {
        if !self.jobs.unconfirmed.on {
            return None;
        }
        let said_end = samples_in(spotted.end);
        let commands = vec![spotted.phrase.clone()];
        let report = Report { commands: commands.clone(), start: spotted.start, end: spotted.end };
        let unconfirmed = &mut self.jobs.unconfirmed;
        (unconfirmed.awaited).insert(Awaited::Doubled(said_end), Standing::Reported(report));
        unconfirmed.doubled_end = Some(said_end);
        if let Some(current) = self.utterances.current {
            self.mark(current, |r| {
                r.reported_until = r.reported_until.max(said_end);
            });
        }
        log::info!(
            "conn={} doubled {commands:?} reported unconfirmed",
            self.conn
        );
        Some(ServerEvent::Command {
            commands,
            start: spotted.start,
            end: spotted.end,
            placement: Placement::Anywhere,
            timing: None,
        })
    }

    /// Whether the short segment `closed` of `utterance`, confirmed by `job`, goes out at once
    /// with the keyword spotter's phrase in `events`, without confirmation; a trigger spotted
    /// in it, said on its own, adds the phrase it serves.
    pub(super) fn reported_unconfirmed(
        &mut self,
        job: u64,
        utterance: u64,
        closed: &ClosedSegment,
        events: &mut Vec<ServerEvent>,
    ) -> bool {
        if !self.jobs.unconfirmed.on {
            return false;
        }
        // A phrase among other speech of its segment waits for its confirmation.
        if first_report(events).is_some_and(|report| !report.fills(closed.bounds)) {
            return false;
        }
        if first_report(events).is_none() {
            events.extend(self.trigger_phrase(closed));
            events.sort_by_key(|e| matches!(e, ServerEvent::SpeechEnd { .. }));
        }
        let unconfirmed = &mut self.jobs.unconfirmed;
        // A hearing that answered for a segment whose phrase never came waits no longer.
        if let Some(last) = unconfirmed.last_segment {
            let answered = unconfirmed.awaited.get(&Awaited::Job(last));
            if matches!(answered, Some(Standing::Answered(..))) {
                unconfirmed.awaited.remove(&Awaited::Job(last));
            }
        }
        let reach = closed.start..=closed.end + samples_in(SEGMENT_SLACK_SECS);
        let standing = unconfirmed.segment_standing(&reach, events);
        unconfirmed.awaited.insert(Awaited::Job(job), standing);
        unconfirmed.last_segment = Some(job);
        // A check of the utterance answered from now on would report its command again.
        self.mark(utterance, |r| r.segment_answered = true);
        true
    }

    /// The phrase a trigger spotted within `closed` serves, as the keyword spotter's.
    fn trigger_phrase(&self, closed: &ClosedSegment) -> Option<ServerEvent> {
        let phrase = self.trigger_heard_in(&self.matcher, closed.bounds)?;
        let Segment { start, end } = closed.bounds;
        Some(ServerEvent::Phrase { phrase, start, end, timing: None })
    }

    /// A phrase in `events`, spotted after the short segment that ended last went out at once,
    /// is that segment's report: its hearing gives the verdict, at once when it already
    /// answered.
    pub(in crate::engine) fn report_late(&mut self, events: &[ServerEvent]) -> Option<ServerEvent> {
        let unconfirmed = &mut self.jobs.unconfirmed;
        let key = Awaited::Job(unconfirmed.last_segment?);
        let report = first_report(events)?;
        match unconfirmed.awaited.remove(&key)? {
            Standing::Unreported => {
                unconfirmed.awaited.insert(key, Standing::Reported(report));
                None
            }
            Standing::Answered(heard, reading) => {
                Some(self.verdict_event(&reading, heard.as_ref(), report, None))
            }
            other => {
                unconfirmed.awaited.insert(key, other);
                None
            }
        }
    }

    /// The `heard` event of what `heard`, read with `reading`, makes of `report`, logged.
    fn verdict_event(
        &self,
        reading: &PhraseMatcher,
        heard: Option<&Heard>,
        report: Report,
        job: Option<u64>,
    ) -> ServerEvent {
        let Report { commands, start, end } = report;
        let verdict = verdict_on(reading, heard, &commands);
        let job = job.map_or_else(String::new, |n| format!(" job={n}"));
        log::info!(
            "conn={}{job} whisper on {commands:?}: {verdict:?}",
            self.conn
        );
        ServerEvent::Heard { commands, start, end, verdict }
    }

    /// The verdict `answer` gives on what was reported without confirmation, when its job
    /// awaits one: the events that resolve the job.
    pub(super) fn verdict_of(
        &mut self,
        answer: &Answer,
        asked: &AskedJob,
    ) -> Option<Vec<ServerEvent>> {
        let n = answer.id.n;
        let awaited = match asked.context {
            JobContext::Spotted { round: Round::Doubled, said_end, .. } => {
                Awaited::Doubled(said_end)
            }
            _ => Awaited::Job(n),
        };
        let events = match self.jobs.unconfirmed.awaited.remove(&awaited)? {
            Standing::Unreported | Standing::Answered(..) => {
                // Confirmed as usual; a phrase of the segment spotted later gets this verdict.
                let answered = Standing::Answered(answer.heard.clone(), Arc::clone(&asked.matcher));
                self.jobs.unconfirmed.awaited.insert(awaited, answered);
                return None;
            }
            Standing::Covered => Vec::new(),
            Standing::Reported(report) => {
                vec![self.verdict_event(&asked.matcher, answer.heard.as_ref(), report, Some(n))]
            }
        };
        self.hearings.stand_ins.remove(&n);
        let context = &asked.context;
        let job = (context.utterance(), context.is_check());
        Some(self.resolve_answer(answer.id, job, asked.timing(), events))
    }
}

/// The first phrase the keyword spotter reported in `events`.
fn first_report(events: &[ServerEvent]) -> Option<Report> {
    events.iter().find_map(|e| match e {
        ServerEvent::Phrase { phrase, start, end, .. } => {
            Some(Report { commands: vec![phrase.clone()], start: *start, end: *end })
        }
        _ => None,
    })
}

/// What `heard` makes of `commands`, read with `reading`: the same commands, other ones, or
/// none at all.
fn verdict_on(reading: &PhraseMatcher, heard: Option<&Heard>, commands: &[String]) -> Verdict {
    let Some(heard) = heard else {
        return Verdict::None;
    };
    let said = reading
        .phrase_in(&heard.text)
        .map(|phrase| vec![phrase])
        .or_else(|| reading.anywhere(&heard.word_texts()).map(|f| f.commands))
        .unwrap_or_default();
    if said.is_empty() {
        Verdict::None
    } else if commands.iter().all(|c| said.contains(c)) {
        Verdict::Agreed
    } else {
        Verdict::Other
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(start: f32, end: f32) -> Report {
        Report { commands: vec!["stop dictation".to_owned()], start, end }
    }

    #[test]
    fn should_take_a_phrase_said_alone_as_filling_its_segment() {
        assert!(report(1.1, 1.9).fills(Segment { start: 1.0, end: 2.1 }));
    }

    #[test]
    fn should_not_take_a_phrase_after_other_speech_as_filling_its_segment() {
        assert!(!report(2.58, 3.75).fills(Segment { start: 1.03, end: 3.94 }));
    }

    #[test]
    fn should_not_take_a_phrase_before_other_speech_as_filling_its_segment() {
        assert!(!report(1.0, 1.6).fills(Segment { start: 1.0, end: 3.0 }));
    }
}
