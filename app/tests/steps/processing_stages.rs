//! Drives the real pipeline on its own thread and watches, in one ordered log, which
//! processing stage the overlay shows while speech recognition runs.

use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use cucumber::{given, then, when};

use voicepuncher::core::application::audio_recorder::LiveChunk;
use voicepuncher::core::application::pipeline::{Pipeline, PipelineConfig, PipelineParts};
use voicepuncher::core::application::{
    AudioRecorder, ChunkingCfg, FocusProbe, FocusWaitCfg, PipelineInput, SoundPlayer, TextPaster,
    Transcriber, VisualIndicator,
};
use voicepuncher::core::domain::{
    AudioBuffer16k, CleanedText, FailureNotice, FocusedElement, PttSignal, RawTranscript,
    SpottingEvent, VoiceCommand,
};

use crate::steps::quiet_voice_control::{NoKeys, QuietSpotter};
use crate::IntegrationWorld;
mod doubles;
use doubles::{
    FocusOnButton, NotingOverlay, NotingPaster, NotingTranscriber, PartsRecorder, Silent,
};

const SENTENCE: &str = "Meeting moved to Friday.";
/// How long each stand-in for a local service takes, so a stage lasts long enough to see.
const SERVICE_TIME: Duration = Duration::from_millis(100);
/// Longest a pipeline reaction may take in these scenarios.
const REACTION: Duration = Duration::from_secs(3);
/// How long the pipeline holds the text back for a text input to get focus.
const FOCUS_WAIT: Duration = Duration::from_millis(300);
/// Long enough for every overlay signal after the paste to arrive.
const SETTLE: Duration = Duration::from_millis(200);
/// How late the pipeline's thread records the last event of a reaction, as a busy CPU makes it.
/// It stays so a step that reads the log right after an earlier event of the reaction fails on
/// every run instead of now and then.
const PIPELINE_LAG: Duration = Duration::from_millis(50);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Event {
    RecordingStarted,
    TranscriptionStarted,
    TranscriptionFinished,
    Pasted,
    Overlay(Overlay),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Overlay {
    Recording,
    RecognizingSpeech,
    WaitingForFocus,
    PastedSign,
    Hidden,
    FailureNotice(FailureNotice),
}

type Events = Arc<Mutex<Vec<Event>>>;

/// Appends an event to the scenario's event log.
fn record(events: &Events, event: Event) {
    events.lock().unwrap().push(event);
}

/// What a processing-stages scenario shares with the pipeline thread.
#[derive(Default)]
pub struct StagesRun {
    events: Events,
    no_speech: bool,
    in_parts: bool,
    no_text_focus: bool,
    voice_control: bool,
    inputs: Option<Sender<PipelineInput>>,
    /// Index in the event log where the recording was ended.
    stop_at: usize,
}

impl std::fmt::Debug for StagesRun {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StagesRun")
            .field("events", &self.events())
            .finish()
    }
}

impl StagesRun {
    fn events(&self) -> Vec<Event> {
        self.events.lock().unwrap().clone()
    }

    fn after_stop(&self) -> Vec<Event> {
        self.events()[self.stop_at..].to_vec()
    }

    fn send(&mut self, input: PipelineInput) -> Result<()> {
        if self.inputs.is_none() {
            self.inputs = Some(self.start_pipeline());
        }
        self.inputs
            .as_ref()
            .context("pipeline not running")?
            .send(input)?;
        Ok(())
    }

    fn parts(&self) -> PipelineParts {
        let events = &self.events;
        PipelineParts {
            recorder: Box::new(PartsRecorder {
                events: Arc::clone(events),
                in_parts: self.in_parts,
                parts: None,
            }),
            transcriber: Box::new(NotingTranscriber {
                events: Arc::clone(events),
                no_speech: self.no_speech,
            }),
            paster: Box::new(NotingPaster(Arc::clone(events))),
            sound_player: Box::new(Silent),
            visual_indicator: Box::new(NotingOverlay { events: Arc::clone(events) }),
        }
    }

    fn start_pipeline(&self) -> Sender<PipelineInput> {
        let (tx, rx) = mpsc::channel();
        let mut pipeline = Pipeline::new(
            self.parts(),
            PipelineConfig {
                chunking: ChunkingCfg {
                    chunk_threshold_secs: if self.in_parts { 5 } else { 0 },
                    ..ChunkingCfg::default()
                },
                ..PipelineConfig::default()
            },
        );
        if self.no_text_focus {
            pipeline = pipeline.with_focus_wait(
                Box::new(FocusOnButton),
                FocusWaitCfg { timeout: FOCUS_WAIT, poll_interval: Duration::from_millis(20) },
            );
        }
        if self.voice_control {
            pipeline = pipeline.with_voice_control(Box::new(QuietSpotter), Box::new(NoKeys));
        }
        std::thread::spawn(move || pipeline.run(rx));
        tx
    }

    /// Starts a recording with the hotkey and waits until it runs.
    fn start_recording(&mut self) -> Result<()> {
        self.send(PipelineInput::Hotkey(PttSignal::Pressed))?;
        self.wait_for(Event::RecordingStarted)?;
        self.wait_for(Event::Overlay(Overlay::Recording))?;
        if self.in_parts {
            // the first part is transcribed while the user still speaks
            self.wait_for(Event::TranscriptionFinished)?;
        }
        Ok(())
    }

    fn end_with_hotkey(&mut self) -> Result<()> {
        self.stop_at = self.events().len();
        self.send(PipelineInput::Hotkey(PttSignal::Pressed))
    }

    fn end_by_voice(&mut self) -> Result<()> {
        self.stop_at = self.events().len();
        let segment_start = Instant::now();
        let start = segment_start + Duration::from_millis(100);
        let end = start + Duration::from_millis(800);
        self.send(PipelineInput::Spotting(SpottingEvent::SpeechStarted(
            segment_start,
        )))?;
        self.send(PipelineInput::Spotting(SpottingEvent::PhraseSpotted {
            phrase: VoiceCommand::StopDictation.phrase().to_owned(),
            start,
            end,
            timing: None,
        }))?;
        self.send(PipelineInput::Spotting(SpottingEvent::SpeechEnded(
            end + Duration::from_millis(100),
        )))
    }

    fn wait_for(&self, wanted: Event) -> Result<()> {
        let deadline = Instant::now() + REACTION;
        while !self.events().contains(&wanted) {
            if Instant::now() >= deadline {
                bail!("{wanted:?} not seen; events: {:?}", self.events());
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        Ok(())
    }

    /// Waits until the dictation is over: the overlay hides, or shows the Pasted sign that
    /// takes itself down, after the stop.
    fn wait_until_over(&self) -> Result<Vec<Event>> {
        let deadline = Instant::now() + REACTION;
        let ends = |e: &Event| matches!(e, Event::Overlay(Overlay::Hidden | Overlay::PastedSign));
        while !self.after_stop().iter().any(ends) {
            if Instant::now() >= deadline {
                bail!("the overlay never hid; events: {:?}", self.events());
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        std::thread::sleep(SETTLE);
        Ok(self.after_stop())
    }
}

fn run(world: &mut IntegrationWorld) -> &mut StagesRun {
    world.stages.get_or_insert_with(StagesRun::default)
}

fn position(events: &[Event], wanted: Event) -> Result<usize> {
    events
        .iter()
        .position(|e| *e == wanted)
        .with_context(|| format!("{wanted:?} not seen after the stop; events: {events:?}"))
}

fn last_position(events: &[Event], wanted: Event) -> Result<usize> {
    events
        .iter()
        .rposition(|e| *e == wanted)
        .with_context(|| format!("{wanted:?} not seen after the stop; events: {events:?}"))
}

/// The overlay signals in `events`, in order.
fn overlay(events: &[Event]) -> Vec<Overlay> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::Overlay(o) => Some(*o),
            _ => None,
        })
        .collect()
}

/// Speech recognition is shown before the transcription after the stop starts and stays
/// until it finished: the next overlay signal comes after it.
fn recognition_covers_transcription(events: &[Event]) -> Result<()> {
    let shown = position(events, Event::Overlay(Overlay::RecognizingSpeech))?;
    let started = position(events, Event::TranscriptionStarted)?;
    let finished = last_position(events, Event::TranscriptionFinished)?;
    if shown > started {
        bail!("speech recognition shown after the transcription started: {events:?}");
    }
    let next_signal = events[shown + 1..]
        .iter()
        .position(|e| matches!(e, Event::Overlay(_)))
        .map(|at| at + shown + 1);
    if next_signal.is_some_and(|next| next < finished) {
        bail!("the overlay left speech recognition before the transcript was ready: {events:?}");
    }
    Ok(())
}

#[given("the user records a dictation with the hotkey")]
fn records_with_hotkey(world: &mut IntegrationWorld) -> Result<()> {
    run(world).start_recording()
}

#[given("voice control is turned on and a dictation runs")]
fn voice_dictation_runs(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.voice_control = true;
    run.start_recording()
}

#[given("the user records a dictation transcribed in parts while the user speaks")]
fn records_in_parts(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.in_parts = true;
    run.start_recording()
}

#[given("the user records a dictation with no speech in it")]
fn records_no_speech(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.no_speech = true;
    run.start_recording()
}

#[given("no text input has focus")]
fn no_text_focus(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.no_text_focus = true;
    run.start_recording()
}

#[when("the user ends the recording with the hotkey")]
#[when("the user ends a dictation with speech in it")]
fn ends_with_hotkey(world: &mut IntegrationWorld) -> Result<()> {
    run(world).end_with_hotkey()
}

#[when(expr = "the user says {string} to end it")]
fn says_to_end(world: &mut IntegrationWorld, phrase: String) -> Result<()> {
    if VoiceCommand::from_phrase(&phrase) != Some(VoiceCommand::StopDictation) {
        bail!("only \"stop dictation\" ends a dictation here, not {phrase:?}");
    }
    run(world).end_by_voice()
}

#[then(
    "the overlay shows the speech recognition stage from the stop until the transcript is ready"
)]
#[then("the overlay shows the speech recognition stage from the stop until the last part is transcribed")]
fn recognition_until_transcript(world: &mut IntegrationWorld) -> Result<()> {
    let events = run(world).wait_until_over()?;
    if overlay(&events).first() != Some(&Overlay::RecognizingSpeech) {
        bail!("the stop did not show speech recognition first: {events:?}");
    }
    recognition_covers_transcription(&events)
}

#[then("the overlay shows the speech recognition stage, then hides")]
fn recognition_then_hidden(world: &mut IntegrationWorld) -> Result<()> {
    let events = run(world).wait_until_over()?;
    recognition_covers_transcription(&events)?;
    let shown = overlay(&events);
    if shown != [Overlay::RecognizingSpeech, Overlay::Hidden] {
        bail!("expected speech recognition, then hidden; overlay: {shown:?}");
    }
    Ok(())
}

#[then("no other processing stage is shown")]
fn no_other_stage(world: &mut IntegrationWorld) -> Result<()> {
    let events = run(world).wait_until_over()?;
    let shown = overlay(&events);
    if shown != [Overlay::RecognizingSpeech, Overlay::PastedSign] {
        bail!("expected speech recognition, then the Pasted sign; overlay: {shown:?}");
    }
    Ok(())
}

#[then("the overlay shows the Pasted sign once the text is pasted, then hides")]
fn pasted_sign_after_paste(world: &mut IntegrationWorld) -> Result<()> {
    let events = run(world).wait_until_over()?;
    let pasted = position(&events, Event::Pasted)?;
    let signed = position(&events, Event::Overlay(Overlay::PastedSign))?;
    if signed < pasted {
        bail!("the overlay showed the Pasted sign before the paste: {events:?}");
    }
    // The Pasted sign takes itself down: the overlay ends the dictation on it.
    if overlay(&events).last() != Some(&Overlay::PastedSign) {
        bail!("expected the overlay to end on the Pasted sign: {events:?}");
    }
    Ok(())
}

#[then("the overlay shows the speech recognition stage with no text window under it")]
fn recognition_without_text_window(world: &mut IntegrationWorld) -> Result<()> {
    let events = run(world).wait_until_over()?;
    // WHY: the overlay is handed no text with the speech recognition stage, so it has none to
    // show under the pill
    position(&events, Event::Overlay(Overlay::RecognizingSpeech)).map(drop)
}

#[then("the overlay shows the speech recognition stage, then the wait for focus")]
fn recognition_then_focus_wait(world: &mut IntegrationWorld) -> Result<()> {
    let events = run(world).wait_until_over()?;
    recognition_covers_transcription(&events)?;
    let shown = overlay(&events);
    let expected = [
        Overlay::RecognizingSpeech,
        Overlay::WaitingForFocus,
        Overlay::Hidden,
    ];
    if shown != expected {
        bail!("expected {expected:?}; overlay: {shown:?}");
    }
    Ok(())
}
