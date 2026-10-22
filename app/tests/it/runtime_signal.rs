//! The hang-up signal reloads the configuration file the runtime was started with: the
//! process sends the signal to itself, a runtime of stand-in pipelines reloads, and an
//! observer and a logger record what happened. Each test runs in its own process.

use std::io::Write;
use std::path::Path;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use anyhow::Result;

use voicepuncher::adapters::secondary::textrefinement::passthrough::PassthroughTextRefiner;
use voicepuncher::adapters::secondary::visualfeedback::quiet::QuietVisualIndicator;
use voicepuncher::config::Config;
use voicepuncher::core::application::audio_recorder::LiveChunk;
use voicepuncher::core::application::pipeline::{Pipeline, PipelineConfig, PipelineParts};
use voicepuncher::core::application::{
    AudioRecorder, ChunkingCfg, HotkeyListener, PipelineInput, SoundPlayer, TextPaster, Transcriber,
};
use voicepuncher::core::domain::{
    AudioBuffer16k, CleanedText, HotKey, HotkeyBinding, PttSignal, RawTranscript, RecordingMode,
};
use voicepuncher::runtime::signals;
use voicepuncher::runtime::{PipelineAssembler, Runtime, RuntimeObserver, RuntimeState};

/// Longest the runtime may take to reload after the signal.
const REACTION: Duration = Duration::from_secs(2);
/// How long a test watches for a reload that must not happen.
const SETTLE: Duration = Duration::from_millis(500);

#[test]
fn should_reload_the_edited_hotkey_on_a_hang_up_signal() {
    // Given
    let file = config_file("[hotkey]\nkey = \"F13\"\n");
    let (runtime, states) = ready_runtime(file.path());
    edit(file.path(), "[hotkey]\nkey = \"F19\"\n");
    // When
    hang_up();
    // Then
    wait_until(&states, |s| {
        matches!(s, RuntimeState::Ready { hotkey: HotKey::F19, .. })
    });
    assert!(states.lock().unwrap().contains(&RuntimeState::Restarting));
    drop(runtime);
}

#[test]
fn should_keep_the_settings_when_the_edited_file_does_not_load() {
    // Given
    let file = config_file("[hotkey]\nkey = \"F13\"\n");
    let (runtime, states) = ready_runtime(file.path());
    edit(file.path(), "[hotkey]\nkey = \"Bogus\"\n");
    // When
    hang_up();
    // Then
    wait_for_rejection("Bogus");
    std::thread::sleep(SETTLE);
    assert_eq!(
        states.lock().unwrap().last(),
        Some(&RuntimeState::Ready {
            hotkey: HotKey::F13,
            recording_mode: RecordingMode::Toggle,
            voice_control: false
        })
    );
    assert!(!states.lock().unwrap().contains(&RuntimeState::Restarting));
    drop(runtime);
}

#[test]
fn should_keep_the_settings_when_the_edited_file_does_not_validate() {
    // Given
    let file = config_file("[hotkey]\nkey = \"F13\"\n");
    let (runtime, states) = ready_runtime(file.path());
    edit(
        file.path(),
        "[hotkey]\nkey = \"F19\"\n[transcription]\nbase_url = \"http://192.0.2.1:8081\"\n",
    );
    // When
    hang_up();
    // Then
    wait_for_rejection("transcription.base_url");
    std::thread::sleep(SETTLE);
    assert_eq!(
        states.lock().unwrap().last(),
        Some(&RuntimeState::Ready {
            hotkey: HotKey::F13,
            recording_mode: RecordingMode::Toggle,
            voice_control: false
        })
    );
    assert!(!states.lock().unwrap().contains(&RuntimeState::Restarting));
    drop(runtime);
}

fn wait_for_rejection(reason: &str) {
    let deadline = Instant::now() + REACTION;
    while !rejection_logged(reason) {
        assert!(
            Instant::now() < deadline,
            "no rejection in the log: {:?}",
            log_lines()
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

type States = Arc<Mutex<Vec<RuntimeState>>>;

fn ready_runtime(path: &Path) -> (Runtime, States) {
    install_logger();
    let config = voicepuncher::config::load(path).unwrap();
    let states = States::default();
    let mut runtime = Runtime::new(Box::new(StandInAssembler), Box::new(NoKeyboard));
    runtime.observe(Box::new(RecordingObserver(Arc::clone(&states))));
    runtime.start(config).unwrap();
    signals::reload_on_hang_up(
        path.to_path_buf(),
        Box::new(|_| {}),
        runtime.reloader().unwrap(),
    )
    .unwrap();
    (runtime, states)
}

fn config_file(text: &str) -> tempfile::NamedTempFile {
    let mut file = tempfile::NamedTempFile::new().unwrap();
    file.write_all(text.as_bytes()).unwrap();
    file
}

fn edit(path: &Path, text: &str) {
    std::fs::write(path, text).unwrap();
}

fn hang_up() {
    signal_hook::low_level::raise(signal_hook::consts::SIGHUP).unwrap();
}

fn wait_until(states: &States, wanted: impl Fn(&RuntimeState) -> bool) {
    let deadline = Instant::now() + REACTION;
    loop {
        if states.lock().unwrap().last().is_some_and(&wanted) {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "states: {:?}",
            states.lock().unwrap()
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

struct RecordingObserver(States);

impl RuntimeObserver for RecordingObserver {
    fn state_changed(&self, state: &RuntimeState) {
        self.0.lock().unwrap().push(state.clone());
    }
}

#[derive(Default)]
struct CapturedLog(Mutex<Vec<String>>);

impl log::Log for CapturedLog {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        if record.level() <= log::Level::Warn {
            self.0.lock().unwrap().push(record.args().to_string());
        }
    }

    fn flush(&self) {}
}

static LOG: OnceLock<&'static CapturedLog> = OnceLock::new();

fn install_logger() {
    LOG.get_or_init(|| {
        let log: &'static CapturedLog = Box::leak(Box::default());
        if log::set_logger(log).is_ok() {
            log::set_max_level(log::LevelFilter::Warn);
        }
        log
    });
}

fn log_lines() -> Vec<String> {
    LOG.get()
        .map(|log| log.0.lock().unwrap().clone())
        .unwrap_or_default()
}

fn rejection_logged(reason: &str) -> bool {
    log_lines()
        .iter()
        .any(|line| line.contains("configuration rejected") && line.contains(reason))
}

struct StandInAssembler;

impl PipelineAssembler for StandInAssembler {
    fn assemble(&mut self, config: &Config, _inputs: Sender<PipelineInput>) -> Result<Pipeline> {
        Ok(Pipeline::new(
            PipelineParts {
                recorder: Box::new(SilentRecorder),
                transcriber: Box::new(NoSpeech),
                refiner: Box::new(PassthroughTextRefiner),
                paster: Box::new(NoPaste),
                sound_player: Box::new(Silent),
                visual_indicator: Box::new(QuietVisualIndicator),
            },
            PipelineConfig {
                min_press_ms: config.hotkey.min_press_ms,
                recording_mode: config.hotkey.recording_mode,
                ..PipelineConfig::default()
            },
        ))
    }
}

struct NoKeyboard;

impl HotkeyListener for NoKeyboard {
    fn spawn(
        self: Box<Self>,
        _binding: HotkeyBinding,
        _tx: Sender<PttSignal>,
    ) -> Result<JoinHandle<()>> {
        Ok(std::thread::spawn(|| {}))
    }
}

struct SilentRecorder;

impl AudioRecorder for SilentRecorder {
    fn start(&mut self) -> Result<()> {
        Ok(())
    }

    fn stop(&mut self) -> Result<AudioBuffer16k> {
        Ok(AudioBuffer16k::new(Vec::new()))
    }

    fn start_live_chunks(&mut self, _cfg: ChunkingCfg) -> Result<Option<Receiver<LiveChunk>>> {
        Ok(None)
    }

    fn flush_live_chunks(&mut self) {}
    voicepuncher::records_whole_buffers_only!();
}

struct NoSpeech;

impl Transcriber for NoSpeech {
    fn transcribe(&mut self, _audio: &AudioBuffer16k) -> Result<Option<RawTranscript>> {
        Ok(None)
    }
}

struct NoPaste;

impl TextPaster for NoPaste {
    fn paste(&mut self, _text: &CleanedText) -> Result<()> {
        Ok(())
    }

    fn leave_in_clipboard(&mut self, _text: &CleanedText) -> Result<()> {
        Ok(())
    }
}

struct Silent;

impl SoundPlayer for Silent {
    fn play_start(&self) {}
    fn play_stop(&self) {}
    fn play_cancel(&self) {}
}
