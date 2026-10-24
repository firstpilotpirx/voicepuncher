//! The command spotting microphone, kept open on a thread of its own.
//!
//! A device that reports an error or stops delivering audio is opened again, and the time
//! it was silent is sent as silence, so the stream's sample count keeps matching the
//! capture time.

use std::any::Any;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};

use crate::adapters::secondary::recording::cpal::capture::{
    CaptureHealth, CapturedAudio, MicInput,
};
use crate::adapters::secondary::recording::cpal::resampler::StreamResampler;
use crate::adapters::secondary::recording::cpal::watchdog::START_LIMIT;
use crate::core::domain::AudioBuffer16k;

/// An open microphone, as its opener hands it to the keeper.
pub struct OpenMic {
    /// Its audio, mono at `rate`.
    pub audio: Box<dyn CapturedAudio>,
    pub health: Arc<CaptureHealth>,
    pub rate: u32,
    /// When its first sample was captured.
    pub started_at: Instant,
    /// What keeps the device capturing until dropped.
    pub device: Box<dyn Any>,
}

/// Opens the input device `device` names (see [`MicInput::open`]) for the keeper.
pub fn open_device(device: &str) -> Result<OpenMic> {
    let (capture, audio) = MicInput::open(device, Arc::new(AtomicU32::new(0)))?;
    log::info!("command spotting: microphone open at {} Hz", capture.rate);
    Ok(OpenMic {
        audio: Box::new(audio),
        health: Arc::clone(capture.health()),
        rate: capture.rate,
        started_at: capture.started_at,
        device: Box::new(capture),
    })
}

/// Opens the microphone; called on the keeper's thread, so the device never leaves it.
pub type MicOpener = Box<dyn FnMut() -> Result<OpenMic> + Send>;

/// How the keeper judges and paces its microphone.
#[derive(Clone, Copy, Debug)]
pub struct KeeperTimings {
    /// How long a microphone that delivered audio may stay silent before it is opened again.
    pub stall: Duration,
    /// How long the keeper waits between failed attempts to open it again.
    pub retry: Duration,
    /// How often the keeper takes the captured audio.
    pub tick: Duration,
}

impl Default for KeeperTimings {
    fn default() -> Self {
        Self {
            stall: Duration::from_secs(2),
            retry: Duration::from_secs(1),
            tick: Duration::from_millis(20),
        }
    }
}

/// The keeper thread; stopped and joined when dropped.
pub struct KeptMic {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl KeptMic {
    /// Opens the microphone with `open` on a thread of its own and keeps it open. Returns
    /// the keeper, its audio at 16 kHz and when the first sample was captured; fails when the
    /// first open fails.
    pub fn start(
        mut open: MicOpener,
        timings: KeeperTimings,
    ) -> Result<(Self, Receiver<Vec<f32>>, Instant)> {
        let (audio_tx, audio_rx) = mpsc::channel();
        let (opened_tx, opened_rx) = mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = Arc::clone(&stop);
        let thread = std::thread::Builder::new()
            .name("command-spotting-mic".into())
            .spawn(move || {
                let first = open();
                match first.and_then(|mic| Keeper::new(mic, open, audio_tx, timings)) {
                    Ok(keeper) => {
                        let _ = opened_tx.send(Ok(keeper.start));
                        keeper.run(&stopped);
                    }
                    Err(error) => {
                        let _ = opened_tx.send(Err(format!("{error:#}")));
                    }
                }
            })?;
        let started_at = opened_rx
            .recv()
            .map_err(|_| anyhow!("the microphone keeper ended before opening"))?
            .map_err(|error| anyhow!(error))?;
        Ok((Self { stop, thread: Some(thread) }, audio_rx, started_at))
    }
}

impl Drop for KeptMic {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            if thread.join().is_err() {
                log::warn!("command spotting: the microphone keeper panicked");
            }
        }
    }
}

/// What the keeper thread owns.
struct Keeper {
    open: MicOpener,
    mic: Option<OpenMic>,
    resampler: StreamResampler,
    audio: Sender<Vec<f32>>,
    timings: KeeperTimings,
    /// When the first sample was captured: the stream's time zero.
    start: Instant,
    /// The samples sent so far, at 16 kHz.
    sent: u64,
    /// When the microphone was lost, while it is.
    lost_at: Option<Instant>,
    next_try: Instant,
    /// Whether a failed reopen was logged since the loss.
    reopen_failed: bool,
}

impl Keeper {
    fn new(
        mic: OpenMic,
        open: MicOpener,
        audio: Sender<Vec<f32>>,
        timings: KeeperTimings,
    ) -> Result<Self> {
        Ok(Self {
            open,
            resampler: StreamResampler::new(mic.rate)?,
            start: mic.started_at,
            mic: Some(mic),
            audio,
            timings,
            sent: 0,
            lost_at: None,
            next_try: Instant::now(),
            reopen_failed: false,
        })
    }

    /// Runs until stopped or until nobody takes the audio any more.
    fn run(mut self, stop: &AtomicBool) {
        while !stop.load(Ordering::Relaxed) {
            let taken = if self.mic.is_some() {
                self.forward()
            } else {
                self.reopen()
            };
            if !taken {
                return;
            }
            std::thread::sleep(self.timings.tick);
        }
    }

    /// Sends what the microphone captured; drops a stalled microphone. False once the
    /// audio has no reader.
    fn forward(&mut self) -> bool {
        let Some(mic) = self.mic.as_mut() else {
            return true;
        };
        let samples = self.resampler.push(&mic.audio.take());
        if let Some(reason) = stall(&mic.health, self.timings.stall) {
            log::warn!("command spotting: microphone {reason}; opening it again");
            let now = Instant::now();
            self.mic = None;
            self.lost_at = Some(now);
            self.next_try = now;
        }
        self.send(samples)
    }

    /// Tries to open the microphone again once its retry time came.
    fn reopen(&mut self) -> bool {
        let now = Instant::now();
        if now < self.next_try {
            return true;
        }
        match (self.open)().and_then(|mic| Ok((StreamResampler::new(mic.rate)?, mic))) {
            Ok((resampler, mic)) => self.reopened(resampler, mic),
            Err(error) => {
                if !self.reopen_failed {
                    log::warn!(
                        "command spotting: cannot open the microphone again: {error:#}; retrying every {} ms",
                        self.timings.retry.as_millis()
                    );
                    self.reopen_failed = true;
                }
                self.next_try = now + self.timings.retry;
                true
            }
        }
    }

    /// Takes the reopened microphone, sending the time it was away as silence.
    fn reopened(&mut self, resampler: StreamResampler, mic: OpenMic) -> bool {
        let away = self.lost_at.map_or(Duration::ZERO, |at| at.elapsed());
        log::info!(
            "command spotting: microphone open again at {} Hz after {} ms",
            mic.rate,
            away.as_millis()
        );
        let gap = silence_between(self.start, self.sent, mic.started_at);
        self.resampler = resampler;
        self.mic = Some(mic);
        self.lost_at = None;
        self.reopen_failed = false;
        self.send(vec![0.0; gap])
    }

    fn send(&mut self, samples: Vec<f32>) -> bool {
        if samples.is_empty() {
            return true;
        }
        self.sent += u64::try_from(samples.len()).unwrap_or(u64::MAX);
        self.audio.send(samples).is_ok()
    }
}

/// Why the microphone counts as stalled, if it does: an error, or no audio for `limit`
/// once it delivered audio (the dictation watchdog's start limit before that).
fn stall(health: &CaptureHealth, limit: Duration) -> Option<String> {
    if health.failed() {
        return Some("reported an error".into());
    }
    let silence = health.since_last_callback();
    let limit = if health.had_callback() {
        limit
    } else {
        START_LIMIT.max(limit)
    };
    (silence > limit).then(|| format!("delivered no audio for {} ms", silence.as_millis()))
}

/// The 16 kHz samples of silence that bring `sent` samples from `start` up to `at`.
fn silence_between(start: Instant, sent: u64, at: Instant) -> usize {
    let rate = u128::from(AudioBuffer16k::SAMPLE_RATE_HZ);
    let due = at.saturating_duration_since(start).as_micros() * rate / 1_000_000;
    usize::try_from(due.saturating_sub(u128::from(sent))).unwrap_or(0)
}

#[cfg(test)]
#[path = "kept_mic_tests.rs"]
mod tests;
