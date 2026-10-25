//! What reaches the paste of the transcript speech recognition returned: the pipeline runs on
//! its own thread with the text-insertion stand-ins and a speech recognition stand-in that
//! answers the scenario's transcript.

use cucumber::{given, then, when};
use voicepuncher::test_doubles::prelude::*;

use super::doubles::{OneSecondRecorder, RecordingIndicator, RecordingPaster, Silent};
use super::{
    bail, mpsc, Arc, AudioBuffer16k, Context, Duration, Event, Instant, Mutex, Pipeline,
    PipelineConfig, PipelineInput, PipelineParts, PttSignal, RawTranscript, Result, Transcriber,
    DICTATED, REACTION,
};
use crate::IntegrationWorld;

/// The transcript speech recognition returns in a filler sounds scenario, and what was pasted.
#[derive(Debug, Default)]
pub struct FillerRun {
    transcript: String,
    pasted: Option<String>,
}

/// Speech recognition that returns the scenario's transcript.
struct Returns(String);

impl Transcriber for Returns {
    fn transcribe(&mut self, _audio: &AudioBuffer16k) -> Result<Option<RawTranscript>> {
        Ok(Some(RawTranscript::new(self.0.clone())?))
    }
}

fn filler(world: &mut IntegrationWorld) -> &mut FillerRun {
    world
        .extra
        .insertion
        .filler
        .get_or_insert_with(FillerRun::default)
}

fn wait_for(
    events: &Mutex<Vec<Event>>,
    wanted: impl Fn(&Event) -> Option<String>,
) -> Result<String> {
    let deadline = Instant::now() + REACTION;
    loop {
        let found = events.lock().unwrap().iter().find_map(&wanted);
        if let Some(found) = found {
            return Ok(found);
        }
        if Instant::now() >= deadline {
            bail!(
                "not seen within {REACTION:?}; events: {:?}",
                events.lock().unwrap()
            );
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// Dictates once through a pipeline whose speech recognition returns `transcript`; what was
/// pasted.
fn dictate(transcript: &str) -> Result<String> {
    let events: Arc<Mutex<Vec<Event>>> = Arc::default();
    let mut pipeline = Pipeline::new(
        PipelineParts {
            recorder: Box::new(OneSecondRecorder::new(&events, Event::RecordingStarted)),
            transcriber: Box::new(Returns(transcript.to_owned())),
            paster: Box::new(RecordingPaster(Arc::clone(&events), Arc::default())),
            sound_player: Box::new(Silent),
            visual_indicator: Box::new(RecordingIndicator(Arc::clone(&events), Arc::default())),
        },
        PipelineConfig::default(),
    );
    let (inputs, rx) = mpsc::channel();
    std::thread::spawn(move || pipeline.run(rx));
    inputs.send(PipelineInput::Hotkey(PttSignal::Pressed))?;
    wait_for(&events, |e| {
        (*e == Event::RecordingStarted).then(String::new)
    })?;
    inputs.send(PipelineInput::Hotkey(PttSignal::Pressed))?;
    wait_for(&events, |e| match e {
        Event::Pasted(text) => Some(text.clone()),
        _ => None,
    })
}

#[given(expr = "speech recognition returns {string}")]
fn returns_transcript(world: &mut IntegrationWorld, transcript: String) {
    filler(world).transcript = transcript;
}

#[given("speech recognition returns a transcript without filler sounds")]
fn returns_plain_transcript(world: &mut IntegrationWorld) {
    DICTATED.clone_into(&mut filler(world).transcript);
}

#[when("the user dictates and the dictation is pasted")]
fn ends_dictation(world: &mut IntegrationWorld) -> Result<()> {
    let run = filler(world);
    run.pasted = Some(dictate(&run.transcript)?);
    Ok(())
}

#[then(expr = "the pasted text is {string}")]
fn pasted_text_is(world: &mut IntegrationWorld, expected: String) -> Result<()> {
    let pasted = filler(world)
        .pasted
        .as_deref()
        .context("nothing was pasted")?;
    ensure!(pasted == expected, "pasted {pasted:?}");
    Ok(())
}

#[then("the pasted text equals the transcript")]
fn pasted_unchanged(world: &mut IntegrationWorld) -> Result<()> {
    let run = filler(world);
    let pasted = run.pasted.as_deref().context("nothing was pasted")?;
    ensure!(
        pasted == run.transcript,
        "pasted {pasted:?} of {:?}",
        run.transcript
    );
    Ok(())
}
