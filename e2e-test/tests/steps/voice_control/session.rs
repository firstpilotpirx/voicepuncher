//! A long hands-free session end to end: one microphone stream feeds both the real command
//! spotting service and the recording, the real pipeline runs voice control, the real speech
//! recognition service transcribes each dictation; only paste, key presses and sounds are
//! stand-ins that record what they were asked to do.

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use cucumber::{given, then, when};

use voicepuncher::adapters::secondary::commandspotting::websocket::AudioStream;
use voicepuncher::adapters::secondary::textrefinement::passthrough::PassthroughTextRefiner;
use voicepuncher::adapters::secondary::transcription::openai::OpenAiTranscriber;
use voicepuncher::config::TranscriptionCfg;
use voicepuncher::core::application::audio_recorder::LiveChunk;
use voicepuncher::core::application::pipeline::{Pipeline, PipelineConfig};
use voicepuncher::core::application::{
    AudioRecorder, ChunkingCfg, CommandSpotter, KeyPresser, PipelineInput, SoundPlayer, TextPaster,
    VisualIndicator,
};
use voicepuncher::core::domain::{AudioBuffer16k, CleanedText, RecordingMode, VoiceCommand};
use voicepuncher_e2e_test::speech::read_wav;
use voicepuncher_e2e_test::voice_commands::spoken_by;

mod speech_after_start;

use super::RUSSIAN_VOICE;
use crate::steps::file_transcription::env;
use crate::E2eWorld;

const SAMPLE_RATE_HZ: f32 = 16_000.0;
const FRAME_SAMPLES: usize = 1_600;
/// How long one frame of the microphone lasts.
const FRAME_TIME: Duration = Duration::from_millis(100);
/// How long the recorder takes to open the microphone, as macOS takes up to 0.3 s: nothing
/// said meanwhile is recorded.
const MICROPHONE_OPENING: Duration = Duration::from_millis(300);
/// Longest the app may take to carry out a command, transcription of a dictation included.
const REACTION: Duration = Duration::from_secs(30);
const ENGLISH_VOICE: &str = "Eddy";
/// The room the user recorded in, between the steps of a session with long pauses.
const LONG_PAUSE_SECS: f32 = 15.0;
/// Room noise around a recorded command, as in a real stream.
const COMMAND_PAUSE_SECS: f32 = 0.8;
/// Share of a dictated sentence's words the pasted text must hold: whisper misspells some
/// Russian words of a synthesized voice (case endings, numbers as digits), so an exact match
/// would test the speech recognition model, not the session.
const MIN_WORDS_KEPT: f32 = 0.6;
/// Words of the command phrases: none may reach the pasted text unless the sentence has it.
const COMMAND_WORDS: [&str; 8] = [
    "start",
    "stop",
    "cancel",
    "dictation",
    "press",
    "enter",
    "clear",
    "please",
];

#[derive(Debug, Clone, PartialEq, Eq)]
enum Event {
    Listening,
    RecordingStarted,
    Pasted(String),
    Cancelled,
    ReturnPressed,
    Cleared,
}

/// What the scenario expects, in order; a paste names the sentence it must hold.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Expected {
    RecordingStarted,
    Pasted(&'static str),
    Cancelled,
    ReturnPressed,
    Cleared,
}

/// Who says the commands of a session.
#[derive(Debug, Clone, Copy)]
enum CommandVoice {
    Synthesized,
    Recorded { long_pauses: bool },
}

/// One step of a session: a command and the app's reaction to wait for, or a dictation.
enum Step {
    Command(&'static str, Expected),
    Dictate(&'static str, &'static str),
}

const SENT_RU: &str = "Встреча переносится на пятницу в десять утра";
const CANCELLED_EN: &str = "Please review the pull request before lunch";
const CLEARED_EN: &str = "The build is green again";
const SENT_AGAIN_RU: &str = "Купи молоко и хлеб по дороге домой";

/// Send, cancel, clear, send again — each with and without "please".
fn session() -> Vec<Step> {
    use Expected::*;
    vec![
        Step::Command("start dictation", RecordingStarted),
        Step::Dictate(RUSSIAN_VOICE, SENT_RU),
        Step::Command("stop dictation", Pasted(SENT_RU)),
        Step::Command("press enter", ReturnPressed),
        Step::Command("start dictation", RecordingStarted),
        Step::Dictate(ENGLISH_VOICE, CANCELLED_EN),
        Step::Command("cancel dictation", Cancelled),
        Step::Command("start dictation please", RecordingStarted),
        Step::Dictate(ENGLISH_VOICE, CLEARED_EN),
        Step::Command("stop dictation please", Pasted(CLEARED_EN)),
        Step::Command("clear text", Cleared),
        Step::Command("start dictation", RecordingStarted),
        Step::Dictate(RUSSIAN_VOICE, SENT_AGAIN_RU),
        Step::Command("stop dictation", Pasted(SENT_AGAIN_RU)),
        Step::Command("press enter please", ReturnPressed),
    ]
}

/// The user's recording of `said`, among the labelled recordings of the first session.
fn recorded(said: &str) -> Result<Vec<f32>> {
    let file = match said {
        "start dictation" => "02-start-dictation.wav",
        "start dictation please" => "05-start-dictation.wav",
        "stop dictation" => "10-stop-dictation.wav",
        "stop dictation please" => "13-stop-dictation.wav",
        "cancel dictation" => "18-cancel-dictation.wav",
        "press enter" => "26-press-enter.wav",
        "press enter please" => "25-press-enter.wav",
        "clear text" => "38-clear-text.wav",
        other => bail!("no recording of {other:?}"),
    };
    let dir = PathBuf::from(env("VOICEPUNCHER_VOICE_SAMPLES")?).join("session-1");
    Ok(to_f32(read_wav(&dir.join(file))?))
}

fn room_noise(secs: f32) -> Result<Vec<f32>> {
    let path = PathBuf::from(env("VOICEPUNCHER_VOICE_SAMPLES")?).join("session-1.wav");
    let room = to_f32(read_wav(&path)?);
    let quiet = &room[(0.5 * SAMPLE_RATE_HZ) as usize..(4.5 * SAMPLE_RATE_HZ) as usize];
    Ok(quiet
        .iter()
        .copied()
        .cycle()
        .take((secs * SAMPLE_RATE_HZ) as usize)
        .collect())
}

fn to_f32(samples: Vec<i16>) -> Vec<f32> {
    samples
        .into_iter()
        .map(|s| f32::from(s) / f32::from(i16::MAX))
        .collect()
}

/// Everything the microphone captured, and when its first sample was captured.
#[derive(Default)]
struct Microphone {
    captured: Mutex<Vec<f32>>,
    started_at: Mutex<Option<Instant>>,
}

impl Microphone {
    fn index_at(&self, at: Instant) -> usize {
        let started = self.started_at.lock().unwrap().unwrap_or(at);
        (at.saturating_duration_since(started).as_secs_f32() * SAMPLE_RATE_HZ) as usize
    }
}

type Events = Arc<Mutex<Vec<Event>>>;

/// What the session scenario shares with the pipeline thread.
pub struct HandsFreeSession {
    events: Events,
    microphone: Arc<Microphone>,
    to_spotter: Sender<Vec<f32>>,
}

impl std::fmt::Debug for HandsFreeSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HandsFreeSession")
            .field("events", &self.events())
            .finish()
    }
}

impl HandsFreeSession {
    fn start(spotting_url: String, transcription_url: String) -> Result<Self> {
        let events = Events::default();
        let microphone = Arc::new(Microphone::default());
        let (to_spotter, heard) = mpsc::channel();
        let (inputs_tx, inputs_rx) = mpsc::channel();
        let transcriber = OpenAiTranscriber::connect(TranscriptionCfg {
            base_url: transcription_url,
            ..TranscriptionCfg::default()
        })?;
        let spotter = MicrophoneSpotter {
            url: spotting_url,
            inputs: inputs_tx,
            heard: Some(heard),
            stream: None,
            microphone: Arc::clone(&microphone),
            events: Arc::clone(&events),
        };
        let recorder = MicrophoneRecorder {
            microphone: Arc::clone(&microphone),
            events: Arc::clone(&events),
            start: 0,
            cut: None,
            lead_in: Vec::new(),
        };
        let mut pipeline = Pipeline::new(
            Box::new(recorder),
            Box::new(transcriber),
            Box::new(PassthroughTextRefiner),
            Box::new(RecordingPaster(Arc::clone(&events))),
            Box::new(RecordingSounds(Arc::clone(&events))),
            Box::new(NoOverlay),
            PipelineConfig {
                min_press_ms: 0,
                recording_mode: RecordingMode::Toggle,
                chunking: ChunkingCfg {
                    chunk_threshold_secs: 0,
                    ..ChunkingCfg::default()
                },
            },
        )
        .with_voice_control(
            Box::new(spotter),
            Box::new(RecordingKeys(Arc::clone(&events))),
        );
        std::thread::spawn(move || pipeline.run(inputs_rx));
        Ok(Self {
            events,
            microphone,
            to_spotter,
        })
    }

    fn events(&self) -> Vec<Event> {
        self.events.lock().unwrap().clone()
    }

    fn hear(&self, samples: &[f32]) -> Result<()> {
        for frame in samples.chunks(FRAME_SAMPLES) {
            self.microphone
                .captured
                .lock()
                .unwrap()
                .extend_from_slice(frame);
            self.to_spotter.send(frame.to_vec())?;
        }
        Ok(())
    }

    /// Waits until the app reacted with `wanted` after the first `seen` events, while the
    /// microphone keeps capturing the room, as a real one does.
    fn reacted(&self, seen: usize, wanted: &Expected, room: &[f32]) -> Result<bool> {
        let deadline = Instant::now() + REACTION;
        for frame in room.chunks(FRAME_SAMPLES).cycle() {
            if self.events()[seen..]
                .iter()
                .any(|e| matches_kind(e, wanted))
            {
                return Ok(true);
            }
            if Instant::now() >= deadline {
                return Ok(false);
            }
            self.hear(frame)?;
            std::thread::sleep(FRAME_TIME);
        }
        Ok(false)
    }

    fn run(&self, voice: CommandVoice) -> Result<()> {
        let room = match voice {
            CommandVoice::Synthesized => vec![0.0; FRAME_SAMPLES],
            CommandVoice::Recorded { .. } => room_noise(COMMAND_PAUSE_SECS)?,
        };
        for step in session() {
            if let CommandVoice::Recorded { long_pauses: true } = voice {
                self.hear(&room_noise(LONG_PAUSE_SECS)?)?;
            }
            match step {
                Step::Dictate(speaker, sentence) => self.hear(&spoken_by(speaker, sentence)?)?,
                Step::Command(said, reaction) => {
                    let seen = self.events().len();
                    self.hear(&command_audio(voice, said)?)?;
                    self.reacted(seen, &reaction, &room)?;
                }
            }
        }
        Ok(())
    }
}

fn command_audio(voice: CommandVoice, said: &str) -> Result<Vec<f32>> {
    match voice {
        CommandVoice::Synthesized => spoken_by(ENGLISH_VOICE, said),
        CommandVoice::Recorded { .. } => {
            let mut audio = room_noise(COMMAND_PAUSE_SECS)?;
            audio.extend(recorded(said)?);
            audio.extend(room_noise(COMMAND_PAUSE_SECS)?);
            Ok(audio)
        }
    }
}

fn matches_kind(event: &Event, expected: &Expected) -> bool {
    matches!(
        (event, expected),
        (Event::RecordingStarted, Expected::RecordingStarted)
            | (Event::Pasted(_), Expected::Pasted(_))
            | (Event::Cancelled, Expected::Cancelled)
            | (Event::ReturnPressed, Expected::ReturnPressed)
            | (Event::Cleared, Expected::Cleared)
    )
}

fn words(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Whether `pasted` holds enough of `sentence`'s words and no word of a command phrase.
fn pasted_as_said(pasted: &str, sentence: &str) -> bool {
    let pasted_words = words(pasted);
    let expected = words(sentence);
    let kept = expected.iter().filter(|w| pasted_words.contains(w)).count();
    let no_command = !pasted_words
        .iter()
        .any(|w| COMMAND_WORDS.contains(&w.as_str()) && !expected.contains(w));
    no_command && kept as f32 >= MIN_WORDS_KEPT * expected.len() as f32
}

fn start_session(world: &mut E2eWorld) -> Result<()> {
    let url = world
        .command_spotting_url
        .clone()
        .context("the command spotting service is not set up")?;
    let session = HandsFreeSession::start(url, env("VOICEPUNCHER_TRANSCRIPTION_URL")?)?;
    let deadline = Instant::now() + REACTION;
    while !session.events().contains(&Event::Listening) {
        if Instant::now() >= deadline {
            bail!("listening did not start");
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    world.session = Some(session);
    Ok(())
}

#[given("listening is on for a whole session")]
fn listening_for_a_session(world: &mut E2eWorld) -> Result<()> {
    start_session(world)
}

#[given("the user's own recordings of the commands exist on the machine")]
fn recordings_exist(_world: &mut E2eWorld) -> Result<()> {
    let dir = PathBuf::from(env("VOICEPUNCHER_VOICE_SAMPLES")?);
    if !dir.join("session-1").join("manifest.jsonl").is_file() {
        bail!("no recorded commands in {}", dir.display());
    }
    Ok(())
}

fn session_of(world: &E2eWorld) -> Result<&HandsFreeSession> {
    world.session.as_ref().context("listening is not on")
}

#[when("a synthesized voice goes through the hands-free session")]
fn synthesized_session(world: &mut E2eWorld) -> Result<()> {
    session_of(world)?.run(CommandVoice::Synthesized)
}

#[when("the hands-free session is spoken with the user's recorded commands")]
fn recorded_session(world: &mut E2eWorld) -> Result<()> {
    session_of(world)?.run(CommandVoice::Recorded { long_pauses: false })
}

#[when("the hands-free session is spoken with the user's recorded commands and long pauses")]
fn recorded_session_with_pauses(world: &mut E2eWorld) -> Result<()> {
    session_of(world)?.run(CommandVoice::Recorded { long_pauses: true })
}

#[then("the app does everything the session said, in order")]
fn session_done(world: &mut E2eWorld) -> Result<()> {
    let events: Vec<Event> = session_of(world)?
        .events()
        .into_iter()
        .filter(|e| *e != Event::Listening)
        .collect();
    let expected: Vec<Expected> = session()
        .into_iter()
        .filter_map(|step| match step {
            Step::Command(_, reaction) => Some(reaction),
            Step::Dictate(..) => None,
        })
        .collect();
    let in_order = events.len() == expected.len()
        && events
            .iter()
            .zip(&expected)
            .all(|(event, wanted)| match (event, wanted) {
                (Event::Pasted(text), Expected::Pasted(sentence)) => pasted_as_said(text, sentence),
                _ => matches_kind(event, wanted),
            });
    if !in_order {
        bail!("expected {expected:?}\n     got {events:?}");
    }
    Ok(())
}

/// The command spotting adapter's streaming path, fed by the session's microphone.
struct MicrophoneSpotter {
    url: String,
    inputs: Sender<PipelineInput>,
    heard: Option<Receiver<Vec<f32>>>,
    stream: Option<AudioStream>,
    microphone: Arc<Microphone>,
    events: Events,
}

impl CommandSpotter for MicrophoneSpotter {
    fn probe(&self) -> Result<()> {
        Ok(())
    }

    fn start_listening(&mut self, commands: &[VoiceCommand]) -> Result<()> {
        let heard = self
            .heard
            .take()
            .context("the microphone is already in use")?;
        let started_at = Instant::now();
        *self.microphone.started_at.lock().unwrap() = Some(started_at);
        let rate = SAMPLE_RATE_HZ as u32;
        let stream = AudioStream::start(
            &self.url,
            commands,
            started_at,
            rate,
            heard,
            self.inputs.clone(),
        )?;
        self.stream = Some(stream);
        self.events.lock().unwrap().push(Event::Listening);
        Ok(())
    }

    fn listen_for(&mut self, commands: &[VoiceCommand]) {
        if let Some(stream) = self.stream.as_ref() {
            stream.listen_for(commands);
        }
    }

    fn stop_listening(&mut self) {
        if let Some(stream) = self.stream.take() {
            stream.stop();
        }
    }

    fn heard_since(&self, since: Instant) -> AudioBuffer16k {
        self.stream.as_ref().map_or_else(
            || AudioBuffer16k::new(Vec::new()),
            |stream| stream.heard_since(since),
        )
    }
}

/// Records from the session's microphone, as the real recorder records from the real one.
struct MicrophoneRecorder {
    microphone: Arc<Microphone>,
    events: Events,
    start: usize,
    cut: Option<usize>,
    /// Heard before the recording started, put at its beginning.
    lead_in: Vec<i16>,
}

impl AudioRecorder for MicrophoneRecorder {
    fn start(&mut self) -> Result<()> {
        std::thread::sleep(MICROPHONE_OPENING);
        self.start = self.microphone.captured.lock().unwrap().len();
        self.cut = None;
        self.lead_in.clear();
        self.events.lock().unwrap().push(Event::RecordingStarted);
        Ok(())
    }

    fn stop(&mut self) -> Result<AudioBuffer16k> {
        let captured = self.microphone.captured.lock().unwrap();
        let end = self.cut.unwrap_or(captured.len()).min(captured.len());
        let recorded = captured[self.start.min(end)..end]
            .iter()
            .map(|s| (s.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16);
        let samples = self.lead_in.iter().copied().chain(recorded).collect();
        Ok(AudioBuffer16k::new(samples))
    }

    fn start_live_chunks(&mut self, _cfg: ChunkingCfg) -> Result<Option<Receiver<LiveChunk>>> {
        Ok(None)
    }

    fn flush_live_chunks(&mut self) {}

    fn truncate_at(&mut self, cut: Instant) {
        self.cut = Some(self.microphone.index_at(cut));
    }

    fn begin_with(&mut self, lead_in: AudioBuffer16k) {
        self.lead_in = lead_in.samples().to_vec();
    }
}

struct RecordingPaster(Events);

impl TextPaster for RecordingPaster {
    fn paste(&mut self, text: &CleanedText) -> Result<()> {
        let pasted = Event::Pasted(text.as_str().to_owned());
        self.0.lock().unwrap().push(pasted);
        Ok(())
    }

    fn leave_in_clipboard(&mut self, text: &CleanedText) -> Result<()> {
        self.paste(text)
    }
}

struct RecordingKeys(Events);

impl KeyPresser for RecordingKeys {
    fn press_enter(&mut self) -> Result<()> {
        self.0.lock().unwrap().push(Event::ReturnPressed);
        Ok(())
    }

    fn clear_all_text(&mut self) -> Result<()> {
        self.0.lock().unwrap().push(Event::Cleared);
        Ok(())
    }
}

struct RecordingSounds(Events);

impl SoundPlayer for RecordingSounds {
    fn play_start(&self) {}
    fn play_stop(&self) {}
    fn play_cancel(&self) {
        self.0.lock().unwrap().push(Event::Cancelled);
    }
}

struct NoOverlay;

impl VisualIndicator for NoOverlay {
    fn show_recording(&self) {}
    fn show_processing(&self) {}
    fn show_waiting_for_focus(&self) {}
    fn hide(&self) {}
}
