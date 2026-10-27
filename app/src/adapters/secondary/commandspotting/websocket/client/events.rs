//! The service's messages as it sends them, and the spotting events they become.

use std::time::{Duration, Instant};

use serde::Deserialize;

use crate::core::domain::{
    CommandPlacement, ConfirmationTiming, ReportTiming, SpottingEvent, VoiceCommand,
};

#[derive(Debug, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub(super) enum ServiceEvent {
    SpeechStart {
        t: f32,
    },
    SpeechEnd {
        t: f32,
    },
    Phrase {
        phrase: String,
        start: f32,
        end: f32,
        timing: Option<Confirmation>,
    },
    Command {
        commands: Vec<String>,
        start: f32,
        end: f32,
        placement: Placement,
        timing: Option<Confirmation>,
    },
    Synced {
        id: u64,
    },
    /// What the speech recognition service heard of commands reported without confirmation.
    Heard {
        commands: Vec<String>,
        verdict: Verdict,
    },
    /// The speech `start..end` could not be confirmed: a command said in it is lost.
    ConfirmationFailed {
        start: f32,
        end: f32,
    },
    /// A confirmation of the service hangs: the connection is to be left.
    Stalled {
        waited_ms: u64,
    },
    /// The service dropped audio it could not keep up with: the stream's clock is broken.
    Dropped {
        samples: u64,
    },
}

/// The confirmation that heard a phrase or command: the service time it was asked at, and
/// how long it took.
#[derive(Debug, Deserialize)]
pub(super) struct Confirmation {
    asked_at: f32,
    took_ms: u64,
}

/// What the speech recognition service made of a command reported without confirmation.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Verdict {
    Agreed,
    Other,
    None,
}

impl Verdict {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Agreed => "agreed",
            Self::Other => "other",
            Self::None => "none",
        }
    }
}

/// The log line of a verdict on commands reported without confirmation; none for any other
/// event.
pub(super) fn verdict_line(event: &ServiceEvent) -> Option<String> {
    match event {
        ServiceEvent::Heard { commands, verdict } => Some(format!(
            "whisper on unconfirmed {commands:?}: {}",
            verdict.as_str()
        )),
        _ => None,
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Placement {
    Tail,
    Head,
    Anywhere,
}

/// A service event with its times, seconds since the first sample, as instants from
/// `audio_start`, the moment that sample was captured; a phrase or command tells how its
/// report came about, which reached the app at `arrived_at`.
pub(super) fn to_spotting_event(
    event: ServiceEvent,
    audio_start: Instant,
    arrived_at: Instant,
) -> Option<SpottingEvent> {
    let times = Times { audio_start, arrived_at };
    Some(match event {
        ServiceEvent::SpeechStart { t } => SpottingEvent::SpeechStarted(times.at(t)),
        ServiceEvent::SpeechEnd { t } => SpottingEvent::SpeechEnded(times.at(t)),
        ServiceEvent::Phrase { phrase, start, end, timing } => SpottingEvent::PhraseSpotted {
            phrase,
            start: times.at(start),
            end: times.at(end),
            timing: Some(times.report(end, timing)),
        },
        ServiceEvent::Command { commands, start, end, placement, timing } => {
            SpottingEvent::CommandsSaid {
                commands: commands
                    .iter()
                    .filter_map(|p| VoiceCommand::from_phrase(p))
                    .collect(),
                start: times.at(start),
                end: times.at(end),
                placement: placement.into(),
                timing: Some(times.report(end, timing)),
            }
        }
        ServiceEvent::ConfirmationFailed { start, end } => {
            SpottingEvent::ConfirmationFailed { start: times.at(start), end: times.at(end) }
        }
        ServiceEvent::Synced { .. }
        | ServiceEvent::Heard { .. }
        | ServiceEvent::Stalled { .. }
        | ServiceEvent::Dropped { .. } => return None,
    })
}

/// The instants of one service event's times.
struct Times {
    audio_start: Instant,
    arrived_at: Instant,
}

impl Times {
    /// `secs` since the first sample as an instant; a time before it is the first sample's.
    fn at(&self, secs: f32) -> Instant {
        self.audio_start + Duration::from_secs_f32(secs.max(0.0))
    }

    /// How a report whose words ended at `end` came about.
    fn report(&self, end: f32, confirmation: Option<Confirmation>) -> ReportTiming {
        ReportTiming {
            words_ended: self.at(end),
            confirmation: confirmation.map(|c| self.confirmation(&c)),
            arrived_at: self.arrived_at,
        }
    }

    fn confirmation(&self, confirmation: &Confirmation) -> ConfirmationTiming {
        ConfirmationTiming {
            asked_at: self.at(confirmation.asked_at),
            took: Duration::from_millis(confirmation.took_ms),
        }
    }
}

impl From<Placement> for CommandPlacement {
    fn from(placement: Placement) -> Self {
        match placement {
            Placement::Tail => Self::Tail,
            Placement::Head => Self::Head,
            Placement::Anywhere => Self::Anywhere,
        }
    }
}
