use std::time::{Duration, Instant};

use super::voice_command::{CommandPlacement, SpokenCommand, VoiceCommand};
use super::voice_start_timing::ReportTiming;

/// What the command spotting service reports about the audio it hears.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpottingEvent {
    SpeechStarted(Instant),
    SpeechEnded(Instant),
    PhraseSpotted {
        phrase: String,
        start: Instant,
        end: Instant,
        timing: Option<ReportTiming>,
    },
    /// Commands said without a pause around them, to carry out at once and in order:
    /// `start..end` spans their words.
    CommandsSaid {
        commands: Vec<VoiceCommand>,
        start: Instant,
        end: Instant,
        placement: CommandPlacement,
        timing: Option<ReportTiming>,
    },
    /// The speech segment `start..end` could not be confirmed: a command said in it is lost.
    ConfirmationFailed {
        start: Instant,
        end: Instant,
    },
}

/// Longest speech allowed before the phrase in its segment for the phrase to count as the
/// whole utterance.
const LEAD_MARGIN: Duration = Duration::from_millis(300);
/// Longest speech allowed after the phrase: the voice activity detector holds quiet speech
/// a little longer than the spotter places its end.
const TAIL_MARGIN: Duration = Duration::from_millis(500);

#[derive(Debug, Clone, Copy)]
struct Spotted {
    command: Option<VoiceCommand>,
    start: Instant,
    end: Instant,
    timing: Option<ReportTiming>,
}

#[derive(Debug, Clone, Copy)]
struct ClosedSegment {
    start: Instant,
    end: Instant,
    /// The command every phrase spotted in the segment named so far.
    command: Option<VoiceCommand>,
    /// Phrases of different commands were spotted: the utterance is no command.
    mixed: bool,
    /// The segment's command was already recognized.
    recognized: bool,
}

impl ClosedSegment {
    /// Counts one more spotted phrase; the command, the first time a phrase of the
    /// segment's one command fills the segment.
    fn take(&mut self, spotted: Spotted) -> Option<SpokenCommand> {
        match (self.command, spotted.command) {
            (_, None) => self.mixed = true,
            (None, Some(command)) if !self.mixed => self.command = Some(command),
            (Some(known), Some(command)) if known != command => self.mixed = true,
            _ => {}
        }
        if self.mixed || self.recognized {
            return None;
        }
        let spoken = fills(*self, spotted)?;
        self.recognized = true;
        Some(spoken)
    }
}

/// Turns spotting events into voice commands, keeping only a phrase spoken on its own:
/// the phrase fills its speech segment, with a pause before and after it.
#[derive(Debug, Default)]
pub struct VoiceCommandRecognizer {
    open_segment_start: Option<Instant>,
    spotted_in_open_segment: Vec<Spotted>,
    last_closed: Option<ClosedSegment>,
    /// Commands said without a pause were carried out up to here: a phrase of the same
    /// words, reported again by the keyword spotter or for the whole utterance, is no
    /// new command.
    carried_out_until: Option<Instant>,
}

impl VoiceCommandRecognizer {
    pub fn new() -> Self {
        Self::default()
    }

    /// The commands `event` completes, in the order to carry them out.
    pub fn feed(&mut self, event: SpottingEvent) -> Vec<SpokenCommand> {
        match event {
            SpottingEvent::SpeechStarted(at) => {
                self.open_segment_start = Some(at);
                self.spotted_in_open_segment.clear();
                self.last_closed = None;
                Vec::new()
            }
            SpottingEvent::SpeechEnded(at) => self.close_segment(at).into_iter().collect(),
            SpottingEvent::PhraseSpotted { phrase, start, end, timing } => {
                if self.carried_out_until.is_some_and(|until| start <= until) {
                    return Vec::new();
                }
                let spotted =
                    Spotted { command: VoiceCommand::from_phrase(&phrase), start, end, timing };
                self.spot(spotted).into_iter().collect()
            }
            SpottingEvent::CommandsSaid { commands, start, end, placement, timing } => {
                self.carried_out_until = Some(end);
                self.spotted_in_open_segment.retain(|s| s.start > end);
                commands
                    .into_iter()
                    .map(|command| SpokenCommand {
                        command,
                        started_at: start,
                        ends_at: end,
                        placement,
                        timing,
                    })
                    .collect()
            }
            SpottingEvent::ConfirmationFailed { .. } => Vec::new(),
        }
    }

    fn close_segment(&mut self, end: Instant) -> Option<SpokenCommand> {
        let start = self.open_segment_start.take()?;
        let spotted = std::mem::take(&mut self.spotted_in_open_segment);
        let mut segment =
            ClosedSegment { start, end, command: None, mixed: false, recognized: false };
        let commands: Vec<SpokenCommand> = spotted
            .into_iter()
            .filter_map(|s| segment.take(s))
            .collect();
        self.last_closed = Some(segment);
        if segment.mixed {
            return None;
        }
        commands.into_iter().next()
    }

    /// A phrase may be reported before or shortly after the end of its segment; one that
    /// began before the segment ended belongs to it, even if the spotter placed its end later.
    fn spot(&mut self, spotted: Spotted) -> Option<SpokenCommand> {
        if self.open_segment_start.is_some() {
            self.spotted_in_open_segment.push(spotted);
            return None;
        }
        let segment = self.last_closed.as_mut()?;
        if spotted.start > segment.end {
            return None;
        }
        segment.take(spotted)
    }
}

fn fills(segment: ClosedSegment, spotted: Spotted) -> Option<SpokenCommand> {
    let lead = spotted.start.saturating_duration_since(segment.start);
    let tail = segment.end.saturating_duration_since(spotted.end);
    if lead > LEAD_MARGIN || tail > TAIL_MARGIN {
        return None;
    }
    spotted.command.map(|command| SpokenCommand {
        command,
        started_at: spotted.start,
        ends_at: spotted.end,
        placement: CommandPlacement::Alone,
        timing: spotted.timing,
    })
}

#[cfg(test)]
mod tests;
