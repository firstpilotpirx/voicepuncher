//! The tail of a chain decoded after its first commands were reported: the English recognizer
//! emits each word only once audio came after it, so the last command of a chain shows up a
//! decode after the ones before it.

use super::*;

#[test]
fn should_report_the_last_command_of_a_chain_decoded_after_the_first_ones() {
    // Given "stop enter start" said 0.6, 0.9 and 1.2 s into the stream, the Russian recognizer
    // hearing the first two words only
    let en = Scripted::new("en", &[("stop", 0.6), ("enter", 0.9), ("start", 1.2)]);
    let ru = Scripted::new("ru", &[("стоп", 0.6), ("энтер", 0.9)]);
    let mut session = session(&[&en, &ru], None);
    // When the audio streams in 20 ms frames, so "stop enter" is decoded before "start"
    let audio = [silence(1.0), speech(1.1), silence(1.0)].concat();
    let mut events: Vec<ServerEvent> = audio
        .chunks(320)
        .flat_map(|frame| session.accept(frame))
        .collect();
    events.extend(session.sync(0));
    // Then the chain's last command follows the first two, once
    assert_eq!(
        commands(&events),
        [
            vec!["stop dictation", "press enter"],
            vec!["start dictation"]
        ],
        "{events:?}"
    );
}
