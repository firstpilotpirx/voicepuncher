//! Stand-in speech recognition for the engine: hung, slow, scripted and the user's recordings.

use super::*;
use voicepuncher_command_spotting::conversions::saturating_usize;

/// What the speech recognition service hears in every clip, word by word.
pub(super) const HEARD_PRESS_ENTER: &str = r#"{"text":" press enter","segments":[{"words":[{"word":" press","start":0.1,"end":0.4},{"word":" enter","start":0.4,"end":0.8}]}]}"#;

/// Speech the keyword spotter does not take for a command.
pub(super) const NOT_A_COMMAND: &str = "hello there";

pub(super) const SHORT_DEADLINE: Duration = Duration::from_millis(800);

/// Hears "press enter" in every clip.
pub(super) struct HearsPressEnter;

impl Hear for HearsPressEnter {
    fn hear(&self, _: &[f32], _: Duration) -> anyhow::Result<Heard> {
        Ok(Heard { text: " press enter".to_owned(), words: Vec::new() })
    }
}

/// An engine of its own that confirms with `transcription_url`, loaded while the shared
/// engine is held so no other test runs models meanwhile.
pub(super) fn own_engine(transcription_url: Option<String>) -> Engine {
    let mut cfg = engine_cfg(transcription_url);
    cfg.confirmation_deadline = SHORT_DEADLINE;
    Engine::load(&cfg).expect("models load")
}

/// A speech recognition service that never answers its first `hung` requests and hears
/// "press enter" in every later one.
pub(super) fn speech_recognition_hanging_first(hung: usize) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || {
        let asked = listener.incoming().flatten().filter_map(|mut stream| {
            (!voicepuncher_test_steps::loopback::answered_as_whisper(&mut stream)).then_some(stream)
        });
        for (n, stream) in asked.enumerate() {
            if n < hung {
                // Never answered and never closed: the client waits on it.
                std::mem::forget(stream);
            } else {
                answer(stream, HEARD_PRESS_ENTER);
            }
        }
    });
    format!("http://127.0.0.1:{port}/v1")
}

pub(super) fn answer(mut stream: TcpStream, body: &str) {
    read_request(&mut stream);
    let _ = write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
}

/// Reads a request whole, headers and body, so the answer is not a reset.
pub(super) fn read_request(stream: &mut TcpStream) {
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

pub(crate) fn reported_phrases(events: &[ServerEvent]) -> Vec<String> {
    events
        .iter()
        .flat_map(|e| match e {
            ServerEvent::Phrase { phrase, .. } => vec![phrase.clone()],
            ServerEvent::Command { commands, .. } => commands.clone(),
            _ => Vec::new(),
        })
        .collect()
}

/// Takes 4 s for every clip, as a slow speech recognition service, and counts the clips.
pub(super) struct SlowHear(pub(crate) std::sync::atomic::AtomicU32);

pub(super) const SLOW_HEARING: Duration = Duration::from_secs(4);

/// Longer than one slow answer, so `settle` waits for each.
pub(super) const PATIENT_DEADLINE: Duration = Duration::from_secs(5);

/// About ten seconds of speech in one breath.
pub(super) const LONG_UTTERANCE: &str = "so I was saying that the weather today is really nice and we could go for a long walk along the river later this afternoon";

/// The early and the full head check, one window (each later one takes the place of the one
/// still waiting), the segment and its tail: a slow service gets no more.
pub(super) const MOST_CLIPS_WHILE_SLOW: u32 = 5;

impl Hear for SlowHear {
    fn hear(&self, _: &[f32], _: Duration) -> anyhow::Result<Heard> {
        self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        std::thread::sleep(SLOW_HEARING);
        Ok(Heard { text: " hello there".to_owned(), words: Vec::new() })
    }
}

/// Never hears anything in time: every confirmation is given up.
pub(super) struct NeverInTime;

impl Hear for NeverInTime {
    fn hear(&self, _: &[f32], deadline: Duration) -> anyhow::Result<Heard> {
        anyhow::bail!("timed out reading response after {deadline:?}")
    }
}

/// A pause just longer than the 0.3 s that ends an utterance.
pub(super) const SHORT_PAUSE_SECS: f32 = 0.32;

/// Said after a command and a short pause; it opens with a hard consonant.
pub(super) const SENTENCE_AFTER_PAUSE: &str = "Coffee machine on the third floor is broken";

/// Quieter samples are silence.
pub(super) const SILENCE_LEVEL: f32 = 0.01;

/// 10 ms: a stretch of sound short enough to be found again in a clip.
pub(super) const MARK_SAMPLES: usize = 160;

/// `say` speech from its first sound to its last, without the silence `say` puts around it.
pub(super) fn sound_only(audio: &[f32]) -> Vec<f32> {
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

/// The ends of the utterances `engine` heard in `command`, `pause_secs` of silence and the
/// sentence; where the sentence starts, in seconds; and every event.
pub(super) fn ends_before_sentence(
    engine: &Engine,
    command: &[f32],
    pause_secs: f32,
) -> (Vec<f32>, f32, Vec<ServerEvent>) {
    let (audio, sentence_start) = command_pause_sentence(command, pause_secs);
    let mut session = listening(engine, &ALL_PHRASES);
    let events = fed(&mut session, &audio);
    (segment_ends(&events), sentence_start, events)
}

/// `command`, `pause_secs` of silence and the sentence, with a pause before and after; and
/// where the sentence starts, in seconds.
pub(super) fn command_pause_sentence(command: &[f32], pause_secs: f32) -> (Vec<f32>, f32) {
    let mut audio = silence(PAUSE_SECS);
    audio.extend_from_slice(command);
    audio.extend(silence(pause_secs));
    let sentence_start = audio.len() as f32 / SAMPLE_RATE as f32;
    audio.extend(sound_only(&common::spoken_by(None, SENTENCE_AFTER_PAUSE)));
    audio.extend(silence(PAUSE_SECS));
    (audio, sentence_start)
}

/// Where each speech segment ends, in seconds.
pub(super) fn segment_ends(events: &[ServerEvent]) -> Vec<f32> {
    events
        .iter()
        .filter_map(|e| match e {
            ServerEvent::SpeechEnd { t } => Some(*t),
            _ => None,
        })
        .collect()
}

/// The user's recording `session` of `~/.voicepuncher/voice-samples/`.
pub(super) fn users_session(session: &str) -> Vec<f32> {
    let root = required_env("VOICEPUNCHER_VOICE_SAMPLES");
    hound::WavReader::open(PathBuf::from(root).join(format!("{session}.wav")))
        .unwrap()
        .samples::<i16>()
        .map(|s| f32::from(s.unwrap()) / f32::from(i16::MAX))
        .collect()
}

/// The user's quiet "restart" said on its own, cut from their labelled recording with the
/// margins the E2E scenarios use; none when the recordings are not on this machine.
pub(super) fn users_quiet_restart() -> Vec<f32> {
    let audio = users_session("session-2");
    let at = |secs: f32| saturating_usize(secs * SAMPLE_RATE as f32);
    // Labelled 27.23–27.52 s; 0.3 s before and 0.2 s after keep its first and last sounds.
    audio[at(26.93)..at(27.72)].to_vec()
}

/// The user's first two recorded "start dictation"s (session-1, labelled 4.98–6.19 s and
/// 6.58–7.41 s): 0.32 s of room between them, broken by a click at 6.42 s.
pub(super) fn users_two_start_dictations() -> Vec<f32> {
    let audio = users_session("session-1");
    let at = |secs: f32| saturating_usize(secs * SAMPLE_RATE as f32);
    // From the session start: the detector hears the room before them as the app streams it;
    // silence after them closes the second one.
    let mut heard = audio[..at(7.9)].to_vec();
    heard.extend(silence(PAUSE_SECS));
    heard
}

/// Keeps every clip it is asked to hear, and hears "restart" in each.
#[derive(Default)]
pub(super) struct KeepsClips(pub(super) Mutex<Vec<Vec<f32>>>);

impl Hear for KeepsClips {
    fn hear(&self, samples: &[f32], _: Duration) -> anyhow::Result<Heard> {
        self.0.lock().unwrap().push(samples.to_vec());
        Ok(Heard { text: "restart".to_owned(), words: Vec::new() })
    }
}

/// Whether `clip` holds `mark` as it was streamed.
pub(super) fn holds(clip: &[f32], mark: &[f32]) -> bool {
    clip.windows(mark.len()).any(|w| w == mark)
}

/// Hears the command at the end of an utterance only in a clip of its last seconds: in the
/// whole utterance the user's "at ten enter" is heard as "at 10am".
pub(super) struct HearsTheCommandOnlyInTheTail;

/// The longest clip in which the command is heard.
pub(super) const TAIL_CLIP_SECS: f32 = 1.8;

/// Said as one utterance of more than 2 s, a command at its end.
pub(super) const SENTENCE_ENDING_IN_ENTER: &str = "We meet on Friday at ten in the morning, enter";

pub(super) fn timed(words: &[&str]) -> Vec<voicepuncher_command_spotting::confirm::TimedWord> {
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
        Ok(Heard { text: words.join(" "), words: timed(words) })
    }
}
