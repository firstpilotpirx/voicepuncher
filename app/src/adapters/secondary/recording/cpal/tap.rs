//! The kept microphone shared with the dictation recorder.
//!
//! While voice control listens, a dictation records the audio command spotting already captures instead of opening the
//! device a second time, which costs a start of its own.

use std::sync::atomic::AtomicU32;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Instant;

use super::capture::{CaptureHealth, CapturedAudio};
use super::Source;
use crate::core::domain::AudioBuffer16k;

/// The fan-out of the kept microphone; clones share it.
#[derive(Clone, Default)]
pub struct MicTap(Arc<Mutex<Taps>>);

#[derive(Default)]
struct Taps {
    /// The tee that feeds the readers, while one runs.
    feeding: Option<u64>,
    tees: u64,
    readers: Vec<Sender<Vec<f32>>>,
}

impl MicTap {
    /// `audio`, 16 kHz, passed through and copied to every reader while it runs.
    pub fn tee(&self, audio: Box<dyn CapturedAudio>) -> Box<dyn CapturedAudio> {
        let mut taps = self.lock();
        taps.tees += 1;
        taps.feeding = Some(taps.tees);
        Box::new(Tee { audio, tap: self.clone(), id: taps.tees })
    }

    /// A recording source on the kept microphone, its level in `level`; `None` when no tee runs.
    pub fn source(&self, level: Arc<AtomicU32>) -> Option<Source> {
        let mut taps = self.lock();
        taps.feeding?;
        let (tx, rx) = mpsc::channel();
        taps.readers.push(tx);
        let health = Arc::new(CaptureHealth::new(level));
        Some(Source {
            audio: Box::new(Reader { audio: rx, health: Arc::clone(&health) }),
            rate: AudioBuffer16k::SAMPLE_RATE_HZ,
            health,
            started_at: Instant::now(),
            input: None,
        })
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Taps> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

struct Tee {
    audio: Box<dyn CapturedAudio>,
    tap: MicTap,
    id: u64,
}

impl CapturedAudio for Tee {
    fn take(&mut self) -> Vec<f32> {
        let taken = self.audio.take();
        if !taken.is_empty() {
            let mut taps = self.tap.lock();
            if taps.feeding == Some(self.id) {
                taps.readers
                    .retain(|reader| reader.send(taken.clone()).is_ok());
            }
        }
        taken
    }
}

impl Drop for Tee {
    fn drop(&mut self) {
        let mut taps = self.tap.lock();
        if taps.feeding == Some(self.id) {
            taps.feeding = None;
            taps.readers.clear();
        }
    }
}

/// A recording's share of the kept microphone.
struct Reader {
    audio: Receiver<Vec<f32>>,
    health: Arc<CaptureHealth>,
}

impl CapturedAudio for Reader {
    fn take(&mut self) -> Vec<f32> {
        let taken: Vec<f32> = self.audio.try_iter().flatten().collect();
        if !taken.is_empty() {
            self.health.note_samples(&taken);
        }
        taken
    }
}
