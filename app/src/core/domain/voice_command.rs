use std::time::Instant;

/// A spoken command of voice control, recognized from its English phrase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceCommand {
    StartDictation,
    StopDictation,
    CancelDictation,
    PressEnter,
    ClearAllText,
}

/// A voice command together with the moment its phrase started.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpokenCommand {
    pub command: VoiceCommand,
    pub started_at: Instant,
}

impl VoiceCommand {
    /// Commands accepted while listening with no dictation running.
    pub const STANDBY: [VoiceCommand; 3] = [
        VoiceCommand::StartDictation,
        VoiceCommand::PressEnter,
        VoiceCommand::ClearAllText,
    ];

    /// Commands accepted during a dictation.
    pub const DICTATING: [VoiceCommand; 2] =
        [VoiceCommand::StopDictation, VoiceCommand::CancelDictation];

    /// Every command, in the order of the spec.
    pub const ALL: [VoiceCommand; 5] = [
        VoiceCommand::StartDictation,
        VoiceCommand::StopDictation,
        VoiceCommand::CancelDictation,
        VoiceCommand::PressEnter,
        VoiceCommand::ClearAllText,
    ];

    pub fn phrase(self) -> &'static str {
        match self {
            VoiceCommand::StartDictation => "start dictation",
            VoiceCommand::StopDictation => "stop dictation",
            VoiceCommand::CancelDictation => "cancel dictation",
            VoiceCommand::PressEnter => "press enter",
            VoiceCommand::ClearAllText => "clear all text",
        }
    }

    pub fn from_phrase(phrase: &str) -> Option<VoiceCommand> {
        Self::ALL.into_iter().find(|c| c.phrase() == phrase)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_map_each_command_to_its_phrase() {
        // Given
        let expected = [
            "start dictation",
            "stop dictation",
            "cancel dictation",
            "press enter",
            "clear all text",
        ];
        // When
        let phrases: Vec<&str> = VoiceCommand::ALL.iter().map(|c| c.phrase()).collect();
        // Then
        assert_eq!(phrases, expected);
    }

    #[test]
    fn should_map_each_phrase_back_to_its_command() {
        for command in VoiceCommand::ALL {
            assert_eq!(VoiceCommand::from_phrase(command.phrase()), Some(command));
        }
    }

    #[test]
    fn should_map_an_unknown_phrase_to_no_command() {
        assert_eq!(VoiceCommand::from_phrase("open the door"), None);
    }

    #[test]
    fn should_accept_start_press_enter_and_clear_in_standby() {
        assert_eq!(
            VoiceCommand::STANDBY.map(VoiceCommand::phrase),
            ["start dictation", "press enter", "clear all text"]
        );
    }

    #[test]
    fn should_accept_stop_and_cancel_during_a_dictation() {
        assert_eq!(
            VoiceCommand::DICTATING.map(VoiceCommand::phrase),
            ["stop dictation", "cancel dictation"]
        );
    }
}
