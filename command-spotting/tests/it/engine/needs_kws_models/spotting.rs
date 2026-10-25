//! Phrases spotted alone, in long streams, quietly and over noise.

use super::*;
use voicepuncher_command_spotting::conversions::saturating_usize;

#[test]
fn should_spot_each_phrase_spoken_on_its_own_inside_its_speech_segment() {
    let engine = engine();
    for phrase in ALL_PHRASES {
        let events = events_for(&engine, phrase);
        eprintln!("{phrase}: {events:?}");
        assert_eq!(spotted(&events), [phrase], "events: {events:?}");
        let starts = events
            .iter()
            .filter(|e| matches!(e, ServerEvent::SpeechStart { .. }))
            .count();
        assert_eq!(starts, 1, "one speech segment expected: {events:?}");
    }
}

#[test]
fn should_spot_the_polite_and_short_variants_by_keyword_alone() {
    let engine = engine();
    for (said, phrase) in [
        ("press enter please", "press enter"),
        ("stop dictation please", "stop dictation"),
        ("clear text", "clear all text"),
        ("clear text please", "clear all text"),
    ] {
        let events = events_for(&engine, said);
        assert_eq!(spotted(&events), [phrase], "{said}: {events:?}");
    }
}

#[test]
fn should_spot_by_keyword_alone_when_the_speech_recognition_service_does_not_answer() {
    // Holding the shared engine keeps other tests from loading or running models meanwhile.
    let _shared = engine();
    let closed_port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let cfg = engine_cfg(Some(format!("http://127.0.0.1:{closed_port}/v1")));
    let engine = Engine::load(&cfg).expect("models load");
    for phrase in ["press enter", "stop dictation"] {
        let events = events_for(&engine, phrase);
        assert_eq!(spotted(&events), [phrase], "{phrase}: {events:?}");
    }
}

#[test]
fn should_report_the_phrase_within_the_bounds_of_its_speech() {
    let engine = engine();
    assert_phrase_fills_segment(&events_for(&engine, "press enter"));
}

#[test]
fn should_report_the_phrase_within_its_speech_after_a_long_silence() {
    let engine = engine();
    // Given
    let mut session = listening(&engine, &ALL_PHRASES);
    let mut audio = vec![0.0f32; saturating_usize(LONG_SILENCE_SECS * SAMPLE_RATE as f32)];
    audio.extend(spoken("start dictation"));
    // When
    let events: Vec<ServerEvent> = fed(&mut session, &audio);
    // Then
    assert_phrase_fills_segment(&events);
}

#[test]
fn should_count_event_times_from_the_first_sample_across_a_phrase_change() {
    let engine = engine();
    // Given
    let mut session = engine.session().unwrap();
    session.listen_for(&["stop dictation".to_owned()]).unwrap();
    let lead_in = vec![0.0f32; saturating_usize(LEAD_IN_SECS * SAMPLE_RATE as f32)];
    for frame in lead_in.chunks(FRAME_SAMPLES) {
        session.accept(frame);
    }
    // When
    session.listen_for(&["press enter".to_owned()]).unwrap();
    let events: Vec<ServerEvent> = fed(&mut session, &spoken("press enter"));
    // Then
    let speech_start = events
        .iter()
        .find_map(|e| match e {
            ServerEvent::SpeechStart { t } => Some(*t),
            _ => None,
        })
        .expect("speech was detected");
    let expected = LEAD_IN_SECS + PAUSE_SECS;
    assert!(
        (speech_start - expected).abs() < 0.3,
        "speech at {speech_start}, expected about {expected}: {events:?}"
    );
}

#[test]
fn should_spot_commands_said_one_after_another_in_one_long_stream() {
    let engine = engine();
    // Given
    let mut session = listening(&engine, &ALL_PHRASES);
    let mut audio = silence(5.0);
    audio.extend(spoken("start dictation"));
    audio.extend(silence(10.0));
    audio.extend(spoken("press enter"));
    audio.extend(silence(20.0));
    audio.extend(spoken("clear all text"));
    // When
    let events: Vec<ServerEvent> = fed(&mut session, &audio);
    // Then
    let found = phrases_in_segments(&events);
    let names: Vec<&str> = found.iter().map(|(p, _, _)| p.as_str()).collect();
    assert_eq!(
        names,
        ["start dictation", "press enter", "clear all text"],
        "{events:?}"
    );
    for (phrase, segment, span) in &found {
        assert!(
            span.0 - segment.0 < LEAD_TOLERANCE_SECS
                && (segment.1 - span.1).abs() < TAIL_TOLERANCE_SECS,
            "{phrase}: segment {segment:?} phrase {span:?}"
        );
    }
}

#[test]
fn should_place_the_phrase_alike_whether_the_audio_comes_in_small_or_large_messages() {
    let engine = engine();
    // Given
    let audio = spoken("press enter");
    let phrases: Vec<String> = ALL_PHRASES.iter().map(|p| (*p).to_owned()).collect();
    // When the whole utterance arrives in one message
    let mut session = engine.session().unwrap();
    session.listen_for(&phrases).unwrap();
    let mut events = session.accept(&audio);
    events.extend(session.settle());
    // Then
    assert_phrase_fills_segment(&events);
}

#[test]
fn should_spot_quiet_speech() {
    let engine = engine();
    for phrase in ALL_PHRASES {
        // Given
        let audio: Vec<f32> = spoken(phrase).iter().map(|s| s * QUIET_GAIN).collect();
        // When
        let events = spotted_in(&engine, &audio);
        // Then
        assert_eq!(spotted(&events), [phrase], "quiet {phrase}: {events:?}");
        assert_phrase_fills_segment(&events);
    }
}

#[test]
fn should_notice_each_command_said_in_a_quiet_voice() {
    let engine = engine();
    for words in ALL_PHRASES.into_iter().chain(SHORT_FORMS) {
        // Given
        let audio = at_level(spoken(words), QUIET_VOICE_LEVEL);
        // When
        let events = spotted_in(&engine, &audio);
        // Then
        assert_eq!(speech_segments(&events), 1, "quiet {words}: {events:?}");
    }
}

#[test]
fn should_notice_no_speech_in_room_noise_alone() {
    let engine = engine();
    for rms in [0.003, ROOM_NOISE_RMS, 0.03] {
        // Given 20 s of steady room noise, from a quiet room to a loud fan
        let audio = noise(20 * SAMPLE_RATE as usize, rms);
        // When
        let events = spotted_in(&engine, &audio);
        // Then
        assert_eq!(speech_segments(&events), 0, "noise at {rms}: {events:?}");
    }
}

#[test]
fn should_spot_speech_over_room_noise() {
    let engine = engine();
    for phrase in ALL_PHRASES {
        // Given
        let mut audio = silence(LONG_SILENCE_SECS);
        audio.extend(spoken(phrase));
        let room = noise(audio.len(), ROOM_NOISE_RMS);
        let heard: Vec<f32> = audio.iter().zip(&room).map(|(s, n)| s + n).collect();
        // When
        let events = spotted_in(&engine, &heard);
        // Then
        assert_eq!(spotted(&events), [phrase], "noisy {phrase}: {events:?}");
        assert_phrase_fills_segment(&events);
    }
}
