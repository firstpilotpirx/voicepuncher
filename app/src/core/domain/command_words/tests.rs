use super::*;

#[test]
fn should_remove_the_command_that_ends_the_text() {
    for (text, kept) in [
        ("We meet on Friday. Stop.", "We meet on Friday."),
        (
            "We meet on Friday, stop dictation please.",
            "We meet on Friday.",
        ),
        ("We meet on Friday stop stop", "We meet on Friday"),
        ("We meet on Friday. Stop, enter.", "We meet on Friday."),
        ("Встреча в пятницу. Стоп.", "Встреча в пятницу."),
        ("Встреча в пятницу, стоп энтер старт", "Встреча в пятницу"),
        ("We meet on Friday. Cancel, clear.", "We meet on Friday."),
    ] {
        assert_eq!(without_trailing_command(text), kept, "{text}");
    }
}

#[test]
fn should_keep_a_text_that_ends_in_other_words() {
    for text in [
        "We meet on Friday.",
        "Please don't stop the server.",
        "Clear text editing is enabled",
        "",
    ] {
        assert_eq!(without_trailing_command(text), text, "{text}");
    }
}

#[test]
fn should_remove_a_start_that_opens_the_text() {
    assert_eq!(
        without_leading_command(
            VoiceCommand::StartDictation,
            "Start, meeting moved to Friday."
        ),
        "Meeting moved to Friday."
    );
    assert_eq!(
        without_leading_command(
            VoiceCommand::StartDictation,
            "start dictation we meet at ten"
        ),
        "We meet at ten"
    );
    assert_eq!(
        without_leading_command(VoiceCommand::StartDictation, "Старт. Встреча в пятницу."),
        "Встреча в пятницу."
    );
    assert_eq!(
        without_leading_command(VoiceCommand::StartDictation, "Starting tomorrow we meet."),
        "Starting tomorrow we meet."
    );
}

#[test]
fn should_remove_a_restart_that_opens_the_text_of_a_restarted_dictation() {
    for (text, kept) in [
        (
            "Restart, restart, coffee machine is broken.",
            "Coffee machine is broken.",
        ),
        ("restart dictation we meet at ten", "We meet at ten"),
        ("Restart. Lunch is at noon.", "Lunch is at noon."),
    ] {
        assert_eq!(
            without_leading_command(VoiceCommand::RestartDictation, text),
            kept,
            "{text}"
        );
    }
}

#[test]
fn should_remove_a_please_that_opens_the_text_after_a_voice_start() {
    for (command, text, kept) in [
        (
            VoiceCommand::StartDictation,
            "please. The build is green again.",
            "The build is green again.",
        ),
        (
            VoiceCommand::StartDictation,
            "Start dictation please, meeting moved to Friday.",
            "Meeting moved to Friday.",
        ),
        (
            VoiceCommand::RestartDictation,
            "Пожалуйста. Встреча в пятницу.",
            "Встреча в пятницу.",
        ),
        (VoiceCommand::StartDictation, "Please.", "Please."),
    ] {
        assert_eq!(without_leading_command(command, text), kept, "{text}");
    }
}

#[test]
fn should_keep_words_of_another_command_that_open_the_text() {
    for (opened_by, text) in [
        (VoiceCommand::StartDictation, "Restart the server tonight."),
        (VoiceCommand::RestartDictation, "Start the engine."),
    ] {
        assert_eq!(without_leading_command(opened_by, text), text, "{text}");
    }
}

#[test]
fn should_remove_a_doubled_start_that_opens_the_text() {
    for (text, kept) in [
        ("Start, start, the build is green.", "The build is green."),
        ("start start we meet at ten", "We meet at ten"),
        ("Старт старт. Встреча в пятницу.", "Встреча в пятницу."),
        ("Start dictation, start the engine.", "Start the engine."),
    ] {
        assert_eq!(
            without_leading_command(VoiceCommand::StartDictation, text),
            kept,
            "{text}"
        );
    }
}

#[test]
fn should_remove_a_paste_said_as_its_own_sentence() {
    for (text, kept) in [
        ("Hello there. Paste. And more.", "Hello there. And more."),
        (
            "Hello there. Paste text, please. And more.",
            "Hello there. And more.",
        ),
        ("Hello there. Paste.", "Hello there."),
        ("Paste. Hello there.", "Hello there."),
    ] {
        assert_eq!(without_spoken_pastes(text, 1), kept, "{text}");
    }
}

#[test]
fn should_remove_a_paste_that_ends_a_sentence() {
    for (text, kept) in [
        ("Here it is, paste. Next one.", "Here it is. Next one."),
        ("Here it is paste", "Here it is"),
    ] {
        assert_eq!(without_spoken_pastes(text, 1), kept, "{text}");
    }
}

#[test]
fn should_keep_a_paste_inside_a_sentence() {
    assert_eq!(
        without_spoken_pastes("Paste the link here. Paste.", 1),
        "Paste the link here."
    );
}

#[test]
fn should_remove_no_more_pastes_than_were_carried_out() {
    assert_eq!(
        without_spoken_pastes("One. Paste. Two. Paste.", 1),
        "One. Two. Paste."
    );
    assert_eq!(without_spoken_pastes("One. Paste.", 0), "One. Paste.");
    assert_eq!(
        without_spoken_pastes("One. Paste. Two. Paste.", 2),
        "One. Two."
    );
}
