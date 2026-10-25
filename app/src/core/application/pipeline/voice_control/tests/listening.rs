//! What the pipeline listens for in each state and across outages of the service.

use super::*;

#[test]
pub(super) fn should_record_with_the_hotkey_and_keep_listening() {
    // Given
    let mut spotter = ordered_spotter(
        Some(&VoiceCommand::STANDBY),
        &[&VoiceCommand::DICTATING, &VoiceCommand::STANDBY],
    );
    spotter.expect_stop_listening().never();
    let mut pipeline = pipeline(
        recording_recorder(),
        pasting(1),
        spotter,
        RecordingMode::Toggle,
    );
    listen(&mut pipeline);
    // When
    press(&mut pipeline);
    press(&mut pipeline);
    // Then
    assert!(!pipeline.is_recording());
    assert!(pipeline.voice.listening());
}

#[test]
pub(super) fn should_end_a_hold_dictation_on_release_and_keep_listening() {
    // Given
    let mut pipeline = pipeline(
        recording_recorder(),
        pasting(1),
        staying_spotter(),
        RecordingMode::Hold,
    );
    listen(&mut pipeline);
    press(&mut pipeline);
    // When
    pipeline
        .handle(PipelineInput::Hotkey(PttSignal::Released))
        .unwrap();
    // Then
    assert!(!pipeline.is_recording());
    assert!(pipeline.voice.listening());
}

#[test]
pub(super) fn should_cut_the_recording_at_the_stop_phrase_before_stopping_it() {
    // Given
    let mut recorder = recording_recorder();
    let cut_at = std::sync::Arc::new(std::sync::Mutex::new(None));
    let seen = std::sync::Arc::clone(&cut_at);
    recorder
        .expect_truncate_at()
        .once()
        .returning(move |cut| *seen.lock().unwrap() = Some(cut));
    let mut pipeline = pipeline(
        recorder,
        pasting(1),
        lenient_spotter(),
        RecordingMode::Toggle,
    );
    listen(&mut pipeline);
    say(&mut pipeline, "start dictation");
    // When
    let phrase_start = say(&mut pipeline, "stop dictation");
    // Then
    assert_eq!(*cut_at.lock().unwrap(), Some(phrase_start));
}

#[test]
pub(super) fn should_listen_for_the_phrases_of_each_state() {
    // Given
    let mut spotter = ordered_spotter(
        Some(&VoiceCommand::STANDBY),
        &[&VoiceCommand::DICTATING, &VoiceCommand::STANDBY],
    );
    hears_nothing(&mut spotter);
    let mut recorder = recording_recorder();
    recorder.expect_truncate_at().return_const(());
    let mut paster = MockTextPaster::new();
    paster.expect_paste().returning(|_| Ok(()));
    let mut pipeline = pipeline(recorder, paster, spotter, RecordingMode::Toggle);
    listen(&mut pipeline);
    // When
    say(&mut pipeline, "start dictation");
    say(&mut pipeline, "stop dictation");
    // Then the mock expectations hold
}

#[test]
pub(super) fn should_keep_recording_and_listening_when_the_service_goes_away_during_a_dictation() {
    // Given
    let spotter = staying_spotter();
    let mut pipeline = dictating_without_pasting(spotter);
    // When
    pipeline.handle(PipelineInput::SpottingLost).unwrap();
    // Then
    assert!(pipeline.is_recording());
}

#[test]
pub(super) fn should_listen_for_the_commands_of_a_running_dictation_when_the_service_is_back() {
    // Given
    let spotter = ordered_spotter(None, &[&VoiceCommand::DICTATING, &VoiceCommand::DICTATING]);
    let mut pipeline = dictating_without_pasting(spotter);
    pipeline.handle(PipelineInput::SpottingLost).unwrap();
    // When
    pipeline.handle(PipelineInput::SpottingBack).unwrap();
    // Then the mock expectations hold
}

#[test]
pub(super) fn should_cancel_a_dictation_on_escape_and_keep_listening() {
    // Given
    let mut spotter = ordered_spotter(None, &[&VoiceCommand::DICTATING, &VoiceCommand::STANDBY]);
    spotter.expect_stop_listening().never();
    let mut pipeline = dictating_without_pasting(spotter);
    // When
    pipeline
        .handle(PipelineInput::Hotkey(PttSignal::Cancel))
        .unwrap();
    // Then
    assert!(!pipeline.is_recording());
    assert!(pipeline.voice.listening());
}

#[test]
pub(super) fn should_stop_listening_on_shutdown() {
    // Given
    let mut spotter = ordered_spotter(Some(&VoiceCommand::STANDBY), &[]);
    spotter.expect_stop_listening().once().return_const(());
    let mut pipeline = idle_pipeline(spotter);
    let (inputs, rx) = std::sync::mpsc::channel::<PipelineInput>();
    inputs.send(PipelineInput::Shutdown).unwrap();
    // When
    let result = pipeline.run(rx);
    // Then
    assert!(result.is_ok());
    assert!(!pipeline.voice.listening());
    drop(inputs);
}

#[test]
pub(super) fn should_start_listening_as_soon_as_the_pipeline_runs() {
    // Given
    let spotter = ordered_spotter(Some(&VoiceCommand::STANDBY), &[]);
    let mut pipeline = idle_pipeline(spotter);
    let (inputs, rx) = std::sync::mpsc::channel::<PipelineInput>();
    drop(inputs);
    // When
    let _ = pipeline.run(rx);
    // Then
    assert!(pipeline.voice.listening());
}

#[test]
pub(super) fn should_keep_listening_when_a_dictation_cannot_start() {
    // Given the microphone cannot be opened for recording
    let listened = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let mut pipeline = unstartable_pipeline(&listened);
    listen(&mut pipeline);
    // When
    let segment_start = Instant::now();
    let start = segment_start + PHRASE_LEAD;
    let end = start + PHRASE_LENGTH;
    let results: Vec<anyhow::Result<()>> = [
        SpottingEvent::SpeechStarted(segment_start),
        SpottingEvent::PhraseSpotted { phrase: "start dictation".into(), start, end, timing: None },
        SpottingEvent::SpeechEnded(end + PHRASE_LEAD),
    ]
    .into_iter()
    .map(|event| pipeline.handle(PipelineInput::Spotting(event)))
    .collect();
    // Then
    assert!(
        results.iter().all(std::result::Result::is_ok),
        "{results:?}"
    );
    assert!(!pipeline.is_recording());
    assert!(pipeline.voice.listening());
    assert_eq!(
        *listened.lock().unwrap(),
        [
            VoiceCommand::DICTATING.to_vec(),
            VoiceCommand::STANDBY.to_vec()
        ]
    );
}

/// A listening pipeline whose recorder cannot start; the commands it listens for go to `listened`.
fn unstartable_pipeline(
    listened: &std::sync::Arc<std::sync::Mutex<Vec<Vec<VoiceCommand>>>>,
) -> Pipeline {
    let mut recorder = MockAudioRecorder::new();
    recorder
        .expect_start()
        .returning(|| Err(anyhow::anyhow!("microphone busy")));
    let mut spotter = MockCommandSpotter::new();
    spotter.expect_start_listening().returning(|_| Ok(()));
    let seen = std::sync::Arc::clone(listened);
    spotter
        .expect_listen_for()
        .returning(move |commands| seen.lock().unwrap().push(commands.to_vec()));
    pipeline(
        recorder,
        MockTextPaster::new(),
        spotter,
        RecordingMode::Toggle,
    )
}

#[test]
pub(super) fn should_listen_for_the_dictation_phrases_before_the_recording_starts() {
    // Given
    let order = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let mut spotter = MockCommandSpotter::new();
    spotter.expect_start_listening().returning(|_| Ok(()));
    let seen = std::sync::Arc::clone(&order);
    spotter
        .expect_listen_for()
        .returning(move |commands| seen.lock().unwrap().push(format!("listen {commands:?}")));
    hears_nothing(&mut spotter);
    let mut recorder = MockAudioRecorder::new();
    let seen = std::sync::Arc::clone(&order);
    recorder.expect_start().returning(move || {
        seen.lock().unwrap().push("record".to_owned());
        Ok(())
    });
    recorder.expect_begin_with().return_const(());
    let mut pipeline = pipeline(
        recorder,
        MockTextPaster::new(),
        spotter,
        RecordingMode::Toggle,
    );
    listen(&mut pipeline);
    // When
    say(&mut pipeline, "start dictation");
    // Then
    let order = order.lock().unwrap();
    assert_eq!(
        order.first().map(String::as_str),
        Some(format!("listen {:?}", VoiceCommand::DICTATING).as_str())
    );
    assert_eq!(order.get(1).map(String::as_str), Some("record"));
}

/// A pipeline listening with `spotter`, which hears nothing, in a dictation started by voice
/// that pastes nothing.
fn dictating_without_pasting(mut spotter: MockCommandSpotter) -> Pipeline {
    hears_nothing(&mut spotter);
    let mut pipeline = pipeline(
        recording_recorder(),
        pasting(0),
        spotter,
        RecordingMode::Toggle,
    );
    listen(&mut pipeline);
    say(&mut pipeline, "start dictation");
    pipeline
}

/// A pipeline that records nothing and pastes nothing, listening with `spotter` once it runs.
fn idle_pipeline(spotter: MockCommandSpotter) -> Pipeline {
    pipeline(
        idle_recorder(),
        MockTextPaster::new(),
        spotter,
        RecordingMode::Toggle,
    )
}
