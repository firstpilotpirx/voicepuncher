use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::*;
use crate::history::HistoryCfg;
use crate::streaming::ports::{Decoded, RecognizerStream};

const RATE: usize = 16_000;

/// Hears speech in a window holding a sample louder than 0.1.
struct LoudGate;

impl SpeechGate for LoudGate {
    fn hears(&mut self, window: &[f32]) -> bool {
        window.iter().any(|s| s.abs() > 0.1)
    }
}

/// A recognizer that decodes its words, each at its time in the stream, once 0.1 s of audio
/// past it came, each decode taking `decode_time`; it counts the streams opened and the audio
/// of each first accept.
struct Scripted {
    name: &'static str,
    respelled: bool,
    words: Arc<Vec<(&'static str, f32)>>,
    decode_time: Duration,
    streams: Arc<AtomicUsize>,
    first_accept: Arc<AtomicUsize>,
}

impl Scripted {
    fn new(name: &'static str, words: &[(&'static str, f32)]) -> Self {
        Self {
            name,
            respelled: name == "ru",
            words: Arc::new(words.to_vec()),
            decode_time: Duration::ZERO,
            streams: Arc::default(),
            first_accept: Arc::default(),
        }
    }

    fn slow(mut self, decode_time: Duration) -> Self {
        self.decode_time = decode_time;
        self
    }
}

struct ScriptedStream {
    words: Arc<Vec<(&'static str, f32)>>,
    decode_time: Duration,
    first_accept: Arc<AtomicUsize>,
    accepted: usize,
    finished: bool,
}

impl StreamingModel for Scripted {
    fn name(&self) -> &str {
        self.name
    }
    fn respelled(&self) -> bool {
        self.respelled
    }
    fn stream(&self) -> Box<dyn RecognizerStream> {
        self.streams.fetch_add(1, Ordering::SeqCst);
        Box::new(ScriptedStream {
            words: Arc::clone(&self.words),
            decode_time: self.decode_time,
            first_accept: Arc::clone(&self.first_accept),
            accepted: 0,
            finished: false,
        })
    }
}

impl RecognizerStream for ScriptedStream {
    fn accept(&mut self, samples: &[f32]) {
        if self.accepted == 0 {
            self.first_accept.store(samples.len(), Ordering::SeqCst);
        }
        self.accepted += samples.len();
    }
    fn finish(&mut self) {
        self.finished = true;
    }
    fn decode(&mut self) {
        std::thread::sleep(self.decode_time);
    }
    fn decoded(&self) -> Decoded {
        let heard_to = self.accepted as f32 / RATE as f32;
        let lag = if self.finished { 0.0 } else { 0.1 };
        let (tokens, times) = self
            .words
            .iter()
            .filter(|(_, at)| at + lag <= heard_to)
            .map(|(word, at)| (format!("▁{word}"), *at))
            .unzip();
        Decoded { tokens, times }
    }
}

fn silence(secs: f32) -> Vec<f32> {
    vec![0.0; saturating_usize(secs * RATE as f32)]
}

fn speech(secs: f32) -> Vec<f32> {
    vec![0.5; saturating_usize(secs * RATE as f32)]
}

fn session<'a>(models: &[&'a Scripted], history: Option<&'a CommandHistory>) -> Session<'a> {
    let models = models.iter().map(|m| *m as &dyn StreamingModel).collect();
    let cfg = SessionCfg { conn: 1, min_silence_secs: 0.3, history, wake: None };
    let mut session = Session::new(models, Box::new(LoudGate), &cfg);
    let phrases = ["stop dictation", "cancel dictation", "press enter"].map(String::from);
    let selection = PhraseSelection { phrases: phrases.to_vec(), ..PhraseSelection::default() };
    session.select(&selection).expect("known phrases");
    session
}

/// The events of an utterance of `secs` heard by `models`, from 1 s into the stream.
fn utterance_events(models: &[&Scripted], secs: f32) -> Vec<ServerEvent> {
    session(models, None).accept(&[silence(1.0), speech(secs), silence(1.0)].concat())
}

fn commands(events: &[ServerEvent]) -> Vec<Vec<String>> {
    events
        .iter()
        .filter_map(|e| match e {
            ServerEvent::Command { commands, .. } => Some(commands.clone()),
            _ => None,
        })
        .collect()
}

fn count(events: &[ServerEvent], speech_start: bool) -> usize {
    let is = |e: &&ServerEvent| match e {
        ServerEvent::SpeechStart { .. } => speech_start,
        ServerEvent::SpeechEnd { .. } => !speech_start,
        _ => false,
    };
    events.iter().filter(is).count()
}

#[test]
fn should_wake_no_recognizer_in_silence() {
    // Given
    let en = Scripted::new("en", &[("stop", 0.6)]);
    let mut session = session(&[&en], None);
    // When
    let events = session.accept(&silence(3.0));
    // Then
    assert_eq!(events, []);
    assert_eq!(en.streams.load(Ordering::SeqCst), 0);
}

#[test]
fn should_replay_the_half_second_before_the_onset_into_every_recognizer() {
    // Given
    let (en, ru) = (Scripted::new("en", &[]), Scripted::new("ru", &[]));
    let mut session = session(&[&en, &ru], None);
    // When
    let events = session.accept(&[silence(1.0), speech(0.3)].concat());
    session.sync(0);
    // Then
    assert_eq!(events, [ServerEvent::SpeechStart { t: 1.0 }]);
    assert_eq!(en.first_accept.load(Ordering::SeqCst), RATE / 2);
    assert_eq!(ru.first_accept.load(Ordering::SeqCst), RATE / 2);
}

#[test]
fn should_place_a_command_in_the_stream_from_the_start_of_the_replay() {
    // Given "press enter" said 0.6 s into a stream opened at 0.5 s
    let en = Scripted::new("en", &[("press", 0.6), ("enter", 0.9)]);
    let mut session = session(&[&en], None);
    // When
    let mut events = session.accept(&[silence(1.0), speech(0.8)].concat());
    events.extend(session.sync(0));
    // Then
    let start = events.iter().find_map(|e| match e {
        ServerEvent::Command { start, .. } => Some(*start),
        _ => None,
    });
    assert!((start.expect("a command") - 1.1).abs() < 1e-3, "{start:?}");
}

#[test]
fn should_close_the_utterance_only_after_the_pause_and_its_tail() {
    // Given half a second of speech from 1.0 s
    let en = Scripted::new("en", &[]);
    let mut session = session(&[&en], None);
    let open = session.accept(&[silence(1.0), speech(0.5)].concat());
    // When
    let within = session.accept(&silence(0.7));
    let after = session.accept(&silence(0.2));
    // Then
    assert_eq!(count(&open, true), 1);
    assert_eq!(count(&within, false), 0);
    let [ServerEvent::SpeechEnd { t }] = after.as_slice() else {
        panic!("one speech end: {after:?}");
    };
    assert!((t - 1.5).abs() <= 0.1 + 1e-4, "{t}");
}

#[test]
fn should_continue_the_utterance_when_speech_returns_within_the_tail() {
    // Given
    let en = Scripted::new("en", &[]);
    let mut session = session(&[&en], None);
    // When
    let audio = [
        silence(1.0),
        speech(0.5),
        silence(0.5),
        speech(0.5),
        silence(1.0),
    ]
    .concat();
    let events = session.accept(&audio);
    // Then
    assert_eq!((count(&events, true), count(&events, false)), (1, 1));
    assert_eq!(en.streams.load(Ordering::SeqCst), 1);
}

#[test]
fn should_report_a_command_both_recognizers_hear_once() {
    // Given
    let en = Scripted::new("en", &[("press", 0.6), ("enter", 0.9)]);
    let ru = Scripted::new("ru", &[("пресс", 0.6), ("энтер", 0.9)]);
    // When
    let events = utterance_events(&[&en, &ru], 0.8);
    // Then
    assert_eq!(commands(&events), [["press enter"]]);
}

#[test]
fn should_report_a_command_said_in_russian_sounds() {
    // Given a doubled "стоп" only the Russian recognizer hears
    let en = Scripted::new("en", &[("stuff", 0.6)]);
    let ru = Scripted::new("ru", &[("стоп", 0.6), ("стоп", 0.9)]);
    // When
    let events = utterance_events(&[&en, &ru], 0.8);
    // Then
    assert_eq!(commands(&events), [["stop dictation"]]);
}

#[test]
fn should_report_a_command_one_recognizer_heard_while_the_other_still_decodes() {
    // Given an English recognizer that takes a second per decode, and "стоп" said at 1.1 s
    let en = Scripted::new("en", &[("stop", 0.6)]).slow(Duration::from_secs(1));
    let ru = Scripted::new("ru", &[("стоп", 0.6)]);
    let mut session = session(&[&en, &ru], None);
    let began = Instant::now();
    // When the audio streams in, the pause after it shorter than the one that closes it
    let mut events = Vec::new();
    for frame in [silence(1.0), speech(0.3), silence(0.5)]
        .concat()
        .chunks(320)
    {
        events.extend(session.accept(frame));
    }
    while commands(&events).is_empty() && began.elapsed() < Duration::from_secs(3) {
        events.extend(session.poll());
    }
    // Then the Russian recognizer's command came before the English one decoded once
    assert_eq!(commands(&events), [["stop dictation"]]);
    assert!(
        began.elapsed() < Duration::from_millis(500),
        "{:?}",
        began.elapsed()
    );
}

#[test]
fn should_report_no_command_for_speech_without_one() {
    // Given
    let en = Scripted::new("en", &[("stop", 0.6), ("the", 0.75), ("car", 0.9)]);
    // When
    let events = utterance_events(&[&en], 1.0);
    // Then
    assert_eq!(commands(&events), Vec::<Vec<String>>::new(), "{events:?}");
}

#[test]
fn should_answer_a_sync_at_once_without_speech() {
    // Given
    let en = Scripted::new("en", &[]);
    let mut session = session(&[&en], None);
    // When
    let events = session.sync(7);
    // Then
    assert_eq!(events, [ServerEvent::Synced { id: 7 }]);
}

#[test]
fn should_keep_an_utterance_with_a_reported_command_in_the_history() {
    // Given
    let dir = tempfile::tempdir().expect("a temporary folder");
    let cfg = HistoryCfg { dir: dir.path().to_owned(), max_size_bytes: 1 << 30 };
    let history = CommandHistory::start(cfg).expect("a history");
    let en = Scripted::new("en", &[("press", 0.6), ("enter", 0.9)]);
    let mut session = session(&[&en], Some(&history));
    // When
    session.accept(&[silence(1.0), speech(0.8), silence(1.0)].concat());
    history.flush();
    // Then
    let kept = std::fs::read_dir(dir.path())
        .expect("the history folder")
        .count();
    assert_eq!(kept, 1);
}
