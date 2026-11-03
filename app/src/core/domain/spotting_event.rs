use std::time::Instant;

use super::voice_command::{CommandPlacement, SpokenCommand, VoiceCommand};
use super::voice_start_timing::ReportTiming;

/// What the command spotting service reports about the audio it hears.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpottingEvent {
    SpeechStarted(Instant),
    SpeechEnded(Instant),
    /// Commands said without a pause around them, to carry out at once and in order:
    /// `start..end` spans their words.
    CommandsSaid {
        commands: Vec<VoiceCommand>,
        start: Instant,
        end: Instant,
        placement: CommandPlacement,
        timing: Option<ReportTiming>,
    },
}

impl SpottingEvent {
    /// The commands the event reports, in the order to carry them out.
    #[must_use]
    pub fn into_commands(self) -> Vec<SpokenCommand> {
        let Self::CommandsSaid { commands, start, end, placement, timing } = self else {
            return Vec::new();
        };
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
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn should_carry_out_commands_said_without_a_pause_at_once_in_order() {
        // Given
        let start = Instant::now();
        let end = start + Duration::from_millis(700);
        let said = SpottingEvent::CommandsSaid {
            commands: vec![VoiceCommand::StopDictation, VoiceCommand::PressEnter],
            start,
            end,
            placement: CommandPlacement::Anywhere,
            timing: None,
        };
        // When
        let commands = said.into_commands();
        // Then
        let carried: Vec<(VoiceCommand, Instant, Instant)> = commands
            .iter()
            .map(|c| (c.command, c.started_at, c.ends_at))
            .collect();
        assert_eq!(
            carried,
            [
                (VoiceCommand::StopDictation, start, end),
                (VoiceCommand::PressEnter, start, end)
            ]
        );
    }

    #[test]
    fn should_carry_out_nothing_at_the_bounds_of_speech() {
        // When
        let commands = SpottingEvent::SpeechEnded(Instant::now()).into_commands();
        // Then
        assert!(commands.is_empty(), "{commands:?}");
    }
}
