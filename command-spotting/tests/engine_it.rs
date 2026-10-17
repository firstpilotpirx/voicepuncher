//! The real keyword spotting model and voice activity detector on speech synthesized with
//! macOS `say`. Gated on the model paths the justfile exports.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::Duration;

mod common;

use voicepuncher_command_spotting::confirm::{Hear, Heard};
use voicepuncher_command_spotting::engine::{Engine, EngineCfg, Session, SAMPLE_RATE};
use voicepuncher_command_spotting::protocol::ServerEvent;

const PAUSE_SECS: f32 = 1.0;
/// As long as a user keeps listening on before saying a command.
const LONG_SILENCE_SECS: f32 = 25.0;
/// How late the reported phrase may start after its speech starts, and how far its end may be
/// from the end of its speech: the app's pause rule margins. An early start is allowed: in
/// noise the spotter may stretch a phrase back.
const LEAD_TOLERANCE_SECS: f32 = 0.3;
const TAIL_TOLERANCE_SECS: f32 = 0.5;
/// Silence streamed before the phrase set changes.
const LEAD_IN_SECS: f32 = 2.0;
const FRAME_SAMPLES: usize = 1_600;
const MIN_SILENCE_SECS: f32 = 0.3;
const ALL_PHRASES: [&str; 6] = [
    "start dictation",
    "stop dictation",
    "cancel dictation",
    "restart dictation",
    "press enter",
    "clear all text",
];

/// ONNX Runtime aborts when models load or run on several test threads at once.
static ENGINE: OnceLock<Option<Mutex<Engine>>> = OnceLock::new();

fn engine() -> Option<MutexGuard<'static, Engine>> {
    ENGINE
        .get_or_init(|| load_engine().map(Mutex::new))
        .as_ref()
        .map(|engine| engine.lock().unwrap_or_else(|e| e.into_inner()))
}

fn load_engine() -> Option<Engine> {
    engine_cfg(None).map(|cfg| Engine::load(&cfg).expect("models load"))
}

/// The engine config, confirming utterances with the speech recognition service at
/// `transcription_url` when given.
fn engine_cfg(transcription_url: Option<String>) -> Option<EngineCfg> {
    let (Ok(model_dir), Ok(vad_model)) = (
        std::env::var("VOICEPUNCHER_KWS_MODEL_DIR"),
        std::env::var("VOICEPUNCHER_KWS_VAD_MODEL"),
    ) else {
        eprintln!(
            "GATED: VOICEPUNCHER_KWS_MODEL_DIR / VOICEPUNCHER_KWS_VAD_MODEL not set, skipping"
        );
        return None;
    };
    Some(EngineCfg {
        model_dir: PathBuf::from(model_dir),
        vad_model: PathBuf::from(vad_model),
        threads: 2,
        min_silence_secs: MIN_SILENCE_SECS,
        save_speech_dir: None,
        keywords: None,
        transcription_url,
        max_command_secs: 3.0,
        confirmation_deadline: Duration::from_secs(3),
    })
}

/// `words` spoken by `say`, with a pause of silence before and after.
fn spoken(words: &str) -> Vec<f32> {
    let pause = vec![0.0f32; (PAUSE_SECS * SAMPLE_RATE as f32) as usize];
    pause
        .iter()
        .copied()
        .chain(common::spoken_by(None, words))
        .chain(pause.iter().copied())
        .collect()
}

fn events_for(engine: &Engine, words: &str) -> Vec<ServerEvent> {
    let mut session = engine.session().unwrap();
    let phrases: Vec<String> = ALL_PHRASES.iter().map(|p| (*p).to_owned()).collect();
    session.listen_for(&phrases).unwrap();
    let audio = spoken(words);
    fed(&mut session, &audio)
}

fn spotted(events: &[ServerEvent]) -> Vec<String> {
    events
        .iter()
        .filter_map(|e| match e {
            ServerEvent::Phrase { phrase, .. } => Some(phrase.clone()),
            _ => None,
        })
        .collect()
}

/// Every event the session reports for `audio` fed frame by frame, its confirmations
/// included once they are answered.
fn fed(session: &mut Session<'_>, audio: &[f32]) -> Vec<ServerEvent> {
    let mut events: Vec<ServerEvent> = audio
        .chunks(FRAME_SAMPLES)
        .flat_map(|frame| session.accept(frame))
        .collect();
    events.extend(session.settle());
    events
}

#[test]
fn should_spot_each_phrase_spoken_on_its_own_inside_its_speech_segment() {
    let Some(engine) = engine() else { return };
    for phrase in ALL_PHRASES {
        let events = events_for(&engine, phrase);
        eprintln!("{phrase}: {events:?}");
        assert_eq!(spotted(&events), [phrase], "events: {events:?}");
        let starts = events
            .iter()
            .filter(|e| matches!(e, ServerEvent::SpeechStart { .. }))
            .count();
        assert_eq!(starts, 1, "one speech segment expected: {events:?}");
    }
}

#[test]
fn should_spot_the_polite_and_short_variants_by_keyword_alone() {
    let Some(engine) = engine() else { return };
    for (said, phrase) in [
        ("press enter please", "press enter"),
        ("stop dictation please", "stop dictation"),
        ("clear text", "clear all text"),
        ("clear text please", "clear all text"),
    ] {
        let events = events_for(&engine, said);
        assert_eq!(spotted(&events), [phrase], "{said}: {events:?}");
    }
}

#[test]
fn should_spot_by_keyword_alone_when_the_speech_recognition_service_does_not_answer() {
    // Holding the shared engine keeps other tests from loading or running models meanwhile.
    let Some(_shared) = engine() else { return };
    let closed_port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let cfg = engine_cfg(Some(format!("http://127.0.0.1:{closed_port}/v1"))).unwrap();
    let engine = Engine::load(&cfg).expect("models load");
    for phrase in ["press enter", "stop dictation"] {
        let events = events_for(&engine, phrase);
        assert_eq!(spotted(&events), [phrase], "{phrase}: {events:?}");
    }
}

/// The speech segment and the phrase spotted in `events`.
fn segment_and_phrase(events: &[ServerEvent]) -> ((f32, f32), (f32, f32)) {
    let (mut segment, mut phrase) = (None, None);
    for event in events {
        match event {
            ServerEvent::SpeechStart { t } => segment = Some((*t, f32::MAX)),
            ServerEvent::SpeechEnd { t } => segment = segment.map(|(s, _)| (s, *t)),
            ServerEvent::Phrase { start, end, .. } => phrase = Some((*start, *end)),
            ServerEvent::Command { .. } | ServerEvent::Synced { .. } => {}
        }
    }
    (
        segment.expect("speech was detected"),
        phrase.expect("the phrase was spotted"),
    )
}

fn assert_phrase_fills_segment(events: &[ServerEvent]) {
    let (segment, phrase) = segment_and_phrase(events);
    assert!(
        phrase.0 - segment.0 < LEAD_TOLERANCE_SECS,
        "lead: segment {segment:?} phrase {phrase:?}"
    );
    assert!(
        (segment.1 - phrase.1).abs() < TAIL_TOLERANCE_SECS,
        "tail: segment {segment:?} phrase {phrase:?}"
    );
}

#[test]
fn should_report_the_phrase_within_the_bounds_of_its_speech() {
    let Some(engine) = engine() else { return };
    assert_phrase_fills_segment(&events_for(&engine, "press enter"));
}

#[test]
fn should_report_the_phrase_within_its_speech_after_a_long_silence() {
    let Some(engine) = engine() else { return };
    // Given
    let mut session = engine.session().unwrap();
    let phrases: Vec<String> = ALL_PHRASES.iter().map(|p| (*p).to_owned()).collect();
    session.listen_for(&phrases).unwrap();
    let mut audio = vec![0.0f32; (LONG_SILENCE_SECS * SAMPLE_RATE as f32) as usize];
    audio.extend(spoken("start dictation"));
    // When
    let events: Vec<ServerEvent> = fed(&mut session, &audio);
    // Then
    assert_phrase_fills_segment(&events);
}

#[test]
fn should_count_event_times_from_the_first_sample_across_a_phrase_change() {
    let Some(engine) = engine() else { return };
    // Given
    let mut session = engine.session().unwrap();
    session.listen_for(&["stop dictation".to_owned()]).unwrap();
    let lead_in = vec![0.0f32; (LEAD_IN_SECS * SAMPLE_RATE as f32) as usize];
    for frame in lead_in.chunks(FRAME_SAMPLES) {
        session.accept(frame);
    }
    // When
    session.listen_for(&["press enter".to_owned()]).unwrap();
    let events: Vec<ServerEvent> = fed(&mut session, &spoken("press enter"));
    // Then
    let speech_start = events
        .iter()
        .find_map(|e| match e {
            ServerEvent::SpeechStart { t } => Some(*t),
            _ => None,
        })
        .expect("speech was detected");
    let expected = LEAD_IN_SECS + PAUSE_SECS;
    assert!(
        (speech_start - expected).abs() < 0.3,
        "speech at {speech_start}, expected about {expected}: {events:?}"
    );
}

fn silence(secs: f32) -> Vec<f32> {
    vec![0.0f32; (secs * SAMPLE_RATE as f32) as usize]
}

/// Each spotted phrase with the bounds of the speech segment around it.
/// A spotted phrase, the bounds of its speech segment and its own bounds, in seconds.
type PhraseInSegment = (String, (f32, f32), (f32, f32));

fn phrases_in_segments(events: &[ServerEvent]) -> Vec<PhraseInSegment> {
    let mut found = Vec::new();
    let mut segment_start = None;
    let mut last_segment: Option<(f32, f32)> = None;
    let mut waiting: Vec<(String, (f32, f32))> = Vec::new();
    for event in events {
        match event {
            ServerEvent::SpeechStart { t } => segment_start = Some(*t),
            // A phrase reported after its segment ended still belongs to it: the spotter
            // may decode it after the short pause that closed the segment.
            ServerEvent::Phrase { phrase, start, end } => match (segment_start, last_segment) {
                (None, Some(segment)) if *start <= segment.1 => {
                    found.push((phrase.clone(), segment, (*start, *end)))
                }
                _ => waiting.push((phrase.clone(), (*start, *end))),
            },
            ServerEvent::SpeechEnd { t } => {
                let start = segment_start.take().expect("speech start before its end");
                last_segment = Some((start, *t));
                found.extend(waiting.drain(..).map(|(p, span)| (p, (start, *t), span)));
            }
            ServerEvent::Command { .. } | ServerEvent::Synced { .. } => {}
        }
    }
    found
}

#[test]
fn should_spot_commands_said_one_after_another_in_one_long_stream() {
    let Some(engine) = engine() else { return };
    // Given
    let mut session = engine.session().unwrap();
    let phrases: Vec<String> = ALL_PHRASES.iter().map(|p| (*p).to_owned()).collect();
    session.listen_for(&phrases).unwrap();
    let mut audio = silence(5.0);
    audio.extend(spoken("start dictation"));
    audio.extend(silence(10.0));
    audio.extend(spoken("press enter"));
    audio.extend(silence(20.0));
    audio.extend(spoken("clear all text"));
    // When
    let events: Vec<ServerEvent> = fed(&mut session, &audio);
    // Then
    let found = phrases_in_segments(&events);
    let names: Vec<&str> = found.iter().map(|(p, _, _)| p.as_str()).collect();
    assert_eq!(
        names,
        ["start dictation", "press enter", "clear all text"],
        "{events:?}"
    );
    for (phrase, segment, span) in &found {
        assert!(
            span.0 - segment.0 < LEAD_TOLERANCE_SECS
                && (segment.1 - span.1).abs() < TAIL_TOLERANCE_SECS,
            "{phrase}: segment {segment:?} phrase {span:?}"
        );
    }
}

#[test]
fn should_place_the_phrase_alike_whether_the_audio_comes_in_small_or_large_messages() {
    let Some(engine) = engine() else { return };
    // Given
    let audio = spoken("press enter");
    let phrases: Vec<String> = ALL_PHRASES.iter().map(|p| (*p).to_owned()).collect();
    // When the whole utterance arrives in one message
    let mut session = engine.session().unwrap();
    session.listen_for(&phrases).unwrap();
    let mut events = session.accept(&audio);
    events.extend(session.settle());
    // Then
    assert_phrase_fills_segment(&events);
}

/// Deterministic noise of about `rms` (xorshift), so every run hears the same background.
fn noise(len: usize, rms: f32) -> Vec<f32> {
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    (0..len)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let uniform = (state >> 11) as f32 / (1u64 << 53) as f32 * 2.0 - 1.0;
            uniform * rms * 3f32.sqrt()
        })
        .collect()
}

fn spotted_in(engine: &Engine, audio: &[f32]) -> Vec<ServerEvent> {
    let mut session = engine.session().unwrap();
    let phrases: Vec<String> = ALL_PHRASES.iter().map(|p| (*p).to_owned()).collect();
    session.listen_for(&phrases).unwrap();
    fed(&mut session, audio)
}

/// About 20 dB quieter than `say`: a user sitting back from the built-in microphone.
const QUIET_GAIN: f32 = 0.1;
/// About -40 dBFS: a fan or an air conditioner in the room.
const ROOM_NOISE_RMS: f32 = 0.01;

#[test]
fn should_spot_quiet_speech() {
    let Some(engine) = engine() else { return };
    for phrase in ALL_PHRASES {
        // Given
        let audio: Vec<f32> = spoken(phrase).iter().map(|s| s * QUIET_GAIN).collect();
        // When
        let events = spotted_in(&engine, &audio);
        // Then
        assert_eq!(spotted(&events), [phrase], "quiet {phrase}: {events:?}");
        assert_phrase_fills_segment(&events);
    }
}

/// The loudest 10 ms of a quiet voice: the user's quietest recorded commands reach 0.0045.
const QUIET_VOICE_LEVEL: f32 = 0.005;
const SHORT_FORMS: [&str; 6] = ["start", "stop", "cancel", "restart", "enter", "clear"];

/// `audio` scaled so its loudest 10 ms has an RMS of `level`.
fn at_level(audio: Vec<f32>, level: f32) -> Vec<f32> {
    let loudest = audio
        .chunks(160)
        .map(|frame| (frame.iter().map(|s| s * s).sum::<f32>() / frame.len() as f32).sqrt())
        .fold(0.0_f32, f32::max);
    audio.into_iter().map(|s| s * level / loudest).collect()
}

fn speech_segments(events: &[ServerEvent]) -> usize {
    events
        .iter()
        .filter(|e| matches!(e, ServerEvent::SpeechStart { .. }))
        .count()
}

#[test]
fn should_notice_each_command_said_in_a_quiet_voice() {
    let Some(engine) = engine() else { return };
    for words in ALL_PHRASES.into_iter().chain(SHORT_FORMS) {
        // Given
        let audio = at_level(spoken(words), QUIET_VOICE_LEVEL);
        // When
        let events = spotted_in(&engine, &audio);
        // Then
        assert_eq!(speech_segments(&events), 1, "quiet {words}: {events:?}");
    }
}

#[test]
fn should_notice_no_speech_in_room_noise_alone() {
    let Some(engine) = engine() else { return };
    for rms in [0.003, ROOM_NOISE_RMS, 0.03] {
        // Given 20 s of steady room noise, from a quiet room to a loud fan
        let audio = noise(20 * SAMPLE_RATE as usize, rms);
        // When
        let events = spotted_in(&engine, &audio);
        // Then
        assert_eq!(speech_segments(&events), 0, "noise at {rms}: {events:?}");
    }
}

#[test]
fn should_spot_speech_over_room_noise() {
    let Some(engine) = engine() else { return };
    for phrase in ALL_PHRASES {
        // Given
        let mut audio = silence(LONG_SILENCE_SECS);
        audio.extend(spoken(phrase));
        let room = noise(audio.len(), ROOM_NOISE_RMS);
        let heard: Vec<f32> = audio.iter().zip(&room).map(|(s, n)| s + n).collect();
        // When
        let events = spotted_in(&engine, &heard);
        // Then
        assert_eq!(spotted(&events), [phrase], "noisy {phrase}: {events:?}");
        assert_phrase_fills_segment(&events);
    }
}

/// What the speech recognition service hears in every clip, word by word.
const HEARD_PRESS_ENTER: &str = r#"{"text":" press enter","segments":[{"words":[{"word":" press","start":0.1,"end":0.4},{"word":" enter","start":0.4,"end":0.8}]}]}"#;
/// Speech the keyword spotter does not take for a command.
const NOT_A_COMMAND: &str = "hello there";
const SHORT_DEADLINE: Duration = Duration::from_millis(800);

/// Hears "press enter" in every clip.
struct HearsPressEnter;

impl Hear for HearsPressEnter {
    fn hear(&self, _: &[f32], _: Duration) -> anyhow::Result<Heard> {
        Ok(Heard {
            text: " press enter".to_owned(),
            words: Vec::new(),
        })
    }
}

/// An engine of its own that confirms with `transcription_url`, loaded while the shared
/// engine is held so no other test runs models meanwhile.
fn own_engine(transcription_url: Option<String>) -> Option<Engine> {
    let mut cfg = engine_cfg(transcription_url)?;
    cfg.confirmation_deadline = SHORT_DEADLINE;
    Some(Engine::load(&cfg).expect("models load"))
}

/// A speech recognition service that never answers its first `hung` requests and hears
/// "press enter" in every later one.
fn speech_recognition_hanging_first(hung: usize) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || {
        let mut held = Vec::new();
        for (n, stream) in listener.incoming().flatten().enumerate() {
            if n < hung {
                held.push(stream);
            } else {
                answer(stream, HEARD_PRESS_ENTER);
            }
        }
    });
    format!("http://127.0.0.1:{port}/v1")
}

fn answer(mut stream: TcpStream, body: &str) {
    read_request(&mut stream);
    let _ = write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
}

/// Reads a request whole, headers and body, so the answer is not a reset.
fn read_request(stream: &mut TcpStream) {
    let mut request = Vec::new();
    let mut buf = [0u8; 8192];
    while let Ok(n @ 1..) = stream.read(&mut buf) {
        request.extend_from_slice(&buf[..n]);
        let text = String::from_utf8_lossy(&request);
        let Some(head_end) = text.find("\r\n\r\n") else {
            continue;
        };
        let length = text[..head_end]
            .lines()
            .find_map(|l| {
                l.to_ascii_lowercase()
                    .strip_prefix("content-length:")
                    .map(|v| v.trim().parse::<usize>().unwrap_or(0))
            })
            .unwrap_or(0);
        if request.len() >= head_end + 4 + length {
            return;
        }
    }
}

fn reported_phrases(events: &[ServerEvent]) -> Vec<String> {
    events
        .iter()
        .flat_map(|e| match e {
            ServerEvent::Phrase { phrase, .. } => vec![phrase.clone()],
            ServerEvent::Command { commands, .. } => commands.clone(),
            _ => Vec::new(),
        })
        .collect()
}

#[test]
fn should_confirm_an_utterance_with_what_the_speech_recognition_service_heard() {
    let Some(_shared) = engine() else { return };
    // Given
    let engine = own_engine(None).unwrap().hearing(Arc::new(HearsPressEnter));
    // When
    let events = events_for(&engine, NOT_A_COMMAND);
    // Then
    assert_eq!(reported_phrases(&events), ["press enter"], "{events:?}");
}

#[test]
fn should_confirm_a_command_said_alone_by_the_second_attempt() {
    let Some(_shared) = engine() else { return };
    // Given
    let engine = own_engine(Some(speech_recognition_hanging_first(1))).unwrap();
    // When
    let events = events_for(&engine, NOT_A_COMMAND);
    // Then
    assert_eq!(reported_phrases(&events), ["press enter"], "{events:?}");
}

#[test]
fn should_spot_a_command_after_a_hung_confirmation() {
    let Some(_shared) = engine() else { return };
    // Given
    let engine = own_engine(Some(speech_recognition_hanging_first(2))).unwrap();
    let mut session = engine.session().unwrap();
    let phrases: Vec<String> = ALL_PHRASES.iter().map(|p| (*p).to_owned()).collect();
    session.listen_for(&phrases).unwrap();
    let mut audio = spoken(NOT_A_COMMAND);
    audio.extend(spoken("press enter"));
    // When
    let events = fed(&mut session, &audio);
    // Then
    assert!(
        reported_phrases(&events).contains(&"press enter".to_owned()),
        "{events:?}"
    );
}

/// Takes 4 s for every clip, as a slow speech recognition service, and counts the clips.
struct SlowHear(std::sync::atomic::AtomicU32);

const SLOW_HEARING: Duration = Duration::from_secs(4);
/// Longer than one slow answer, so `settle` waits for each.
const PATIENT_DEADLINE: Duration = Duration::from_secs(5);
/// About ten seconds of speech in one breath.
const LONG_UTTERANCE: &str = "so I was saying that the weather today is really nice and we could go for a long walk along the river later this afternoon";
/// The head check, at most one window after it, and the segment: a slow service gets no more.
const MOST_CLIPS_WHILE_SLOW: u32 = 4;

impl Hear for SlowHear {
    fn hear(&self, _: &[f32], _: Duration) -> anyhow::Result<Heard> {
        self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        std::thread::sleep(SLOW_HEARING);
        Ok(Heard {
            text: " hello there".to_owned(),
            words: Vec::new(),
        })
    }
}

#[test]
fn should_skip_the_checks_of_a_long_utterance_while_one_waits_for_a_slow_service() {
    let Some(_shared) = engine() else { return };
    // Given
    let hear = Arc::new(SlowHear(std::sync::atomic::AtomicU32::new(0)));
    let mut cfg = engine_cfg(None).unwrap();
    cfg.confirmation_deadline = PATIENT_DEADLINE;
    let engine = Engine::load(&cfg)
        .expect("models load")
        .hearing(hear.clone());
    // When
    let events = events_for(&engine, LONG_UTTERANCE);
    // Then
    let clips = hear.0.load(std::sync::atomic::Ordering::SeqCst);
    assert!(
        clips <= MOST_CLIPS_WHILE_SLOW,
        "{clips} clips were sent for one utterance while the service was slow: {events:?}"
    );
}

/// Never hears anything in time: every confirmation is given up.
struct NeverInTime;

impl Hear for NeverInTime {
    fn hear(&self, _: &[f32], deadline: Duration) -> anyhow::Result<Heard> {
        anyhow::bail!("timed out reading response after {deadline:?}")
    }
}

#[test]
fn should_leave_the_keyword_spotted_phrase_to_decide_when_its_segment_confirmation_is_given_up() {
    let Some(_shared) = engine() else { return };
    // Given
    let engine = own_engine(None).unwrap().hearing(Arc::new(NeverInTime));
    // When
    let events = events_for(&engine, "press enter");
    // Then
    assert_eq!(spotted(&events), ["press enter"], "{events:?}");
}

/// A pause just longer than the 0.3 s that ends an utterance.
const SHORT_PAUSE_SECS: f32 = 0.32;
/// Said after a command and a short pause; it opens with a hard consonant.
const SENTENCE_AFTER_PAUSE: &str = "Coffee machine on the third floor is broken";
/// Quieter samples are silence.
const SILENCE_LEVEL: f32 = 0.01;
/// 10 ms: a stretch of sound short enough to be found again in a clip.
const MARK_SAMPLES: usize = 160;

/// `say` speech from its first sound to its last, without the silence `say` puts around it.
fn sound_only(audio: Vec<f32>) -> Vec<f32> {
    let first = audio
        .iter()
        .position(|s| s.abs() > SILENCE_LEVEL)
        .unwrap_or(0);
    let last = audio
        .iter()
        .rposition(|s| s.abs() > SILENCE_LEVEL)
        .map_or(first, |at| at + 1);
    audio[first..last].to_vec()
}

/// `command`, `pause_secs` of silence and the sentence, with a pause before and after; and
/// where the sentence starts, in seconds.
fn command_pause_sentence(command: &[f32], pause_secs: f32) -> (Vec<f32>, f32) {
    let mut audio = silence(PAUSE_SECS);
    audio.extend_from_slice(command);
    audio.extend(silence(pause_secs));
    let sentence_start = audio.len() as f32 / SAMPLE_RATE as f32;
    audio.extend(sound_only(common::spoken_by(None, SENTENCE_AFTER_PAUSE)));
    audio.extend(silence(PAUSE_SECS));
    (audio, sentence_start)
}

/// Where each speech segment ends, in seconds.
fn segment_ends(events: &[ServerEvent]) -> Vec<f32> {
    events
        .iter()
        .filter_map(|e| match e {
            ServerEvent::SpeechEnd { t } => Some(*t),
            _ => None,
        })
        .collect()
}

/// The user's quiet "restart" said on its own, cut from their labelled recording with the
/// margins the E2E scenarios use; none when the recordings are not on this machine.
fn users_quiet_restart() -> Option<Vec<f32>> {
    let Ok(root) = std::env::var("VOICEPUNCHER_VOICE_SAMPLES") else {
        eprintln!("GATED: VOICEPUNCHER_VOICE_SAMPLES not set, skipping");
        return None;
    };
    let path = PathBuf::from(root).join("session-2.wav");
    let audio: Vec<f32> = hound::WavReader::open(path)
        .ok()?
        .samples::<i16>()
        .map(|s| f32::from(s.unwrap()) / f32::from(i16::MAX))
        .collect();
    let at = |secs: f32| (secs * SAMPLE_RATE as f32) as usize;
    // Labelled 27.23–27.52 s; 0.3 s before and 0.2 s after keep its first and last sounds.
    Some(audio[at(26.93)..at(27.72)].to_vec())
}

/// The user's first two recorded "start dictation"s (session-1, labelled 4.98–6.19 s and
/// 6.58–7.41 s): 0.32 s of room between them, broken by a click at 6.42 s.
fn users_two_start_dictations() -> Option<Vec<f32>> {
    let Ok(root) = std::env::var("VOICEPUNCHER_VOICE_SAMPLES") else {
        eprintln!("GATED: VOICEPUNCHER_VOICE_SAMPLES not set, skipping");
        return None;
    };
    let path = PathBuf::from(root).join("session-1.wav");
    let audio: Vec<f32> = hound::WavReader::open(path)
        .ok()?
        .samples::<i16>()
        .map(|s| f32::from(s.unwrap()) / f32::from(i16::MAX))
        .collect();
    let at = |secs: f32| (secs * SAMPLE_RATE as f32) as usize;
    // From the session start: the detector hears the room before them as the app streams it;
    // silence after them closes the second one.
    let mut heard = audio[..at(7.9)].to_vec();
    heard.extend(silence(PAUSE_SECS));
    Some(heard)
}

#[test]
fn should_keep_the_users_two_commands_apart_across_a_short_pause_with_a_click() {
    let Some(engine) = engine() else { return };
    let Some(audio) = users_two_start_dictations() else {
        return;
    };
    // When
    let events = spotted_in(&engine, &audio);
    // Then
    assert_eq!(speech_segments(&events), 2, "{events:?}");
}

#[test]
fn should_end_an_utterance_at_a_short_pause_before_more_speech() {
    let Some(engine) = engine() else { return };
    // Given
    let command = sound_only(common::spoken_by(None, "Restart"));
    let (audio, sentence_start) = command_pause_sentence(&command, SHORT_PAUSE_SECS);
    let mut session = engine.session().unwrap();
    let phrases: Vec<String> = ALL_PHRASES.iter().map(|p| (*p).to_owned()).collect();
    session.listen_for(&phrases).unwrap();
    // When
    let events = fed(&mut session, &audio);
    // Then
    let ends = segment_ends(&events);
    assert!(
        ends.first().is_some_and(|end| *end < sentence_start),
        "the command's utterance did not end before the sentence at {sentence_start:.2}s: {events:?}"
    );
}

#[test]
fn should_end_the_users_quiet_command_at_a_short_pause_before_more_speech() {
    let Some(engine) = engine() else { return };
    let Some(command) = users_quiet_restart() else {
        return;
    };
    for pause_secs in [0.4, 0.5] {
        // Given
        let (audio, sentence_start) = command_pause_sentence(&command, pause_secs);
        let mut session = engine.session().unwrap();
        let phrases: Vec<String> = ALL_PHRASES.iter().map(|p| (*p).to_owned()).collect();
        session.listen_for(&phrases).unwrap();
        // When
        let events = fed(&mut session, &audio);
        // Then
        let ends = segment_ends(&events);
        assert!(
            ends.first().is_some_and(|end| *end < sentence_start),
            "after a {pause_secs} s pause the quiet command's utterance did not end before the sentence at {sentence_start:.2}s: {events:?}"
        );
    }
}

/// Keeps every clip it is asked to hear, and hears "restart" in each.
#[derive(Default)]
struct KeepsClips(Mutex<Vec<Vec<f32>>>);

impl Hear for KeepsClips {
    fn hear(&self, samples: &[f32], _: Duration) -> anyhow::Result<Heard> {
        self.0.lock().unwrap().push(samples.to_vec());
        Ok(Heard {
            text: "restart".to_owned(),
            words: Vec::new(),
        })
    }
}

/// Whether `clip` holds `mark` as it was streamed.
fn holds(clip: &[f32], mark: &[f32]) -> bool {
    clip.windows(mark.len()).any(|w| w == mark)
}

#[test]
fn should_confirm_a_command_followed_by_a_short_pause_without_the_speech_after_it() {
    let Some(_shared) = engine() else { return };
    // Given
    let hear = Arc::new(KeepsClips::default());
    let engine = own_engine(None).unwrap().hearing(hear.clone());
    let Some(command) = users_quiet_restart() else {
        return;
    };
    // The detector holds the quiet voice's speech about 0.55 s past its last sound: a clip
    // running past that end takes in the sentence's first sound.
    let (audio, sentence_start) = command_pause_sentence(&command, 0.6);
    let at = (sentence_start * SAMPLE_RATE as f32) as usize;
    let sentence_onset = audio[at..at + MARK_SAMPLES].to_vec();
    let command_end = command[command.len() - MARK_SAMPLES..].to_vec();
    let mut session = engine.session().unwrap();
    let phrases: Vec<String> = ALL_PHRASES.iter().map(|p| (*p).to_owned()).collect();
    session.listen_for(&phrases).unwrap();
    // When
    fed(&mut session, &audio);
    // Then
    let clips = hear.0.lock().unwrap();
    let mixed = clips
        .iter()
        .filter(|clip| holds(clip, &command_end) && holds(clip, &sentence_onset))
        .count();
    let command_clips = clips
        .iter()
        .filter(|clip| holds(clip, &command_end))
        .count();
    assert!(
        command_clips > 0 && mixed == 0,
        "{mixed} of {command_clips} clips holding the command also hold the sentence's first sound"
    );
}

/// Hears the command at the end of an utterance only in a clip of its last seconds: in the
/// whole utterance the user's "at ten enter" is heard as "at 10am".
struct HearsTheCommandOnlyInTheTail;

/// The longest clip in which the command is heard.
const TAIL_CLIP_SECS: f32 = 1.8;
/// Said as one utterance of more than 2 s, a command at its end.
const SENTENCE_ENDING_IN_ENTER: &str = "We meet on Friday at ten in the morning, enter";

fn timed(words: &[&str]) -> Vec<voicepuncher_command_spotting::confirm::TimedWord> {
    words
        .iter()
        .enumerate()
        .map(|(n, w)| voicepuncher_command_spotting::confirm::TimedWord {
            text: format!(" {w}"),
            start: 0.1 + n as f32 * 0.2,
            end: 0.25 + n as f32 * 0.2,
        })
        .collect()
}

impl Hear for HearsTheCommandOnlyInTheTail {
    fn hear(&self, samples: &[f32], _: Duration) -> anyhow::Result<Heard> {
        let words: &[&str] = if samples.len() as f32 <= TAIL_CLIP_SECS * SAMPLE_RATE as f32 {
            &["morning,", "enter."]
        } else {
            &["We", "meet", "on", "Friday", "at", "10am."]
        };
        Ok(Heard {
            text: words.join(" "),
            words: timed(words),
        })
    }
}

#[test]
fn should_hear_a_command_ending_a_long_utterance_in_its_last_seconds() {
    let Some(_shared) = engine() else { return };
    // Given
    let engine = own_engine(None)
        .unwrap()
        .hearing(Arc::new(HearsTheCommandOnlyInTheTail));
    // When
    let events = events_for(&engine, SENTENCE_ENDING_IN_ENTER);
    // Then
    assert_eq!(reported_phrases(&events), ["press enter"], "{events:?}");
}

const DICTATING_PHRASES: [&str; 4] = [
    "stop dictation",
    "cancel dictation",
    "restart dictation",
    "press enter",
];

/// Hears each answer in turn, one per clip, and nothing once they run out; counts the clips.
struct HearsInTurn {
    answers: Mutex<std::collections::VecDeque<&'static str>>,
    clips: Mutex<usize>,
}

impl HearsInTurn {
    fn new(answers: &[&'static str]) -> Arc<Self> {
        Arc::new(Self {
            answers: Mutex::new(answers.iter().copied().collect()),
            clips: Mutex::new(0),
        })
    }

    fn clips(&self) -> usize {
        *self.clips.lock().unwrap()
    }
}

impl Hear for HearsInTurn {
    fn hear(&self, _: &[f32], _: Duration) -> anyhow::Result<Heard> {
        *self.clips.lock().unwrap() += 1;
        let text = self.answers.lock().unwrap().pop_front().unwrap_or("");
        let words: Vec<&str> = text.split_whitespace().collect();
        Ok(Heard {
            text: text.to_owned(),
            words: timed(&words),
        })
    }
}

/// The events of `words` said on its own while `phrases` are selected, heard by `hear`.
fn heard_alone(hear: Arc<HearsInTurn>, phrases: &[&str], words: &str) -> Vec<ServerEvent> {
    let engine = own_engine(None).unwrap().hearing(hear);
    let mut session = engine.session().unwrap();
    let phrases: Vec<String> = phrases.iter().map(|p| (*p).to_owned()).collect();
    session.listen_for(&phrases).unwrap();
    fed(&mut session, &spoken(words))
}

#[test]
fn should_take_a_near_miss_of_restart_as_restart_when_its_second_hearing_says_restart() {
    let Some(_shared) = engine() else { return };
    // Given
    let hear = HearsInTurn::new(&["Let's start.", "Restart."]);
    // When
    let events = heard_alone(hear.clone(), &DICTATING_PHRASES, "Restart");
    // Then
    assert_eq!(
        reported_phrases(&events),
        ["restart dictation"],
        "{events:?}"
    );
    assert_eq!(hear.clips(), 2);
}

#[test]
fn should_place_the_restart_of_a_second_hearing_at_the_start_of_its_speech() {
    let Some(_shared) = engine() else { return };
    // Given
    let hear = HearsInTurn::new(&["Let's start.", "Restart."]);
    // When
    let events = heard_alone(hear, &DICTATING_PHRASES, "Restart");
    // Then
    let (segment, phrase) = segment_and_phrase(&events);
    assert!(
        (phrase.0 - segment.0).abs() < LEAD_TOLERANCE_SECS,
        "lead: segment {segment:?} phrase {phrase:?}"
    );
    assert!(
        phrase.1 <= segment.1 + TAIL_TOLERANCE_SECS,
        "tail: segment {segment:?} phrase {phrase:?}"
    );
}

#[test]
fn should_take_no_command_when_the_second_hearing_of_a_near_miss_says_it_again() {
    let Some(_shared) = engine() else { return };
    // Given
    let hear = HearsInTurn::new(&["Let's start.", "Let's start."]);
    // When
    let events = heard_alone(hear.clone(), &DICTATING_PHRASES, "Let's start");
    // Then
    assert!(reported_phrases(&events).is_empty(), "{events:?}");
    assert_eq!(hear.clips(), 2);
}

#[test]
fn should_not_hear_start_again_when_restart_is_not_selected() {
    let Some(_shared) = engine() else { return };
    // Given
    let standby = ["start dictation", "press enter", "clear all text"];
    let hear = HearsInTurn::new(&["Let's start.", "Restart."]);
    // When
    heard_alone(hear.clone(), &standby, "Let's start");
    // Then
    assert_eq!(hear.clips(), 1);
}

/// Hears exactly the words of the stream that a clip holds, with their times in the clip: a
/// word the clip's end cuts off is heard as "style", as the speech recognition service heard
/// the user's cut-off "start". Records the words a clip cut off.
struct HearsTheWordsInTheClip {
    stream: Vec<f32>,
    /// Each word and the stream samples it spans.
    words: Vec<(&'static str, std::ops::Range<usize>)>,
    cut_off: Mutex<Vec<&'static str>>,
}

/// Loud enough to be the start of a word, not the room.
const SOUND_LEVEL: f32 = 0.01;
/// Samples that place a clip in the stream.
const MARK_LEN: usize = 64;

impl HearsTheWordsInTheClip {
    /// `words` said one after another, `gap_secs` of silence between them, after a pause.
    fn said(words: &[&'static str], gap_secs: f32) -> Arc<Self> {
        let gap = vec![0.0f32; (gap_secs * SAMPLE_RATE as f32) as usize];
        let mut stream = vec![0.0f32; (PAUSE_SECS * SAMPLE_RATE as f32) as usize];
        let mut spans = Vec::new();
        for (n, word) in words.iter().enumerate() {
            if n > 0 {
                stream.extend(&gap);
            }
            let start = stream.len();
            stream.extend(without_silence(&common::spoken_by(None, word)));
            spans.push((*word, start..stream.len()));
        }
        stream.extend(vec![0.0f32; (PAUSE_SECS * SAMPLE_RATE as f32) as usize]);
        Arc::new(Self {
            stream,
            words: spans,
            cut_off: Mutex::new(Vec::new()),
        })
    }

    /// Where `clip` starts in the stream, found by its first sound.
    fn origin(&self, clip: &[f32]) -> Option<usize> {
        let at = clip.iter().position(|s| s.abs() > SOUND_LEVEL)?;
        let mark = clip.get(at..at + MARK_LEN)?;
        let found = self.stream.windows(MARK_LEN).position(|w| w == mark)?;
        found.checked_sub(at)
    }
}

/// `audio` from its first sound to its last: `say` puts silence around every word.
fn without_silence(audio: &[f32]) -> &[f32] {
    let first = audio
        .iter()
        .position(|s| s.abs() > SOUND_LEVEL)
        .unwrap_or(0);
    let last = audio
        .iter()
        .rposition(|s| s.abs() > SOUND_LEVEL)
        .unwrap_or(0);
    &audio[first..=last.max(first)]
}

impl Hear for HearsTheWordsInTheClip {
    fn hear(&self, clip: &[f32], _: Duration) -> anyhow::Result<Heard> {
        let Some(origin) = self.origin(clip) else {
            return Ok(Heard::default());
        };
        let end = origin + clip.len();
        let secs = |at: usize| (at - origin) as f32 / SAMPLE_RATE as f32;
        let mut words = Vec::new();
        for (word, span) in &self.words {
            if span.start < origin || span.start >= end {
                continue;
            }
            let cut = span.end > end;
            if cut {
                self.cut_off.lock().unwrap().push(word);
            }
            words.push(voicepuncher_command_spotting::confirm::TimedWord {
                text: format!(" {}", if cut { "style" } else { word }),
                start: secs(span.start),
                end: secs(span.end.min(end)),
            });
        }
        Ok(Heard {
            text: words.iter().map(|w| w.text.as_str()).collect(),
            words,
        })
    }
}

/// Every event for `audio` fed frame by frame, each confirmation answered before the next
/// frame: a service that answers at once, so every check of the running utterance is asked.
fn fed_answering_at_once(session: &mut Session<'_>, audio: &[f32]) -> Vec<ServerEvent> {
    let mut events = Vec::new();
    for frame in audio.chunks(FRAME_SAMPLES) {
        events.extend(session.accept(frame));
        events.extend(session.settle());
    }
    events.extend(session.settle());
    events
}

#[test]
fn should_carry_out_the_rest_of_a_chain_a_check_heard_only_in_part() {
    let Some(_shared) = engine() else { return };
    // Given a running utterance whose check is cut off inside its last command
    let hear = HearsTheWordsInTheClip::said(&["never mind", "cancel", "enter", "start"], 0.12);
    let engine = own_engine(None).unwrap().hearing(hear.clone());
    let mut session = engine.session().unwrap();
    let phrases: Vec<String> = DICTATING_PHRASES.iter().map(|p| (*p).to_owned()).collect();
    session.listen_for(&phrases).unwrap();
    // When the whole utterance is heard once it ends
    let events = fed_answering_at_once(&mut session, &hear.stream);
    // Then
    assert!(
        hear.cut_off.lock().unwrap().contains(&"start"),
        "no check was cut off inside \"start\": {:?}",
        hear.cut_off.lock().unwrap()
    );
    assert_eq!(
        reported_phrases(&events),
        ["cancel dictation", "press enter", "start dictation"],
        "{events:?}"
    );
}
