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
    let port_taken = FailingPart::ServicePortTaken(ForeignProgram {
        port: 8081,
        program: "com.docker.bac (pid 4194304)".to_owned(),
    });
    for part in FailingPart::ALL.into_iter().chain([port_taken]) {
        notices.push(FailureNotice::DictationLost(part.clone()));
        notices.push(FailureNotice::PartOfDictationLost(part.clone()));
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
    let notice =
        FailureNotice::CommandFailed(VoiceCommand::PressEnter, Some(FailingPart::Accessibility));
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

#[test]
fn should_name_the_port_and_the_program_holding_it() {
    let part = FailingPart::ServicePortTaken(ForeignProgram {
        port: 8081,
        program: "app (pid 28344)".to_owned(),
    });
    let notice = FailureNotice::DictationLost(part);
    assert_eq!(
        notice.message(),
        "Dictation lost: port 8081 is held by app (pid 28344) — quit it"
    );
    assert_eq!(notice.part_name(), "STT");
}
