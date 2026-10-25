//! A long hands-free session end to end: one microphone stream feeds both the real command
//! spotting service and the recording, the real pipeline runs voice control, the real speech
//! recognition service transcribes each dictation; only paste, key presses and sounds are
//! stand-ins that record what they were asked to do.

use voicepuncher::test_doubles::prelude::*;
use voicepuncher_e2e_test::speech::split_words;

use cucumber::{given, then, when};

use voicepuncher::adapters::secondary::commandspotting::websocket::{AudioStream, MicrophoneAudio};
use voicepuncher::adapters::secondary::transcription::openai::OpenAiTranscriber;
use voicepuncher::config::TranscriptionCfg;
use voicepuncher_e2e_test::speech::read_wav;
use voicepuncher_e2e_test::voice_commands::spoken_by;

mod recorded_starts;
pub(crate) mod speech_after_start;
mod start_cue;

pub(super) use speech_after_start::{recorded_form, until_last_sound, users_quiet_restart};

use super::RUSSIAN_VOICE;
use crate::steps::file_transcription::env;
use crate::steps::off_thread;
use crate::E2eWorld;
mod doubles;
use doubles::{MicrophoneRecorder, NoOverlay, RecordingSounds};

use super::stand_ins::{run_with_voice_control, Event, Events, RecordingPaster, StreamingSpotter};

mod audio;
use audio::{command_audio, room_noise, session, to_f32};
mod session_steps;
use session_steps::session_of;
use voicepuncher_e2e_test::conversions::{saturating_u32, saturating_usize};

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

/// Everything the microphone captured, and when its first sample was captured.
#[derive(Default)]
struct Microphone {
    captured: Mutex<Vec<f32>>,
    started_at: Mutex<Option<Instant>>,
}

impl Microphone {
    fn index_at(&self, at: Instant) -> usize {
        let started = self.started_at.lock().unwrap().unwrap_or(at);
        saturating_usize(at.saturating_duration_since(started).as_secs_f32() * SAMPLE_RATE_HZ)
    }
}

/// When each start cue was given: the start sound or the restart sound.
type Cues = Arc<Mutex<Vec<Instant>>>;

/// What the session scenario shares with the pipeline thread.
pub struct HandsFreeSession {
    events: Events,
    cues: Cues,
    microphone: Arc<Microphone>,
    to_spotter: Sender<Vec<f32>>,
    /// The voice starts the scenario measured, in order.
    starts: Mutex<Vec<start_cue::MeasuredStart>>,
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
        let cues = Cues::default();
        let microphone = Arc::new(Microphone::default());
        let (to_spotter, heard) = mpsc::channel();
        let (inputs_tx, inputs_rx) = mpsc::channel();
        let transcriber = OpenAiTranscriber::connect(TranscriptionCfg {
            base_url: transcription_url,
            ..TranscriptionCfg::default()
        })?;
        let mut spotter = StreamingSpotter::new(spotting_url, inputs_tx, &events);
        spotter.heard = Some(heard);
        spotter.keeps_heard = true;
        let heard_by = Arc::clone(&microphone);
        spotter.source = Box::new(move || {
            let started_at = Instant::now();
            *heard_by.started_at.lock().unwrap() = Some(started_at);
            (saturating_u32(SAMPLE_RATE_HZ), started_at)
        });
        let recorder = MicrophoneRecorder {
            microphone: Arc::clone(&microphone),
            events: Arc::clone(&events),
            start: 0,
            cut: None,
            lead_in: Vec::new(),
        };
        let parts = PipelineParts {
            recorder: Box::new(recorder),
            transcriber: Box::new(transcriber),
            paster: Box::new(RecordingPaster::new(&events, Event::Pasted)),
            sound_player: Box::new(RecordingSounds(Arc::clone(&events), Arc::clone(&cues))),
            visual_indicator: Box::new(NoOverlay),
        };
        run_with_voice_control(parts, spotter, &events, inputs_rx);
        Ok(Self { events, cues, microphone, to_spotter, starts: Mutex::default() })
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
        let mut frame_due = Instant::now();
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
            // WHY: on a fixed schedule, so a slow send does not stretch the room's sound.
            frame_due += FRAME_TIME;
            std::thread::sleep(frame_due.saturating_duration_since(Instant::now()));
        }
        Ok(false)
    }

    fn run(&self, voice: CommandVoice) -> Result<()> {
        let room = match voice {
            CommandVoice::Synthesized => vec![0.0; FRAME_SAMPLES],
            CommandVoice::Recorded { .. } => room_noise(COMMAND_PAUSE_SECS)?,
        };
        for step in session() {
            if matches!(voice, CommandVoice::Recorded { long_pauses: true }) {
                self.hear(&room_noise(LONG_PAUSE_SECS)?)?;
            }
            self.take(step, voice, &room)?;
        }
        Ok(())
    }

    /// Says `step` in `voice`; a command's reaction is waited for over the `room`'s sound.
    fn take(&self, step: Step, voice: CommandVoice, room: &[f32]) -> Result<()> {
        match step {
            Step::Dictate(speaker, sentence) => self.hear(&spoken_by(speaker, sentence)?),
            Step::Command(said, reaction) => {
                let seen = self.events().len();
                self.hear(&command_audio(voice, said)?)?;
                self.reacted(seen, &reaction, room).map(drop)
            }
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

/// Whether `pasted` holds enough of `sentence`'s words and no word of a command phrase.
fn pasted_as_said(pasted: &str, sentence: &str) -> bool {
    let pasted_words = split_words(pasted);
    let expected = split_words(sentence);
    let kept = expected.iter().filter(|w| pasted_words.contains(w)).count();
    let no_command = !pasted_words
        .iter()
        .any(|w| COMMAND_WORDS.contains(&w.as_str()) && !expected.contains(w));
    no_command && kept as f32 >= MIN_WORDS_KEPT * expected.len() as f32
}
