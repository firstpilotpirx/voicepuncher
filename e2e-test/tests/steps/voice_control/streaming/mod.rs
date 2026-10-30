//! The streaming recognizer of the command spotting service, run in this process on the real
//! streaming models as the service runs it: speech synthesized by `say` streamed in 100 ms
//! frames, the events it brought and the log lines of its utterances.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;

use cucumber::{given, then, when};
use voicepuncher::test_doubles::prelude::*;

use voicepuncher_command_spotting::history::HistoryCfg;
use voicepuncher_command_spotting::protocol::{PhraseSelection, ServerEvent};
use voicepuncher_command_spotting::streaming::engine::{StreamingCfg, StreamingEngine};
use voicepuncher_e2e_test::conversions::saturating_usize;
use voicepuncher_e2e_test::voice_commands::{
    spoken_by, spoken_with_pauses, without_trailing_silence,
};
use voicepuncher_test_steps::required_env;

use super::spotting_service::scheduling::ENGINE_RUNS;
use super::RUSSIAN_VOICE;
use crate::steps::local_services::{log_mark, logged_since};
use crate::steps::off_thread;
use crate::E2eWorld;

mod history;
mod timed;

const SAMPLE_RATE: usize = 16_000;
/// 100 ms of audio per message, as the app sends it.
const FRAME_SAMPLES: usize = 1_600;

/// The engine every scenario without a command history shares: its models load once.
static SHARED: OnceLock<StreamingEngine> = OnceLock::new();
/// Each run's connection number, so its log lines are its own among the other scenarios'.
static CONNECTIONS: AtomicU64 = AtomicU64::new(9_000);

/// A scenario's streaming run: what it fed and what came back.
#[derive(Debug, Default)]
pub struct StreamingRun {
    history: history::StreamingHistory,
    timed: timed::Timed,
    events: Vec<ServerEvent>,
    conn: u64,
    log_from: usize,
}

fn run(world: &mut E2eWorld) -> &mut StreamingRun {
    &mut world.voice.streaming
}

fn cfg(command_history: Option<HistoryCfg>) -> StreamingCfg {
    StreamingCfg {
        en_model_dir: required_env("VOICEPUNCHER_STREAMING_EN_MODEL_DIR").into(),
        ru_model_dir: required_env("VOICEPUNCHER_STREAMING_RU_MODEL_DIR").into(),
        vad_model: required_env("VOICEPUNCHER_KWS_VAD_MODEL").into(),
        threads: 2,
        min_silence_secs: 0.3,
        command_history,
    }
}

/// Every command phrase, as a session in a dictation and in standby listens for together.
fn every_phrase() -> Vec<String> {
    VoiceCommand::ALL
        .iter()
        .map(|c| c.phrase().to_owned())
        .collect()
}

impl StreamingRun {
    /// Streams `audio` frame by frame into a new session, with a sync asked after it when
    /// `sync`, keeping every event.
    fn stream(&mut self, audio: &[f32], sync: bool) -> Result<()> {
        let _one_at_a_time = ENGINE_RUNS
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let own = self
            .history
            .cfg()
            .map(|h| StreamingEngine::load(&cfg(Some(h))))
            .transpose()?;
        let engine = match &own {
            Some(engine) => engine,
            None => shared()?,
        };
        self.feed(engine, audio, sync)?;
        engine.flush_history();
        Ok(())
    }

    /// Streams `audio` into a new session of `engine`, then a sync, whose answer is kept
    /// only when `sync`.
    fn feed(&mut self, engine: &StreamingEngine, audio: &[f32], sync: bool) -> Result<()> {
        self.conn = CONNECTIONS.fetch_add(1, Ordering::Relaxed);
        self.log_from = log_mark();
        let mut session = engine.session_for(self.conn)?;
        let selection = PhraseSelection { phrases: every_phrase(), ..PhraseSelection::default() };
        session.select(&selection)?;
        for frame in audio.chunks(FRAME_SAMPLES) {
            self.events.extend(session.accept(frame));
        }
        // WHY: the recognizers decode on threads of their own; the sync waits for what is left.
        let mut left = session.sync(1);
        if !sync {
            left.retain(|e| !matches!(e, ServerEvent::Synced { .. }));
        }
        self.events.extend(left);
        Ok(())
    }

    /// The commands reported, in order.
    fn reported(&self) -> Vec<String> {
        self.events
            .iter()
            .flat_map(|event| match event {
                ServerEvent::Command { commands, .. } => commands.clone(),
                _ => Vec::new(),
            })
            .collect()
    }

    /// The log lines of the utterances this run's connection closed.
    fn utterances_logged(&self) -> Vec<String> {
        logged_since(self.log_from, &format!("conn={} utterance", self.conn))
    }
}

fn shared() -> Result<&'static StreamingEngine> {
    if let Some(engine) = SHARED.get() {
        return Ok(engine);
    }
    let engine = StreamingEngine::load(&cfg(None))?;
    Ok(SHARED.get_or_init(|| engine))
}

fn silence(secs: f32) -> Vec<f32> {
    vec![0.0; saturating_usize(secs * SAMPLE_RATE as f32)]
}

#[given("the command spotting service runs with the streaming recognizer")]
fn runs_streaming(world: &mut E2eWorld) {
    *run(world) = StreamingRun::default();
}

#[when(expr = "an app streams {string} followed by half a second of silence and asks for a sync")]
async fn streams_then_syncs(world: &mut E2eWorld, words: String) -> Result<()> {
    off_thread(world, move |world| {
        let spoken = without_trailing_silence(spoken_with_pauses(&words)?);
        run(world).stream(&[spoken, silence(0.5)].concat(), true)
    })
    .await
}

#[when("an app streams three seconds of silence")]
async fn streams_silence(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| run(world).stream(&silence(3.0), false)).await
}

#[when(expr = "an app streams the sentence {string}")]
async fn streams_sentence(world: &mut E2eWorld, sentence: String) -> Result<()> {
    off_thread(world, move |world| {
        run(world).stream(&spoken_with_pauses(&sentence)?, false)
    })
    .await
}

#[when(expr = "an app streams {string} followed by silence")]
async fn streams_words(world: &mut E2eWorld, words: String) -> Result<()> {
    off_thread(world, move |world| {
        run(world).stream(&spoken_with_pauses(&words)?, false)
    })
    .await
}

#[when(expr = "an app streams {string} said by a Russian voice followed by silence")]
async fn streams_russian(world: &mut E2eWorld, words: String) -> Result<()> {
    off_thread(world, move |world| {
        run(world).stream(&spoken_by(RUSSIAN_VOICE, &words)?, false)
    })
    .await
}

#[then(expr = "the streaming recognizer reports {string} before it answers the sync")]
fn reported_before_sync(world: &mut E2eWorld, command: String) -> Result<()> {
    let events = &run(world).events;
    let said = events.iter().position(
        |e| matches!(e, ServerEvent::Command { commands, .. } if commands == &[command.clone()]),
    );
    let ended = events
        .iter()
        .position(|e| matches!(e, ServerEvent::SpeechEnd { .. }));
    let synced = events
        .iter()
        .position(|e| matches!(e, ServerEvent::Synced { .. }));
    let before_the_pause_ends = said.is_some_and(|said| ended.is_none_or(|ended| said < ended));
    if !before_the_pause_ends || said.zip(synced).is_none_or(|(said, synced)| said >= synced) {
        bail!(
            "expected {command:?} before the utterance ends and the sync is answered: {events:?}"
        );
    }
    Ok(())
}

#[then(expr = "the streaming recognizer reports {string} exactly once")]
fn reported_once(world: &mut E2eWorld, command: String) -> Result<()> {
    let reported = run(world).reported();
    if reported != [command.clone()] {
        bail!("expected {command:?} once, the streaming recognizer reported {reported:?}");
    }
    Ok(())
}

#[then("the streaming recognizer reports no command")]
fn reported_nothing(world: &mut E2eWorld) -> Result<()> {
    let reported = run(world).reported();
    if !reported.is_empty() {
        bail!("expected no command, the streaming recognizer reported {reported:?}");
    }
    Ok(())
}

#[then("the service's log shows that no streaming recognizer ran")]
fn no_recognizer_ran(world: &mut E2eWorld) -> Result<()> {
    let run = run(world);
    let opened = run
        .events
        .iter()
        .any(|e| matches!(e, ServerEvent::SpeechStart { .. }));
    let logged = run.utterances_logged();
    if opened || !logged.is_empty() {
        bail!(
            "an utterance opened in silence: {logged:?}, {:?}",
            run.events
        );
    }
    Ok(())
}
