//! Drives the real pipeline on its own thread and watches, in one ordered log, which
//! processing stage the overlay shows while speech recognition and text refinement run.

use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use cucumber::{given, then, when};

use voicepuncher::adapters::secondary::textrefinement::passthrough::PassthroughTextRefiner;
use voicepuncher::core::application::audio_recorder::LiveChunk;
use voicepuncher::core::application::pipeline::{Pipeline, PipelineConfig};
use voicepuncher::core::application::{
    AudioRecorder, ChunkingCfg, CommandSpotter, FocusProbe, FocusWaitCfg, KeyPresser,
    PipelineInput, SoundPlayer, TextPaster, TextRefiner, Transcriber, VisualIndicator,
};
use voicepuncher::core::domain::{
    AudioBuffer16k, CleanedText, FailureNotice, FocusedElement, PttSignal, RawTranscript,
    RecordingMode, SpottingEvent, VoiceCommand,
};

use crate::IntegrationWorld;

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
    RefinementStarted,
    RefinementFinished,
    Pasted,
    Overlay(Overlay),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Overlay {
    Recording,
    RecognizingSpeech,
    RefiningText,
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
    refinement_off: bool,
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

    fn start_pipeline(&self) -> Sender<PipelineInput> {
        let (tx, rx) = mpsc::channel();
        let events = &self.events;
        let refiner: Box<dyn TextRefiner> = if self.refinement_off {
            Box::new(PassthroughTextRefiner)
        } else {
            Box::new(NotingRefiner(Arc::clone(events)))
        };
        let mut pipeline = Pipeline::new(
            Box::new(PartsRecorder {
                events: Arc::clone(events),
                in_parts: self.in_parts,
                parts: None,
            }),
            Box::new(NotingTranscriber {
                events: Arc::clone(events),
                no_speech: self.no_speech,
            }),
            refiner,
            Box::new(NotingPaster(Arc::clone(events))),
            Box::new(Silent),
            Box::new(NotingOverlay {
                events: Arc::clone(events),
            }),
            PipelineConfig {
                min_press_ms: 0,
                recording_mode: RecordingMode::Toggle,
                chunking: ChunkingCfg {
                    chunk_threshold_secs: if self.in_parts { 5 } else { 0 },
                    ..ChunkingCfg::default()
                },
            },
        );
        if self.refinement_off {
            pipeline = pipeline.without_text_refinement_stage();
        }
        if self.no_text_focus {
            pipeline = pipeline.with_focus_wait(
                Box::new(FocusOnButton),
                FocusWaitCfg {
                    timeout: FOCUS_WAIT,
                    poll_interval: Duration::from_millis(20),
                },
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
        let ends = |e: &Event| {
            matches!(
                e,
                Event::Overlay(Overlay::Hidden) | Event::Overlay(Overlay::PastedSign)
            )
        };
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
#[given("text refinement is enabled")]
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

#[given("text refinement is disabled")]
fn refinement_disabled(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.refinement_off = true;
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

#[then("the text refinement stage is not shown")]
fn refinement_not_shown(world: &mut IntegrationWorld) -> Result<()> {
    let events = run(world).wait_until_over()?;
    if events.contains(&Event::Overlay(Overlay::RefiningText)) {
        bail!("text refinement was shown: {events:?}");
    }
    Ok(())
}

#[then(
    "the overlay shows the speech recognition stage, then the text refinement stage until the corrected text is ready"
)]
fn recognition_then_refinement(world: &mut IntegrationWorld) -> Result<()> {
    let events = run(world).wait_until_over()?;
    recognition_covers_transcription(&events)?;
    let transcribed = last_position(&events, Event::TranscriptionFinished)?;
    let shown = position(&events, Event::Overlay(Overlay::RefiningText))?;
    let started = position(&events, Event::RefinementStarted)?;
    let finished = position(&events, Event::RefinementFinished)?;
    if !(transcribed < shown && shown < started) {
        bail!("text refinement not shown between the transcript and its start: {events:?}");
    }
    let next_signal = events[shown + 1..]
        .iter()
        .position(|e| matches!(e, Event::Overlay(_)))
        .map(|at| at + shown + 1);
    if next_signal.is_some_and(|next| next < finished) {
        bail!("the overlay left text refinement before the text was ready: {events:?}");
    }
    let shown = overlay(&events);
    if shown[..2] != [Overlay::RecognizingSpeech, Overlay::RefiningText] {
        bail!("expected speech recognition, then text refinement; overlay: {shown:?}");
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

#[then("the overlay shows the text refinement stage with no text window under it")]
fn refinement_without_text_window(world: &mut IntegrationWorld) -> Result<()> {
    let events = run(world).wait_until_over()?;
    // WHY: the overlay is handed no text with the refinement stage, so it has none to show
    // under the pill
    position(&events, Event::Overlay(Overlay::RefiningText))?;
    Ok(())
}

#[then(
    "the overlay shows the speech recognition stage, then the text refinement stage, then the wait for focus"
)]
fn stages_then_focus_wait(world: &mut IntegrationWorld) -> Result<()> {
    let events = run(world).wait_until_over()?;
    let shown = overlay(&events);
    let expected = [
        Overlay::RecognizingSpeech,
        Overlay::RefiningText,
        Overlay::WaitingForFocus,
        Overlay::Hidden,
    ];
    if shown != expected {
        bail!("expected {expected:?}; overlay: {shown:?}");
    }
    let refined = position(&events, Event::RefinementFinished)?;
    let waiting = position(&events, Event::Overlay(Overlay::WaitingForFocus))?;
    if waiting < refined {
        bail!("the wait for focus was shown before the text was refined: {events:?}");
    }
    Ok(())
}

/// Hands over one second of audio at the stop; recorded in parts, it sends one part while
/// the user speaks and the last one when the recording is flushed.
struct PartsRecorder {
    events: Events,
    in_parts: bool,
    parts: Option<mpsc::Sender<LiveChunk>>,
}

impl PartsRecorder {
    fn one_second() -> AudioBuffer16k {
        AudioBuffer16k::new(vec![1_000i16; AudioBuffer16k::SAMPLE_RATE_HZ as usize])
    }

    fn send_part(&self) {
        if let Some(parts) = &self.parts {
            let _ = parts.send(LiveChunk {
                audio: Self::one_second(),
                enqueued_at: Instant::now(),
            });
        }
    }
}

impl AudioRecorder for PartsRecorder {
    fn start(&mut self) -> Result<()> {
        record(&self.events, Event::RecordingStarted);
        Ok(())
    }

    fn stop(&mut self) -> Result<AudioBuffer16k> {
        Ok(Self::one_second())
    }

    fn start_live_chunks(&mut self, _cfg: ChunkingCfg) -> Result<Option<Receiver<LiveChunk>>> {
        if !self.in_parts {
            return Ok(None);
        }
        let (tx, rx) = mpsc::channel();
        self.parts = Some(tx);
        self.send_part();
        Ok(Some(rx))
    }

    fn flush_live_chunks(&mut self) {
        self.send_part();
        // the transcription of the parts ends with the last one
        self.parts = None;
    }
}

struct NotingTranscriber {
    events: Events,
    no_speech: bool,
}

impl Transcriber for NotingTranscriber {
    fn transcribe(&mut self, _audio: &AudioBuffer16k) -> Result<Option<RawTranscript>> {
        record(&self.events, Event::TranscriptionStarted);
        std::thread::sleep(SERVICE_TIME);
        record(&self.events, Event::TranscriptionFinished);
        if self.no_speech {
            return Ok(None);
        }
        Ok(Some(RawTranscript::new(SENTENCE.to_owned())?))
    }
}

struct NotingRefiner(Events);

impl TextRefiner for NotingRefiner {
    fn refine(&mut self, raw: &RawTranscript) -> Result<CleanedText> {
        record(&self.0, Event::RefinementStarted);
        std::thread::sleep(SERVICE_TIME);
        record(&self.0, Event::RefinementFinished);
        Ok(CleanedText::new(raw.as_str().to_owned())?)
    }
}

struct NotingPaster(Events);

impl TextPaster for NotingPaster {
    fn paste(&mut self, _text: &CleanedText) -> Result<()> {
        record(&self.0, Event::Pasted);
        Ok(())
    }

    fn leave_in_clipboard(&mut self, text: &CleanedText) -> Result<()> {
        self.paste(text)
    }
}

struct NotingOverlay {
    events: Events,
}

impl VisualIndicator for NotingOverlay {
    fn show_recording(&self) {
        std::thread::sleep(PIPELINE_LAG);
        record(&self.events, Event::Overlay(Overlay::Recording));
    }

    fn show_recognizing_speech(&self) {
        record(&self.events, Event::Overlay(Overlay::RecognizingSpeech));
    }

    fn show_refining_text(&self) {
        record(&self.events, Event::Overlay(Overlay::RefiningText));
    }

    fn show_pasted(&self) {
        record(&self.events, Event::Overlay(Overlay::PastedSign));
    }

    fn show_waiting_for_focus(&self) {
        record(&self.events, Event::Overlay(Overlay::WaitingForFocus));
    }

    fn hide(&self) {
        record(&self.events, Event::Overlay(Overlay::Hidden));
    }

    fn show_failure_notice(&self, notice: &FailureNotice) {
        record(
            &self.events,
            Event::Overlay(Overlay::FailureNotice(*notice)),
        );
    }
}

/// A button has focus: the text is held back until the wait runs out.
struct FocusOnButton;

impl FocusProbe for FocusOnButton {
    fn focused_element(&self) -> Result<Option<FocusedElement>> {
        Ok(Some(FocusedElement {
            role: "AXButton".to_owned(),
            has_text_selection: false,
            value_settable: false,
            in_web_content: false,
        }))
    }
}

/// The command spotting service as the pipeline sees it; the scenario feeds the phrase it
/// reports straight into the pipeline's input channel.
struct QuietSpotter;

impl CommandSpotter for QuietSpotter {
    fn probe(&self) -> Result<()> {
        Ok(())
    }

    fn start_listening(&mut self, _commands: &[VoiceCommand]) -> Result<()> {
        Ok(())
    }

    fn listen_for(&mut self, _commands: &[VoiceCommand]) {}

    fn stop_listening(&mut self) {}
}

struct NoKeys;

impl KeyPresser for NoKeys {
    fn press_enter(&mut self) -> Result<()> {
        Ok(())
    }

    fn clear_all_text(&mut self) -> Result<()> {
        Ok(())
    }

    fn paste(&mut self) -> Result<()> {
        Ok(())
    }
}

struct Silent;

impl SoundPlayer for Silent {
    fn play_start(&self) {}
    fn play_stop(&self) {}
    fn play_cancel(&self) {}
    fn play_listening_on(&self) {}
    fn play_listening_off(&self) {}
}
