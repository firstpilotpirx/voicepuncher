//! The recorder between a 48 kHz microphone and speech recognition: the stand-in microphone
//! pushes frames into the capture ring as its callback would, and the scenario reads the
//! recording and the live chunks the recorder makes of it.

use std::sync::{atomic::AtomicU32, mpsc::Receiver, Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use cucumber::{given, then, when};
use rtrb::{Producer, RingBuffer};

use voicepuncher::adapters::secondary::recording::cpal::{
    capture::{on_frames, CaptureHealth, RING_SECONDS},
    CpalAudioRecorder, Source,
};
use voicepuncher::core::application::audio_recorder::LiveChunk;
use voicepuncher::core::application::{AudioRecorder, ChunkingCfg};

use crate::steps::voice_control::pipeline_log;
use crate::IntegrationWorld;

const MIC_RATE_HZ: u32 = 48_000;
const MIC_RATE: usize = MIC_RATE_HZ as usize;
/// Microphone samples per recorded sample at 16 kHz.
const PER_SAMPLE: usize = 3;
/// Samples per microphone callback: 10 ms.
const CALLBACK: usize = 480;
/// The ring of the scenario whose recording cannot keep up.
const SMALL_RING: usize = 1_000;
const DROPPED_LINE: &str = "ms of audio dropped";

/// A recording on the stand-in microphone, and what came of it.
#[derive(Default)]
pub struct RecordingRun {
    recorder: Option<CpalAudioRecorder>,
    ring: Option<Producer<f32>>,
    health: Option<Arc<CaptureHealth>>,
    chunks: Option<Receiver<LiveChunk>>,
    /// Microphone samples delivered.
    delivered: usize,
    recorded: Vec<i16>,
    joined: Vec<i16>,
    dropped_logged: Vec<String>,
}

impl std::fmt::Debug for RecordingRun {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RecordingRun")
            .field("delivered", &self.delivered)
            .field("recorded", &self.recorded.len())
            .finish_non_exhaustive()
    }
}

fn recording(world: &mut IntegrationWorld) -> Result<&mut RecordingRun> {
    world
        .extra
        .speech
        .recording
        .as_mut()
        .context("no recording scenario")
}

/// A recorder whose microphone holds back `ring` samples before it drops audio.
fn on_microphone(world: &mut IntegrationWorld, ring: usize) {
    let (ring, captured) = RingBuffer::new(ring);
    let health = Arc::new(CaptureHealth::new(Arc::new(AtomicU32::new(0))));
    let first_start = Mutex::new(Some(Source {
        rate: MIC_RATE_HZ,
        started_at: Instant::now(),
        health: Arc::clone(&health),
        audio: Box::new(captured),
        input: None,
    }));
    let recorder =
        CpalAudioRecorder::new(600, Arc::new(AtomicU32::new(0))).with_source(move |_, _| {
            first_start
                .lock()
                .unwrap()
                .take()
                .context("the recorder started twice")
        });
    world.extra.speech.recording = Some(RecordingRun {
        recorder: Some(recorder),
        ring: Some(ring),
        health: Some(health),
        ..RecordingRun::default()
    });
}

impl RecordingRun {
    fn recorder(&mut self) -> Result<&mut CpalAudioRecorder> {
        self.recorder.as_mut().context("no recorder")
    }

    /// Pushes `audio` one callback at a time, every `pace`.
    fn deliver(&mut self, audio: &[f32], pace: Duration) -> Result<()> {
        let ring = self.ring.as_mut().context("no microphone")?;
        let health = self.health.as_ref().context("no microphone")?;
        for frame in audio.chunks(CALLBACK) {
            on_frames(frame, 1, ring, health);
            std::thread::sleep(pace);
        }
        self.delivered += audio.len();
        Ok(())
    }

    fn stop(&mut self) -> Result<()> {
        let before = pipeline_log().warnings.lock().unwrap().len();
        self.recorded = self.recorder()?.stop()?.samples().to_vec();
        self.dropped_logged = pipeline_log().warnings.lock().unwrap()[before..]
            .iter()
            .map(|(_, line)| line.clone())
            .filter(|line| line.contains(DROPPED_LINE))
            .collect();
        Ok(())
    }
}

/// How much each sample of `rising` audio is louder than the one before: 0.9 after 300 000.
const RISE: f32 = 3e-6;

/// Audio that rises from silence, `RISE` per sample: a lost or swapped piece shows as a fall.
fn rising(samples: usize) -> Vec<f32> {
    let step = RISE;
    std::iter::successors(Some(0.0f32), |level| Some(level + step))
        .take(samples)
        .collect()
}

#[given("a recording is set up on a microphone that records at 48 kHz")]
fn microphone(world: &mut IntegrationWorld) {
    on_microphone(world, RING_SECONDS * MIC_RATE);
}

#[given(
    "a recording is set up on a microphone whose audio the recording can hold back only briefly"
)]
fn small_ring(world: &mut IntegrationWorld) {
    pipeline_log();
    on_microphone(world, SMALL_RING);
}

#[when("the part that collects the audio is held up for 5 seconds while the microphone keeps delivering")]
fn held_up(world: &mut IntegrationWorld) -> Result<()> {
    let run = recording(world)?;
    let audio = rising(5 * MIC_RATE + 10 * CALLBACK);
    let (before_start, after) = audio.split_at(5 * MIC_RATE);
    run.deliver(before_start, Duration::ZERO)?;
    run.recorder()?.start()?;
    run.deliver(after, Duration::from_millis(10))?;
    run.stop()
}

#[when("the microphone delivers a second of audio at once and then goes on")]
fn overrun(world: &mut IntegrationWorld) -> Result<()> {
    let run = recording(world)?;
    run.recorder()?.start()?;
    std::thread::sleep(Duration::from_millis(50));
    run.deliver(&vec![0.5; MIC_RATE], Duration::ZERO)?;
    std::thread::sleep(Duration::from_millis(50));
    run.deliver(&[0.9; CALLBACK], Duration::from_millis(10))?;
    run.stop()
}

#[when("the microphone delivers a second of steady sound and the user stops the dictation")]
fn steady_second(world: &mut IntegrationWorld) -> Result<()> {
    let run = recording(world)?;
    run.recorder()?.start()?;
    run.deliver(&vec![0.5; MIC_RATE], Duration::from_micros(2_500))?;
    run.stop()
}

#[when("the user dictates three and a half seconds cut into chunks of a second and stops")]
fn chunked(world: &mut IntegrationWorld) -> Result<()> {
    let run = recording(world)?;
    let cfg = ChunkingCfg { chunk_threshold_secs: 1, chunk_overlap_secs: 0, vad_lookback_secs: 0 };
    run.recorder()?.start()?;
    run.chunks = run.recorder()?.start_live_chunks(cfg)?;
    run.deliver(&rising(MIC_RATE * 7 / 2), Duration::from_micros(2_500))?;
    run.recorder()?.flush_live_chunks();
    run.stop()?;
    run.collect_chunks()
}

impl RecordingRun {
    /// Keeps the whole recording and the live chunks laid end to end.
    fn collect_chunks(&mut self) -> Result<()> {
        let whole = self.recorder()?.recording().context("no whole recording")?;
        self.recorded = whole.samples().to_vec();
        let chunks = self
            .chunks
            .take()
            .context("the recorder made no live chunks")?;
        self.joined = chunks
            .try_iter()
            .flat_map(|c| c.audio.samples().to_vec())
            .collect();
        Ok(())
    }
}

fn rises_throughout(samples: &[i16]) -> bool {
    let inner = samples
        .get(100..samples.len().saturating_sub(100))
        .unwrap_or_default();
    inner.windows(2).all(|pair| pair[1] >= pair[0] - 2)
}

#[then("the audio delivered meanwhile is in the recording in order, with nothing missing")]
fn kept_in_order(world: &mut IntegrationWorld) -> Result<()> {
    let run = recording(world)?;
    let dropped = run.health.as_ref().map_or(0, |h| h.dropped_samples());
    if run.recorded.len() != run.delivered / PER_SAMPLE
        || dropped != 0
        || !rises_throughout(&run.recorded)
    {
        bail!(
            "recorded {} of {} samples ({dropped} dropped), in order: {}",
            run.recorded.len(),
            run.delivered / PER_SAMPLE,
            rises_throughout(&run.recorded)
        );
    }
    Ok(())
}

#[then("the recording goes on and the log reports once how much audio was dropped")]
fn drop_reported_once(world: &mut IntegrationWorld) -> Result<()> {
    let run = recording(world)?;
    let after_overrun = (SMALL_RING + CALLBACK) / PER_SAMPLE;
    let loud_end = run.recorded.last().is_some_and(|&s| s > i16::MAX / 2);
    if run.recorded.len() != after_overrun || !loud_end || run.dropped_logged.len() != 1 {
        bail!(
            "recorded {} samples, the drop logged as {:?}",
            run.recorded.len(),
            run.dropped_logged
        );
    }
    Ok(())
}

#[then("the recording is exactly as long as the sound and loud to its last sample")]
fn ends_with_the_sound(world: &mut IntegrationWorld) -> Result<()> {
    let run = recording(world)?;
    let last = run.recorded.last().copied().unwrap_or(0);
    if run.recorded.len() != MIC_RATE / PER_SAMPLE || last <= i16::MAX / 10 {
        bail!("recorded {} samples ending at {last}", run.recorded.len());
    }
    Ok(())
}

#[then("the chunks laid end to end are the whole recording, with no gap or repeated samples")]
fn chunks_join(world: &mut IntegrationWorld) -> Result<()> {
    let run = recording(world)?;
    if run.joined.is_empty() || run.joined != run.recorded {
        bail!(
            "{} chunk samples against {} recorded",
            run.joined.len(),
            run.recorded.len()
        );
    }
    Ok(())
}
