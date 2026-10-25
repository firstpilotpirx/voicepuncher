//! Drives the real pipeline in Hold mode on the stand-ins of the text insertion scenarios: a
//! press starts the recording, a release ends it, and the paster records what it pasted.

use voicepuncher::test_doubles::prelude::*;

use cucumber::{given, then, when};

use crate::steps::text_insertion::doubles::{
    FixedTranscript, OneSecondRecorder, RecordingIndicator, RecordingPaster, Silent,
};
use crate::steps::text_insertion::{Event, DICTATED};
use crate::IntegrationWorld;

/// Longest a pipeline reaction may take.
const REACTION: Duration = Duration::from_secs(1);
/// How long a step watches for something that must not happen.
const SETTLE: Duration = Duration::from_millis(300);
/// A shortest recording no press in these scenarios reaches.
const LONGER_THAN_ANY_PRESS_MS: u64 = 60_000;

/// What a push-to-talk scenario shares with the pipeline thread.
#[derive(Debug, Default)]
pub struct HoldRun {
    events: Arc<Mutex<Vec<Event>>>,
    transcribed: Arc<AtomicUsize>,
    signals: Option<Sender<PipelineInput>>,
    /// Whether a paste came while the hotkey was still held.
    pasted_while_held: bool,
}

/// The transcriber of the stand-ins, counting its calls.
struct CountingTranscript(Arc<AtomicUsize>);

impl Transcriber for CountingTranscript {
    fn transcribe(&mut self, audio: &AudioBuffer16k) -> Result<Option<RawTranscript>> {
        self.0.fetch_add(1, Ordering::SeqCst);
        FixedTranscript.transcribe(audio)
    }
}

impl HoldRun {
    fn start(&mut self, min_press_ms: u64) {
        let (tx, rx) = mpsc::channel();
        let events = &self.events;
        let parts = PipelineParts {
            recorder: Box::new(OneSecondRecorder::new(events, Event::RecordingStarted)),
            transcriber: Box::new(CountingTranscript(Arc::clone(&self.transcribed))),
            paster: Box::new(RecordingPaster(Arc::clone(events), Arc::default())),
            sound_player: Box::new(Silent),
            visual_indicator: Box::new(RecordingIndicator(Arc::clone(events), Arc::default())),
        };
        let config = PipelineConfig {
            recording_mode: RecordingMode::Hold,
            min_press_ms,
            ..Default::default()
        };
        let mut pipeline = Pipeline::new(parts, config);
        std::thread::spawn(move || pipeline.run(rx));
        self.signals = Some(tx);
    }

    fn send(&self, signal: PttSignal) -> Result<()> {
        let signals = self
            .signals
            .as_ref()
            .context("the pipeline is not running")?;
        Ok(signals.send(PipelineInput::Hotkey(signal))?)
    }

    fn events(&self) -> Vec<Event> {
        self.events.lock().unwrap().clone()
    }

    fn wait_for(&self, wanted: &Event) -> Result<()> {
        let deadline = Instant::now() + REACTION;
        while !self.events().contains(wanted) {
            ensure!(
                Instant::now() < deadline,
                "no {wanted:?}: {:?}",
                self.events()
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        Ok(())
    }
}

fn run(world: &mut IntegrationWorld) -> &mut HoldRun {
    world.extra.ptt.hold.get_or_insert_with(HoldRun::default)
}

#[given("the app is ready in Hold mode and a text field has keyboard focus")]
fn ready_in_hold_mode(world: &mut IntegrationWorld) {
    run(world).start(0);
}

#[given("the app is ready in Hold mode with a shortest recording no press reaches")]
fn ready_with_long_minimum(world: &mut IntegrationWorld) {
    run(world).start(LONGER_THAN_ANY_PRESS_MS);
}

#[when("the user holds the hotkey, speaks a phrase and releases the hotkey")]
fn holds_and_releases(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.send(PttSignal::Pressed)?;
    run.wait_for(&Event::RecordingStarted)?;
    std::thread::sleep(SETTLE);
    run.pasted_while_held = run.events().iter().any(|e| matches!(e, Event::Pasted(_)));
    run.send(PttSignal::Released)
}

#[when("the user presses and releases the hotkey at once")]
fn presses_briefly(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.send(PttSignal::Pressed)?;
    run.send(PttSignal::Released)
}

#[then("a recording ran while the hotkey was held")]
fn recording_ran(world: &mut IntegrationWorld) -> Result<()> {
    run(world).wait_for(&Event::RecordingStarted)
}

#[then("the phrase is pasted into the text field after the release")]
fn pasted_after_release(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    ensure!(!run.pasted_while_held, "pasted while held");
    run.wait_for(&Event::Pasted(DICTATED.to_owned()))
}

#[then("nothing is transcribed or pasted")]
fn nothing_transcribed(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    std::thread::sleep(SETTLE);
    let transcribed = run.transcribed.load(Ordering::SeqCst);
    if transcribed > 0 || run.events().iter().any(|e| matches!(e, Event::Pasted(_))) {
        bail!("{transcribed} transcriptions: {:?}", run.events());
    }
    Ok(())
}
