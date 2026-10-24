use super::*;

#[test]
fn should_give_every_command_a_form_of_its_own_words() {
    for phrase in COMMANDS {
        let words: Vec<&str> = phrase.split(' ').collect();
        assert!(
            FORMS
                .iter()
                .any(|f| f.phrase == phrase && f.words == words.as_slice()),
            "{phrase} has no form of its own words"
        );
    }
}

#[test]
fn should_accept_each_command_in_some_state() {
    for phrase in COMMANDS {
        assert!(
            STANDBY.contains(&phrase) || DICTATING.contains(&phrase),
            "{phrase} is accepted in no state"
        );
    }
}

#[test]
fn should_name_only_declared_commands_in_mishearings() {
    for row in MISHEARINGS {
        assert!(
            row.means.iter().all(|p| COMMANDS.contains(p)),
            "{:?} means an undeclared command",
            row.heard
        );
    }
}

#[test]
fn should_hear_one_word_for_each_command_of_an_after_command_row() {
    for row in MISHEARINGS
        .iter()
        .filter(|m| m.kind == MishearingKind::AfterCommand)
    {
        assert_eq!(row.heard.len(), row.means.len(), "{:?}", row.heard);
    }
}
