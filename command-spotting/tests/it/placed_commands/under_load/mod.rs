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

#[test]
fn should_hear_a_clip_a_copy_decodes_past_the_idle_budget() {
    // Given the hearing of the real-service tests, confirming with a copy that decodes every
    // confirmation for 1.2 s, as the shared service does beside another check
    let (stream, script) = scripted("stop dictation");
    let hearing = ScriptedHearing { stream: stream.clone(), script, completes_cut_words: false };
    let url = scripted_copy::serve(hearing, LOADED_DECODING);
    // When it hears the speech
    let heard = real_hearing(&url).hear(&stream, REAL_DEADLINE).unwrap();
    // Then every word is heard: the confirmation is not cut short at the idle budget
    assert_eq!(
        heard.text.split_whitespace().collect::<Vec<_>>(),
        ["stop", "dictation"]
    );
}

/// The speech of the E2E scenario "Restart, a short pause, then speech that starts like
/// another word": a dictated sentence, then "Restart", a short pause and another sentence.
const BEFORE_RESTART: &str = "Weather is sunny in Berlin today";
const AFTER_RESTART: &str = "Coffee machine on the third floor is broken";
/// The pause after "Restart": longer than the one that ends an utterance.
const SHORT_PAUSE_SECS: f32 = 0.4;

/// Each part's words said one by one, each part followed by its pause, after a pause; and
/// where each word lies.
fn scripted_with_pauses(parts: &[(&str, f32)]) -> (Vec<f32>, Vec<ScriptedWord>) {
    let mut stream = silence(PAUSE_SECS);
    let mut script = Vec::new();
    for (words, pause_secs) in parts {
        for word in words.split_whitespace() {
            let said = sound_only(speech(word));
            script.push(ScriptedWord {
                text: word.to_owned(),
                said: stream.len()..stream.len() + said.len(),
            });
            stream.extend(said);
            stream.extend(silence(WORD_GAP_SECS));
        }
        stream.extend(silence(*pause_secs));
    }
    (stream, script)
}

#[test]
fn should_restart_before_a_short_pause_and_more_speech_under_load() {
    // Given a dictation, confirmed by a copy that decodes every confirmation for 1.2 s
    let Some(cfg) = engine_cfg(None) else {
        return;
    };
    let (stream, script) = scripted_with_pauses(&[
        (BEFORE_RESTART, PAUSE_SECS),
        ("Restart", SHORT_PAUSE_SECS),
        (AFTER_RESTART, PAUSE_SECS),
    ]);
    let hearing = ScriptedHearing { stream: stream.clone(), script, completes_cut_words: false };
    let engine = clocked_engine(&cfg, loaded_copy(hearing), SLOW_HEARING, None);
    let mut session = engine.engine.session().unwrap();
    session.listen_for(&owned(&DICTATING)).unwrap();
    // When "restart" is said, then about 0.4 s of silence and another sentence
    let events = fed_as_spoken(&mut session, &engine.clock, &stream);
    // Then the restart is reported
    let restarts = phrases(&events)
        .into_iter()
        .chain(carried_out(&events))
        .filter(|phrase| phrase == "restart dictation")
        .count();
    assert_eq!(restarts, 1, "{events:?}");
}
