//! Chains of commands, look-alike commands and filler words.

use super::*;

/// Commands said one after another, each with its first and last word.
type Chain = &'static [(&'static str, usize, usize)];

/// Chains said anywhere, in a dictation, or in standby when the flag is false.
const CHAINS: [(bool, &str, Chain); 6] = [
    (
        true,
        "that is it, stop, enter.",
        &[("stop dictation", 3, 3), ("press enter", 4, 4)],
    ),
    (
        true,
        "stop enter start",
        &[
            ("stop dictation", 0, 0),
            ("press enter", 1, 1),
            ("start dictation", 2, 2),
        ],
    ),
    (
        true,
        "stop dictation press enter please",
        &[("stop dictation", 0, 1), ("press enter", 2, 4)],
    ),
    (
        true,
        "cancel clear",
        &[("cancel dictation", 0, 0), ("clear all text", 1, 1)],
    ),
    (
        true,
        "stop clear enter",
        &[
            ("stop dictation", 0, 0),
            ("clear all text", 1, 1),
            ("press enter", 2, 2),
        ],
    ),
    (
        false,
        "ok enter start",
        &[("press enter", 1, 1), ("start dictation", 2, 2)],
    ),
];

#[test]
pub(super) fn should_find_a_chain_of_commands_anywhere() {
    for (in_dictation, text, chain) in CHAINS {
        let matcher = if in_dictation { dictating() } else { standby() };
        assert_eq!(
            matcher.anywhere(&words(text)),
            Some(found(chain, Placement::Anywhere)),
            "{text}"
        );
    }
}

#[test]
pub(super) fn should_not_find_chains_the_states_do_not_allow_or_single_commands() {
    for text in [
        "start stop",
        "start cancel",
        "enter stop",
        "stop start stop",
    ] {
        let found = standby().anywhere(&words(text));
        assert!(
            found
                .as_ref()
                .is_none_or(|f| !f.commands.contains(&"stop dictation".to_owned())),
            "{text}: {found:?}"
        );
    }
    assert_eq!(standby().anywhere(&words("enter stop")), None);
    assert_eq!(dictating().anywhere(&words("clear enter")), None);
    assert_eq!(
        dictating().anywhere(&words("enter clear")),
        Some(found(
            &[("press enter", 0, 0), ("clear all text", 1, 1)],
            Placement::Anywhere
        ))
    );
    for text in [
        "we should stop the server",
        "stop",
        "the shop shop",
        "clean clean",
        "",
    ] {
        assert_eq!(dictating().anywhere(&words(text)), None, "{text}");
        assert_eq!(standby().anywhere(&words(text)), None, "{text}");
    }
}

#[test]
pub(super) fn should_find_paste_like_the_other_standby_commands() {
    assert_eq!(
        standby().at_end(&words("okay, paste.")),
        Some(found(&[("paste text", 1, 1)], Placement::Tail))
    );
    assert_eq!(
        standby().anywhere(&words("fine paste paste")),
        Some(found(&[("paste text", 1, 2)], Placement::Anywhere))
    );
    assert_eq!(
        standby().anywhere(&words("paste enter")),
        Some(found(
            &[("paste text", 0, 0), ("press enter", 1, 1)],
            Placement::Anywhere
        ))
    );
    assert_eq!(
        dictating().anywhere(&words("stop paste text enter")),
        Some(found(
            &[
                ("stop dictation", 0, 0),
                ("paste text", 1, 2),
                ("press enter", 3, 3)
            ],
            Placement::Anywhere
        ))
    );
    assert_eq!(dictating().at_end(&words("we should paste")), None);
    assert_eq!(
        standby().at_end(&words("I will paste the link later")),
        None
    );
}

#[test]
pub(super) fn should_find_restart_like_the_other_commands() {
    assert_eq!(
        dictating().at_end(&words("let me say it again restart")),
        Some(found(&[("restart dictation", 5, 5)], Placement::Tail))
    );
    assert_eq!(
        dictating().anywhere(&words("wait restart restart")),
        Some(found(&[("restart dictation", 1, 2)], Placement::Anywhere))
    );
    // After "cancel" no dictation runs, so a "restart" after it is no chain.
    assert_eq!(dictating().anywhere(&words("cancel restart")), None);
    assert_eq!(dictating().anywhere(&words("restart stop")), None);
    assert_eq!(dictating().at_end(&words("we start")), None);
}

#[test]
pub(super) fn should_not_find_a_command_followed_by_a_filler_word() {
    assert_eq!(standby().at_end(&words("Please clear the...")), None);
    assert_eq!(dictating().at_end(&words("we had to stop the")), None);
}

#[test]
pub(super) fn should_find_start_as_all_said_so_far_in_a_running_utterance() {
    assert_eq!(
        standby().start_so_far(&words("Start dictation.")),
        Some(found(&[("start dictation", 0, 1)], Placement::Head))
    );
    assert_eq!(standby().start_so_far(&words("start the meeting")), None);
    assert_eq!(dictating().start_so_far(&words("start")), None);
}

#[test]
pub(super) fn should_tell_restart_dictation_from_start_dictation() {
    assert_eq!(
        dictating().at_end(&words("Let me try once more, restart dictation.")),
        Some(found(&[("restart dictation", 5, 6)], Placement::Tail))
    );
    assert_eq!(
        standby().at_end(&words("Okay, start dictation.")),
        Some(found(&[("start dictation", 1, 2)], Placement::Tail))
    );
}

#[test]
pub(super) fn should_find_no_command_in_start_stop() {
    let said = words("Okay, start stop");
    assert_eq!(standby().anywhere(&said), None);
    assert_eq!(standby().at_end(&said), None);
    assert_eq!(standby().start_at_head(&words("start stop")), None);
    assert_eq!(standby().start_so_far(&words("start stop")), None);
}

#[test]
pub(super) fn should_take_stuck_dictation_as_start_dictation() {
    assert_eq!(
        standby().start_at_head(&words("Stuck dictation, the build is green again.")),
        Some(found(&[("start dictation", 0, 1)], Placement::Head))
    );
}

#[test]
pub(super) fn should_take_and_restart_after_a_command_as_enter_start() {
    // "enter start" said quickly is heard as "and restart"
    assert_eq!(
        dictating().anywhere(&words("That is all. Stop and restart.")),
        Some(found(
            &[
                ("stop dictation", 3, 3),
                ("press enter", 4, 4),
                ("start dictation", 5, 5)
            ],
            Placement::Anywhere
        ))
    );
    assert_eq!(
        dictating().anywhere(&words("we stop and restart the server")),
        None
    );
}
