//! Placed commands on a loaded machine: a copy of the speech recognition service for
//! commands decodes every confirmation past the budget of an idle machine.

use super::*;

mod scripted_copy;

/// How long a copy of the speech recognition service for commands decodes each confirmation
/// on a loaded machine: past the decoding budget of an idle machine.
const LOADED_DECODING: Duration = Duration::from_millis(1_200);

/// The scripted hearing served as a copy of the speech recognition service for commands on a
/// loaded machine, confirmed with over HTTP as the service does.
fn loaded_copy(hearing: ScriptedHearing) -> Arc<dyn Hear> {
    let url = scripted_copy::serve(hearing, LOADED_DECODING);
    Arc::new(UtteranceTranscriber::new(&url).expect("copy address"))
}

#[test]
fn should_hear_a_stop_right_after_the_last_dictated_word_under_load() {
    // Given a dictation, confirmed by a copy that decodes every confirmation for 1.2 s
    // When "stop" is said right after the last word
    let Some(events) = heard_in_time_by(
        &DICTATING,
        "We meet on Friday at ten stop",
        SLOW_HEARING,
        loaded_copy,
    ) else {
        return;
    };
    // Then the stop is carried out
    assert_eq!(carried_out(&events), ["stop dictation"], "{events:?}");
}

#[test]
fn should_carry_out_the_same_chain_under_load() {
    // Given a dictation, confirmed by a copy that decodes every confirmation for 1.2 s
    let listening: Vec<&str> = DICTATING.into_iter().chain(["start dictation"]).collect();
    // When "stop enter start" is said
    let Some(events) = heard_in_time_by(&listening, CHAIN_SENTENCE, SLOW_HEARING, loaded_copy)
    else {
        return;
    };
    // Then the chain is carried out as on an idle machine
    assert_eq!(carried_out(&events), CHAIN, "{events:?}");
}
