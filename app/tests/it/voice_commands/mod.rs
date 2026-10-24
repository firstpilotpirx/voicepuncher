//! The app's own lists of the commands agree with the one declaration in `voice-commands`:
//! the phrases each state accepts, and every English form of a command that can reach a
//! dictated text.

use voicepuncher::core::domain::{VoiceCommand, COMMAND_FORMS};
use voicepuncher_voice_commands::{COMMANDS, DICTATING, FORMS, STANDBY, START};

fn phrases(commands: &[VoiceCommand]) -> Vec<&'static str> {
    commands.iter().map(|c| c.phrase()).collect()
}

#[test]
fn should_list_the_declared_commands_in_each_state() {
    assert_eq!(phrases(&VoiceCommand::STANDBY), STANDBY);
    assert_eq!(phrases(&VoiceCommand::DICTATING), DICTATING);
    assert_eq!(phrases(&VoiceCommand::ALL), COMMANDS);
}

#[test]
fn should_strip_every_form_of_a_command_that_reaches_a_dictation() {
    let reaching = FORMS
        .iter()
        .filter(|f| DICTATING.contains(&f.phrase) || f.phrase == START);
    for form in reaching {
        assert!(
            COMMAND_FORMS.contains(&form.words),
            "the app does not strip {:?} from a dictation",
            form.words
        );
    }
}
