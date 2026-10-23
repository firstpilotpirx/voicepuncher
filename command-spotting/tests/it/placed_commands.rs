//! Commands said without a pause around them, heard by the real keyword spotter, voice
//! activity detector and speech recognition service for commands, on speech synthesized
//! with macOS `say`. Needs the models (`needs_kws_models`), and the tests on the real speech
//! recognition service for commands need it too (`needs_command_stt`), so the engine confirms
//! as the command spotting service does.

use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use crate::common;
use crate::kws_models::kws_models;
use crate::scripted_hearing;
use crate::stream_clock;

mod synced;
mod under_load;

use scripted_hearing::{ScriptedHearing, ScriptedWord};
use stream_clock::{samples, Clocked, StreamClock};
use voicepuncher_command_spotting::confirm::{Hear, Heard, TimedWord, UtteranceTranscriber};
use voicepuncher_command_spotting::engine::{Engine, EngineCfg, Session, SAMPLE_RATE};
use voicepuncher_command_spotting::protocol::{Placement, ServerEvent};
use voicepuncher_test_steps::required_env;

const STANDBY: [&str; 3] = ["start dictation", "press enter", "clear all text"];
const DICTATING: [&str; 4] = [
    "stop dictation",
    "cancel dictation",
    "restart dictation",
    "press enter",
];
const FRAME_SAMPLES: usize = 1_600;
/// Silence before the speech and after it, long enough to end the utterance.
const PAUSE_SECS: f32 = 1.0;
/// A short breath between a sentence and a command: longer than the pause that ends an
/// utterance.
const BREATH_SECS: f32 = 0.35;
const MIN_SILENCE_SECS: f32 = 0.3;
/// Quieter samples are silence.
const SILENCE_LEVEL: f32 = 0.01;
const VOICE: &str = "Eddy";
/// How long the engine waits for one confirmation: the command spotting service's default.
const CONFIRMATION_DEADLINE: Duration = Duration::from_secs(3);
/// A slow speech recognition service for commands, as under GPU load: within the deadline.
/// Confirmations slower than a pause between two commands.
const SLOWER_HEARING: Duration = Duration::from_millis(2500);
const SLOW_HEARING: Duration = Duration::from_secs(2);
/// Between two words of a scripted sentence: no pause that would end the utterance.
const WORD_GAP_SECS: f32 = 0.06;
/// How long the speech recognition service for commands takes to answer, in the speech: a
/// quick answer, however long the shared service really takes.
const REAL_HEARING: Duration = Duration::from_millis(300);
/// How long the shared service may take to answer for real: the speech waits for it.
const REAL_DEADLINE: Duration = Duration::from_secs(60);

/// ONNX Runtime aborts when models load or run on several test threads at once.
static ENGINE_RUNS: Mutex<()> = Mutex::new(());

/// An engine whose confirmations come back on the clock of the speech fed to it; the only one
/// loaded while it lives.
struct ClockedEngine {
    engine: Engine,
    clock: Arc<StreamClock>,
    _one_at_a_time: MutexGuard<'static, ()>,
}

/// The engine `cfg` describes, hearing with `hearing`, each answer `delay` after it is asked
/// in the speech, with `deadline` in place of the engine's when given.
fn clocked_engine(
    cfg: &EngineCfg,
    hearing: Arc<dyn Hear>,
    delay: Duration,
    deadline: Option<Duration>,
) -> ClockedEngine {
    clocked_engine_on(Arc::default(), cfg, hearing, delay, deadline)
}

/// As `clocked_engine`, on `clock`.
fn clocked_engine_on(
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
fn engine() -> ClockedEngine {
    let url = required_env("VOICEPUNCHER_COMMAND_TRANSCRIPTION_URL");
    let cfg = engine_cfg(None);
    let hearing = real_hearing(&url);
    clocked_engine(&cfg, hearing, REAL_HEARING, Some(REAL_DEADLINE))
}

/// The speech recognition service for commands at `url`, as the real-service tests hear with
/// it: never cut short before `REAL_DEADLINE`, as the shared service may decode past the
/// budget of an idle machine while other runs load it.
fn real_hearing(url: &str) -> Arc<dyn Hear> {
    Arc::new(UtteranceTranscriber::with_decoding_floor(url, REAL_DEADLINE).expect("address"))
}

/// The engine config, confirming with the speech recognition service at `transcription_url`
/// when given.
fn engine_cfg(transcription_url: Option<String>) -> EngineCfg {
    let (model_dir, vad_model) = kws_models();
    EngineCfg {
        model_dir,
        vad_model,
        threads: 2,
        min_silence_secs: MIN_SILENCE_SECS,
        save_speech_dir: None,
        keywords: None,
        transcription_url,
        max_command_secs: 3.0,
        confirmation_deadline: CONFIRMATION_DEADLINE,
    }
}

/// `words` spoken by `say`, without any pause.
fn speech(words: &str) -> Vec<f32> {
    common::spoken_by(Some(VOICE), words)
}

/// `say` speech from its first sound to its last.
fn sound_only(audio: Vec<f32>) -> Vec<f32> {
    let loud = |s: &f32| s.abs() > SILENCE_LEVEL;
    let first = audio.iter().position(loud).unwrap_or(0);
    let last = audio.iter().rposition(loud).map_or(first, |at| at + 1);
    audio[first..last].to_vec()
}

fn silence(secs: f32) -> Vec<f32> {
    vec![0.0f32; (secs * SAMPLE_RATE as f32) as usize]
}

/// Every event the service reports for `audio`, with a pause before and after it, while
/// it listens for `phrases`.
fn heard(engine: &ClockedEngine, phrases: &[&str], audio: &[f32]) -> Vec<ServerEvent> {
    let mut session = engine.engine.session().unwrap();
    let phrases: Vec<String> = phrases.iter().map(|p| (*p).to_owned()).collect();
    session.listen_for(&phrases).unwrap();
    let stream: Vec<f32> = silence(PAUSE_SECS)
        .into_iter()
        .chain(audio.iter().copied())
        .chain(silence(PAUSE_SECS))
        .collect();
    dbg!(fed_as_spoken(&mut session, &engine.clock, &stream))
}

fn commands(events: &[ServerEvent]) -> Vec<(Vec<String>, Placement)> {
    events
        .iter()
        .filter_map(|e| match e {
            ServerEvent::Command { commands, placement, .. } => {
                Some((commands.clone(), *placement))
            }
            _ => None,
        })
        .collect()
}

fn phrases(events: &[ServerEvent]) -> Vec<String> {
    events
        .iter()
        .filter_map(|e| match e {
            ServerEvent::Phrase { phrase, .. } => Some(phrase.clone()),
            _ => None,
        })
        .collect()
}

fn command(names: &[&str], placement: Placement) -> (Vec<String>, Placement) {
    (names.iter().map(|n| (*n).to_owned()).collect(), placement)
}

/// `sentence` said word by word by `say`, between pauses, and where each word lies.
fn scripted(sentence: &str) -> (Vec<f32>, Vec<ScriptedWord>) {
    scripted_utterances(&[sentence])
}

/// Each of `utterances` said word by word by `say`, a pause before, between and after
/// them, and where each word lies.
fn scripted_utterances(utterances: &[&str]) -> (Vec<f32>, Vec<ScriptedWord>) {
    let mut stream = silence(PAUSE_SECS);
    let mut script = Vec::new();
    for word_or_pause in utterances.join(" | ").split_whitespace() {
        if word_or_pause == "|" {
            stream.extend(silence(PAUSE_SECS));
            continue;
        }
        let said = sound_only(speech(word_or_pause));
        script.push(ScriptedWord {
            text: word_or_pause.to_owned(),
            said: stream.len()..stream.len() + said.len(),
        });
        stream.extend(said);
        stream.extend(silence(WORD_GAP_SECS));
    }
    stream.extend(silence(PAUSE_SECS));
    (stream, script)
}

/// Every event for `sentence`, fed as it is spoken, while the service listens for `phrases`
/// and each confirmation comes back `delay` later in the speech.
fn heard_in_time(phrases: &[&str], sentence: &str, delay: Duration) -> Vec<ServerEvent> {
    heard_in_time_by(phrases, sentence, delay, |hearing| Arc::new(hearing))
}

/// As `heard_in_time`, heard by what `hearing` makes of the scripted hearing of the sentence.
fn heard_in_time_by(
    phrases: &[&str],
    sentence: &str,
    delay: Duration,
    hearing: impl FnOnce(ScriptedHearing) -> Arc<dyn Hear>,
) -> Vec<ServerEvent> {
    let cfg = engine_cfg(None);
    let (stream, script) = scripted(sentence);
    let scripted = ScriptedHearing { stream: stream.clone(), script, completes_cut_words: false };
    let engine = clocked_engine(&cfg, hearing(scripted), delay, None);
    let mut session = engine.engine.session().unwrap();
    let phrases: Vec<String> = phrases.iter().map(|p| (*p).to_owned()).collect();
    session.listen_for(&phrases).unwrap();
    dbg!(fed_as_spoken(&mut session, &engine.clock, &stream))
}

/// Every event the session reports for `audio` fed frame by frame on `clock`, each
/// confirmation coming back where its delay in the speech puts it, however busy the machine.
fn fed_as_spoken(
    session: &mut Session<'_>,
    clock: &StreamClock,
    audio: &[f32],
) -> Vec<ServerEvent> {
    fed_as_spoken_while(session, clock, audio, |_, _, _| {})
}

/// As `fed_as_spoken`, with `before_frame` called before each frame with the samples fed
/// so far.
fn fed_as_spoken_while(
    session: &mut Session<'_>,
    clock: &StreamClock,
    audio: &[f32],
    before_frame: impl FnMut(&mut Session<'_>, usize, &[ServerEvent]),
) -> Vec<ServerEvent> {
    fed_in_frames(session, clock, audio, FRAME_SAMPLES, before_frame)
}

/// As `fed_as_spoken_while`, in frames of `frame_samples`; once the speech ends, every
/// confirmation still held comes back.
fn fed_in_frames(
    session: &mut Session<'_>,
    clock: &StreamClock,
    audio: &[f32],
    frame_samples: usize,
    mut before_frame: impl FnMut(&mut Session<'_>, usize, &[ServerEvent]),
) -> Vec<ServerEvent> {
    let mut events = Vec::new();
    for (n, frame) in audio.chunks(frame_samples).enumerate() {
        let fed = n * frame_samples;
        before_frame(session, fed, &events);
        events.extend(clock.feed(session, frame, fed));
    }
    clock.release_all();
    events.extend(session.settle());
    events
}

/// Every event for `utterances` said in standby, fed as they are spoken, with each
/// confirmation `delay` late in the speech and a word cut off by a clip heard whole as whisper hears it;
/// the session moves to the phrases of a running dictation in the pause after the first
/// utterance, before its confirmation comes back, as a dictation the hotkey starts moves it.
fn heard_with_dictation_started_at_first_pause(
    utterances: &[&str],
    delay: Duration,
) -> Vec<ServerEvent> {
    let cfg = engine_cfg(None);
    let (stream, script) = scripted_utterances(utterances);
    let first_words = utterances[0].split_whitespace().count();
    let switch_at =
        script[first_words - 1].said.end + (PAUSE_SECS / 2.0 * SAMPLE_RATE as f32) as usize;
    let hearing = ScriptedHearing { stream: stream.clone(), script, completes_cut_words: true };
    let engine = clocked_engine(&cfg, Arc::new(hearing), delay, None);
    let mut session = engine.engine.session().unwrap();
    session.listen_for(&owned(&STANDBY)).unwrap();
    let mut dictating = false;
    dbg!(fed_as_spoken_while(
        &mut session,
        &engine.clock,
        &stream,
        |session, fed, _| {
            if !dictating && fed >= switch_at {
                dictating = true;
                session.listen_for(&owned(&DICTATING)).unwrap();
            }
        }
    ))
}

/// Every event for `utterances` said while dictating, fed as they are spoken, with each
/// confirmation `delay` late in the speech; the session moves to the standby phrases once the stop
/// is reported, as the app does when the stop ends its dictation.
fn heard_with_standby_once_stopped(utterances: &[&str], delay: Duration) -> Vec<ServerEvent> {
    let cfg = engine_cfg(None);
    let (mut stream, script) = scripted_utterances(utterances);
    // Silence after the last word, while the late confirmations come back.
    stream.extend(vec![0.0; 2 * samples(delay)]);
    let hearing = ScriptedHearing { stream: stream.clone(), script, completes_cut_words: true };
    let engine = clocked_engine(&cfg, Arc::new(hearing), delay, None);
    let mut session = engine.engine.session().unwrap();
    session.listen_for(&owned(&DICTATING)).unwrap();
    let mut stopped = false;
    dbg!(fed_as_spoken_while(
        &mut session,
        &engine.clock,
        &stream,
        |session, _, events| {
            if !stopped && said(events).iter().any(|c| c == "stop dictation") {
                stopped = true;
                session.listen_for(&owned(&STANDBY)).unwrap();
            }
        },
    ))
}

fn owned(phrases: &[&str]) -> Vec<String> {
    phrases.iter().map(|p| (*p).to_owned()).collect()
}

/// How many times `phrase` was carried out in a command said without a pause.
fn times_carried_out(events: &[ServerEvent], phrase: &str) -> usize {
    commands(events)
        .iter()
        .flat_map(|(names, _)| names.clone())
        .filter(|name| name == phrase)
        .count()
}

/// The commands carried out, in order, whether said in one chain or in several.
fn carried_out(events: &[ServerEvent]) -> Vec<String> {
    commands(events)
        .into_iter()
        .flat_map(|(names, _)| names)
        .collect()
}

/// The chain said without pauses in a dictation, and the commands it carries out in order.
const CHAIN_SENTENCE: &str =
    "We moved the weekly meeting stop enter start and the notes begin here";
const CHAIN: [&str; 3] = ["stop dictation", "press enter", "start dictation"];

/// Asserts the chain is carried out when each confirmation takes `hearing`. One test per speed,
/// so nextest runs them in processes of their own at the same time.
fn assert_chain_carried_out(hearing: Duration) {
    let listening: Vec<&str> = DICTATING.into_iter().chain(["start dictation"]).collect();
    let events = heard_in_time(&listening, CHAIN_SENTENCE, hearing);
    assert_eq!(
        carried_out(&events),
        CHAIN,
        "hearing in {hearing:?}: {events:?}"
    );
}

/// How long a busy machine keeps the threads that hear waiting for the processor.
const STARVED_FOR: Duration = Duration::from_millis(900);

/// A hearing on a busy machine: `inner` hears only after waiting `STARVED_FOR`.
struct Starved<H>(H);

impl<H: Hear> Hear for Starved<H> {
    fn hear(&self, clip: &[f32], deadline: Duration) -> anyhow::Result<Heard> {
        std::thread::sleep(STARVED_FOR);
        self.0.hear(clip, deadline)
    }
}

/// Every command reported, alone or placed, in order.
fn said(events: &[ServerEvent]) -> Vec<String> {
    events
        .iter()
        .flat_map(|event| match event {
            ServerEvent::Phrase { phrase, .. } => vec![phrase.clone()],
            ServerEvent::Command { commands, .. } => commands.clone(),
            _ => Vec::new(),
        })
        .collect()
}

/// A hearing that answers each clip of a segment `first` and its steady clip `steady`, each
/// `delay` after it is asked in the speech, and counts the steady clips.
struct HearsFirstThenSteady {
    first: &'static str,
    steady: &'static str,
    delay: Duration,
    steady_clips: Mutex<usize>,
}

/// The steady clip starts 0.25 s before its first sound; every other clip starts with the
/// pre-roll of 0.8 s before the detected speech.
const STEADY_LEAD_MAX_SECS: f32 = 0.45;

impl Hear for HearsFirstThenSteady {
    fn hear(&self, clip: &[f32], _deadline: Duration) -> anyhow::Result<Heard> {
        let onset = clip
            .iter()
            .position(|s| s.abs() > SILENCE_LEVEL)
            .unwrap_or(0);
        let onset_secs = onset as f32 / SAMPLE_RATE as f32;
        let text = if onset_secs < STEADY_LEAD_MAX_SECS {
            *self.steady_clips.lock().unwrap() += 1;
            self.steady
        } else {
            self.first
        };
        Ok(Heard {
            text: format!(" {text}"),
            words: vec![TimedWord {
                text: format!(" {text}"),
                start: onset_secs,
                end: clip.len() as f32 / SAMPLE_RATE as f32,
            }],
        })
    }
}

/// The events of "restart" said alone in a dictation, heard by `hearing`, and how long after
/// the end of the word the first phrase came out.
fn restart_alone_heard_by(hearing: Arc<HearsFirstThenSteady>) -> (Vec<ServerEvent>, f32) {
    let cfg = engine_cfg(None);
    // The default voice, as the keyword spotter hears its "restart".
    let word = sound_only(common::spoken_by(None, "Restart"));
    let mut stream = silence(PAUSE_SECS);
    stream.extend(&word);
    let said_end = stream.len();
    // Long enough for two confirmations one after another to come back while it is fed.
    stream.extend(silence(3.0));
    let delay = hearing.delay;
    let engine = clocked_engine(&cfg, hearing, delay, None);
    let mut session = engine.engine.session().unwrap();
    session.listen_for(&owned(&DICTATING)).unwrap();
    let mut first_phrase_at = None;
    let events = fed_as_spoken_while(&mut session, &engine.clock, &stream, |_, fed, events| {
        if first_phrase_at.is_none() && !phrases(events).is_empty() {
            first_phrase_at = Some(fed);
        }
    });
    eprintln!("{events:?}");
    let after = first_phrase_at.map_or(f32::MAX, |at| {
        at.saturating_sub(said_end) as f32 / SAMPLE_RATE as f32
    });
    (events, after)
}

/// 10 ms: frames this short time an event to within 10 ms.
const FINE_FRAME_SAMPLES: usize = 160;

/// A scripted hearing that notes the sample of the speech at which each clip is asked, and
/// hears a word wrong in every clip asked before a sample.
struct NotedHearing {
    hearing: ScriptedHearing,
    clock: Arc<StreamClock>,
    asked: Mutex<Vec<usize>>,
    /// A word and what it is heard as, in clips asked before `heard_right_from`.
    mishears: Option<(&'static str, &'static str)>,
    heard_right_from: usize,
}

impl Hear for NotedHearing {
    fn hear(&self, clip: &[f32], deadline: Duration) -> anyhow::Result<Heard> {
        let asked = self.clock.now();
        self.asked.lock().unwrap().push(asked);
        let mut heard = self.hearing.hear(clip, deadline)?;
        if let Some((word, wrong)) = self.mishears.filter(|_| asked < self.heard_right_from) {
            for timed in &mut heard.words {
                timed.text = timed.text.replace(word, wrong);
            }
            heard.text = heard.text.replace(word, wrong);
        }
        Ok(heard)
    }
}

/// What a stream of words fed in 10 ms frames brought.
struct FinelyHeard {
    events: Vec<ServerEvent>,
    /// How long after the last word ended the first command came out.
    said_after: Option<f32>,
    /// How long after the last word ended each clip was asked, in seconds; negative before.
    asked_after: Vec<f32>,
    /// How long the last word lasted, in seconds.
    last_word_secs: f32,
    /// Where the last word ended in the stream, in seconds.
    said_end_secs: f32,
}

/// Every event for `words`, each said by `say` after the gap of seconds given with it and fed
/// in 10 ms frames as it is spoken, while the service listens for `phrases` and each
/// confirmation comes back `delay` later in the speech.
fn heard_finely(phrases: &[&str], words: &[(f32, &str)], delay: Duration) -> FinelyHeard {
    heard_finely_misheard(phrases, words, delay, None)
}

/// [`heard_finely`], with the clips asked before the pause after the last word closes hearing
/// the word `mishears` names as the other one.
#[expect(clippy::too_many_lines, reason = "size baseline")]
fn heard_finely_misheard(
    phrases: &[&str],
    words: &[(f32, &str)],
    delay: Duration,
    mishears: Option<(&'static str, &'static str)>,
) -> FinelyHeard {
    let cfg = engine_cfg(None);
    let mut stream = silence(PAUSE_SECS);
    let mut script = Vec::new();
    for (gap, word) in words {
        stream.extend(silence(*gap));
        let said = sound_only(speech(word));
        script.push(ScriptedWord {
            text: (*word).to_owned(),
            said: stream.len()..stream.len() + said.len(),
        });
        stream.extend(said);
    }
    let said_end = stream.len();
    let last_word_secs = script
        .last()
        .map_or(0.0, |w| w.said.len() as f32 / SAMPLE_RATE as f32);
    // Long enough for the confirmations to come back while it is fed.
    stream.extend(silence(PAUSE_SECS + 2.0 * delay.as_secs_f32()));
    let clock = Arc::new(StreamClock::default());
    let hearing = Arc::new(NotedHearing {
        hearing: ScriptedHearing { stream: stream.clone(), script, completes_cut_words: false },
        clock: Arc::clone(&clock),
        asked: Mutex::new(Vec::new()),
        mishears,
        heard_right_from: said_end + samples(Duration::from_secs_f32(MIN_SILENCE_SECS)),
    });
    let engine = clocked_engine_on(
        clock,
        &cfg,
        Arc::clone(&hearing) as Arc<dyn Hear>,
        delay,
        None,
    );
    let mut session = engine.engine.session().unwrap();
    session.listen_for(&owned(phrases)).unwrap();
    let secs_after_end = |at: usize| (at as f32 - said_end as f32) / SAMPLE_RATE as f32;
    let mut said_at = None;
    let events = fed_in_frames(
        &mut session,
        &engine.clock,
        &stream,
        FINE_FRAME_SAMPLES,
        |_, fed, events| {
            if said_at.is_none() && !said(events).is_empty() {
                said_at = Some(fed);
            }
        },
    );
    eprintln!("{events:?}");
    let asked_after = hearing
        .asked
        .lock()
        .unwrap()
        .iter()
        .map(|&asked| secs_after_end(asked))
        .collect();
    FinelyHeard {
        events,
        said_after: said_at.map(secs_after_end),
        asked_after,
        last_word_secs,
        said_end_secs: said_end as f32 / SAMPLE_RATE as f32,
    }
}

mod needs_kws_models {
    use super::*;

    #[test]
    fn should_restart_once_in_a_dictation_while_confirmations_come_back_slowly() {
        let events = heard_in_time(
            &DICTATING,
            "We moved the weekly meeting to Friday restart restart and the notes begin here with a few more words for everyone",
            SLOW_HEARING,
        );
        assert_eq!(
            times_carried_out(&events, "restart dictation"),
            1,
            "{events:?}"
        );
    }

    #[test]
    fn should_carry_out_the_chain_when_confirmations_come_back_at_once() {
        assert_chain_carried_out(Duration::ZERO);
    }

    #[test]
    fn should_carry_out_the_same_chain_when_confirmations_come_back_slowly() {
        assert_chain_carried_out(SLOW_HEARING);
    }

    #[test]
    fn should_carry_out_the_same_chain_on_a_busy_machine() {
        // Given a dictation, each confirmation 2 s late, on a machine so busy that the threads
        // that hear wait for the processor while the speech goes on
        let listening: Vec<&str> = DICTATING.into_iter().chain(["start dictation"]).collect();
        // When "stop enter start" is said
        let events = heard_in_time_by(&listening, CHAIN_SENTENCE, SLOW_HEARING, |hearing| {
            Arc::new(Starved(hearing))
        });
        // Then the chain is carried out as on an idle machine
        assert_eq!(carried_out(&events), CHAIN, "{events:?}");
    }

    #[test]
    fn should_keep_a_start_confirmed_after_the_dictation_began_a_start() {
        let events = heard_with_dictation_started_at_first_pause(
            &[
                "start dictation",
                "and the notes begin here with a few more words",
            ],
            SLOW_HEARING,
        );
        let said = said(&events);
        assert!(said.iter().any(|c| c == "start dictation"), "{events:?}");
        assert!(!said.iter().any(|c| c == "restart dictation"), "{events:?}");
    }

    #[test]
    fn should_keep_a_start_said_right_after_a_slowly_confirmed_stop_a_start() {
        // Given a dictation running, every confirmation 2.5 s late
        // When "stop dictation" is said and then, before its confirmation, "start"
        let events = heard_with_standby_once_stopped(&["stop dictation", "start"], SLOWER_HEARING);
        // Then the dictation ends and a new one starts: the spotter's report of the stop and
        // its confirmation are one command
        let mut said = said(&events);
        said.dedup();
        assert_eq!(said, ["stop dictation", "start dictation"], "{events:?}");
    }

    #[test]
    fn should_report_a_restart_misheard_at_first_one_hearing_after_its_pause() {
        // Given "restart" said alone in a dictation, its segment heard first as another word and
        // its steady clip as "restart", each answer 0.6 s after it is asked
        let delay = Duration::from_millis(600);
        let hearing = Arc::new(HearsFirstThenSteady {
            first: "Restocked.",
            steady: "Restart!",
            delay,
            steady_clips: Mutex::new(0),
        });
        // When
        let (events, after) = restart_alone_heard_by(hearing);
        // Then the restart comes out sooner than two hearings in a row after the pause that
        // closes it: the keyword spotter reports the trigger up to 0.2 s after that pause
        assert_eq!(phrases(&events), ["restart dictation"], "{events:?}");
        let two_hearings = MIN_SILENCE_SECS + 2.0 * delay.as_secs_f32();
        assert!(
            after < two_hearings,
            "restart out {after:.2}s after the word, two hearings take {two_hearings:.2}s: {events:?}"
        );
    }

    #[test]
    fn should_report_a_restart_heard_at_first_once_and_drop_its_early_second_hearing() {
        // Given "restart" said alone in a dictation, heard as "restart" by both hearings
        let hearing = Arc::new(HearsFirstThenSteady {
            first: "Restart.",
            steady: "Restart!",
            delay: Duration::from_millis(600),
            steady_clips: Mutex::new(0),
        });
        // When
        let (events, _) = restart_alone_heard_by(Arc::clone(&hearing));
        // Then the second hearing was asked at once, and the restart is reported once
        assert_eq!(*hearing.steady_clips.lock().unwrap(), 1, "{events:?}");
        assert_eq!(phrases(&events), ["restart dictation"], "{events:?}");
    }

    #[test]
    fn should_report_start_said_alone_in_standby_sooner_than_one_hearing_after_its_pause() {
        // Given each confirmation 0.6 s late
        let delay = Duration::from_millis(600);
        // When "start" is said alone in standby
        let heard = heard_finely(&STANDBY, &[(0.0, "start")], delay);
        // Then the start comes out well before one hearing after the pause that closes it
        let events = &heard.events;
        assert_eq!(
            said(events).first().map(String::as_str),
            Some("start dictation"),
            "{events:?}"
        );
        let one_hearing_after_pause = MIN_SILENCE_SECS + delay.as_secs_f32();
        let after = heard.said_after.unwrap_or(f32::MAX);
        assert!(
            after < one_hearing_after_pause - MIN_SILENCE_SECS / 4.0,
            "start out {after:.2}s after the word, one hearing after the pause is \
             {one_hearing_after_pause:.2}s: {events:?}"
        );
    }

    #[test]
    fn should_start_nothing_for_start_and_stop_with_a_dip_shorter_than_a_pause_in_standby() {
        // Given each confirmation 0.6 s late
        // When "start", a dip shorter than a pause and "stop" are said in standby
        let heard = heard_finely(
            &STANDBY,
            // With the quiet ends of the words: past half a pause, short of a whole one.
            &[(0.0, "start"), (0.15, "stop")],
            Duration::from_millis(600),
        );
        // Then no dictation starts: the hearing asked at the dip is dropped
        let events = &heard.events;
        assert!(
            heard
                .asked_after
                .iter()
                .any(|&after| after < -heard.last_word_secs),
            "no clip asked during the dip: {:?}, {events:?}",
            heard.asked_after
        );
        assert!(
            !said(events).iter().any(|c| c == "start dictation"),
            "{events:?}"
        );
    }

    #[test]
    fn should_end_a_start_closed_by_please_where_please_ends() {
        // Given each confirmation 0.3 s late
        // When "start dictation please" is said in standby
        let heard = heard_finely(
            &STANDBY,
            &[(0.0, "start"), (0.0, "dictation"), (0.0, "please")],
            Duration::from_millis(300),
        );
        // Then a start a check placed ends no sooner than "please" does: the dictation after it
        // holds none of it
        let events = &heard.events;
        assert!(!said(events).is_empty(), "no start: {events:?}");
        for event in events {
            if let ServerEvent::Command { end, .. } = event {
                assert!(
                    *end >= heard.said_end_secs - 0.05,
                    "start ends at {end:.2}s, \"please\" at {:.2}s: {events:?}",
                    heard.said_end_secs
                );
            }
        }
    }

    #[test]
    fn should_hear_no_utterance_in_a_dictation_before_its_pause_ends() {
        // Given each confirmation 0.6 s late
        // When a short word is said alone in a dictation
        let heard = heard_finely(&DICTATING, &[(0.0, "Friday")], Duration::from_millis(600));
        // Then nothing is asked before most of the pause has passed
        assert!(!heard.asked_after.is_empty(), "{:?}", heard.events);
        let earliest = heard.asked_after.iter().copied().fold(f32::MAX, f32::min);
        assert!(
            earliest > MIN_SILENCE_SECS * 0.75,
            "a clip asked {earliest:.2}s after the word: {:?}",
            heard.events
        );
    }

    mod needs_command_stt {
        use super::*;

        #[test]
        fn should_hear_a_stop_right_after_the_last_dictated_word() {
            let engine = engine();
            let events = heard(
                &engine,
                &DICTATING,
                &speech("We meet on Friday at ten stop"),
            );
            assert_eq!(
                commands(&events),
                [command(&["stop dictation"], Placement::Tail)]
            );
        }

        #[test]
        fn should_hear_enter_right_after_other_speech() {
            let engine = engine();
            let events = heard(&engine, &STANDBY, &speech("Okay, press enter"));
            // A phrase for the whole utterance would fail the app's pause rule here: only a
            // command placed at the end counts.
            assert_eq!(
                commands(&events),
                [command(&["press enter"], Placement::Tail)],
                "{events:?}"
            );
        }

        #[test]
        fn should_hear_start_opening_an_utterance_and_place_it_before_the_speech() {
            let engine = engine();
            let audio = speech("Start, the meeting moved to Friday at ten");
            let events = heard(&engine, &STANDBY, &audio);
            assert_eq!(
                commands(&events),
                [command(&["start dictation"], Placement::Head)]
            );
            let end = events
                .iter()
                .find_map(|e| match e {
                    ServerEvent::Command { end, .. } => Some(*end),
                    _ => None,
                })
                .unwrap();
            assert!(end < PAUSE_SECS + 1.2, "start ends at {end}");
        }

        #[test]
        fn should_hear_a_doubled_stop_in_the_middle_of_speech() {
            let engine = engine();
            let events = heard(
                &engine,
                &DICTATING,
                &speech("We move the meeting to Friday stop stop and some more words here"),
            );
            assert_eq!(
                commands(&events),
                [command(&["stop dictation"], Placement::Anywhere)]
            );
        }

        #[test]
        fn should_hear_a_chain_of_commands() {
            let engine = engine();
            let events = heard(&engine, &DICTATING, &speech("That is all, stop enter"));
            assert_eq!(
                commands(&events),
                [command(
                    &["stop dictation", "press enter"],
                    Placement::Anywhere
                )]
            );
        }

        #[test]
        fn should_hear_a_stop_after_a_short_breath_as_an_utterance_of_its_own() {
            let engine = engine();
            // `say` ends a phrase with about 0.4 s of its own silence: the breath is cut from the
            // last sound to the next.
            let audio: Vec<f32> = sound_only(speech("We meet on Friday"))
                .into_iter()
                .chain(silence(BREATH_SECS))
                .chain(sound_only(speech("stop dictation")))
                .collect();
            let events = heard(&engine, &DICTATING, &audio);
            let starts = events
                .iter()
                .filter(|e| matches!(e, ServerEvent::SpeechStart { .. }))
                .count();
            assert_eq!(starts, 2, "two utterances expected: {events:?}");
            assert!(
                phrases(&events).contains(&"stop dictation".to_owned()),
                "{events:?}"
            );
        }
    }
}
