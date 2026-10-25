//! The engine on a test clock and its configuration.

use super::*;

pub(super) const DICTATING: [&str; 4] = [
    "stop dictation",
    "cancel dictation",
    "restart dictation",
    "press enter",
];

pub(super) const FRAME_SAMPLES: usize = 1_600;

/// Silence before the speech and after it, long enough to end the utterance.
pub(super) const PAUSE_SECS: f32 = 1.0;

/// A short breath between a sentence and a command: longer than the pause that ends an
/// utterance.
pub(super) const BREATH_SECS: f32 = 0.35;

pub(super) const MIN_SILENCE_SECS: f32 = 0.3;

/// Quieter samples are silence.
pub(super) const SILENCE_LEVEL: f32 = 0.01;

pub(super) const VOICE: &str = "Eddy";

/// The engine config of the engine tests: the same models and silence, and the command
/// spotting service's default deadline for one confirmation.
pub(super) use crate::engine::engine_cfg;

/// A slow speech recognition service for commands, as under GPU load: within the deadline.
/// Confirmations slower than a pause between two commands.
pub(super) const SLOWER_HEARING: Duration = Duration::from_millis(2500);

pub(super) const SLOW_HEARING: Duration = Duration::from_secs(2);

/// Between two words of a scripted sentence: no pause that would end the utterance.
pub(super) const WORD_GAP_SECS: f32 = 0.06;

/// How long the speech recognition service for commands takes to answer, in the speech: a
/// quick answer, however long the shared service really takes.
pub(super) const REAL_HEARING: Duration = Duration::from_millis(300);

/// How long the shared service may take to answer for real: the speech waits for it.
pub(super) const REAL_DEADLINE: Duration = Duration::from_secs(60);

/// ONNX Runtime aborts when models load or run on several test threads at once.
pub(super) static ENGINE_RUNS: Mutex<()> = Mutex::new(());

/// An engine whose confirmations come back on the clock of the speech fed to it; the only one
/// loaded while it lives.
pub(super) struct ClockedEngine {
    pub(crate) engine: Engine,
    pub(crate) clock: Arc<StreamClock>,
    _one_at_a_time: MutexGuard<'static, ()>,
}

/// The engine `cfg` describes, hearing with `hearing`, each answer `delay` after it is asked
/// in the speech, with `deadline` in place of the engine's when given.
pub(super) fn clocked_engine(
    cfg: &EngineCfg,
    hearing: Arc<dyn Hear>,
    delay: Duration,
    deadline: Option<Duration>,
) -> ClockedEngine {
    clocked_engine_on(Arc::default(), cfg, hearing, delay, deadline)
}

/// As `clocked_engine`, on `clock`.
pub(super) fn clocked_engine_on(
    clock: Arc<StreamClock>,
    cfg: &EngineCfg,
    hearing: Arc<dyn Hear>,
    delay: Duration,
    deadline: Option<Duration>,
) -> ClockedEngine {
    let one_at_a_time = ENGINE_RUNS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let engine = Engine::load(cfg)
        .expect("models load")
        .hearing(Arc::new(Clocked {
            inner: hearing,
            clock: Arc::clone(&clock),
            delay,
            deadline,
        }));
    ClockedEngine { engine, clock, _one_at_a_time: one_at_a_time }
}

/// The engine confirming with the real speech recognition service for commands.
pub(super) fn engine() -> ClockedEngine {
    let url = required_env("VOICEPUNCHER_COMMAND_TRANSCRIPTION_URL");
    let cfg = engine_cfg(None);
    let hearing = real_hearing(&url);
    clocked_engine(&cfg, hearing, REAL_HEARING, Some(REAL_DEADLINE))
}

/// The speech recognition service for commands at `url`, as the real-service tests hear with
/// it: never cut short before `REAL_DEADLINE`, as the shared service may decode past the
/// budget of an idle machine while other runs load it.
pub(super) fn real_hearing(url: &str) -> Arc<dyn Hear> {
    Arc::new(UtteranceTranscriber::with_decoding_floor(url, REAL_DEADLINE).expect("address"))
}
