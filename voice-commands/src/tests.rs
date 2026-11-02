use super::*;

#[test]
fn should_give_every_command_exactly_one_form_of_one_word() {
    for phrase in COMMANDS {
        let forms: Vec<&Form> = FORMS.iter().filter(|f| f.phrase == phrase).collect();
        assert!(
            forms.len() == 1 && forms[0].words.len() == 1,
            "{phrase} has forms {forms:?}, not one form of one word"
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
