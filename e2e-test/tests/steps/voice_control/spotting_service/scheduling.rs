//! When the command spotting service asks its confirmations and when it reports what they
//! heard: its engine, on the real keyword spotter, runs in this process on speech synthesized
//! word by word, and the scripted hearing of its tests answers each confirmation a set time
//! later in the speech (the stream clock), however busy the machine is.

use cucumber::{given, then, when};
use voicepuncher::test_doubles::prelude::*;

use voicepuncher_command_spotting::engine::{Engine, EngineCfg, Session};
use voicepuncher_command_spotting::history::HistoryCfg;
use voicepuncher_command_spotting::protocol::ServerEvent;
use voicepuncher_e2e_test::speech_cache::{synthesized, SayRequest};
use voicepuncher_test_steps::required_env;

use crate::scripted_hearing::{ScriptedHearing, ScriptedWord};
use crate::steps::off_thread;
use crate::E2eWorld;

mod hearings;
mod history;
mod spotted;

use hearings::stream_clock::{samples, Clocked, StreamClock};
use hearings::Answers;

const VOICE: &str = "Eddy";
/// Silence before and after the speech: long enough to end an utterance.
const PAUSE_SECS: f32 = 1.0;
/// Between two words of a sentence: no pause that would end the utterance.
const WORD_GAP_SECS: f32 = 0.06;
/// Quieter samples are silence.
const SILENCE_LEVEL: f32 = 0.01;
const FRAME_SAMPLES: usize = 1_600;
/// The frames of a run that asks for a sync: 10 ms, so the sync is asked where it is due.
const SYNC_FRAME_SAMPLES: usize = 160;
/// How late each confirmation comes back in a run that asks for a sync.
const SYNC_RUN_DELAY: Duration = Duration::from_millis(600);
/// How late a confirmation of a loaded copy comes back, in the speech.
const LOADED_DELAY: Duration = Duration::from_secs(2);

/// ONNX Runtime aborts when models load or run on several threads at once.
static ENGINE_RUNS: Mutex<()> = Mutex::new(());

/// The engine's set-up in a scenario, and what each run of it reported.
#[derive(Debug, Default)]
pub struct SchedulingRun {
    listening: Vec<String>,
    answers: Answers,
    delay: Duration,
    /// The events of each run, in the order the runs were made.
    runs: Vec<Vec<ServerEvent>>,
    spotted: spotted::SpottedRun,
    history: history::HistoryRun,
}

fn run(world: &mut E2eWorld) -> &mut SchedulingRun {
    &mut world.services.owned_services.scheduling
}

fn engine_cfg(command_history: Option<HistoryCfg>) -> EngineCfg {
    EngineCfg {
        model_dir: required_env("VOICEPUNCHER_KWS_MODEL_DIR").into(),
        vad_model: required_env("VOICEPUNCHER_KWS_VAD_MODEL").into(),
        threads: 2,
        min_silence_secs: 0.3,
        save_speech_dir: None,
        keywords: None,
        transcription_url: None,
        max_command_secs: 3.0,
        confirmation_deadline: Duration::from_secs(3),
        command_history,
    }
}

/// `sentence` said word by word, a pause before and after it, and where each word lies.
fn said_word_by_word(sentence: &str) -> Result<(Vec<f32>, Vec<ScriptedWord>)> {
    let pause = |secs: f32| std::iter::repeat_n(0.0, samples(Duration::from_secs_f32(secs)));
    let mut stream: Vec<f32> = pause(PAUSE_SECS).collect();
    let mut script = Vec::new();
    for word in sentence.split_whitespace() {
        let request = SayRequest { voice: Some(VOICE), words_per_minute: None, text: word };
        let audio: Vec<f32> = synthesized(&request)?
            .into_iter()
            .map(|s| f32::from(s) / f32::from(i16::MAX))
            .collect();
        let loud = |s: &f32| s.abs() > SILENCE_LEVEL;
        let first = audio.iter().position(loud).unwrap_or(0);
        let last = audio.iter().rposition(loud).map_or(first, |at| at + 1);
        let start = stream.len();
        stream.extend_from_slice(&audio[first..last]);
        script.push(ScriptedWord { text: word.to_owned(), said: start..stream.len() });
        stream.extend(pause(WORD_GAP_SECS));
    }
    stream.extend(pause(PAUSE_SECS));
    Ok((stream, script))
}

impl SchedulingRun {
    /// Feeds `sentence` as it is spoken; with `sync`, a sync is asked `delay` after its last
    /// word, while that word's first hearing is still out.
    fn feed(&mut self, sentence: &str, sync: bool) -> Result<()> {
        let _one_at_a_time = ENGINE_RUNS
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let (mut stream, script) = said_word_by_word(sentence)?;
        let said_end = script.last().map_or(0, |w| w.said.end);
        // Silence while the late confirmations come back
        stream.extend(std::iter::repeat_n(0.0, 2 * samples(self.delay)));
        let clock = Arc::new(StreamClock::default());
        let scripted =
            ScriptedHearing { stream: stream.clone(), script, completes_cut_words: false };
        let inner = self
            .history
            .hearing(self.answers.hearing(scripted, &clock, said_end)?);
        let clocked =
            Clocked { inner, clock: Arc::clone(&clock), delay: self.delay, deadline: None };
        let engine = Engine::load(&engine_cfg(self.history.cfg()))?.hearing(Arc::new(clocked));
        self.history.engine_kept |= engine.keeps_history();
        let mut session = engine.session()?;
        session.listen_for(&self.listening)?;
        let sync_at = sync.then(|| said_end + samples(self.delay));
        let events = fed(&mut session, &clock, &stream, sync_at);
        self.runs.push(events);
        Ok(())
    }
}

/// Every event `stream` brought, fed frame by frame on `clock`, a sync asked at `sync_at`.
fn fed(
    session: &mut Session<'_>,
    clock: &StreamClock,
    stream: &[f32],
    mut sync_at: Option<usize>,
) -> Vec<ServerEvent> {
    let frame = if sync_at.is_some() {
        SYNC_FRAME_SAMPLES
    } else {
        FRAME_SAMPLES
    };
    let mut events = Vec::new();
    for (n, samples) in stream.chunks(frame).enumerate() {
        let at = n * frame;
        if sync_at.is_some_and(|due| at >= due) {
            sync_at = None;
            events.extend(session.sync(1));
        }
        events.extend(clock.feed(session, samples, at));
    }
    clock.release_all();
    events.extend(session.settle());
    events
}

/// The commands reported, alone (`phrase`) or placed (`command`), in order.
fn reported(events: &[ServerEvent], placed_only: bool) -> Vec<String> {
    events
        .iter()
        .flat_map(|event| match event {
            ServerEvent::Phrase { phrase, .. } if !placed_only => vec![phrase.clone()],
            ServerEvent::Command { commands, .. } => commands.clone(),
            _ => Vec::new(),
        })
        .collect()
}

fn phrases(commands: &[VoiceCommand]) -> impl Iterator<Item = String> + '_ {
    commands.iter().map(|c| c.phrase().to_owned())
}

#[given("the command spotting service listens for the commands of a dictation")]
fn listens_while_dictating(world: &mut E2eWorld) {
    run(world).listening = phrases(&VoiceCommand::DICTATING).collect();
}

#[given("the command spotting service listens for the commands of a dictation and for \"start dictation\"")]
fn listens_for_a_chain(world: &mut E2eWorld) {
    let listening = phrases(&VoiceCommand::DICTATING).chain(["start dictation".to_owned()]);
    run(world).listening = listening.collect();
}

#[given("the command spotting service listens for the commands of standby")]
fn listens_in_standby(world: &mut E2eWorld) {
    run(world).listening = phrases(&VoiceCommand::STANDBY).collect();
}

#[given("every confirmation of a command takes over a second, as under machine load")]
fn confirmations_loaded(world: &mut E2eWorld) {
    let run = run(world);
    run.answers = Answers::Loaded;
    run.delay = LOADED_DELAY;
}

#[given(expr = "every confirmation of a command comes back {int} seconds late")]
fn confirmations_late(world: &mut E2eWorld, secs: u64) {
    run(world).delay = Duration::from_secs(secs);
}

#[given("the first hearing of the speech comes back empty and late, as under machine load, and the next one right")]
fn first_hearing_empty(world: &mut E2eWorld) {
    let run = run(world);
    run.answers = Answers::EmptyUntilClosed;
    run.delay = SYNC_RUN_DELAY;
}

#[given("the hearing asked before the pause after the speech ends hears \"clear\" as \"near\", and the hearing of the whole speech hears it right")]
fn early_hearing_misheard(world: &mut E2eWorld) {
    let run = run(world);
    run.answers = Answers::MishearsClearEarly;
    run.delay = SYNC_RUN_DELAY;
}

#[when(expr = "the user says {string} to the command spotting service as it is spoken")]
async fn says(world: &mut E2eWorld, sentence: String) -> Result<()> {
    off_thread(world, move |world| run(world).feed(&sentence, false)).await
}

#[when(
    expr = "the user says {string} once while confirmations come back at once and once while each takes {int} seconds"
)]
async fn says_at_two_speeds(world: &mut E2eWorld, sentence: String, secs: u64) -> Result<()> {
    off_thread(world, move |world| {
        let run = run(world);
        run.feed(&sentence, false)?;
        run.delay = Duration::from_secs(secs);
        run.feed(&sentence, false)
    })
    .await
}

#[when(expr = "the user says {string} alone and the app asks for a sync right after the speech")]
async fn says_then_syncs(world: &mut E2eWorld, sentence: String) -> Result<()> {
    off_thread(world, move |world| run(world).feed(&sentence, true)).await
}

#[then(expr = "every run carries out {string}, in that order and each once")]
fn carries_out(world: &mut E2eWorld, commands: String) -> Result<()> {
    let expected: Vec<&str> = commands.split(", ").collect();
    let runs = &run(world).runs;
    if runs.is_empty() || runs.iter().any(|events| reported(events, true) != expected) {
        bail!("expected {expected:?} in every run: {runs:?}");
    }
    Ok(())
}

#[then(expr = "{string} is carried out exactly once")]
fn carried_out_once(world: &mut E2eWorld, command: String) -> Result<()> {
    let events = run(world).runs.concat();
    let times = reported(&events, true)
        .iter()
        .filter(|c| **c == command)
        .count();
    if times != 1 {
        bail!("{command:?} carried out {times} times: {events:?}");
    }
    Ok(())
}

#[then(expr = "the command spotting service reports {string} before it answers the sync")]
fn reported_before_sync(world: &mut E2eWorld, command: String) -> Result<()> {
    let events = run(world).runs.concat();
    let said = reported(&events, false);
    let first_said = events
        .iter()
        .position(|e| !reported(std::slice::from_ref(e), false).is_empty());
    let synced = events
        .iter()
        .position(|e| matches!(e, ServerEvent::Synced { .. }));
    let in_time = first_said
        .zip(synced)
        .is_some_and(|(said, synced)| said < synced);
    if said.is_empty() || said.iter().any(|c| *c != command) || !in_time {
        bail!("expected only {command:?}, before the sync is answered: {events:?}");
    }
    Ok(())
}
