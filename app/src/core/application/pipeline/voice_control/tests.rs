use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

use super::*;
use crate::core::application::audio_recorder::{LiveChunk, MockAudioRecorder};
use crate::core::application::command_spotter::MockCommandSpotter;
use crate::core::application::key_presser::MockKeyPresser;
use crate::core::application::live_transcript_view::HiddenLiveTranscript;
use crate::core::application::pipeline::{PipelineConfig, PipelineParts};
use crate::core::application::sound_player::MockSoundPlayer;
use crate::core::application::text_paster::MockTextPaster;
use crate::core::application::text_refiner::MockTextRefiner;
use crate::core::application::transcriber::MockTranscriber;
use crate::core::application::visual_indicator::MockVisualIndicator;
use crate::core::application::{ChunkingCfg, PipelineInput};
use crate::core::domain::{
    AudioBuffer16k, CleanedText, PttSignal, RawTranscript, RecordingMode, SpottingEvent,
    VoiceCommand,
};

const PHRASE_LEAD: Duration = Duration::from_millis(100);
const PHRASE_LENGTH: Duration = Duration::from_millis(800);
/// Half a second of sound at 16 kHz.
const LEAD_IN_SAMPLES: usize = 8_000;

fn sounds() -> MockSoundPlayer {
    let mut sounds = MockSoundPlayer::new();
    sounds.expect_play_start().return_const(());
    sounds.expect_play_stop().return_const(());
    sounds.expect_play_cancel().return_const(());
    sounds.expect_play_listening_on().return_const(());
    sounds.expect_play_enter().return_const(());
    sounds.expect_play_clear().return_const(());
    sounds.expect_play_restart().return_const(());
    sounds
}

fn overlay() -> MockVisualIndicator {
    let mut overlay = MockVisualIndicator::new();
    overlay.expect_show_recording().return_const(());
    overlay.expect_show_recognizing_speech().return_const(());
    overlay.expect_show_refining_text().return_const(());
    overlay.expect_show_pasted().return_const(());
    overlay.expect_hide().return_const(());
    overlay.expect_show_command_sign().return_const(());
    overlay.expect_show_failure_notice().return_const(());
    overlay
        .expect_live_transcript_view()
        .returning(|| Box::new(HiddenLiveTranscript));
    overlay
}

fn idle_recorder() -> MockAudioRecorder {
    let mut recorder = MockAudioRecorder::new();
    recorder.expect_start().never();
    recorder
}

fn recording_recorder() -> MockAudioRecorder {
    let mut recorder = recording_recorder_without_lead_in();
    recorder.expect_begin_with().return_const(());
    recorder
}

fn recording_recorder_without_lead_in() -> MockAudioRecorder {
    let mut recorder = MockAudioRecorder::new();
    recorder.expect_start().returning(|| Ok(()));
    recorder.expect_start_live_chunks().returning(|_| Ok(None));
    recorder.expect_flush_live_chunks().return_const(());
    recorder
        .expect_stop()
        .returning(|| Ok(AudioBuffer16k::new(vec![0i16; 16_000])));
    recorder
}

fn answering_transcriber() -> MockTranscriber {
    let mut transcriber = MockTranscriber::new();
    transcriber
        .expect_transcribe()
        .returning(|_| Ok(Some(RawTranscript::new("hello".into()).unwrap())));
    transcriber
}

fn passthrough_refiner() -> MockTextRefiner {
    let mut refiner = MockTextRefiner::new();
    refiner
        .expect_refine()
        .returning(|raw| Ok(CleanedText::new(raw.as_str().to_owned()).unwrap()));
    refiner
}

fn listening_spotter() -> MockCommandSpotter {
    let mut spotter = MockCommandSpotter::new();
    spotter.expect_start_listening().returning(|_| Ok(()));
    spotter.expect_listen_for().return_const(());
    spotter.expect_stop_listening().return_const(());
    spotter
}

fn lenient_spotter() -> MockCommandSpotter {
    let mut spotter = listening_spotter();
    hears_nothing(&mut spotter);
    spotter
}

fn hears_nothing(spotter: &mut MockCommandSpotter) {
    spotter
        .expect_heard_since()
        .returning(|_| AudioBuffer16k::new(Vec::new()));
}

fn said(command: VoiceCommand, placement: CommandPlacement, start: Instant) -> PipelineInput {
    PipelineInput::Spotting(SpottingEvent::CommandsSaid {
        commands: vec![command],
        start,
        end: start + PHRASE_LENGTH,
        placement,
        timing: None,
    })
}

fn pipeline(
    recorder: MockAudioRecorder,
    paster: MockTextPaster,
    spotter: MockCommandSpotter,
    mode: RecordingMode,
) -> Pipeline {
    pipeline_with(
        recorder,
        paster,
        spotter,
        sounds(),
        MockKeyPresser::new(),
        mode,
    )
}

/// Toggle recording, transcribed in one go when it ends.
fn toggle_cfg() -> PipelineConfig {
    PipelineConfig::default()
}

/// The parts of a pipeline over `recorder` and `paster`, the other ports answering doubles.
fn parts(recorder: MockAudioRecorder, paster: MockTextPaster) -> PipelineParts {
    PipelineParts {
        recorder: Box::new(recorder),
        transcriber: Box::new(answering_transcriber()),
        refiner: Box::new(passthrough_refiner()),
        paster: Box::new(paster),
        sound_player: Box::new(sounds()),
        visual_indicator: Box::new(overlay()),
    }
}

#[expect(clippy::too_many_arguments, reason = "size baseline")]
fn pipeline_with(
    recorder: MockAudioRecorder,
    paster: MockTextPaster,
    spotter: MockCommandSpotter,
    sounds: MockSoundPlayer,
    keys: MockKeyPresser,
    mode: RecordingMode,
) -> Pipeline {
    let cfg = PipelineConfig { recording_mode: mode, ..toggle_cfg() };
    let parts = PipelineParts { sound_player: Box::new(sounds), ..parts(recorder, paster) };
    Pipeline::new(parts, cfg).with_voice_control(Box::new(spotter), Box::new(keys))
}

/// A pipeline in standby whose overlay is `overlay` and whose sounds are `sounds`.
fn standby_pipeline(overlay: MockVisualIndicator, sounds: MockSoundPlayer) -> Pipeline {
    let parts = PipelineParts {
        sound_player: Box::new(sounds),
        visual_indicator: Box::new(overlay),
        ..parts(idle_recorder(), MockTextPaster::new())
    };
    Pipeline::new(parts, toggle_cfg())
        .with_voice_control(Box::new(lenient_spotter()), Box::new(MockKeyPresser::new()))
}

/// A pipeline that transcribes a dictation in live chunks while it runs.
fn live_chunking_pipeline(recorder: MockAudioRecorder, spotter: MockCommandSpotter) -> Pipeline {
    let cfg = PipelineConfig {
        chunking: ChunkingCfg { chunk_threshold_secs: 1, ..ChunkingCfg::default() },
        ..PipelineConfig::default()
    };
    Pipeline::new(parts(recorder, MockTextPaster::new()), cfg)
        .with_voice_control(Box::new(spotter), Box::new(MockKeyPresser::new()))
}

fn listen(pipeline: &mut Pipeline) {
    pipeline.turn_listening_on();
}

fn press(pipeline: &mut Pipeline) {
    pipeline
        .handle(PipelineInput::Hotkey(PttSignal::Pressed))
        .unwrap();
}

/// Says `phrase` on its own and returns the moment the phrase started.
fn say(pipeline: &mut Pipeline, phrase: &str) -> Instant {
    say_at(pipeline, phrase, Instant::now())
}

/// Says `phrase` on its own in a segment starting at `segment_start`.
fn say_at(pipeline: &mut Pipeline, phrase: &str, segment_start: Instant) -> Instant {
    let start = segment_start + PHRASE_LEAD;
    let end = start + PHRASE_LENGTH;
    let events = [
        SpottingEvent::SpeechStarted(segment_start),
        SpottingEvent::PhraseSpotted { phrase: phrase.to_owned(), start, end, timing: None },
        SpottingEvent::SpeechEnded(end + PHRASE_LEAD),
    ];
    for event in events {
        pipeline.handle(PipelineInput::Spotting(event)).unwrap();
    }
    start
}

#[test]
fn should_record_with_the_hotkey_and_keep_listening() {
    // Given
    let mut spotter = MockCommandSpotter::new();
    let mut order = mockall::Sequence::new();
    spotter
        .expect_start_listening()
        .with(mockall::predicate::eq(VoiceCommand::STANDBY.to_vec()))
        .once()
        .in_sequence(&mut order)
        .returning(|_| Ok(()));
    spotter
        .expect_listen_for()
        .with(mockall::predicate::eq(VoiceCommand::DICTATING.to_vec()))
        .once()
        .in_sequence(&mut order)
        .return_const(());
    spotter
        .expect_listen_for()
        .with(mockall::predicate::eq(VoiceCommand::STANDBY.to_vec()))
        .once()
        .in_sequence(&mut order)
        .return_const(());
    spotter.expect_stop_listening().never();
    let mut paster = MockTextPaster::new();
    paster.expect_paste().once().returning(|_| Ok(()));
    let mut pipeline = pipeline(recording_recorder(), paster, spotter, RecordingMode::Toggle);
    listen(&mut pipeline);
    // When
    press(&mut pipeline);
    press(&mut pipeline);
    // Then
    assert!(!pipeline.is_recording());
    assert!(pipeline.voice.listening());
}

#[test]
fn should_end_a_hold_dictation_on_release_and_keep_listening() {
    // Given
    let mut spotter = MockCommandSpotter::new();
    spotter.expect_start_listening().returning(|_| Ok(()));
    spotter.expect_listen_for().return_const(());
    spotter.expect_stop_listening().never();
    let mut paster = MockTextPaster::new();
    paster.expect_paste().once().returning(|_| Ok(()));
    let mut pipeline = pipeline(recording_recorder(), paster, spotter, RecordingMode::Hold);
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
fn should_cut_the_recording_at_the_stop_phrase_before_stopping_it() {
    // Given
    let mut recorder = recording_recorder();
    let cut_at = std::sync::Arc::new(std::sync::Mutex::new(None));
    let seen = std::sync::Arc::clone(&cut_at);
    recorder
        .expect_truncate_at()
        .once()
        .returning(move |cut| *seen.lock().unwrap() = Some(cut));
    let mut paster = MockTextPaster::new();
    paster.expect_paste().once().returning(|_| Ok(()));
    let mut pipeline = pipeline(recorder, paster, lenient_spotter(), RecordingMode::Toggle);
    listen(&mut pipeline);
    say(&mut pipeline, "start dictation");
    // When
    let phrase_start = say(&mut pipeline, "stop dictation");
    // Then
    assert_eq!(*cut_at.lock().unwrap(), Some(phrase_start));
}

#[test]
fn should_listen_for_the_phrases_of_each_state() {
    // Given
    let mut spotter = MockCommandSpotter::new();
    let mut order = mockall::Sequence::new();
    spotter
        .expect_start_listening()
        .with(mockall::predicate::eq(VoiceCommand::STANDBY.to_vec()))
        .once()
        .in_sequence(&mut order)
        .returning(|_| Ok(()));
    spotter
        .expect_listen_for()
        .with(mockall::predicate::eq(VoiceCommand::DICTATING.to_vec()))
        .once()
        .in_sequence(&mut order)
        .return_const(());
    spotter
        .expect_listen_for()
        .with(mockall::predicate::eq(VoiceCommand::STANDBY.to_vec()))
        .once()
        .in_sequence(&mut order)
        .return_const(());
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
fn should_keep_recording_and_listening_when_the_service_goes_away_during_a_dictation() {
    // Given
    let mut spotter = MockCommandSpotter::new();
    spotter.expect_start_listening().returning(|_| Ok(()));
    spotter.expect_listen_for().return_const(());
    spotter.expect_stop_listening().never();
    hears_nothing(&mut spotter);
    let mut paster = MockTextPaster::new();
    paster.expect_paste().never();
    let mut pipeline = pipeline(recording_recorder(), paster, spotter, RecordingMode::Toggle);
    listen(&mut pipeline);
    say(&mut pipeline, "start dictation");
    // When
    pipeline.handle(PipelineInput::SpottingLost).unwrap();
    // Then
    assert!(pipeline.is_recording());
}

#[test]
fn should_listen_for_the_commands_of_a_running_dictation_when_the_service_is_back() {
    // Given
    let mut spotter = MockCommandSpotter::new();
    let mut order = mockall::Sequence::new();
    spotter.expect_start_listening().returning(|_| Ok(()));
    hears_nothing(&mut spotter);
    spotter
        .expect_listen_for()
        .with(mockall::predicate::eq(VoiceCommand::DICTATING.to_vec()))
        .times(2)
        .in_sequence(&mut order)
        .return_const(());
    let mut paster = MockTextPaster::new();
    paster.expect_paste().never();
    let mut pipeline = pipeline(recording_recorder(), paster, spotter, RecordingMode::Toggle);
    listen(&mut pipeline);
    say(&mut pipeline, "start dictation");
    pipeline.handle(PipelineInput::SpottingLost).unwrap();
    // When
    pipeline.handle(PipelineInput::SpottingBack).unwrap();
    // Then the mock expectations hold
}

#[test]
fn should_cancel_a_dictation_on_escape_and_keep_listening() {
    // Given
    let mut spotter = MockCommandSpotter::new();
    let mut order = mockall::Sequence::new();
    spotter.expect_start_listening().returning(|_| Ok(()));
    spotter
        .expect_listen_for()
        .with(mockall::predicate::eq(VoiceCommand::DICTATING.to_vec()))
        .once()
        .in_sequence(&mut order)
        .return_const(());
    spotter
        .expect_listen_for()
        .with(mockall::predicate::eq(VoiceCommand::STANDBY.to_vec()))
        .once()
        .in_sequence(&mut order)
        .return_const(());
    spotter.expect_stop_listening().never();
    hears_nothing(&mut spotter);
    let mut paster = MockTextPaster::new();
    paster.expect_paste().never();
    let mut pipeline = pipeline(recording_recorder(), paster, spotter, RecordingMode::Toggle);
    listen(&mut pipeline);
    say(&mut pipeline, "start dictation");
    // When
    pipeline
        .handle(PipelineInput::Hotkey(PttSignal::Cancel))
        .unwrap();
    // Then
    assert!(!pipeline.is_recording());
    assert!(pipeline.voice.listening());
}

#[test]
fn should_stop_listening_on_shutdown() {
    // Given
    let mut spotter = MockCommandSpotter::new();
    spotter
        .expect_start_listening()
        .once()
        .returning(|_| Ok(()));
    spotter.expect_stop_listening().once().return_const(());
    let mut pipeline = pipeline(
        idle_recorder(),
        MockTextPaster::new(),
        spotter,
        RecordingMode::Toggle,
    );
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
fn should_start_listening_as_soon_as_the_pipeline_runs() {
    // Given
    let mut spotter = MockCommandSpotter::new();
    spotter
        .expect_start_listening()
        .with(mockall::predicate::eq(VoiceCommand::STANDBY.to_vec()))
        .once()
        .returning(|_| Ok(()));
    let mut pipeline = pipeline(
        idle_recorder(),
        MockTextPaster::new(),
        spotter,
        RecordingMode::Toggle,
    );
    let (inputs, rx) = std::sync::mpsc::channel::<PipelineInput>();
    drop(inputs);
    // When
    let _ = pipeline.run(rx);
    // Then
    assert!(pipeline.voice.listening());
}

#[test]
#[expect(clippy::too_many_lines, reason = "size baseline")]
fn should_keep_listening_when_a_dictation_cannot_start() {
    // Given the microphone cannot be opened for recording
    let mut recorder = MockAudioRecorder::new();
    recorder
        .expect_start()
        .returning(|| Err(anyhow::anyhow!("microphone busy")));
    let listened = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let mut spotter = MockCommandSpotter::new();
    spotter.expect_start_listening().returning(|_| Ok(()));
    let seen = std::sync::Arc::clone(&listened);
    spotter
        .expect_listen_for()
        .returning(move |commands| seen.lock().unwrap().push(commands.to_vec()));
    let mut pipeline = pipeline(
        recorder,
        MockTextPaster::new(),
        spotter,
        RecordingMode::Toggle,
    );
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

#[test]
fn should_listen_for_the_dictation_phrases_before_the_recording_starts() {
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

/// A spotter that heard half a second since `end`, and nothing since any other moment.
fn heard_half_a_second_since(end: Instant) -> MockCommandSpotter {
    let mut spotter = listening_spotter();
    spotter.expect_heard_since().returning(move |since| {
        let samples = if since == end { LEAD_IN_SAMPLES } else { 0 };
        AudioBuffer16k::new(vec![0; samples])
    });
    spotter
}

/// A recorder that notes the length of every lead-in it begins with.
fn noting_lead_ins(lead_ins: &Arc<Mutex<Vec<usize>>>) -> MockAudioRecorder {
    let mut recorder = recording_recorder_without_lead_in();
    let seen = Arc::clone(lead_ins);
    recorder
        .expect_begin_with()
        .returning(move |lead_in| seen.lock().unwrap().push(lead_in.samples().len()));
    recorder
}

#[test]
fn should_begin_a_voice_dictation_where_the_start_ended_wherever_it_was_said() {
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
fn should_begin_a_restarted_dictation_where_restart_ended() {
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
fn should_paste_a_restarted_dictation_without_the_restart_its_lead_in_caught() {
    // Given a dictation runs, and its restart's lead-in caught the end of "restart restart"
    let mut transcriber = MockTranscriber::new();
    transcriber.expect_transcribe().returning(|_| {
        Ok(Some(
            RawTranscript::new("Restart, lunch is at noon.".into()).unwrap(),
        ))
    });
    let pasted = Arc::new(Mutex::new(Vec::new()));
    let seen = Arc::clone(&pasted);
    let mut paster = MockTextPaster::new();
    paster.expect_paste().returning(move |text| {
        seen.lock().unwrap().push(text.as_str().to_owned());
        Ok(())
    });
    let parts =
        PipelineParts { transcriber: Box::new(transcriber), ..parts(recording_recorder(), paster) };
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
#[expect(clippy::too_many_lines, reason = "size baseline")]
fn should_take_the_heard_speech_once_the_recorder_runs_and_before_live_chunks() {
    // Given
    let order = Arc::new(Mutex::new(Vec::new()));
    let note = |step: &'static str| {
        let seen = Arc::clone(&order);
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
    let mut pipeline = live_chunking_pipeline(recorder, spotter);
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

#[test]
fn should_paste_and_then_press_return_on_enter_during_a_dictation() {
    // Given
    let order = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let mut recorder = recording_recorder();
    recorder.expect_truncate_at().return_const(());
    let mut paster = MockTextPaster::new();
    let seen = std::sync::Arc::clone(&order);
    paster.expect_paste().once().returning(move |_| {
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
        recorder,
        paster,
        lenient_spotter(),
        sounds(),
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
fn should_play_only_the_restart_sound_on_restart() {
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
    let mut paster = MockTextPaster::new();
    paster.expect_paste().never();
    let mut pipeline = pipeline_with(
        recording_recorder(),
        paster,
        lenient_spotter(),
        sounds,
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

/// How long the transcriber of the dropped dictation holds its last chunk at most.
const SLOW_REQUEST: Duration = Duration::from_secs(2);

/// A transcriber that notes "transcribed" once `released` lets its request end.
fn slow_transcriber(
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
fn recorder_with_one_live_chunk() -> MockAudioRecorder {
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
fn should_give_the_restart_cue_before_the_dropped_dictation_is_transcribed() {
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
fn should_not_carry_out_again_a_command_the_service_reports_twice_across_an_outage() {
    // Given
    let mut keys = MockKeyPresser::new();
    keys.expect_press_enter().once().returning(|| Ok(()));
    let mut pipeline = pipeline_with(
        idle_recorder(),
        MockTextPaster::new(),
        lenient_spotter(),
        sounds(),
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
fn should_play_no_sound_when_the_service_goes_away_and_comes_back() {
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
fn should_show_commands_unavailable_once_while_the_service_stays_away() {
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
fn should_not_touch_the_sign_after_a_short_outage() {
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
fn should_not_show_the_sign_when_the_service_is_not_away() {
    // Given
    let mut overlay = overlay();
    overlay.expect_show_commands_unavailable().never();
    let mut pipeline = standby_pipeline(overlay, sounds());
    listen(&mut pipeline);
    // When
    pipeline.handle(PipelineInput::SpottingStillAway).unwrap();
    // Then the mock expectations hold
}
