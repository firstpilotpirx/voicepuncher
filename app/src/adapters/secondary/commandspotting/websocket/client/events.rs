//! The service's messages as it sends them, and the spotting events they become.

use std::time::{Duration, Instant};

use serde::Deserialize;

use crate::core::domain::{CommandPlacement, ReportTiming, SpottingEvent, VoiceCommand};

#[derive(Debug, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub(super) enum ServiceEvent {
    SpeechStart {
        t: f32,
    },
    SpeechEnd {
        t: f32,
    },
    Command {
        commands: Vec<String>,
        start: f32,
        end: f32,
        placement: Placement,
    },
    Synced {
        id: u64,
    },
    /// The service dropped audio it could not keep up with: the stream's clock is broken.
    Dropped {
        samples: u64,
    },
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Placement {
    Tail,
    Head,
    Anywhere,
}

/// A service event with its times, seconds since the first sample, as instants from
/// `audio_start`, the moment that sample was captured; a command tells how its report came
/// about, which reached the app at `arrived_at`.
pub(super) fn to_spotting_event(
    event: ServiceEvent,
    audio_start: Instant,
    arrived_at: Instant,
) -> Option<SpottingEvent> {
    let times = Times { audio_start, arrived_at };
    Some(match event {
        ServiceEvent::SpeechStart { t } => SpottingEvent::SpeechStarted(times.at(t)),
        ServiceEvent::SpeechEnd { t } => SpottingEvent::SpeechEnded(times.at(t)),
        ServiceEvent::Command { commands, start, end, placement } => SpottingEvent::CommandsSaid {
            commands: commands
                .iter()
                .filter_map(|p| VoiceCommand::from_phrase(p))
                .collect(),
            start: times.at(start),
            end: times.at(end),
            placement: placement.into(),
            timing: Some(times.report(end)),
        },
        ServiceEvent::Synced { .. } | ServiceEvent::Dropped { .. } => return None,
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
    fn report(&self, end: f32) -> ReportTiming {
        ReportTiming { words_ended: self.at(end), arrived_at: self.arrived_at }
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
