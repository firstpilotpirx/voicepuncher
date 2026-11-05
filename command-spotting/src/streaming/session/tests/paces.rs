//! When the session sees a decode: as it arrives from the recognizer's thread, or replayed at
//! the stream time it would have ended when fed in real time.

use super::*;

/// 20 ms: the frames the bench feeds.
const FRAME: usize = 320;

fn session_delivering<'a>(models: &[&'a Scripted], delivery: Delivery) -> Session<'a> {
    let models = models.iter().map(|m| *m as &dyn StreamingModel).collect();
    let cfg = SessionCfg { conn: 1, min_silence_secs: 0.3, history: None, wake: None, delivery };
    let mut session = Session::new(models, Box::new(LoudGate), &cfg);
    let phrases = vec!["stop dictation".to_owned()];
    let selection = PhraseSelection { phrases, ..PhraseSelection::default() };
    session.select(&selection).expect("known phrases");
    session
}

/// Streams `audio` frame by frame: the stream time at the end of the frame whose accept
/// reported the first command, when one did before the sync.
fn command_time(session: &mut Session<'_>, audio: &[f32]) -> Option<f32> {
    let mut reported = None;
    let mut streamed = 0;
    for frame in audio.chunks(FRAME) {
        let events = session.accept(frame);
        streamed += frame.len();
        if reported.is_none() && !commands(&events).is_empty() {
            reported = Some(streamed as f32 / RATE as f32);
        }
    }
    reported
}

#[test]
fn should_see_a_decode_when_it_would_have_ended() {
    // Given "stop" said at 1.1 s to a recognizer taking 30 ms per decode, replayed
    let en = Scripted::new("en", &[("stop", 0.6)]).slow(Duration::from_millis(30));
    let mut session = session_delivering(&[&en], Delivery::Replayed);
    // When the stream runs to 1 s past the pause
    let audio = [silence(1.0), speech(0.3), silence(1.0)].concat();
    let at = command_time(&mut session, &audio).expect("reported while streaming");
    // Then the opening at 1.0 s and each 20 ms frame after it queue 30 ms of decoding, so
    // the flush fed at the pause (the 18th frame, 1.36 s) is seen at the earliest at
    // 1.0 s + 19 decodes × 30 ms + its own 30 ms
    assert!(at >= 1.6, "{at}");
}

#[test]
fn should_keep_delivering_as_decoded_by_default() {
    // Given a recognizer taking 20 ms per decode, a frame's worth, delivered as decoded
    let en = Scripted::new("en", &[("stop", 0.6)]).slow(Duration::from_millis(20));
    let mut session = session_delivering(&[&en], Delivery::AsDecoded);
    let audio = [silence(1.0), speech(0.3), silence(0.5)].concat();
    // When
    let began = Instant::now();
    command_time(&mut session, &audio);
    let fed_in = began.elapsed();
    // Then the 90 frames were fed without waiting for their 1.8 s of decoding
    assert!(fed_in < Duration::from_secs(1), "{fed_in:?}");
}
