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
    let at = |secs: f32| audio_start + Duration::from_secs_f32(secs.max(0.0));
    let report = |end: f32, confirmation: Option<Confirmation>| {
        Some(ReportTiming {
            words_ended: at(end),
            confirmation: confirmation.map(|c| ConfirmationTiming {
                asked_at: at(c.asked_at),
                took: Duration::from_millis(c.took_ms),
            }),
            arrived_at,
        })
    };
    Some(match event {
        ServiceEvent::SpeechStart { t } => SpottingEvent::SpeechStarted(at(t)),
        ServiceEvent::SpeechEnd { t } => SpottingEvent::SpeechEnded(at(t)),
        ServiceEvent::Phrase { phrase, start, end, timing } => SpottingEvent::PhraseSpotted {
            phrase,
            start: at(start),
            end: at(end),
            timing: report(end, timing),
        },
        ServiceEvent::Command { commands, start, end, placement, timing } => {
            SpottingEvent::CommandsSaid {
                commands: commands
                    .iter()
                    .filter_map(|phrase| VoiceCommand::from_phrase(phrase))
                    .collect(),
                start: at(start),
                end: at(end),
                placement: placement.into(),
                timing: report(end, timing),
            }
        }
        ServiceEvent::ConfirmationFailed { start, end } => {
            SpottingEvent::ConfirmationFailed { start: at(start), end: at(end) }
        }
        ServiceEvent::Synced { .. }
        | ServiceEvent::Stalled { .. }
        | ServiceEvent::Dropped { .. } => return None,
    })
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
