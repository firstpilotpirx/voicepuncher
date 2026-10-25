//! Voice starts, restarts and Enter during a dictation.

use super::*;

/// A spotter that heard half a second since `end`, and nothing since any other moment.
pub(super) fn heard_half_a_second_since(end: Instant) -> MockCommandSpotter {
    let mut spotter = listening_spotter();
    spotter.expect_heard_since().returning(move |since| {
        let samples = if since == end { LEAD_IN_SAMPLES } else { 0 };
        AudioBuffer16k::new(vec![0; samples])
    });
    spotter
}

/// A recorder that notes the length of every lead-in it begins with.
pub(super) fn noting_lead_ins(lead_ins: &Arc<Mutex<Vec<usize>>>) -> MockAudioRecorder {
    let mut recorder = recording_recorder_without_lead_in();
    let seen = Arc::clone(lead_ins);
    recorder
        .expect_begin_with()
        .returning(move |lead_in| seen.lock().unwrap().push(lead_in.samples().len()));
    recorder
}

#[test]
pub(super) fn should_begin_a_voice_dictation_where_the_start_ended_wherever_it_was_said() {
    for placement in [
        CommandPlacement::Alone,
        CommandPlacement::Head,
        CommandPlacement::Tail,
        CommandPlacement::Anywhere,
    ] {
        // Given half a second heard since "start" ended
        let start = Instant::now();
        let lead_ins = Arc::new(Mutex::new(Vec::new()));
        let spotter = heard_half_a_second_since(start + PHRASE_LENGTH);
        let recorder = noting_lead_ins(&lead_ins);
        let mut pipeline = pipeline(
            recorder,
            MockTextPaster::new(),
            spotter,
            RecordingMode::Toggle,
        );
        listen(&mut pipeline);
        // When
        let started = pipeline.handle(said(VoiceCommand::StartDictation, placement, start));
        // Then
        assert!(started.is_ok() && pipeline.is_recording(), "{placement:?}");
        assert_eq!(
            *lead_ins.lock().unwrap(),
            [LEAD_IN_SAMPLES],
            "{placement:?}"
        );
    }
}

#[test]
pub(super) fn should_begin_a_restarted_dictation_where_restart_ended() {
    // Given a dictation runs and half a second was heard since "restart" ended
    let restart_at = Instant::now() + Duration::from_secs(5);
    let lead_ins = Arc::new(Mutex::new(Vec::new()));
    let spotter = heard_half_a_second_since(restart_at + PHRASE_LENGTH);
    let recorder = noting_lead_ins(&lead_ins);
    let mut pipeline = pipeline(
        recorder,
        MockTextPaster::new(),
        spotter,
        RecordingMode::Toggle,
    );
    listen(&mut pipeline);
    say(&mut pipeline, "start dictation");
    // When
    let restarted = pipeline.handle(said(
        VoiceCommand::RestartDictation,
        CommandPlacement::Alone,
        restart_at,
    ));
    // Then
    assert!(restarted.is_ok() && pipeline.is_recording());
    assert_eq!(lead_ins.lock().unwrap().last(), Some(&LEAD_IN_SAMPLES));
}

#[test]
pub(super) fn should_paste_a_restarted_dictation_without_the_restart_its_lead_in_caught() {
    // Given a dictation runs, and its restart's lead-in caught the end of "restart restart"
    let mut transcriber = MockTranscriber::new();
    transcriber.expect_transcribe().returning(|_| {
        Ok(Some(
            RawTranscript::new("Restart, lunch is at noon.".into()).unwrap(),
        ))
    });
    let pasted = Arc::new(Mutex::new(Vec::new()));
    let seen = Arc::clone(&pasted);
    let mut text_paster = MockTextPaster::new();
    text_paster.expect_paste().returning(move |text| {
        seen.lock().unwrap().push(text.as_str().to_owned());
        Ok(())
    });
    let parts = PipelineParts {
        transcriber: Box::new(transcriber),
        ..parts(recording_recorder(), text_paster)
    };
    let mut pipeline = Pipeline::new(parts, toggle_cfg())
        .with_voice_control(Box::new(lenient_spotter()), Box::new(MockKeyPresser::new()));
    listen(&mut pipeline);
    say(&mut pipeline, "start dictation");
    let restart = said(
        VoiceCommand::RestartDictation,
        CommandPlacement::Anywhere,
        Instant::now(),
    );
    pipeline.handle(restart).unwrap();
    // When
    press(&mut pipeline);
    // Then
    assert_eq!(*pasted.lock().unwrap(), ["Lunch is at noon."]);
}

#[test]
pub(super) fn should_take_the_heard_speech_once_the_recorder_runs_and_before_live_chunks() {
    // Given
    let order = Arc::new(Mutex::new(Vec::new()));
    let mut pipeline = noting_start_pipeline(&order);
    listen(&mut pipeline);
    // When
    let start = Instant::now();
    let started = pipeline.handle(said(
        VoiceCommand::StartDictation,
        CommandPlacement::Head,
        start,
    ));
    // Then
    assert!(started.is_ok());
    assert_eq!(
        *order.lock().unwrap(),
        [
            "start the recorder",
            "take the heard speech",
            "begin with the heard speech",
            "start live chunks"
        ]
    );
}

/// A live chunking pipeline whose recorder and spotter note each step of a voice start in `order`.
fn noting_start_pipeline(order: &Arc<Mutex<Vec<&'static str>>>) -> Pipeline {
    let note = |step: &'static str| {
        let seen = Arc::clone(order);
        move || seen.lock().unwrap().push(step)
    };
    let mut spotter = listening_spotter();
    let heard = note("take the heard speech");
    spotter.expect_heard_since().returning(move |_| {
        heard();
        AudioBuffer16k::new(Vec::new())
    });
    let mut recorder = MockAudioRecorder::new();
    let started = note("start the recorder");
    recorder.expect_start().returning(move || {
        started();
        Ok(())
    });
    let joined = note("begin with the heard speech");
    recorder.expect_begin_with().returning(move |_| joined());
    let chunked = note("start live chunks");
    recorder.expect_start_live_chunks().returning(move |_| {
        chunked();
        Ok(None)
    });
    live_chunking_pipeline(recorder, spotter)
}

#[test]
pub(super) fn should_paste_and_then_press_return_on_enter_during_a_dictation() {
    // Given
    let order = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let mut recorder = recording_recorder();
    recorder.expect_truncate_at().return_const(());
    let mut text_paster = MockTextPaster::new();
    let seen = std::sync::Arc::clone(&order);
    text_paster.expect_paste().once().returning(move |_| {
        seen.lock().unwrap().push("paste");
        Ok(())
    });
    let mut keys = MockKeyPresser::new();
    let seen = std::sync::Arc::clone(&order);
    keys.expect_press_enter().once().returning(move || {
        seen.lock().unwrap().push("return");
        Ok(())
    });
    let mut pipeline = pipeline_with(
        sounding_parts(recorder, text_paster, sounds()),
        lenient_spotter(),
        keys,
        RecordingMode::Toggle,
    );
    listen(&mut pipeline);
    say(&mut pipeline, "start dictation");
    // When
    say(&mut pipeline, "press enter");
    // Then
    assert!(!pipeline.is_recording());
    assert_eq!(*order.lock().unwrap(), ["paste", "return"]);
}

#[test]
pub(super) fn should_play_only_the_restart_sound_on_restart() {
    // Given
    let played = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let mut sounds = MockSoundPlayer::new();
    let seen = std::sync::Arc::clone(&played);
    sounds
        .expect_play_start()
        .returning(move || seen.lock().unwrap().push("start"));
    let seen = std::sync::Arc::clone(&played);
    sounds
        .expect_play_cancel()
        .returning(move || seen.lock().unwrap().push("cancel"));
    let seen = std::sync::Arc::clone(&played);
    sounds
        .expect_play_restart()
        .returning(move || seen.lock().unwrap().push("restart"));
    sounds.expect_play_listening_on().return_const(());
    let mut text_paster = MockTextPaster::new();
    text_paster.expect_paste().never();
    let mut pipeline = pipeline_with(
        sounding_parts(recording_recorder(), text_paster, sounds),
        lenient_spotter(),
        MockKeyPresser::new(),
        RecordingMode::Toggle,
    );
    listen(&mut pipeline);
    say(&mut pipeline, "start dictation");
    // When
    say(&mut pipeline, "restart dictation");
    // Then
    assert!(pipeline.is_recording());
    assert_eq!(*played.lock().unwrap(), ["start", "restart"]);
}
