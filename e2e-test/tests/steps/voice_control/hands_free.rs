//! A message dictated and sent by voice alone: the real pipeline with voice control and the
//! real command spotting service hear `say` speech; the recorder, speech recognition, paste
//! and key presses are stand-ins that record what they were asked to do.

use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use cucumber::{given, then, when};

use voicepuncher::adapters::secondary::commandspotting::websocket::AudioStream;
use voicepuncher::adapters::secondary::textrefinement::passthrough::PassthroughTextRefiner;
use voicepuncher::core::application::audio_recorder::LiveChunk;
use voicepuncher::core::application::pipeline::{Pipeline, PipelineConfig};
use voicepuncher::core::application::{
    AudioRecorder, ChunkingCfg, CommandSpotter, KeyPresser, PipelineInput, SoundPlayer, TextPaster,
    Transcriber, VisualIndicator,
};
use voicepuncher::core::domain::{
    AudioBuffer16k, CleanedText, FailureNotice, RawTranscript, RecordingMode, VoiceCommand,
};
use voicepuncher_e2e_test::voice_commands::spoken_with_pauses;

use crate::steps::off_thread;
use crate::E2eWorld;

pub(super) const SENTENCE: &str = "Meeting moved to Friday.";
const SAY_SAMPLE_RATE_HZ: u32 = 16_000;
const FRAME_SAMPLES: usize = 1_600;
/// Longest the app may take to react to a spoken command.
const REACTION: Duration = Duration::from_secs(15);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Event {
    Listening,
    RecordingStarted,
    Pasted(String),
    ReturnPressed,
    ListeningOffSound,
    ListeningOnSound,
}

/// One frame of audio, as long as it lasts in real time.
const FRAME: Duration = Duration::from_millis(100);

type Events = Arc<Mutex<Vec<Event>>>;

/// What the hands-free scenario shares with the pipeline thread.
pub struct HandsFree {
    events: Events,
    microphone: Sender<Vec<f32>>,
}

impl std::fmt::Debug for HandsFree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HandsFree")
            .field("events", &self.events())
            .finish()
    }
}

impl HandsFree {
    pub(super) fn start(url: String) -> Self {
        let events = Events::default();
        let (microphone, heard) = mpsc::channel();
        let (inputs_tx, inputs_rx) = mpsc::channel();
        let spotter = SpeakerSpotter {
            url,
            inputs: inputs_tx,
            heard: Some(heard),
            stream: None,
            events: Arc::clone(&events),
        };
        let mut pipeline = Pipeline::new(
            Box::new(StandInRecorder(Arc::clone(&events))),
            Box::new(FixedTranscript),
            Box::new(PassthroughTextRefiner),
            Box::new(RecordingPaster(Arc::clone(&events))),
            Box::new(ListeningSounds(Arc::clone(&events))),
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
        Self { events, microphone }
    }

    pub(super) fn events(&self) -> Vec<Event> {
        self.events.lock().unwrap().clone()
    }

    /// Waits until `wanted` happened; returns whether it did within the reaction time.
    pub(super) fn saw(&self, wanted: impl Fn(&Event) -> bool) -> bool {
        let deadline = Instant::now() + REACTION;
        while Instant::now() < deadline {
            if self.events().iter().any(&wanted) {
                return true;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        false
    }

    pub(super) fn say(&self, words: &str) -> Result<()> {
        for frame in spoken_with_pauses(words)?.chunks(FRAME_SAMPLES) {
            self.microphone.send(frame.to_vec())?;
        }
        Ok(())
    }

    fn position(&self, wanted: impl Fn(&Event) -> bool) -> Option<usize> {
        self.events().iter().position(wanted)
    }

    /// Says `words` again and again, frame by frame in real time, for `duration`.
    pub(super) fn keep_saying(&self, words: &str, duration: Duration) -> Result<()> {
        let speech = spoken_with_pauses(words)?;
        let started = Instant::now();
        for frame in speech.chunks(FRAME_SAMPLES).cycle() {
            if started.elapsed() >= duration {
                return Ok(());
            }
            self.microphone.send(frame.to_vec())?;
            std::thread::sleep(FRAME);
        }
        Ok(())
    }

    /// Silence in real time for `duration`, as a microphone hears between two utterances.
    pub(super) fn keep_quiet(&self, duration: Duration) -> Result<()> {
        let started = Instant::now();
        while started.elapsed() < duration {
            self.microphone.send(vec![0.0; FRAME_SAMPLES])?;
            std::thread::sleep(FRAME);
        }
        Ok(())
    }
}

#[given("listening is on")]
fn listening_on(world: &mut E2eWorld) -> Result<()> {
    let url = world
        .command_spotting_url
        .clone()
        .context("the command spotting service is not set up")?;
    let hands_free = HandsFree::start(url);
    if !hands_free.saw(|e| *e == Event::Listening) {
        bail!("listening did not start: {:?}", hands_free.events());
    }
    world.hands_free = Some(hands_free);
    Ok(())
}

#[when(
    "the user says \"start dictation\", a sentence, \"stop dictation\" and \"press enter\", each command on its own"
)]
async fn dictate_and_send(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, move |world| {
        let hands_free = world.hands_free.as_ref().context("listening is not on")?;
        hands_free.say("start dictation")?;
        hands_free.saw(|e| *e == Event::RecordingStarted);
        hands_free.say(SENTENCE)?;
        hands_free.say("stop dictation")?;
        hands_free.saw(|e| matches!(e, Event::Pasted(_)));
        hands_free.say("press enter")?;
        hands_free.saw(|e| *e == Event::ReturnPressed);
        Ok(())
    })
    .await
}

#[then("the sentence is pasted into the focused app")]
fn sentence_pasted(world: &mut E2eWorld) -> Result<()> {
    let hands_free = world.hands_free.as_ref().context("listening is not on")?;
    let events = hands_free.events();
    if !events.contains(&Event::Pasted(SENTENCE.to_owned())) {
        bail!("the sentence was not pasted: {events:?}");
    }
    Ok(())
}

#[then("the Return key is pressed after it")]
fn return_after_paste(world: &mut E2eWorld) -> Result<()> {
    let hands_free = world.hands_free.as_ref().context("listening is not on")?;
    let pasted = hands_free.position(|e| matches!(e, Event::Pasted(_)));
    let entered = hands_free.position(|e| *e == Event::ReturnPressed);
    match (pasted, entered) {
        (Some(pasted), Some(entered)) if entered > pasted => Ok(()),
        _ => bail!(
            "Return was not pressed after the paste: {:?}",
            hands_free.events()
        ),
    }
}

/// The command spotting adapter's streaming path, with `say` speech in place of the microphone.
struct SpeakerSpotter {
    url: String,
    inputs: Sender<PipelineInput>,
    heard: Option<Receiver<Vec<f32>>>,
    stream: Option<AudioStream>,
    events: Events,
}

impl CommandSpotter for SpeakerSpotter {
    fn probe(&self) -> Result<()> {
        Ok(())
    }

    fn start_listening(&mut self, commands: &[VoiceCommand]) -> Result<()> {
        let heard = self.heard.take().context("the speaker is already in use")?;
        let stream = AudioStream::start(
            &self.url,
            commands,
            Instant::now(),
            SAY_SAMPLE_RATE_HZ,
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
}

struct StandInRecorder(Events);

impl AudioRecorder for StandInRecorder {
    fn start(&mut self) -> Result<()> {
        self.0.lock().unwrap().push(Event::RecordingStarted);
        Ok(())
    }

    fn stop(&mut self) -> Result<AudioBuffer16k> {
        let one_second = AudioBuffer16k::SAMPLE_RATE_HZ as usize;
        Ok(AudioBuffer16k::new(vec![1_000i16; one_second]))
    }

    fn start_live_chunks(&mut self, _cfg: ChunkingCfg) -> Result<Option<Receiver<LiveChunk>>> {
        Ok(None)
    }

    fn flush_live_chunks(&mut self) {}
}

struct FixedTranscript;

impl Transcriber for FixedTranscript {
    fn transcribe(&mut self, _audio: &AudioBuffer16k) -> Result<Option<RawTranscript>> {
        Ok(Some(RawTranscript::new(SENTENCE.to_owned())?))
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
        Ok(())
    }
}

/// Plays nothing; notes the listening-on and listening-off sounds.
struct ListeningSounds(Events);

impl SoundPlayer for ListeningSounds {
    fn play_start(&self) {}
    fn play_stop(&self) {}
    fn play_cancel(&self) {}
    fn play_listening_on(&self) {
        self.0.lock().unwrap().push(Event::ListeningOnSound);
    }
    fn play_listening_off(&self) {
        self.0.lock().unwrap().push(Event::ListeningOffSound);
    }
}

struct NoOverlay;

impl VisualIndicator for NoOverlay {
    fn show_recording(&self) {}
    fn show_recognizing_speech(&self) {}
    fn show_refining_text(&self) {}
    fn show_waiting_for_focus(&self) {}
    fn hide(&self) {}
    fn show_failure_notice(&self, _notice: &FailureNotice) {}
}
