//! Dictations whose speech recognition goes wrong: the live worker dies, or a request never
//! answers. Each dictation delivers one live chunk numbered by the recording it belongs to.

use voicepuncher::test_doubles::prelude::*;

use cucumber::{given, then, when};

use crate::IntegrationWorld;

/// The text the speech recognition service gives the second dictation.
const SECOND_TEXT: &str = "The second dictation reaches the text field.";
/// The error of a request to the stopped service.
const SERVICE_STOPPED: &str = "the speech recognition service is not running";
/// How long a stopped dictation waits for its transcript in these scenarios.
const STOP_WAIT: Duration = Duration::from_millis(500);
/// How long a step waits for the pipeline to show what it expects.
const STEP_WAIT: Duration = Duration::from_secs(5);

/// What the first dictation's speech recognition does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Loss {
    WorkerDies,
    NeverAnswers,
    ServiceStopped,
}

/// What a loss scenario shares with the pipeline thread.
#[derive(Debug, Default)]
pub struct LossRun {
    loss: Option<Loss>,
    pasted: Arc<Mutex<Vec<String>>>,
    notices: Arc<Mutex<Vec<FailureNotice>>>,
    recordings: Arc<AtomicUsize>,
    /// Lets the request that never answers return, once the scenario has seen its notice.
    release: Option<Sender<()>>,
}

fn run(world: &mut IntegrationWorld) -> Result<&mut LossRun> {
    world.pipeline.loss.as_mut().context("no loss scenario")
}

#[given("the live transcription worker of a dictation stops without a result")]
fn worker_dies(world: &mut IntegrationWorld) {
    world.pipeline.loss = Some(LossRun { loss: Some(Loss::WorkerDies), ..LossRun::default() });
}

#[given("the speech recognition service never answers the last chunk of a dictation")]
fn never_answers(world: &mut IntegrationWorld) {
    world.pipeline.loss = Some(LossRun { loss: Some(Loss::NeverAnswers), ..LossRun::default() });
}

#[given("the speech recognition service has stopped before a dictation ends")]
fn service_stopped(world: &mut IntegrationWorld) {
    crate::steps::voice_control::pipeline_log();
    world.pipeline.loss = Some(LossRun { loss: Some(Loss::ServiceStopped), ..LossRun::default() });
}

/// A pipeline over the stand-ins of `run`, on a thread of its own.
fn start_pipeline(run: &mut LossRun) -> Result<(Sender<PipelineInput>, JoinHandle<()>)> {
    let loss = run.loss.context("no loss")?;
    let (release_tx, release_rx) = mpsc::channel();
    run.release = Some(release_tx);
    let mut pipeline = Pipeline::new(
        PipelineParts {
            recorder: Box::new(OneChunkRecorder(run.recordings.clone())),
            transcriber: Box::new(FirstDictationLost { loss, release: Some(release_rx) }),
            paster: Box::new(CapturingPaster(run.pasted.clone())),
            sound_player: Box::new(Quiet(run.notices.clone())),
            visual_indicator: Box::new(Quiet(run.notices.clone())),
        },
        PipelineConfig {
            chunking: ChunkingCfg { chunk_threshold_secs: 30, ..ChunkingCfg::default() },
            ..PipelineConfig::default()
        },
    )
    .with_stop_wait(STOP_WAIT);
    let (tx, rx) = mpsc::channel();
    let thread = std::thread::spawn(move || {
        let _ = pipeline.run(rx);
    });
    Ok((tx, thread))
}

fn press(inputs: &Sender<PipelineInput>) -> Result<()> {
    inputs.send(PipelineInput::Hotkey(PttSignal::Pressed))?;
    Ok(())
}

#[when("the user finishes that dictation and then dictates again")]
fn dictate_twice(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world)?;
    let (inputs, thread) = start_pipeline(run)?;
    for _ in 0..4 {
        press(&inputs)?;
    }
    drop(inputs);
    thread
        .join()
        .map_err(|_| anyhow::anyhow!("the pipeline thread panicked"))?;
    Ok(())
}

#[when("the user finishes the dictation and presses the hotkey during the wait")]
fn press_during_wait(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world)?;
    let (inputs, thread) = start_pipeline(run)?;
    // start, stop, and a press while the stop waits for the transcript
    for _ in 0..3 {
        press(&inputs)?;
    }
    let noticed = wait_until(STEP_WAIT, || !run.notices.lock().unwrap().is_empty());
    release_and_stop(run, inputs, thread)?;
    if !noticed {
        bail!("no failure notice within {STEP_WAIT:?} of the stop");
    }
    Ok(())
}

/// Ends the wait, stops the recording the press during it started and joins the pipeline.
fn release_and_stop(
    run: &mut LossRun,
    inputs: Sender<PipelineInput>,
    thread: JoinHandle<()>,
) -> Result<()> {
    if let Some(release) = run.release.take() {
        let _ = release.send(());
    }
    press(&inputs)?;
    drop(inputs);
    thread
        .join()
        .map_err(|_| anyhow::anyhow!("the pipeline thread panicked"))
}

fn wait_until(limit: Duration, mut done: impl FnMut() -> bool) -> bool {
    let start = Instant::now();
    while start.elapsed() < limit {
        if done() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    done()
}

#[then("the first dictation ends with the notice that speech recognition failed")]
#[then("the dictation ends with the notice that speech recognition failed once the wait is over")]
fn speech_recognition_failed(world: &mut IntegrationWorld) -> Result<()> {
    let notices = run(world)?.notices.lock().unwrap().clone();
    let lost = FailureNotice::DictationLost(FailingPart::SpeechRecognition);
    if notices.first() != Some(&lost) {
        bail!("the first dictation did not end with {lost:?}: {notices:?}");
    }
    Ok(())
}

#[then("the failure of the first dictation is logged")]
fn failure_logged(_world: &mut IntegrationWorld) -> Result<()> {
    let log = crate::steps::voice_control::pipeline_log();
    let errors = log.errors.lock().unwrap();
    if !errors.iter().any(|e| e.contains(SERVICE_STOPPED)) {
        bail!("no error names the stopped service: {:?}", *errors);
    }
    Ok(())
}

#[then("the text of the second dictation is pasted")]
fn second_pasted(world: &mut IntegrationWorld) -> Result<()> {
    let pasted = run(world)?.pasted.lock().unwrap().clone();
    if pasted != [SECOND_TEXT] {
        bail!("pasted {pasted:?} instead of only the second dictation");
    }
    Ok(())
}

#[then("the hotkey press made during the wait starts a new recording")]
fn press_started_recording(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world)?;
    let recordings = run.recordings.load(Ordering::SeqCst);
    if recordings != 2 {
        bail!("{recordings} recordings started instead of 2");
    }
    second_pasted(world)
}

/// Delivers one live chunk per recording; its first sample numbers the recording from 0.
struct OneChunkRecorder(Arc<AtomicUsize>);

impl AudioRecorder for OneChunkRecorder {
    voicepuncher::records_nothing!();

    fn start_live_chunks(&mut self, _cfg: ChunkingCfg) -> Result<Option<Receiver<LiveChunk>>> {
        let n = self.0.fetch_add(1, Ordering::SeqCst);
        let audio = voicepuncher::test_doubles::numbered_second(n);
        let (tx, rx) = mpsc::channel();
        tx.send(LiveChunk { audio, enqueued_at: Instant::now() })?;
        Ok(Some(rx))
    }

    fn flush_live_chunks(&mut self) {}
    voicepuncher::records_whole_buffers_only!();
}

/// Loses the first dictation's chunk as `loss` says and answers the second one.
struct FirstDictationLost {
    loss: Loss,
    release: Option<Receiver<()>>,
}

impl Transcriber for FirstDictationLost {
    fn transcribe(&mut self, audio: &AudioBuffer16k) -> Result<Option<RawTranscript>> {
        if audio.samples()[0] == 0 {
            match self.loss {
                Loss::WorkerDies => panic!("the live transcription worker dies"),
                Loss::ServiceStopped => bail!("{SERVICE_STOPPED}"),
                Loss::NeverAnswers => {
                    let _ = self.release.take().map(|release| release.recv());
                    return Ok(Some(RawTranscript::new("A late answer.".to_owned())?));
                }
            }
        }
        Ok(Some(RawTranscript::new(SECOND_TEXT.to_owned())?))
    }
}

use super::doubles::CapturingPaster;

/// Plays nothing and keeps the failure notices it is shown.
struct Quiet(Arc<Mutex<Vec<FailureNotice>>>);

impl SoundPlayer for Quiet {
    fn play_start(&self) {}
    fn play_stop(&self) {}
    fn play_cancel(&self) {}
}

impl VisualIndicator for Quiet {
    fn show_recording(&self) {}
    fn show_recognizing_speech(&self) {}
    fn show_waiting_for_focus(&self) {}
    fn show_pasted(&self) {}
    fn hide(&self) {}
    fn show_failure_notice(&self, notice: &FailureNotice) {
        self.0.lock().unwrap().push(notice.clone());
    }
    voicepuncher::shows_no_voice_control_signs!();
}
