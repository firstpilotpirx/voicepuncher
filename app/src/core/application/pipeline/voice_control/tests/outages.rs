//! The restart cue and the signs while the command spotting service goes away.

use super::*;

/// How long the transcriber of the dropped dictation holds its last chunk at most.
pub(super) const SLOW_REQUEST: Duration = Duration::from_secs(2);

/// A transcriber that notes "transcribed" once `released` lets its request end.
pub(super) fn slow_transcriber(
    released: mpsc::Receiver<()>,
    order: Arc<Mutex<Vec<&'static str>>>,
) -> MockTranscriber {
    let released = Mutex::new(released);
    let mut transcriber = MockTranscriber::new();
    transcriber.expect_transcribe().returning(move |_| {
        let _ = released.lock().unwrap().recv_timeout(SLOW_REQUEST);
        order.lock().unwrap().push("transcribed");
        Ok(None)
    });
    transcriber
}

/// A recorder whose first recording holds one chunk and whose second holds none.
pub(super) fn recorder_with_one_live_chunk() -> MockAudioRecorder {
    let (first_tx, first) = mpsc::channel();
    first_tx
        .send(LiveChunk {
            audio: AudioBuffer16k::new(vec![0i16; 16_000]),
            enqueued_at: Instant::now(),
        })
        .unwrap();
    let (_, second) = mpsc::channel();
    let recordings = Mutex::new(vec![second, first]);
    let mut recorder = MockAudioRecorder::new();
    recorder.expect_start().returning(|| Ok(()));
    recorder.expect_begin_with().return_const(());
    recorder.expect_flush_live_chunks().return_const(());
    recorder
        .expect_stop()
        .returning(|| Ok(AudioBuffer16k::new(Vec::new())));
    recorder
        .expect_start_live_chunks()
        .returning(move |_| Ok(recordings.lock().unwrap().pop()));
    recorder
}

#[test]
pub(super) fn should_give_the_restart_cue_before_the_dropped_dictation_is_transcribed() {
    // Given a dictation whose last chunk is still being transcribed
    let order = Arc::new(Mutex::new(Vec::new()));
    let (release, released) = mpsc::channel();
    let mut sounds = MockSoundPlayer::new();
    sounds.expect_play_start().return_const(());
    sounds.expect_play_listening_on().return_const(());
    let seen = Arc::clone(&order);
    sounds
        .expect_play_restart()
        .returning(move || seen.lock().unwrap().push("restart"));
    let mut paster = MockTextPaster::new();
    paster.expect_paste().never();
    let parts = PipelineParts {
        transcriber: Box::new(slow_transcriber(released, Arc::clone(&order))),
        sound_player: Box::new(sounds),
        ..parts(recorder_with_one_live_chunk(), paster)
    };
    let mut pipeline = Pipeline::new(
        parts,
        PipelineConfig {
            chunking: ChunkingCfg { chunk_threshold_secs: 1, ..ChunkingCfg::default() },
            ..PipelineConfig::default()
        },
    )
    .with_voice_control(Box::new(lenient_spotter()), Box::new(MockKeyPresser::new()));
    listen(&mut pipeline);
    say(&mut pipeline, "start dictation");
    // When
    say(&mut pipeline, "restart dictation");
    let heard = order.lock().unwrap().clone();
    drop(release);
    // Then
    assert!(pipeline.is_recording());
    assert_eq!(heard, ["restart"]);
}

#[test]
pub(super) fn should_not_carry_out_again_a_command_the_service_reports_twice_across_an_outage() {
    // Given
    let mut keys = MockKeyPresser::new();
    keys.expect_press_enter().once().returning(|| Ok(()));
    let mut pipeline = pipeline_with(
        sounding_parts(idle_recorder(), MockTextPaster::new(), sounds()),
        lenient_spotter(),
        keys,
        RecordingMode::Toggle,
    );
    listen(&mut pipeline);
    let said_at = Instant::now();
    say_at(&mut pipeline, "press enter", said_at);
    pipeline.handle(PipelineInput::SpottingLost).unwrap();
    pipeline.handle(PipelineInput::SpottingBack).unwrap();
    // When the audio sent again reports the same command
    say_at(&mut pipeline, "press enter", said_at);
    // Then the mock expectations hold
}

#[test]
pub(super) fn should_play_no_sound_when_the_service_goes_away_and_comes_back() {
    // Given
    let mut sounds = MockSoundPlayer::new();
    sounds.expect_play_listening_on().once().return_const(());
    sounds.expect_play_listening_off().never();
    let mut pipeline = standby_pipeline(overlay(), sounds);
    listen(&mut pipeline);
    // When
    pipeline.handle(PipelineInput::SpottingLost).unwrap();
    pipeline.handle(PipelineInput::SpottingBack).unwrap();
    // Then the mock expectations hold
}

#[test]
pub(super) fn should_show_commands_unavailable_once_while_the_service_stays_away() {
    // Given
    let mut overlay = overlay();
    let mut order = mockall::Sequence::new();
    overlay
        .expect_show_commands_unavailable()
        .once()
        .in_sequence(&mut order)
        .return_const(());
    overlay
        .expect_hide_commands_unavailable()
        .once()
        .in_sequence(&mut order)
        .return_const(());
    let mut pipeline = standby_pipeline(overlay, sounds());
    listen(&mut pipeline);
    pipeline.handle(PipelineInput::SpottingLost).unwrap();
    // When
    pipeline.handle(PipelineInput::SpottingStillAway).unwrap();
    pipeline.handle(PipelineInput::SpottingStillAway).unwrap();
    pipeline.handle(PipelineInput::SpottingBack).unwrap();
    // Then the mock expectations hold
}

#[test]
pub(super) fn should_not_touch_the_sign_after_a_short_outage() {
    // Given
    let mut overlay = overlay();
    overlay.expect_show_commands_unavailable().never();
    overlay.expect_hide_commands_unavailable().never();
    let mut pipeline = standby_pipeline(overlay, sounds());
    listen(&mut pipeline);
    // When
    pipeline.handle(PipelineInput::SpottingLost).unwrap();
    pipeline.handle(PipelineInput::SpottingBack).unwrap();
    // Then the mock expectations hold
}

#[test]
pub(super) fn should_not_show_the_sign_when_the_service_is_not_away() {
    // Given
    let mut overlay = overlay();
    overlay.expect_show_commands_unavailable().never();
    let mut pipeline = standby_pipeline(overlay, sounds());
    listen(&mut pipeline);
    // When
    pipeline.handle(PipelineInput::SpottingStillAway).unwrap();
    // Then the mock expectations hold
}
