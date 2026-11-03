//! How long a voice start took to give its start cue, stage by stage, from the end of the
//! command's words: the line the app logs at every voice start.

use std::fmt;
use std::time::{Duration, Instant};

use super::voice_command::SpokenCommand;

/// How the report of a command came about, by the clock of the microphone capture: when the
/// command's words were captured and when the report reached the app.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReportTiming {
    pub words_ended: Instant,
    pub arrived_at: Instant,
}

/// The stages of one voice start; a stage its report did not tell is unknown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VoiceStartTiming {
    pub total: Duration,
    /// From the end of the command's words to its report's arrival.
    pub hear: Option<Duration>,
    pub cue: Option<Duration>,
    /// From the start cue to the overlay's recording sign on screen.
    pub overlay: Option<Duration>,
}

impl VoiceStartTiming {
    /// The stages of the start `spoken` gave its start cue for at `cue_at`, its overlay drawn
    /// at `drawn_at`.
    pub fn measure(spoken: &SpokenCommand, cue_at: Instant, drawn_at: Option<Instant>) -> Self {
        let overlay = drawn_at.map(|drawn| drawn.saturating_duration_since(cue_at));
        let Some(report) = spoken.timing else {
            return Self {
                total: cue_at.saturating_duration_since(spoken.ends_at),
                hear: None,
                cue: None,
                overlay,
            };
        };
        Self {
            total: cue_at.saturating_duration_since(report.words_ended),
            hear: Some(
                report
                    .arrived_at
                    .saturating_duration_since(report.words_ended),
            ),
            cue: Some(cue_at.saturating_duration_since(report.arrived_at)),
            overlay,
        }
    }
}

/// `total=640ms hear=300ms cue=110ms overlay=8ms`, `-` for an unknown stage.
impl fmt::Display for VoiceStartTiming {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let stage = |d: Option<Duration>| d.map_or_else(|| "-".to_owned(), ms);
        write!(
            f,
            "total={} hear={} cue={} overlay={}",
            ms(self.total),
            stage(self.hear),
            stage(self.cue),
            stage(self.overlay)
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
        // Given words captured until `end`, reported at 250 ms
        let end = Instant::now();
        let after = |ms: u64| end + Duration::from_millis(ms);
        let report = ReportTiming { words_ended: end, arrived_at: after(250) };
        // When the start cue comes at 300 ms
        let timing = VoiceStartTiming::measure(
            &start_ending_at(end, Some(report)),
            after(300),
            Some(after(310)),
        );
        // Then
        assert_eq!(
            timing.to_string(),
            "total=300ms hear=250ms cue=50ms overlay=10ms"
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
            None,
        );
        // Then
        assert_eq!(timing.to_string(), "total=400ms hear=- cue=- overlay=-");
    }
}
