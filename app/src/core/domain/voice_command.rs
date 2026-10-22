use std::time::Instant;

use super::voice_start_timing::ReportTiming;

/// A spoken command of voice control, recognized from its English phrase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceCommand {
    StartDictation,
    StopDictation,
    CancelDictation,
    RestartDictation,
    PressEnter,
    ClearAllText,
    PasteText,
}

/// Where a command sat in its utterance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandPlacement {
    /// The whole utterance, with a pause before and after it.
    Alone,
    /// The utterance ended with it.
    Tail,
    /// "start" opened the utterance and speech followed.
    Head,
    /// A doubled short form or a chain of commands, wherever it sat.
    Anywhere,
}

/// A voice command together with when its words were said and where they sat.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpokenCommand {
    pub command: VoiceCommand,
    pub started_at: Instant,
    pub ends_at: Instant,
    pub placement: CommandPlacement,
    /// How the report of the command came about; none when the service did not tell.
    pub timing: Option<ReportTiming>,
}

impl VoiceCommand {
    /// Commands accepted while listening with no dictation running.
    pub const STANDBY: [Self; 4] = [
        Self::StartDictation,
        Self::PressEnter,
        Self::ClearAllText,
        Self::PasteText,
    ];

    /// Commands accepted during a dictation; "press enter" there also ends it.
    pub const DICTATING: [Self; 4] = [
        Self::StopDictation,
        Self::CancelDictation,
        Self::RestartDictation,
        Self::PressEnter,
    ];

    /// Every command, in the order of the spec.
    pub const ALL: [Self; 7] = [
        Self::StartDictation,
        Self::StopDictation,
        Self::CancelDictation,
        Self::RestartDictation,
        Self::PressEnter,
        Self::ClearAllText,
        Self::PasteText,
    ];

    pub fn phrase(self) -> &'static str {
        match self {
            Self::StartDictation => "start dictation",
            Self::StopDictation => "stop dictation",
            Self::CancelDictation => "cancel dictation",
            Self::RestartDictation => "restart dictation",
            Self::PressEnter => "press enter",
            Self::ClearAllText => "clear all text",
            Self::PasteText => "paste text",
        }
    }

    pub fn from_phrase(phrase: &str) -> Option<Self> {
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
            "restart dictation",
            "press enter",
            "clear all text",
            "paste text",
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
    fn should_accept_start_press_enter_clear_and_paste_in_standby() {
        assert_eq!(
            VoiceCommand::STANDBY.map(VoiceCommand::phrase),
            [
                "start dictation",
                "press enter",
                "clear all text",
                "paste text"
            ]
        );
    }

    #[test]
    fn should_accept_stop_cancel_restart_and_enter_during_a_dictation() {
        assert_eq!(
            VoiceCommand::DICTATING.map(VoiceCommand::phrase),
            [
                "stop dictation",
                "cancel dictation",
                "restart dictation",
                "press enter"
            ]
        );
    }
}
