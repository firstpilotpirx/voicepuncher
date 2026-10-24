use super::permission::Permission;
use super::voice_command::VoiceCommand;

/// The part whose failure an action the user started ran into.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailingPart {
    SpeechRecognition,
    CommandSpotting,
    Microphone,
    Accessibility,
}

impl FailingPart {
    pub const ALL: [Self; 4] = [
        Self::SpeechRecognition,
        Self::CommandSpotting,
        Self::Microphone,
        Self::Accessibility,
    ];

    /// The short uppercase name the overlay pill shows.
    pub fn short_name(self) -> &'static str {
        match self {
            Self::SpeechRecognition => "STT",
            Self::CommandSpotting => "VOICE",
            Self::Microphone => "MIC",
            Self::Accessibility => "ACCESS",
        }
    }

    fn failure(self) -> &'static str {
        match self {
            Self::SpeechRecognition => "speech recognition failed",
            Self::CommandSpotting => "command spotting failed",
            Self::Microphone => "the microphone failed",
            Self::Accessibility => "Accessibility permission missing",
        }
    }

    fn remedy(self) -> &'static str {
        match self {
            Self::SpeechRecognition | Self::CommandSpotting => "start the local services",
            Self::Microphone => "check the microphone and its permission",
            Self::Accessibility => "check the Accessibility permission",
        }
    }

    fn cause(self) -> String {
        format!("{} — {}", self.failure(), self.remedy())
    }
}

const PILL_LABEL_WITHOUT_PART: &str = "FAILED";

/// What the overlay tells the user when an action failed: what was lost and which part failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureNotice {
    DictationLost(FailingPart),
    PartOfDictationLost(FailingPart),
    NotPasted(Option<FailingPart>),
    RecordingNotStarted,
    CommandFailed(VoiceCommand, Option<FailingPart>),
    VoiceCommandsUnavailable,
    /// The runtime cancelled the dictation because the user revoked the permission.
    PermissionRevoked(Permission),
}

impl FailureNotice {
    pub fn part(&self) -> Option<FailingPart> {
        match *self {
            Self::DictationLost(part) | Self::PartOfDictationLost(part) => Some(part),
            Self::NotPasted(part) | Self::CommandFailed(_, part) => part,
            Self::RecordingNotStarted => Some(FailingPart::Microphone),
            Self::VoiceCommandsUnavailable => Some(FailingPart::CommandSpotting),
            Self::PermissionRevoked(_) => None,
        }
    }

    /// The short sentence the overlay shows: what was lost, what failed and, when known, the remedy.
    pub fn message(&self) -> String {
        let loss: String = self.loss();
        match (self.part(), self) {
            (Some(part), _) => format!("{loss}: {}", part.cause()),
            (None, Self::NotPasted(_)) => format!("{loss}: pasting failed"),
            (None, Self::PermissionRevoked(permission)) => {
                format!("{loss}: {permission} permission was revoked")
            }
            (None, _) => loss,
        }
    }

    /// The short uppercase label the overlay pill shows next to the warning sign.
    pub fn part_name(&self) -> &'static str {
        self.part()
            .map_or(PILL_LABEL_WITHOUT_PART, FailingPart::short_name)
    }

    fn loss(&self) -> String {
        let loss: &str = match self {
            Self::DictationLost(_) => "Dictation lost",
            Self::PartOfDictationLost(_) => "Part of the dictation lost",
            Self::NotPasted(_) => "Text not pasted",
            Self::RecordingNotStarted => "Recording not started",
            Self::CommandFailed(command, _) => return format!("\"{}\" failed", command.phrase()),
            Self::VoiceCommandsUnavailable => "Voice commands unavailable",
            Self::PermissionRevoked(_) => "Dictation cancelled",
        };
        loss.to_owned()
    }
}

/// The process is not trusted for accessibility, so macOS would drop the events it posts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("the Accessibility permission is missing")]
pub struct AccessibilityMissing;

#[cfg(test)]
mod tests {
    use super::*;

    const OVERLAY_TEXT_LIMIT: usize = 110;
    const PART_NAME_LIMIT: usize = 7;

    fn every_notice() -> Vec<FailureNotice> {
        let mut notices: Vec<FailureNotice> = vec![
            FailureNotice::PermissionRevoked(Permission::InputMonitoring),
            FailureNotice::NotPasted(None),
            FailureNotice::RecordingNotStarted,
            FailureNotice::VoiceCommandsUnavailable,
        ];
        for part in FailingPart::ALL {
            notices.push(FailureNotice::DictationLost(part));
            notices.push(FailureNotice::PartOfDictationLost(part));
            notices.push(FailureNotice::NotPasted(Some(part)));
        }
        for command in VoiceCommand::ALL {
            notices.push(FailureNotice::CommandFailed(command, None));
            for part in FailingPart::ALL {
                notices.push(FailureNotice::CommandFailed(command, Some(part)));
            }
        }
        notices
    }

    #[test]
    fn should_name_the_revoked_permission() {
        let notice = FailureNotice::PermissionRevoked(Permission::Microphone);
        assert_eq!(
            notice.message(),
            "Dictation cancelled: Microphone permission was revoked"
        );
    }

    #[test]
    fn should_name_a_lost_dictation_with_the_service_remedy() {
        let notice = FailureNotice::DictationLost(FailingPart::SpeechRecognition);
        assert_eq!(
            notice.message(),
            "Dictation lost: speech recognition failed — start the local services"
        );
    }

    #[test]
    fn should_name_a_lost_part_of_the_dictation() {
        let notice = FailureNotice::PartOfDictationLost(FailingPart::SpeechRecognition);
        assert_eq!(
            notice.message(),
            "Part of the dictation lost: speech recognition failed — start the local services"
        );
    }

    #[test]
    fn should_name_a_refused_paste_with_the_accessibility_remedy() {
        let notice = FailureNotice::NotPasted(Some(FailingPart::Accessibility));
        assert_eq!(
            notice.message(),
            "Text not pasted: Accessibility permission missing — check the Accessibility permission"
        );
    }

    #[test]
    fn should_name_a_failed_paste_without_a_part() {
        assert_eq!(
            FailureNotice::NotPasted(None).message(),
            "Text not pasted: pasting failed"
        );
    }

    #[test]
    fn should_name_a_recording_that_did_not_start_with_the_microphone_remedy() {
        assert_eq!(
            FailureNotice::RecordingNotStarted.message(),
            "Recording not started: the microphone failed — check the microphone and its permission"
        );
    }

    #[test]
    fn should_name_a_failed_command_by_its_phrase() {
        let notice = FailureNotice::CommandFailed(
            VoiceCommand::PressEnter,
            Some(FailingPart::Accessibility),
        );
        assert_eq!(
            notice.message(),
            "\"press enter\" failed: Accessibility permission missing — check the Accessibility permission"
        );
    }

    #[test]
    fn should_name_a_failed_command_without_a_part() {
        let notice = FailureNotice::CommandFailed(VoiceCommand::ClearAllText, None);
        assert_eq!(notice.message(), "\"clear all text\" failed");
    }

    #[test]
    fn should_name_unavailable_voice_commands_with_the_service_remedy() {
        assert_eq!(
            FailureNotice::VoiceCommandsUnavailable.message(),
            "Voice commands unavailable: command spotting failed — start the local services"
        );
    }

    #[test]
    fn should_fit_every_message_into_two_overlay_lines() {
        for notice in every_notice() {
            let length = notice.message().chars().count();
            assert!(length <= OVERLAY_TEXT_LIMIT, "{notice:?} is {length} chars");
        }
    }

    #[test]
    fn should_label_the_pill_with_the_failing_part() {
        let labels: [&str; 4] =
            FailingPart::ALL.map(|part| FailureNotice::DictationLost(part).part_name());
        assert_eq!(labels, ["STT", "VOICE", "MIC", "ACCESS"]);
    }

    #[test]
    fn should_label_the_pill_failed_without_a_part() {
        assert_eq!(FailureNotice::NotPasted(None).part_name(), "FAILED");
    }

    #[test]
    fn should_keep_every_pill_label_short() {
        for notice in every_notice() {
            assert!(
                notice.part_name().chars().count() <= PART_NAME_LIMIT,
                "{notice:?}"
            );
        }
    }

    #[test]
    fn should_tie_fixed_notices_to_their_part() {
        assert_eq!(
            FailureNotice::RecordingNotStarted.part(),
            Some(FailingPart::Microphone)
        );
        assert_eq!(
            FailureNotice::VoiceCommandsUnavailable.part(),
            Some(FailingPart::CommandSpotting)
        );
    }

    #[test]
    fn should_describe_the_missing_accessibility_permission() {
        assert_eq!(
            AccessibilityMissing.to_string(),
            "the Accessibility permission is missing"
        );
    }
}
