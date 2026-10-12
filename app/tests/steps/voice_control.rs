//! Drives the real pipeline on its own thread with voice control: a scenario speaks by
//! sending the speech and phrase events the command spotting service would report, and
//! watches the ordered log of what the stand-ins in `doubles` were asked to do.

mod doubles;

use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use cucumber::{given, then, when};

use voicepuncher::adapters::secondary::textrefinement::passthrough::PassthroughTextRefiner;
use voicepuncher::core::application::pipeline::{Pipeline, PipelineConfig};
use voicepuncher::core::application::{ChunkingCfg, PipelineInput, VisualIndicator};
use voicepuncher::core::domain::{PttSignal, RecordingMode, SpottingEvent};

use doubles::{
    Cut, ErrorLog, Event, Events, NotingRecorder, RecordingKeyPresser, RecordingPaster,
    RecordingSounds, ScriptedSpotter, Speech, SpeechBeforeCut, TRANSCRIPTION_TIME,
};

use crate::IntegrationWorld;

const SENTENCE: &str = "Meeting moved to Friday.";
const STOP_PHRASE: &str = "stop dictation";
/// Longest a pipeline reaction may take in these scenarios.
const REACTION: Duration = Duration::from_secs(2);
/// How long a scenario watches for something that must not happen.
const SETTLE: Duration = Duration::from_millis(300);
/// Where a phrase spoken on its own sits in its speech segment.
const PHRASE_LEAD: Duration = Duration::from_millis(100);
const PHRASE_LENGTH: Duration = Duration::from_millis(800);
const SEGMENT_TAIL: Duration = Duration::from_millis(100);
/// How much speech surrounds a phrase said inside a sentence.
const SENTENCE_AROUND_PHRASE: Duration = Duration::from_secs(1);

static ERRORS: OnceLock<&'static ErrorLog> = OnceLock::new();

fn error_log() -> &'static ErrorLog {
    ERRORS.get_or_init(|| {
        let errors: &'static ErrorLog = Box::leak(Box::new(ErrorLog(Mutex::default())));
        if log::set_logger(errors).is_ok() {
            log::set_max_level(log::LevelFilter::Error);
        }
        errors
    })
}

/// What a voice-control scenario shares with the pipeline thread.
#[derive(Default)]
pub struct VoiceRun {
    voice_control: bool,
    events: Events,
    speech: Speech,
    cut: Cut,
    inputs: Option<Sender<PipelineInput>>,
    /// Index in the event log where the scenario's `When` started.
    mark: usize,
}

impl std::fmt::Debug for VoiceRun {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VoiceRun")
            .field("events", &self.events())
            .finish()
    }
}

impl VoiceRun {
    fn events(&self) -> Vec<Event> {
        self.events.lock().unwrap().clone()
    }

    fn since_mark(&self) -> Vec<Event> {
        self.events()[self.mark..].to_vec()
    }

    fn start_when(&mut self) {
        self.mark = self.events.lock().unwrap().len();
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

    /// Starts the pipeline unless it already runs.
    fn ensure_started(&mut self) {
        if self.inputs.is_none() {
            self.inputs = Some(self.start_pipeline());
        }
    }

    fn press_hotkey(&mut self) -> Result<()> {
        self.send(PipelineInput::Hotkey(PttSignal::Pressed))
    }

    fn start_pipeline(&self) -> Sender<PipelineInput> {
        error_log();
        let (tx, rx) = mpsc::channel();
        let mut pipeline = Pipeline::new(
            Box::new(NotingRecorder {
                events: Arc::clone(&self.events),
                speech: Arc::clone(&self.speech),
                cut: Arc::clone(&self.cut),
            }),
            Box::new(SpeechBeforeCut {
                speech: Arc::clone(&self.speech),
                cut: Arc::clone(&self.cut),
            }),
            Box::new(PassthroughTextRefiner),
            Box::new(RecordingPaster(Arc::clone(&self.events))),
            Box::new(RecordingSounds(Arc::clone(&self.events))),
            Box::new(NoOverlay),
            PipelineConfig {
                min_press_ms: 0,
                recording_mode: RecordingMode::Toggle,
                chunking: ChunkingCfg {
                    chunk_threshold_secs: 0,
                    ..ChunkingCfg::default()
                },
            },
        );
        if self.voice_control {
            pipeline = pipeline.with_voice_control(
                Box::new(ScriptedSpotter {
                    events: Arc::clone(&self.events),
                    phrases: Arc::default(),
                }),
                Box::new(RecordingKeyPresser(Arc::clone(&self.events))),
            );
        }
        std::thread::spawn(move || pipeline.run(rx));
        tx
    }

    /// Says `words` as a whole utterance, with a pause before and after.
    fn say_alone(&mut self, words: &str) -> Result<()> {
        let segment_start = Instant::now();
        let phrase_start = segment_start + PHRASE_LEAD;
        let phrase_end = phrase_start + PHRASE_LENGTH;
        self.speak(words, phrase_start);
        self.send_segment(
            words,
            segment_start,
            phrase_start,
            phrase_end,
            phrase_end + SEGMENT_TAIL,
        )
    }

    /// Says `sentence`, which holds a command phrase, without pausing around the phrase.
    fn say_in_sentence(&mut self, sentence: &str, phrase: &str) -> Result<()> {
        let segment_start = Instant::now();
        let phrase_start = segment_start + SENTENCE_AROUND_PHRASE;
        let phrase_end = phrase_start + PHRASE_LENGTH;
        self.speak(sentence, segment_start);
        let segment_end = phrase_end + SENTENCE_AROUND_PHRASE;
        self.send_segment(phrase, segment_start, phrase_start, phrase_end, segment_end)
    }

    fn speak(&self, words: &str, at: Instant) {
        self.speech.lock().unwrap().push((at, words.to_owned()));
    }

    fn send_segment(
        &mut self,
        phrase: &str,
        segment_start: Instant,
        start: Instant,
        end: Instant,
        segment_end: Instant,
    ) -> Result<()> {
        self.send(PipelineInput::Spotting(SpottingEvent::SpeechStarted(
            segment_start,
        )))?;
        let phrase = phrase.to_owned();
        self.send(PipelineInput::Spotting(SpottingEvent::PhraseSpotted {
            phrase,
            start,
            end,
        }))?;
        self.send(PipelineInput::Spotting(SpottingEvent::SpeechEnded(
            segment_end,
        )))
    }

    fn wait_for(&self, wanted: impl Fn(&Event) -> bool) -> Result<usize> {
        let deadline = Instant::now() + REACTION;
        loop {
            if let Some(at) = self.events().iter().position(&wanted) {
                return Ok(at);
            }
            if Instant::now() >= deadline {
                bail!("not seen within {REACTION:?}; events: {:?}", self.events());
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    /// Waits until `wanted` happens after the scenario's `When` started.
    fn wait_since_mark(&self, wanted: impl Fn(&Event) -> bool) -> Result<()> {
        let deadline = Instant::now() + REACTION;
        while !self.since_mark().iter().any(&wanted) {
            if Instant::now() >= deadline {
                bail!(
                    "not seen after the action within {REACTION:?}; events: {:?}",
                    self.events()
                );
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        Ok(())
    }

    fn never_since_mark(&self, unwanted: impl Fn(&Event) -> bool, what: &str) -> Result<()> {
        std::thread::sleep(SETTLE);
        if let Some(event) = self.since_mark().into_iter().find(unwanted) {
            bail!("{what}: {event:?}; events: {:?}", self.events());
        }
        Ok(())
    }

    fn listening(&self) -> bool {
        self.events()
            .iter()
            .rev()
            .find(|e| matches!(e, Event::ListeningStarted | Event::ListeningStopped))
            == Some(&Event::ListeningStarted)
    }

    fn pasted(&self) -> Result<String> {
        let at = self.wait_for(|e| matches!(e, Event::Pasted(_)))?;
        match &self.events()[at] {
            Event::Pasted(text) => Ok(text.clone()),
            other => bail!("expected a paste, got {other:?}"),
        }
    }
}

struct NoOverlay;

impl VisualIndicator for NoOverlay {
    fn show_recording(&self) {}
    fn show_processing(&self) {}
    fn show_waiting_for_focus(&self) {}
    fn hide(&self) {}
}

fn run(world: &mut IntegrationWorld) -> &mut VoiceRun {
    world.voice.get_or_insert_with(VoiceRun::default)
}

fn is_key_press(e: &Event) -> bool {
    matches!(e, Event::ReturnPressed | Event::AllTextCleared)
}

#[given("voice control is not turned on")]
fn voice_control_off(world: &mut IntegrationWorld) {
    run(world).voice_control = false;
}

#[given("voice control is turned on")]
fn voice_control_on(world: &mut IntegrationWorld) {
    run(world).voice_control = true;
}

#[given("listening is off")]
fn listening_off_given(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.ensure_started();
    run.wait_for(|e| *e == Event::ListeningStarted)?;
    run.press_hotkey()?;
    run.wait_for(|e| *e == Event::ListeningStopped)?;
    Ok(())
}

#[given("no dictation runs")]
fn no_dictation_given(_world: &mut IntegrationWorld) {}

#[given("listening is on")]
fn listening_on_given(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.voice_control = true;
    run.ensure_started();
    run.wait_for(|e| *e == Event::ListeningStarted)?;
    Ok(())
}

#[given("a dictation runs")]
fn dictation_runs(world: &mut IntegrationWorld) -> Result<()> {
    listening_on_given(world)?;
    let run = run(world);
    run.say_alone("start dictation")?;
    run.wait_for(|e| *e == Event::RecordingStarted)?;
    Ok(())
}

#[given("the user has said a sentence in it")]
fn sentence_said(world: &mut IntegrationWorld) {
    run(world).speak(SENTENCE, Instant::now());
}

#[given(expr = "the user has said {string}")]
fn phrase_said(world: &mut IntegrationWorld, phrase: String) -> Result<()> {
    let run = run(world);
    run.speak(SENTENCE, Instant::now());
    run.say_alone(&phrase)
}

#[when("the app starts")]
fn app_starts(world: &mut IntegrationWorld) {
    let run = run(world);
    run.start_when();
    run.inputs = Some(run.start_pipeline());
}

#[when("the user presses the hotkey once")]
fn press_hotkey_once(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.start_when();
    run.press_hotkey()
}

#[when(expr = "the user says {string}")]
#[when(expr = "the user says {string} before the text is pasted")]
fn says(world: &mut IntegrationWorld, phrase: String) -> Result<()> {
    let run = run(world);
    run.start_when();
    run.say_alone(&phrase)
}

#[when(expr = "the user says {string} without pausing")]
fn says_in_sentence(world: &mut IntegrationWorld, sentence: String) -> Result<()> {
    let run = run(world);
    run.start_when();
    run.say_in_sentence(&sentence, STOP_PHRASE)
}

#[when("the command spotting service stops answering")]
fn service_stops(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.start_when();
    run.send(PipelineInput::SpottingLost)
}

#[then("a dictation starts")]
fn dictation_starts(world: &mut IntegrationWorld) -> Result<()> {
    run(world).wait_for(|e| *e == Event::RecordingStarted)?;
    Ok(())
}

#[then("no dictation starts")]
fn no_dictation_starts(world: &mut IntegrationWorld) -> Result<()> {
    run(world).never_since_mark(|e| *e == Event::RecordingStarted, "a dictation started")
}

#[then("listening is on")]
#[then("listening stays on")]
fn listening_on(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.wait_for(|e| *e == Event::ListeningStarted)?;
    std::thread::sleep(SETTLE);
    if !run.listening() {
        bail!("listening is off; events: {:?}", run.events());
    }
    Ok(())
}

#[then("listening is off")]
fn listening_off(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    std::thread::sleep(SETTLE + TRANSCRIPTION_TIME);
    if run.listening() {
        bail!("listening is on; events: {:?}", run.events());
    }
    Ok(())
}

#[then("spoken commands have no effect")]
fn commands_have_no_effect(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.start_when();
    run.say_alone("press enter")?;
    run.say_alone("start dictation")?;
    let effect = |e: &Event| is_key_press(e) || *e == Event::RecordingStarted;
    run.never_since_mark(effect, "a spoken command took effect")
}

#[then(expr = "the {word} sound plays")]
fn sound_plays(world: &mut IntegrationWorld, sound: String) -> Result<()> {
    let name = match sound.as_str() {
        "listening-on" => "listening on",
        "listening-off" => "listening off",
        "start" => "start",
        "cancel" => "cancel",
        "command" => "command",
        other => bail!("unknown sound {other}"),
    };
    run(world).wait_since_mark(|e| *e == Event::Sound(name))
}

#[then("no sound plays")]
fn no_sound(world: &mut IntegrationWorld) -> Result<()> {
    run(world).never_since_mark(|e| matches!(e, Event::Sound(_)), "a sound played")
}

#[then("the sentence is pasted into the focused app")]
fn sentence_pasted(world: &mut IntegrationWorld) -> Result<()> {
    let pasted = run(world).pasted()?;
    if !pasted.contains(SENTENCE) {
        bail!("pasted {pasted:?}, expected the sentence {SENTENCE:?}");
    }
    Ok(())
}

#[then("the pasted text does not contain the stop phrase")]
fn no_stop_phrase(world: &mut IntegrationWorld) -> Result<()> {
    let pasted = run(world).pasted()?;
    if pasted.contains(STOP_PHRASE) {
        bail!("the stop phrase reached the pasted text: {pasted:?}");
    }
    Ok(())
}

#[then("no text is pasted")]
fn no_text_pasted(world: &mut IntegrationWorld) -> Result<()> {
    run(world).never_since_mark(|e| matches!(e, Event::Pasted(_)), "text was pasted")
}

#[then("the Return key is pressed in the focused app")]
fn return_pressed(world: &mut IntegrationWorld) -> Result<()> {
    run(world).wait_for(|e| *e == Event::ReturnPressed)?;
    Ok(())
}

#[then("select all and delete are pressed in the focused app")]
fn all_text_cleared(world: &mut IntegrationWorld) -> Result<()> {
    run(world).wait_for(|e| *e == Event::AllTextCleared)?;
    Ok(())
}

#[then("the dictation keeps running")]
fn dictation_keeps_running(world: &mut IntegrationWorld) -> Result<()> {
    let ended = |e: &Event| {
        matches!(
            e,
            Event::Pasted(_) | Event::RecordingCancelled | Event::Sound("stop")
        )
    };
    run(world).never_since_mark(ended, "the dictation ended")
}

#[then("no key is pressed")]
fn no_key_pressed(world: &mut IntegrationWorld) -> Result<()> {
    run(world).never_since_mark(is_key_press, "a key was pressed")
}

#[then("the text is pasted first")]
fn text_pasted_first(world: &mut IntegrationWorld) -> Result<()> {
    run(world).pasted()?;
    Ok(())
}

#[then("the Return key is pressed after it")]
fn return_after_paste(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    let pasted_at = run.wait_for(|e| matches!(e, Event::Pasted(_)))?;
    let return_at = run.wait_for(|e| *e == Event::ReturnPressed)?;
    if return_at < pasted_at {
        bail!("Return was pressed before the paste: {:?}", run.events());
    }
    Ok(())
}

#[then("an error naming the command spotting service is logged")]
fn error_logged(_world: &mut IntegrationWorld) -> Result<()> {
    let deadline = Instant::now() + REACTION;
    loop {
        let errors = error_log().0.lock().unwrap().clone();
        if errors
            .iter()
            .any(|e| e.contains("command spotting service"))
        {
            return Ok(());
        }
        if Instant::now() >= deadline {
            bail!("no error naming the command spotting service; errors: {errors:?}");
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}
