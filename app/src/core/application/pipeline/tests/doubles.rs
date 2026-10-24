//! The doubles of the pipeline's unit tests: quiet ones, noting ones and failing ones.

use std::sync::{Arc, Mutex, OnceLock};

use anyhow::anyhow;

use crate::core::application::live_transcript_view::HiddenLiveTranscript;
use crate::core::application::pipeline_activity::{PipelineActivity, PipelineActivitySink};

/// The mocks of the five ports, shared with the builder.
pub(super) use crate::core::application::{
    audio_recorder::MockAudioRecorder, sound_player::MockSoundPlayer, text_paster::MockTextPaster,
    transcriber::MockTranscriber, visual_indicator::MockVisualIndicator,
};
use crate::core::domain::{AudioBuffer16k, RawTranscript};

/// What was noted, in order, shared with the double that notes it.
pub(super) type Noted<T> = Arc<Mutex<Vec<T>>>;

pub(super) fn noted<T>() -> Noted<T> {
    Arc::new(Mutex::new(Vec::new()))
}

pub(super) fn seen<T: Clone>(noted: &Noted<T>) -> Vec<T> {
    noted.lock().unwrap().clone()
}

/// A recorder started once and stopped once, with `audio` recorded.
pub(super) fn recording(audio: AudioBuffer16k) -> MockAudioRecorder {
    let mut recorder = started_once();
    recorder
        .expect_stop()
        .once()
        .returning(move || Ok(audio.clone()));
    recorder
}

/// A recorder started once and never stopped by the test's end.
pub(super) fn started_once() -> MockAudioRecorder {
    let mut recorder = MockAudioRecorder::new();
    recorder.expect_start().once().returning(|| Ok(()));
    recorder
        .expect_start_live_chunks()
        .times(..)
        .returning(|_| Ok(None));
    recorder
        .expect_flush_live_chunks()
        .times(..)
        .return_const(());
    recorder
}

/// A recorder never started.
pub(super) fn idle_recorder() -> MockAudioRecorder {
    let mut recorder = MockAudioRecorder::new();
    recorder.expect_start().never();
    recorder.expect_stop().never();
    recorder
}

/// Speech recognition that hears `text` in every recording.
pub(in crate::core::application::pipeline) fn transcribing(text: &'static str) -> MockTranscriber {
    let mut transcriber = MockTranscriber::new();
    transcriber
        .expect_transcribe()
        .returning(move |_| Ok(Some(RawTranscript::new(text.to_owned())?)));
    transcriber
}

pub(super) fn transcription_down() -> MockTranscriber {
    let mut transcriber = MockTranscriber::new();
    transcriber
        .expect_transcribe()
        .returning(|_| Err(anyhow!("whisper failure")));
    transcriber
}

/// A paster that takes every text.
pub(super) fn pasting() -> MockTextPaster {
    let mut paster = MockTextPaster::new();
    paster.expect_paste().returning(|_| Ok(()));
    paster
}

/// A paster that notes every text it pastes.
pub(super) fn noting_paster() -> (MockTextPaster, Noted<String>) {
    let texts = noted();
    let mut paster = MockTextPaster::new();
    let noting = Arc::clone(&texts);
    paster.expect_paste().returning(move |text| {
        noting.lock().unwrap().push(text.as_str().to_owned());
        Ok(())
    });
    (paster, texts)
}

pub(super) fn paste_down() -> MockTextPaster {
    let mut paster = MockTextPaster::new();
    paster
        .expect_paste()
        .returning(|_| Err(anyhow!("accessibility denied")));
    paster
}

pub(super) fn quiet_sounds() -> MockSoundPlayer {
    let mut sounds = MockSoundPlayer::new();
    sounds.expect_play_start().times(..).return_const(());
    sounds.expect_play_stop().times(..).return_const(());
    sounds.expect_play_cancel().times(..).return_const(());
    sounds
}

pub(super) fn quiet_overlay() -> MockVisualIndicator {
    let mut overlay = MockVisualIndicator::new();
    overlay.expect_show_recording().times(..).return_const(());
    overlay
        .expect_show_recognizing_speech()
        .times(..)
        .return_const(());
    overlay.expect_show_pasted().times(..).return_const(());
    overlay.expect_hide().times(..).return_const(());
    overlay
        .expect_show_failure_notice()
        .times(..)
        .return_const(());
    overlay
        .expect_live_transcript_view()
        .times(..)
        .returning(|| Box::new(HiddenLiveTranscript));
    overlay
}

/// An overlay that notes each sign it shows, a failure notice by its debug form.
pub(super) fn noting_overlay() -> (MockVisualIndicator, Noted<String>) {
    let shown = noted();
    let mut overlay = MockVisualIndicator::new();
    let note = |sign: &'static str| {
        let shown = Arc::clone(&shown);
        move || shown.lock().unwrap().push(sign.to_owned())
    };
    overlay.expect_show_recording().returning(note("recording"));
    overlay
        .expect_show_recognizing_speech()
        .returning(note("recognizing speech"));
    overlay.expect_show_pasted().returning(note("pasted"));
    overlay.expect_hide().returning(note("hidden"));
    let noting = Arc::clone(&shown);
    overlay
        .expect_show_failure_notice()
        .returning(move |notice| noting.lock().unwrap().push(format!("{notice:?}")));
    overlay
        .expect_live_transcript_view()
        .times(..)
        .returning(|| Box::new(HiddenLiveTranscript));
    (overlay, shown)
}

pub(super) struct NotingActivitySink(Noted<PipelineActivity>);

impl PipelineActivitySink for NotingActivitySink {
    fn publish(&self, activity: PipelineActivity) {
        self.0.lock().unwrap().push(activity);
    }
}

/// An activity sink that notes every activity published.
pub(super) fn noting_activity() -> (Box<dyn PipelineActivitySink>, Noted<PipelineActivity>) {
    let published = noted();
    (
        Box::new(NotingActivitySink(Arc::clone(&published))),
        published,
    )
}

/// Every line the pipeline logs at info and above, for the lines the manual run reads.
pub(crate) struct CapturedLog(Mutex<Vec<String>>);

impl log::Log for CapturedLog {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::Level::Info
    }

    fn log(&self, record: &log::Record) {
        self.0.lock().unwrap().push(record.args().to_string());
    }

    fn flush(&self) {}
}

impl CapturedLog {
    pub(crate) fn lines_starting_with(&self, prefix: &str) -> Vec<String> {
        self.0
            .lock()
            .unwrap()
            .iter()
            .filter(|line| line.starts_with(prefix))
            .cloned()
            .collect()
    }
}

/// The process's log, captured from the first call on; nextest runs each test in a process
/// of its own, so it holds one test's lines.
pub(crate) fn captured_log() -> &'static CapturedLog {
    static LOG: OnceLock<&'static CapturedLog> = OnceLock::new();
    LOG.get_or_init(|| {
        let log: &'static CapturedLog = Box::leak(Box::new(CapturedLog(Mutex::new(Vec::new()))));
        if log::set_logger(log).is_ok() {
            log::set_max_level(log::LevelFilter::Info);
        }
        log
    })
}
