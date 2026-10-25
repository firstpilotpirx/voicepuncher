//! The recording buffer: 16 kHz mono, appended by the drain thread, read in slices by the
//! live watcher, bounded by a cut and by the recording limit.

use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use crate::core::domain::conversions::saturating_usize;
use crate::core::domain::AudioBuffer16k;

const RATE: usize = AudioBuffer16k::SAMPLE_RATE_HZ as usize;

pub struct RecordingBuffer {
    samples: Mutex<Vec<i16>>,
    /// Samples never appended beyond: the recording limit, or a cut.
    limit: Mutex<usize>,
    /// The recording limit itself, which a cut never lowers.
    max_samples: usize,
}

impl RecordingBuffer {
    pub fn new(max_samples: usize) -> Self {
        Self { samples: Mutex::new(Vec::new()), limit: Mutex::new(max_samples), max_samples }
    }

    fn lock(&self) -> MutexGuard<'_, Vec<i16>> {
        self.samples.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn limit(&self) -> usize {
        *self.limit.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Appends what fits under the limit.
    pub fn append(&self, samples: &[i16]) {
        let limit = self.limit();
        let mut held = self.lock();
        let room = limit.saturating_sub(held.len());
        held.extend_from_slice(&samples[..samples.len().min(room)]);
    }

    pub fn len(&self) -> usize {
        self.lock().len()
    }

    pub fn is_empty(&self) -> bool {
        self.lock().is_empty()
    }

    /// Whether the recording limit was reached; a cut never counts.
    pub fn is_full(&self) -> bool {
        self.lock().len() >= self.max_samples
    }

    /// Drops everything from `len` on and takes nothing more past it.
    pub fn cut_at(&self, len: usize) {
        let mut limit = self.limit.lock().unwrap_or_else(PoisonError::into_inner);
        *limit = (*limit).min(len);
        self.lock().truncate(len);
    }

    /// Puts `lead_in` before everything captured; how long it lasts.
    pub fn put_first(&self, lead_in: &[i16]) -> Duration {
        self.lock().splice(0..0, lead_in.iter().copied());
        Duration::from_secs_f64(lead_in.len() as f64 / RATE as f64)
    }

    /// Reads the samples held now.
    pub fn read<T>(&self, read: impl FnOnce(&[i16]) -> T) -> T {
        read(&self.lock())
    }

    /// The samples from `start` to `end`, bounded by what the buffer holds now: a recording
    /// cut by `cut_at` may be shorter than when its length was read.
    pub fn between(&self, start: usize, end: usize) -> Vec<i16> {
        self.read(|samples| {
            let end = end.min(samples.len());
            samples
                .get(start.min(end)..end)
                .unwrap_or_default()
                .to_vec()
        })
    }

    /// Everything held, leaving the buffer empty.
    pub fn take(&self) -> Vec<i16> {
        std::mem::take(&mut *self.lock())
    }
}

/// Number of 16 kHz samples captured between `started` and `cut`.
pub fn samples_before(started: Instant, cut: Instant) -> usize {
    saturating_usize(cut.saturating_duration_since(started).as_secs_f64() * RATE as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_append_only_up_to_the_limit() {
        let buffer = RecordingBuffer::new(3);
        buffer.append(&[1, 2]);
        buffer.append(&[3, 4]);
        assert!(buffer.is_full());
        assert_eq!(buffer.take(), [1, 2, 3]);
    }

    #[test]
    fn should_not_count_a_cut_buffer_as_full() {
        let buffer = RecordingBuffer::new(100);
        buffer.append(&[1, 2, 3, 4]);
        buffer.cut_at(4);
        assert!(!buffer.is_full());
    }

    #[test]
    fn should_take_nothing_more_after_a_cut() {
        let buffer = RecordingBuffer::new(100);
        buffer.append(&[1, 2, 3, 4]);
        buffer.cut_at(2);
        buffer.append(&[5]);
        assert_eq!(buffer.take(), [1, 2]);
    }

    #[test]
    fn should_keep_every_sample_captured_before_a_late_cut() {
        let buffer = RecordingBuffer::new(100_000);
        buffer.append(&[1; 8_000]);
        let started = Instant::now();
        buffer.cut_at(samples_before(started, started + Duration::from_secs(2)));
        assert_eq!(buffer.len(), 8_000);
    }

    #[test]
    fn should_put_the_lead_in_before_the_recorded_samples() {
        let buffer = RecordingBuffer::new(100_000);
        buffer.append(&[1, 2, 3]);
        let lead_in_time = buffer.put_first(&[7; 4_000]);
        assert_eq!(lead_in_time, Duration::from_millis(250));
        let samples = buffer.take();
        assert_eq!(samples[..4_000], [7; 4_000]);
        assert_eq!(samples[4_000..], [1, 2, 3]);
    }

    #[test]
    fn should_take_only_the_samples_left_when_the_buffer_was_cut_after_its_length_was_read() {
        let buffer = RecordingBuffer::new(100_000);
        buffer.append(&vec![1; 16_000]);
        let read_len = buffer.len();
        buffer.cut_at(10_000);
        assert_eq!(buffer.between(4_000, read_len).len(), 6_000);
    }

    #[test]
    fn should_take_nothing_when_the_cut_is_before_the_chunk_start() {
        let buffer = RecordingBuffer::new(100_000);
        buffer.append(&[1; 3_000]);
        assert!(buffer.between(4_000, 16_000).is_empty());
    }

    #[test]
    fn should_keep_one_second_of_samples_for_a_cut_one_second_after_the_start() {
        let started = Instant::now();
        assert_eq!(
            samples_before(started, started + Duration::from_secs(1)),
            16_000
        );
    }

    #[test]
    fn should_keep_no_samples_for_a_cut_before_the_start() {
        let cut = Instant::now();
        assert_eq!(samples_before(cut + Duration::from_millis(500), cut), 0);
    }
}
