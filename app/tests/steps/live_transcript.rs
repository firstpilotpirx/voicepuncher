//! Drives the real pipeline on its own thread and watches, in one ordered log, what the
//! overlay shows while the parts of a recording are transcribed during the dictation.

use std::collections::VecDeque;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use cucumber::{given, then, when};

use voicepuncher::core::application::audio_recorder::LiveChunk;
use voicepuncher::core::application::chunking::merge_chunk_texts;
use voicepuncher::core::application::pipeline::{Pipeline, PipelineConfig, PipelineParts};
use voicepuncher::core::application::{
    AudioRecorder, ChunkingCfg, LiveTranscriptView, PipelineInput, SoundPlayer, TextPaster,
    Transcriber, VisualIndicator,
};
use voicepuncher::core::domain::{
    AudioBuffer16k, CleanedText, FailureNotice, PttSignal, RawTranscript, SpottingEvent,
    VoiceCommand,
};

use crate::steps::quiet_voice_control::{NoKeys, QuietSpotter};
use crate::IntegrationWorld;
mod doubles;
use doubles::{CapturingPaster, NotingOverlay, PartsOnDemand, ScriptedTranscriber, Silent};

const FIRST_PART: &str = "we moved the weekly meeting to friday because";
const NEXT_PART: &str = "to friday because the big room was busy";
const LAST_PART: &str = "the room was busy all day";
/// Longest a pipeline reaction may take in these scenarios.
const REACTION: Duration = Duration::from_secs(3);
/// Long enough for a signal the pipeline should not send to have arrived.
const SETTLE: Duration = Duration::from_millis(300);
/// Chunk threshold of the scenarios; a part is handed over only when a step says so.
const CHUNK_THRESHOLD_SECS: u32 = 5;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Shown {
    Recording,
    LiveTranscript(String),
    RecognizingSpeech,
    Other,
    PastedSign,
    Hidden,
}

type Log = Arc<Mutex<Vec<Shown>>>;
type Parts = Arc<Mutex<Option<Sender<LiveChunk>>>>;

/// What a live-transcript scenario shares with the pipeline thread.
#[derive(Default)]
pub struct LiveRun {
    shown: Log,
    parts: Parts,
    answers: Arc<Mutex<VecDeque<Option<&'static str>>>>,
    transcribed: Arc<Mutex<usize>>,
    pasted: Arc<Mutex<Option<String>>>,
    inputs: Option<Sender<PipelineInput>>,
    /// Index in the overlay log where the recording was ended.
    stop_at: usize,
}

impl std::fmt::Debug for LiveRun {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LiveRun")
            .field("shown", &self.shown())
            .finish()
    }
}

impl LiveRun {
    fn shown(&self) -> Vec<Shown> {
        self.shown.lock().unwrap().clone()
    }

    fn live_transcripts(&self) -> Vec<String> {
        live_transcripts(&self.shown())
    }

    fn start_pipeline(&mut self, voice_control: bool) {
        let (tx, rx) = mpsc::channel();
        let mut pipeline = Pipeline::new(
            PipelineParts {
                recorder: Box::new(PartsOnDemand(Arc::clone(&self.parts))),
                transcriber: Box::new(ScriptedTranscriber {
                    answers: Arc::clone(&self.answers),
                    transcribed: Arc::clone(&self.transcribed),
                }),
                paster: Box::new(CapturingPaster(Arc::clone(&self.pasted))),
                sound_player: Box::new(Silent),
                visual_indicator: Box::new(NotingOverlay(Arc::clone(&self.shown))),
            },
            PipelineConfig {
                chunking: ChunkingCfg {
                    chunk_threshold_secs: CHUNK_THRESHOLD_SECS,
                    ..ChunkingCfg::default()
                },
                ..PipelineConfig::default()
            },
        );
        if voice_control {
            pipeline = pipeline.with_voice_control(Box::new(QuietSpotter), Box::new(NoKeys));
        }
        std::thread::spawn(move || pipeline.run(rx));
        self.inputs = Some(tx);
    }

    fn send(&self, input: PipelineInput) -> Result<()> {
        self.inputs
            .as_ref()
            .context("pipeline not running")?
            .send(input)?;
        Ok(())
    }

    fn start_with_hotkey(&mut self) -> Result<()> {
        self.start_pipeline(false);
        self.send(PipelineInput::Hotkey(PttSignal::Pressed))?;
        self.wait_until("the recording shows", |shown| {
            shown.contains(&Shown::Recording)
        })
    }

    fn start_by_voice(&mut self) -> Result<()> {
        self.start_pipeline(true);
        self.say(VoiceCommand::StartDictation)?;
        self.wait_until("the recording shows", |shown| {
            shown.contains(&Shown::Recording)
        })
    }

    fn say(&self, command: VoiceCommand) -> Result<()> {
        let segment_start = Instant::now();
        let start = segment_start + Duration::from_millis(100);
        let end = start + Duration::from_millis(800);
        self.send(PipelineInput::Spotting(SpottingEvent::SpeechStarted(
            segment_start,
        )))?;
        self.send(PipelineInput::Spotting(SpottingEvent::PhraseSpotted {
            phrase: command.phrase().to_owned(),
            start,
            end,
            timing: None,
        }))?;
        self.send(PipelineInput::Spotting(SpottingEvent::SpeechEnded(
            end + Duration::from_millis(100),
        )))
    }

    /// Hands over the next part of the recording and waits until it is transcribed.
    fn transcribe_part(&self, answer: Option<&'static str>) -> Result<()> {
        let before = *self.transcribed.lock().unwrap();
        self.answers.lock().unwrap().push_back(answer);
        send_part(&self.parts)?;
        let deadline = Instant::now() + REACTION;
        while *self.transcribed.lock().unwrap() == before {
            if Instant::now() >= deadline {
                bail!(
                    "the part was never transcribed; overlay: {:?}",
                    self.shown()
                );
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        std::thread::sleep(SETTLE);
        Ok(())
    }

    fn end(&mut self) -> Result<()> {
        self.answers.lock().unwrap().push_back(Some(LAST_PART));
        self.stop_at = self.shown().len();
        self.send(PipelineInput::Hotkey(PttSignal::Pressed))?;
        // The Pasted sign takes itself down: a pasted dictation ends on it.
        self.wait_until("the overlay hides after the stop", |shown| {
            shown
                .iter()
                .any(|s| matches!(s, Shown::Hidden | Shown::PastedSign))
        })
    }

    fn wait_until(&self, what: &str, done: impl Fn(&[Shown]) -> bool) -> Result<()> {
        let deadline = Instant::now() + REACTION;
        while !done(&self.shown()[self.stop_at..]) {
            if Instant::now() >= deadline {
                bail!("{what} never happened; overlay: {:?}", self.shown());
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        Ok(())
    }
}

fn live_transcripts(shown: &[Shown]) -> Vec<String> {
    shown
        .iter()
        .filter_map(|s| match s {
            Shown::LiveTranscript(text) => Some(text.clone()),
            _ => None,
        })
        .collect()
}

fn send_part(parts: &Parts) -> Result<()> {
    let one_second = AudioBuffer16k::new(vec![1_000i16; AudioBuffer16k::SAMPLE_RATE_HZ as usize]);
    let sent = parts
        .lock()
        .unwrap()
        .as_ref()
        .map(|tx| tx.send(LiveChunk { audio: one_second, enqueued_at: Instant::now() }));
    match sent {
        Some(Ok(())) => Ok(()),
        _ => bail!("no recording takes parts"),
    }
}

fn run(world: &mut IntegrationWorld) -> &mut LiveRun {
    world.live.get_or_insert_with(LiveRun::default)
}

fn expect_latest(run: &LiveRun, expected: &str) -> Result<()> {
    let shown = run.live_transcripts();
    if shown.last().map(String::as_str) != Some(expected) {
        bail!("expected the overlay to show {expected:?}; it showed {shown:?}");
    }
    Ok(())
}

#[given("the user dictates with the hotkey")]
#[given("the user dictates with the hotkey for less than one part")]
fn dictates_with_hotkey(world: &mut IntegrationWorld) -> Result<()> {
    run(world).start_with_hotkey()
}

#[given(expr = "voice control is turned on and the user started a dictation by saying {string}")]
fn started_by_voice(world: &mut IntegrationWorld, phrase: String) -> Result<()> {
    if VoiceCommand::from_phrase(&phrase) != Some(VoiceCommand::StartDictation) {
        bail!("only \"start dictation\" starts a dictation here, not {phrase:?}");
    }
    run(world).start_by_voice()
}

#[given("the overlay shows the text of the first part of a running dictation")]
fn first_part_shown(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.start_with_hotkey()?;
    run.transcribe_part(Some(FIRST_PART))?;
    expect_latest(run, FIRST_PART)
}

#[when("the first part of the recording is transcribed while the user speaks")]
fn first_part_transcribed(world: &mut IntegrationWorld) -> Result<()> {
    run(world).transcribe_part(Some(FIRST_PART))
}

#[when("the next part is transcribed")]
fn next_part_transcribed(world: &mut IntegrationWorld) -> Result<()> {
    run(world).transcribe_part(Some(NEXT_PART))
}

#[when("the next part holds no speech and is transcribed")]
fn silent_part_transcribed(world: &mut IntegrationWorld) -> Result<()> {
    run(world).transcribe_part(None)
}

#[when("the recording runs")]
fn recording_runs(_world: &mut IntegrationWorld) {
    std::thread::sleep(SETTLE);
}

#[when("the user ends the dictation")]
fn ends_dictation(world: &mut IntegrationWorld) -> Result<()> {
    run(world).end()
}

#[then("the overlay shows the text of that part next to the recording sign")]
fn shows_first_part(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    let shown = run.shown();
    if shown.first() != Some(&Shown::Recording) || shown.contains(&Shown::RecognizingSpeech) {
        bail!("the text was not shown during the recording: {shown:?}");
    }
    expect_latest(run, FIRST_PART)
}

#[then("the overlay shows the text of both parts joined, with their shared words once")]
fn shows_both_parts(world: &mut IntegrationWorld) -> Result<()> {
    let joined = merge_chunk_texts(vec![FIRST_PART.to_owned(), NEXT_PART.to_owned()]);
    if joined.matches("friday").count() != 1 {
        bail!("the parts' shared words are not kept once: {joined:?}");
    }
    expect_latest(run(world), &joined)
}

#[then("the overlay shows the recording sign without any text")]
fn recording_sign_alone(world: &mut IntegrationWorld) -> Result<()> {
    let shown = run(world).shown();
    if shown != [Shown::Recording] {
        bail!("expected the recording sign alone; overlay: {shown:?}");
    }
    Ok(())
}

#[then("the overlay keeps showing the same text")]
fn same_text(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    let shown = run.live_transcripts();
    if shown.iter().any(|text| text != FIRST_PART) {
        bail!("the part without speech changed the text: {shown:?}");
    }
    expect_latest(run, FIRST_PART)
}

#[then("the overlay shows the speech recognition stage without that text")]
fn recognition_without_text(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    let after_stop = run.shown()[run.stop_at..].to_vec();
    if after_stop.first() != Some(&Shown::RecognizingSpeech) {
        bail!("the stop did not show speech recognition first: {after_stop:?}");
    }
    if !live_transcripts(&after_stop).is_empty() {
        bail!("the text was shown after the stop: {after_stop:?}");
    }
    Ok(())
}

#[then("the pasted text is the stitched text of every part")]
fn pasted_stitched(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    let expected = merge_chunk_texts(vec![FIRST_PART.to_owned(), LAST_PART.to_owned()]);
    let pasted = run.pasted.lock().unwrap().clone();
    if pasted.as_deref() != Some(expected.as_str()) {
        bail!("expected {expected:?} pasted; pasted {pasted:?}");
    }
    Ok(())
}
