//! At an utterance's close, a recognizer that heard a whole command word speaks before one that
//! heard only its end: a quiet "restart" loses its "re" in one recognizer and is "start" there.

use super::*;

fn restart_session<'a>(models: &[&'a Scripted]) -> Session<'a> {
    let models = models.iter().map(|m| *m as &dyn StreamingModel).collect();
    let cfg = SessionCfg { conn: 1, min_silence_secs: 0.3, history: None, wake: None };
    let mut session = Session::new(models, Box::new(LoudGate), &cfg);
    let phrases = ["start dictation", "restart dictation"].map(String::from);
    let selection = PhraseSelection { phrases: phrases.to_vec(), ..PhraseSelection::default() };
    session.select(&selection).expect("known phrases");
    session
}

#[test]
fn should_report_restart_when_one_recognizer_heard_only_start_at_the_close() {
    // Given both recognizers emit their word only once the utterance closes
    let en = Scripted::new("en", &[("start", 0.7)]).lagging(10.0);
    let ru = Scripted::new("ru", &[("рестарт", 0.6)]).lagging(10.0);
    let mut session = restart_session(&[&en, &ru]);
    // When
    let mut events = session.accept(&[silence(1.0), speech(0.4), silence(1.0)].concat());
    events.extend(session.sync(0));
    // Then
    assert_eq!(commands(&events), [["restart dictation"]], "{events:?}");
}
