//! How long a voice start took to give its start cue, stage by stage, from the end of the
//! command's words: the line the app logs at every voice start.

use std::fmt;
use std::time::{Duration, Instant};

use super::voice_command::SpokenCommand;

/// How the report of a command came about, by the clock of the microphone capture: when the
/// command's words were captured, the confirmation that heard them, and when the report
/// reached the app.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReportTiming {
    pub words_ended: Instant,
    pub confirmation: Option<ConfirmationTiming>,
    pub arrived_at: Instant,
}

/// The confirmation that heard a command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfirmationTiming {
    /// When the audio the stream had reached as the confirmation was asked was captured.
    pub asked_at: Instant,
    pub took: Duration,
}

/// The stages of one voice start; a stage its report did not tell is unknown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VoiceStartTiming {
    pub total: Duration,
    pub hear: Option<Duration>,
    pub confirm: Option<Duration>,
    pub deliver: Option<Duration>,
    pub cue: Option<Duration>,
}

impl VoiceStartTiming {
    /// The stages of the start `spoken` gave its start cue for at `cue_at`.
    pub fn measure(spoken: &SpokenCommand, cue_at: Instant) -> Self {
        let Some(report) = spoken.timing else {
            return Self {
                total: cue_at.saturating_duration_since(spoken.ends_at),
                hear: None,
                confirm: None,
                deliver: None,
                cue: None,
            };
        };
        let confirmation = report.confirmation;
        Self {
            total: cue_at.saturating_duration_since(report.words_ended),
            hear: confirmation.map(|c| c.asked_at.saturating_duration_since(report.words_ended)),
            confirm: confirmation.map(|c| c.took),
            deliver: confirmation.map(|c| {
                report
                    .arrived_at
                    .saturating_duration_since(c.asked_at + c.took)
            }),
            cue: Some(cue_at.saturating_duration_since(report.arrived_at)),
        }
    }
}

/// `total=640ms hear=300ms confirm=210ms deliver=20ms cue=110ms`, `-` for an unknown stage.
impl fmt::Display for VoiceStartTiming {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let stage = |d: Option<Duration>| d.map_or_else(|| "-".to_owned(), ms);
        write!(
            f,
            "total={} hear={} confirm={} deliver={} cue={}",
            ms(self.total),
            stage(self.hear),
            stage(self.confirm),
            stage(self.deliver),
            stage(self.cue)
        )
    }
}

fn ms(duration: Duration) -> String {
    format!("{}ms", duration.as_millis())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::domain::{CommandPlacement, VoiceCommand};

    fn start_ending_at(ends_at: Instant, timing: Option<ReportTiming>) -> SpokenCommand {
        SpokenCommand {
            command: VoiceCommand::StartDictation,
            started_at: ends_at,
            ends_at,
            placement: CommandPlacement::Anywhere,
            timing,
        }
    }

    #[test]
    fn should_measure_each_stage_from_the_end_of_the_words() {
        // Given words captured until `end`, confirmed 300 ms later in 200 ms, reported at 550 ms
        let end = Instant::now();
        let after = |ms: u64| end + Duration::from_millis(ms);
        let report = ReportTiming {
            words_ended: end,
            confirmation: Some(ConfirmationTiming {
                asked_at: after(300),
                took: Duration::from_millis(200),
            }),
            arrived_at: after(550),
        };
        // When the start cue comes at 600 ms
        let timing = VoiceStartTiming::measure(&start_ending_at(end, Some(report)), after(600));
        // Then
        assert_eq!(
            timing.to_string(),
            "total=600ms hear=300ms confirm=200ms deliver=50ms cue=50ms"
        );
    }

    #[test]
    fn should_leave_the_stages_of_an_unconfirmed_report_unknown() {
        // Given
        let end = Instant::now();
        let report = ReportTiming {
            words_ended: end,
            confirmation: None,
            arrived_at: end + Duration::from_millis(250),
        };
        // When
        let timing = VoiceStartTiming::measure(
            &start_ending_at(end, Some(report)),
            end + Duration::from_millis(300),
        );
        // Then
        assert_eq!(
            timing.to_string(),
            "total=300ms hear=- confirm=- deliver=- cue=50ms"
        );
    }

    #[test]
    fn should_measure_only_the_total_without_a_report() {
        // Given
        let end = Instant::now();
        // When
        let timing = VoiceStartTiming::measure(
            &start_ending_at(end, None),
            end + Duration::from_millis(400),
        );
        // Then
        assert_eq!(
            timing.to_string(),
            "total=400ms hear=- confirm=- deliver=- cue=-"
        );
    }
}
