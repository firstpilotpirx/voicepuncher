//! A command ending long speech is reported as soon as it is decoded, while the gate still
//! hears speech: no pause after it is awaited.

use super::paces::session_delivering;
use super::*;

/// 20 ms: the frames the bench feeds.
const FRAME: usize = 320;

/// The stream time at the end of the frame whose accept reported the first command, `audio`
/// replayed frame by frame as in real time.
fn first_report(session: &mut Session<'_>, audio: &[f32]) -> Option<f32> {
    let mut streamed = 0;
    audio.chunks(FRAME).find_map(|frame| {
        let events = session.accept(frame);
        streamed += frame.len();
        (!commands(&events).is_empty()).then(|| streamed as f32 / RATE as f32)
    })
}

#[test]
fn should_report_a_stop_ending_long_speech_while_the_gate_still_hears_speech() {
    // Given eleven seconds of dictated words, then "stop" at 11 s of the utterance's audio
    // (11.5 s of the stream), and the speech going on 0.8 s past it
    let mut script: Vec<(&str, f32)> = (0..22).map(|i| ("word", 0.5 * i as f32)).collect();
    script.push(("stop", 11.0));
    let en = Scripted::new("en", &script);
    let mut session = session_delivering(&[&en], Delivery::Replayed);
    // When
    let at = first_report(&mut session, &[silence(1.0), speech(11.3)].concat());
    // Then it is reported within 0.6 s of the word's end, before any pause
    let at = at.expect("reported while the speech goes on");
    assert!(at <= 11.6 + 0.6, "{at}");
}

#[test]
fn should_take_no_english_stop_heading_a_russian_sentence_that_goes_on() {
    // Given "чтобы зайти в гетлаб", whose first word the English recognizer hears as "stop"
    let en = Scripted::new(
        "en",
        &[("stop", 0.6), ("zayti", 0.9), ("v", 1.1), ("getlab", 1.3)],
    );
    let ru = Scripted::new(
        "ru",
        &[("чтобы", 0.6), ("зайти", 0.9), ("в", 1.1), ("гетлаб", 1.3)],
    );
    // When
    let events = utterance_events(&[&en, &ru], 1.5);
    // Then
    assert_eq!(commands(&events), Vec::<Vec<String>>::new(), "{events:?}");
}

#[test]
fn should_take_an_english_stop_the_russian_recognizer_heard_as_a_word_at_the_pause_after_it() {
    // Given "stop" said alone, the Russian recognizer hearing another word
    let en = Scripted::new("en", &[("stop", 0.6)]);
    let ru = Scripted::new("ru", &[("стопка", 0.6)]);
    // When
    let events = utterance_events(&[&en, &ru], 0.4);
    // Then
    assert_eq!(commands(&events), [["stop dictation"]], "{events:?}");
}

/// A replayed session over `models` listening for `phrases`.
fn replayed<'a>(models: &[&'a Scripted], phrases: &[&str]) -> Session<'a> {
    let cfg = SessionCfg { delivery: Delivery::Replayed, ..cfg(None) };
    session_listening(models, &cfg, phrases)
}

#[test]
fn should_report_an_english_enter_over_another_russian_word_at_once_while_no_dictation_runs() {
    // Given "enter" said before more speech while no dictation runs, the Russian recognizer
    // hearing other words over it
    let en = Scripted::new(
        "en",
        &[("enter", 0.6), ("the", 0.9), ("room", 1.2), ("now", 1.5)],
    );
    let ru = Scripted::new(
        "ru",
        &[("этот", 0.6), ("зе", 0.9), ("рум", 1.2), ("нау", 1.5)],
    );
    let mut session = replayed(&[&en, &ru], &["press enter", "start dictation"]);
    // When
    let at = first_report(&mut session, &[silence(1.0), speech(2.5)].concat());
    // Then it is reported before the speech ends, with no pause awaited
    let at = at.expect("reported while the speech goes on");
    assert!(at <= 1.6 + 0.6, "{at}");
}

#[test]
fn should_report_a_doubled_english_stop_over_other_russian_words_at_once_during_a_dictation() {
    // Given "stop stop" said in a dictation before more speech, the Russian recognizer
    // hearing other words over it
    let en = Scripted::new(
        "en",
        &[("stop", 0.6), ("stop", 0.9), ("the", 1.2), ("car", 1.5)],
    );
    let ru = Scripted::new(
        "ru",
        &[("стопка", 0.6), ("стопка", 0.9), ("зе", 1.2), ("кар", 1.5)],
    );
    let mut session = replayed(&[&en, &ru], &["stop dictation"]);
    // When
    let at = first_report(&mut session, &[silence(1.0), speech(2.5)].concat());
    // Then it is reported before the speech ends, with no pause awaited
    let at = at.expect("reported while the speech goes on");
    assert!(at <= 1.9 + 0.6, "{at}");
}
