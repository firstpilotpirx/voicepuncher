//! Stand-in parts and pipelines, and the speech the tests say to them.

use super::*;

pub(super) const PHRASE_LEAD: Duration = Duration::from_millis(100);

pub(super) const PHRASE_LENGTH: Duration = Duration::from_millis(800);

/// Half a second of sound at 16 kHz.
pub(super) const LEAD_IN_SAMPLES: usize = 8_000;

pub(super) fn sounds() -> MockSoundPlayer {
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

pub(super) fn overlay() -> MockVisualIndicator {
    let mut overlay = crate::core::application::prelude::accepting_overlay();
    overlay
        .expect_live_transcript_view()
        .returning(|| Box::new(HiddenLiveTranscript));
    overlay
}

pub(super) fn idle_recorder() -> MockAudioRecorder {
    let mut recorder = MockAudioRecorder::new();
    recorder.expect_start().never();
    recorder
}

pub(super) fn recording_recorder() -> MockAudioRecorder {
    let mut recorder = recording_recorder_without_lead_in();
    recorder.expect_begin_with().return_const(());
    recorder
}

pub(super) fn recording_recorder_without_lead_in() -> MockAudioRecorder {
    let mut recorder = MockAudioRecorder::new();
    recorder.expect_start().returning(|| Ok(()));
    recorder.expect_start_live_chunks().returning(|_| Ok(None));
    recorder.expect_flush_live_chunks().return_const(());
    recorder
        .expect_stop()
        .returning(|| Ok(AudioBuffer16k::new(vec![0i16; 16_000])));
    recorder
}

pub(super) fn answering_transcriber() -> MockTranscriber {
    let mut transcriber = MockTranscriber::new();
    transcriber
        .expect_transcribe()
        .returning(|_| Ok(Some(RawTranscript::new("hello".into()).unwrap())));
    transcriber
}

pub(super) fn listening_spotter() -> MockCommandSpotter {
    let mut spotter = MockCommandSpotter::new();
    spotter.expect_start_listening().returning(|_| Ok(()));
    spotter.expect_listen_for().return_const(());
    spotter.expect_stop_listening().return_const(());
    spotter
}

pub(super) fn lenient_spotter() -> MockCommandSpotter {
    let mut spotter = listening_spotter();
    hears_nothing(&mut spotter);
    spotter
}

/// A spotter that starts listening for `starts` once (any commands, as often as asked, when
/// `None`), then listens for each of `listens` once, in this order.
pub(super) fn ordered_spotter(
    starts: Option<&'static [VoiceCommand]>,
    listens: &[&'static [VoiceCommand]],
) -> MockCommandSpotter {
    let mut spotter = MockCommandSpotter::new();
    let mut order = mockall::Sequence::new();
    match starts {
        Some(commands) => {
            spotter
                .expect_start_listening()
                .with(mockall::predicate::eq(commands.to_vec()))
                .once()
                .in_sequence(&mut order)
                .returning(|_| Ok(()));
        }
        None => {
            spotter.expect_start_listening().returning(|_| Ok(()));
        }
    }
    for commands in listens {
        spotter
            .expect_listen_for()
            .with(mockall::predicate::eq(commands.to_vec()))
            .once()
            .in_sequence(&mut order)
            .return_const(());
    }
    spotter
}

/// A spotter that listens for whatever it is asked and never stops listening.
pub(super) fn staying_spotter() -> MockCommandSpotter {
    let mut spotter = MockCommandSpotter::new();
    spotter.expect_start_listening().returning(|_| Ok(()));
    spotter.expect_listen_for().return_const(());
    spotter.expect_stop_listening().never();
    spotter
}

/// A paster that pastes exactly `times` times.
pub(super) fn pasting(times: usize) -> MockTextPaster {
    let mut paster = MockTextPaster::new();
    paster.expect_paste().times(times).returning(|_| Ok(()));
    paster
}

pub(super) fn hears_nothing(spotter: &mut MockCommandSpotter) {
    spotter
        .expect_heard_since()
        .returning(|_| AudioBuffer16k::new(Vec::new()));
}

pub(super) fn said(
    command: VoiceCommand,
    placement: CommandPlacement,
    start: Instant,
) -> PipelineInput {
    PipelineInput::Spotting(SpottingEvent::CommandsSaid {
        commands: vec![command],
        start,
        end: start + PHRASE_LENGTH,
        placement,
        timing: None,
    })
}

pub(super) fn pipeline(
    recorder: MockAudioRecorder,
    paster: MockTextPaster,
    spotter: MockCommandSpotter,
    mode: RecordingMode,
) -> Pipeline {
    pipeline_with(
        sounding_parts(recorder, paster, sounds()),
        spotter,
        MockKeyPresser::new(),
        mode,
    )
}

/// Toggle recording, transcribed in one go when it ends.
pub(super) fn toggle_cfg() -> PipelineConfig {
    PipelineConfig::default()
}

/// The parts of a pipeline over `recorder` and `paster`, the other ports answering doubles.
pub(super) fn parts(recorder: MockAudioRecorder, paster: MockTextPaster) -> PipelineParts {
    PipelineParts {
        recorder: Box::new(recorder),
        transcriber: Box::new(answering_transcriber()),
        paster: Box::new(paster),
        sound_player: Box::new(sounds()),
        visual_indicator: Box::new(overlay()),
    }
}

pub(super) fn pipeline_with(
    parts: PipelineParts,
    spotter: MockCommandSpotter,
    keys: MockKeyPresser,
    mode: RecordingMode,
) -> Pipeline {
    let cfg = PipelineConfig { recording_mode: mode, ..toggle_cfg() };
    Pipeline::new(parts, cfg).with_voice_control(Box::new(spotter), Box::new(keys))
}

/// The parts of a pipeline that records with `recorder`, pastes with `paster` and plays `sounds`.
pub(super) fn sounding_parts(
    recorder: MockAudioRecorder,
    paster: MockTextPaster,
    sounds: MockSoundPlayer,
) -> PipelineParts {
    PipelineParts { sound_player: Box::new(sounds), ..parts(recorder, paster) }
}

/// A pipeline in standby whose overlay is `overlay` and whose sounds are `sounds`.
pub(super) fn standby_pipeline(overlay: MockVisualIndicator, sounds: MockSoundPlayer) -> Pipeline {
    let parts = PipelineParts {
        sound_player: Box::new(sounds),
        visual_indicator: Box::new(overlay),
        ..parts(idle_recorder(), MockTextPaster::new())
    };
    Pipeline::new(parts, toggle_cfg())
        .with_voice_control(Box::new(lenient_spotter()), Box::new(MockKeyPresser::new()))
}

/// A pipeline that transcribes a dictation in live chunks while it runs.
pub(super) fn live_chunking_pipeline(
    recorder: MockAudioRecorder,
    spotter: MockCommandSpotter,
) -> Pipeline {
    let cfg = PipelineConfig {
        chunking: ChunkingCfg { chunk_threshold_secs: 1, ..ChunkingCfg::default() },
        ..PipelineConfig::default()
    };
    Pipeline::new(parts(recorder, MockTextPaster::new()), cfg)
        .with_voice_control(Box::new(spotter), Box::new(MockKeyPresser::new()))
}

pub(super) fn listen(pipeline: &mut Pipeline) {
    pipeline.turn_listening_on();
}

pub(super) fn press(pipeline: &mut Pipeline) {
    pipeline
        .handle(PipelineInput::Hotkey(PttSignal::Pressed))
        .unwrap();
}

/// Says `phrase` on its own and returns the moment the phrase started.
pub(super) fn say(pipeline: &mut Pipeline, phrase: &str) -> Instant {
    say_at(pipeline, phrase, Instant::now())
}

/// Says `phrase` on its own in a segment starting at `segment_start`.
pub(super) fn say_at(pipeline: &mut Pipeline, phrase: &str, segment_start: Instant) -> Instant {
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
