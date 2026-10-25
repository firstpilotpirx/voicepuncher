//! Commands found at the end, at the head and anywhere in an utterance.

use super::*;

#[test]
pub(super) fn should_find_a_command_that_ends_the_utterance() {
    assert_eq!(
        dictating().at_end(&words("We meet on Friday. Stop.")),
        Some(found(&[("stop dictation", 4, 4)], Placement::Tail))
    );
    assert_eq!(
        dictating().at_end(&words("that is all stop dictation please")),
        Some(found(&[("stop dictation", 3, 5)], Placement::Tail))
    );
    assert_eq!(
        dictating().at_end(&words("never mind, cancel dictation.")),
        Some(found(&[("cancel dictation", 2, 3)], Placement::Tail))
    );
    assert_eq!(
        standby().at_end(&words("Okay, press enter.")),
        Some(found(&[("press enter", 1, 2)], Placement::Tail))
    );
    assert_eq!(
        standby().at_end(&words("okay enter")),
        Some(found(&[("press enter", 1, 1)], Placement::Tail))
    );
}

#[test]
pub(super) fn should_not_find_a_command_that_does_not_end_the_utterance() {
    for text in [
        "we should stop the server tonight",
        "stop and think",
        "",
        "the shop",
        "keep it clean",
    ] {
        assert_eq!(dictating().at_end(&words(text)), None, "{text}");
        assert_eq!(standby().at_end(&words(text)), None, "{text}");
    }
    assert_eq!(standby().at_end(&words("we should stop")), None);
    assert_eq!(
        dictating().at_end(&words("please cancel")),
        Some(found(&[("cancel dictation", 1, 1)], Placement::Tail))
    );
}

#[test]
pub(super) fn should_find_start_opening_the_utterance() {
    assert_eq!(
        standby().start_at_head(&words("Start, meeting moved to Friday.")),
        Some(found(&[("start dictation", 0, 0)], Placement::Head))
    );
    assert_eq!(
        standby().start_at_head(&words("start dictation we meet at ten")),
        Some(found(&[("start dictation", 0, 1)], Placement::Head))
    );
}

#[test]
pub(super) fn should_find_start_closed_by_please_with_nothing_after_it() {
    assert_eq!(
        standby().start_at_head(&words("Start please.")),
        Some(found(&[("start dictation", 0, 1)], Placement::Head))
    );
    assert_eq!(
        standby().start_at_head(&words("Start dictation, please.")),
        Some(found(&[("start dictation", 0, 2)], Placement::Head))
    );
}

#[test]
pub(super) fn should_not_find_start_without_speech_after_it_or_other_commands_at_the_head() {
    for text in [
        "Start.",
        "enter the building on the left",
        "clear the table",
        "starting tomorrow we meet",
    ] {
        assert_eq!(standby().start_at_head(&words(text)), None, "{text}");
    }
    assert_eq!(dictating().start_at_head(&words("start the report")), None);
}

#[test]
pub(super) fn should_find_a_doubled_short_form_anywhere() {
    assert_eq!(
        dictating().anywhere(&words("we move to Friday stop stop and more")),
        Some(found(&[("stop dictation", 4, 5)], Placement::Anywhere))
    );
    assert_eq!(
        standby().anywhere(&words("fine, enter, enter, thanks")),
        Some(found(&[("press enter", 1, 2)], Placement::Anywhere))
    );
    assert_eq!(
        dictating().anywhere(&words("forget it cancel cancel")),
        Some(found(&[("cancel dictation", 2, 3)], Placement::Anywhere))
    );
    assert_eq!(
        standby().anywhere(&words("Clear. Clear.")),
        Some(found(&[("clear all text", 0, 1)], Placement::Anywhere))
    );
    assert_eq!(
        standby().anywhere(&words("start start")),
        Some(found(&[("start dictation", 0, 1)], Placement::Anywhere))
    );
}
