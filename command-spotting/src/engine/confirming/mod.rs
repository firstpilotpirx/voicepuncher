//! The confirmations of a session: what each asks the speech recognition service, and what
//! its answer makes of the utterance once it arrives.

use std::sync::Arc;
use std::time::Instant;

use crate::matcher::{Found, PhraseMatcher};
use crate::orderer::{Segment, Spotted};
use crate::protocol::{ServerEvent, Timing};
use crate::worker::JobKind;

use super::spotted::{Round, CHECKED_WHEN_SPOTTED};
use super::{samples_in, seconds};

/// How much of an utterance's end is heard again when a quiet pause closed it and no command
/// was heard in it: in the whole utterance the speech recognition may join a command to the
/// words before it ("at ten enter" heard as "at 10am").
const TAIL_CHECK_SECS: f32 = 1.5;
/// How many deadlines `settle` waits for one answer: a segment's confirmation may be tried
/// twice, and each attempt may take its deadline.
const SETTLE_DEADLINES: u32 = 3;

/// What a confirmation asked for, kept until its answer is applied.
pub(super) enum JobContext {
    /// A check of the utterance's start; `early`: asked before the start could be heard
    /// alone, so only a command opening speech counts.
    Head {
        utterance: u64,
        from: usize,
        early: bool,
    },
    Window {
        utterance: u64,
        from: usize,
    },
    /// A check asked because the keyword spotter heard "start" or "restart" ending at sample
    /// `said_end`; `at_head`: its audio starts where the utterance did.
    Spotted {
        utterance: u64,
        from: usize,
        at_head: bool,
        round: Round,
        said_end: usize,
    },
    Segment {
        utterance: u64,
        segment: SegmentSpan,
    },
    Unheld {
        spotted: Spotted,
        from: usize,
    },
    Tail {
        utterance: u64,
        from: usize,
    },
    /// A short utterance in standby heard at half its pause, before its segment closed.
    HalfPause {
        utterance: u64,
    },
    /// A short segment heard as a near miss of `phrase`, heard again through a clip whose
    /// first sample stands for stream position `origin`.
    Rehear {
        utterance: u64,
        bounds: Segment,
        origin: usize,
        phrase: String,
    },
}

/// A confirmation asked for: what for, how far the stream had come, and when.
pub(super) struct AskedJob {
    context: JobContext,
    asked_at: usize,
    asked: Instant,
    /// The phrases listened for when the job was asked.
    matcher: Arc<PhraseMatcher>,
}

impl AskedJob {
    fn timing(&self) -> Timing {
        Timing {
            asked_at: seconds(self.asked_at),
            took_ms: u64::try_from(self.asked.elapsed().as_millis()).unwrap_or(u64::MAX),
        }
    }
}

/// A closed speech segment by sample position, and the audio sent to confirm it.
pub(super) struct SegmentSpan {
    end: usize,
    bounds: Segment,
    short: bool,
    from: usize,
    to: usize,
    /// At or below this loudness a frame of the segment is quiet.
    quiet_level: f32,
    /// A quiet pause closed it, and it ran long enough for its tail to be heard again.
    tail_check: bool,
}

mod answer;
mod ask;
mod half_pause;
mod heard;
mod place;
mod rehear;
mod segment;
mod unconfirmed;
mod unheld;

pub(super) use half_pause::HalfPauseHearing;
use rehear::EarlyAnswer;
pub(super) use rehear::{EarlyRehears, Rehear};
pub(super) use segment::ClosedSegment;
pub(super) use unconfirmed::Unconfirmed;

impl JobContext {
    fn utterance(&self) -> Option<u64> {
        match self {
            Self::Head { utterance, .. }
            | Self::Window { utterance, .. }
            | Self::Spotted { utterance, .. }
            | Self::Segment { utterance, .. }
            | Self::Tail { utterance, .. }
            | Self::HalfPause { utterance }
            | Self::Rehear { utterance, .. } => Some(*utterance),
            Self::Unheld { .. } => None,
        }
    }

    /// The report of a closed segment whose confirmation got no answer; none for any other
    /// confirmation, as the segment's own decides.
    fn failed(&self) -> Option<ServerEvent> {
        match self {
            Self::Segment { segment, .. } => Some(ServerEvent::ConfirmationFailed {
                start: segment.bounds.start,
                end: segment.bounds.end,
            }),
            _ => None,
        }
    }

    /// The kind of job this confirmation is asked as.
    fn kind(&self) -> JobKind {
        match self {
            Self::Head { .. } => JobKind::Head,
            Self::Window { .. } => JobKind::Window,
            Self::Spotted { .. } => JobKind::Spotted,
            Self::Segment { .. } | Self::HalfPause { .. } => JobKind::Segment,
            Self::Unheld { .. } => JobKind::Unheld,
            Self::Tail { .. } => JobKind::Tail,
            Self::Rehear { .. } => JobKind::Rehear,
        }
    }

    fn is_check(&self) -> bool {
        self.kind().is_check()
    }

    /// What a second hearing of this short segment needs, before any phrase is known.
    fn short_segment(&self) -> Option<Rehear> {
        match self {
            Self::Segment { segment, .. } if segment.short => self.rehear(String::new()),
            _ => None,
        }
    }

    /// What a second hearing of this segment as a near miss of `phrase` needs.
    fn rehear(&self, phrase: String) -> Option<Rehear> {
        match self {
            Self::Segment { utterance, segment } => Some(Rehear {
                utterance: *utterance,
                bounds: segment.bounds,
                from: segment.from,
                to: segment.to,
                quiet_level: segment.quiet_level,
                phrase,
            }),
            _ => None,
        }
    }

    /// The utterance and the audio to hear again when this segment's answer holds no
    /// command.
    fn tail_check(&self) -> Option<(u64, usize, usize)> {
        match self {
            Self::Segment { utterance, segment } if segment.tail_check => Some((
                *utterance,
                segment.end.saturating_sub(samples_in(TAIL_CHECK_SECS)),
                segment.to,
            )),
            _ => None,
        }
    }
}

/// The command the start of a running utterance says: a doubled start or a chain ending in
/// one, "start" opening it, or "start" heard alone so far, since the utterance goes on and
/// has speech after it.
fn head_command(matcher: &PhraseMatcher, words: &[&str]) -> Option<Found> {
    starting_anywhere(matcher, words)
        .or_else(|| matcher.start_at_head(words))
        .or_else(|| matcher.start_so_far(words))
}

/// The command a check asked by a spotted "start" or "restart" hears: a doubled command or
/// a chain anywhere, and in audio from the utterance's start also what the head check takes.
fn spotted_command(
    matcher: &PhraseMatcher,
    words: &[&str],
    at_head: bool,
    round: Round,
) -> Option<Found> {
    match round {
        Round::Doubled => return matcher.anywhere(words),
        Round::Short => return matcher.anywhere(words).filter(|f| f.commands.len() == 1),
        Round::First | Round::Second => {}
    }
    let found = starting_anywhere(matcher, words);
    if !at_head {
        return found;
    }
    // "start" heard alone may be cut off before the word after it: "start stop" starts nothing.
    found
        .or_else(|| matcher.start_at_head(words))
        .or_else(|| (round == Round::Second).then(|| matcher.start_so_far(words))?)
}

/// A doubled command or a chain in `words` that starts a dictation: other ones are left to
/// the checks of the utterance's last seconds and its segment, which the keyword spotter's
/// phrase of the segment does not repeat.
fn starting_anywhere(matcher: &PhraseMatcher, words: &[&str]) -> Option<Found> {
    matcher.anywhere(words).filter(|found| {
        found
            .commands
            .last()
            .is_some_and(|last| CHECKED_WHEN_SPOTTED.contains(&last.as_str()))
    })
}

/// `event` with the timing of the confirmation that heard it, when it names a command.
fn timed(event: ServerEvent, confirmation: Timing) -> ServerEvent {
    match event {
        ServerEvent::Phrase { phrase, start, end, .. } => {
            ServerEvent::Phrase { phrase, start, end, timing: Some(confirmation) }
        }
        ServerEvent::Command { commands, start, end, placement, .. } => {
            ServerEvent::Command { commands, start, end, placement, timing: Some(confirmation) }
        }
        other => other,
    }
}

#[cfg(test)]
mod tests;
