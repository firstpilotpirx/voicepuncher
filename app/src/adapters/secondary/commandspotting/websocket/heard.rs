//! The last seconds of the 16 kHz audio streamed to the service, so the speech said right
//! after a "start" can begin the recording that starts only once the command is heard.

use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::core::domain::AudioBuffer16k;

const SAMPLE_RATE: f32 = AudioBuffer16k::SAMPLE_RATE_HZ as f32;
/// How much audio is held: more than the time from a "start" or "restart" to its report,
/// which waits behind a dictation's own requests at the speech recognition service.
const HELD_SECS: f32 = 10.0;
/// How long one 16 kHz sample lasts.
const SAMPLE_NANOS: u64 = 1_000_000_000 / AudioBuffer16k::SAMPLE_RATE_HZ as u64;

/// A block of streamed samples: the stream position just past its last sample, and when that
/// sample was captured.
#[derive(Debug, Clone, Copy)]
struct Block {
    end: usize,
    captured_at: Instant,
}

pub struct HeardAudio {
    audio_start: Instant,
    /// Position in the stream of the first sample held.
    first: usize,
    samples: VecDeque<f32>,
    /// The blocks of the samples held, oldest first: audio lost before a block leaves its
    /// stream position behind the clock.
    blocks: VecDeque<Block>,
}

impl HeardAudio {
    /// Audio whose first sample was captured at `audio_start`.
    pub fn new(audio_start: Instant) -> Self {
        Self { audio_start, first: 0, samples: VecDeque::new(), blocks: VecDeque::new() }
    }

    /// Holds `samples`, the last of them captured at `captured_at`.
    pub fn push(&mut self, samples: &[f32], captured_at: Instant) {
        self.samples.extend(samples.iter().copied());
        self.blocks
            .push_back(Block { end: self.first + self.samples.len(), captured_at });
        let excess = self
            .samples
            .len()
            .saturating_sub((HELD_SECS * SAMPLE_RATE) as usize);
        self.samples.drain(..excess);
        self.first += excess;
        while self.blocks.front().is_some_and(|b| b.end <= self.first) {
            self.blocks.pop_front();
        }
    }

    /// When the sample at the stream position `dated` stands for — the moment it would have
    /// been captured had no audio been lost — was really captured.
    pub fn captured_at(&self, dated: Instant) -> Instant {
        let position = Self::position(dated.saturating_duration_since(self.audio_start));
        match self.blocks.iter().find(|b| b.end > position) {
            Some(block) => block.captured_at - Self::span(block.end - position),
            None => self.blocks.back().map_or(dated, |last| {
                last.captured_at + Self::span(position - last.end)
            }),
        }
    }

    /// The stream date of the sample captured at `captured` — the inverse of
    /// [`Self::captured_at`]: behind the clock once audio was lost.
    pub fn dated_at(&self, captured: Instant) -> Instant {
        let position = match self.blocks.iter().find(|b| b.captured_at >= captured) {
            Some(block) => block.end.saturating_sub(Self::position(
                block.captured_at.saturating_duration_since(captured),
            )),
            None => match self.blocks.back() {
                Some(last) => {
                    last.end + Self::position(captured.saturating_duration_since(last.captured_at))
                }
                None => return captured,
            },
        };
        self.audio_start + Self::span(position)
    }

    fn position(since_audio_start: Duration) -> usize {
        (since_audio_start.as_nanos() / u128::from(SAMPLE_NANOS)) as usize
    }

    fn span(samples: usize) -> Duration {
        Duration::from_nanos(samples as u64 * SAMPLE_NANOS)
    }

    /// The samples heard from `since` on, as far as they are still held, and when the first
    /// of them was captured: the audio to send a service that missed it.
    pub fn replay_from(&self, since: Instant) -> (Instant, Vec<f32>) {
        let skip = self.skip_to(since);
        let first = Duration::from_secs_f32((self.first + skip) as f32 / SAMPLE_RATE);
        (
            self.audio_start + first,
            self.samples.range(skip..).copied().collect(),
        )
    }

    fn skip_to(&self, since: Instant) -> usize {
        let position = (since
            .saturating_duration_since(self.audio_start)
            .as_secs_f32()
            * SAMPLE_RATE) as usize;
        position.saturating_sub(self.first).min(self.samples.len())
    }

    /// The samples heard from `since` on, as far as they are still held.
    pub fn since(&self, since: Instant) -> AudioBuffer16k {
        AudioBuffer16k::new(
            self.samples
                .range(self.skip_to(since)..)
                .map(|s| (s.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16)
                .collect(),
        )
    }
}

/// The stream date of the moment `captured`, as `heard` holds it; that moment itself when
/// the held audio cannot be read.
pub fn dated(heard: &Mutex<HeardAudio>, captured: Instant) -> Instant {
    heard
        .lock()
        .map_or(captured, |heard| heard.dated_at(captured))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One second of 16 kHz samples.
    const SECOND: usize = AudioBuffer16k::SAMPLE_RATE_HZ as usize;

    #[test]
    fn should_give_the_audio_heard_since_a_moment() {
        // Given one second heard
        let start = Instant::now();
        let mut heard = HeardAudio::new(start);
        heard.push(&vec![0.5; SAMPLE_RATE as usize], start);
        // When
        let last_quarter = heard.since(start + Duration::from_millis(750));
        // Then
        assert_eq!(last_quarter.duration_ms(), 250);
    }

    #[test]
    fn should_map_a_stream_position_to_its_capture_time_after_audio_was_lost() {
        // Given one second heard, three seconds that never arrived, then one more second
        let start = Instant::now();
        let mut heard = HeardAudio::new(start);
        heard.push(
            &vec![0.0; SAMPLE_RATE as usize],
            start + Duration::from_secs(1),
        );
        heard.push(
            &vec![0.0; SAMPLE_RATE as usize],
            start + Duration::from_secs(5),
        );
        // When the middle of the second second, dated by its stream position, is mapped
        let captured = heard.captured_at(start + Duration::from_millis(1_500));
        // Then it was captured after the lost seconds
        assert_eq!(captured, start + Duration::from_millis(4_500));
    }

    #[test]
    fn should_map_a_stream_position_to_its_own_moment_when_no_audio_was_lost() {
        // Given two seconds heard as they were captured
        let start = Instant::now();
        let mut heard = HeardAudio::new(start);
        heard.push(
            &vec![0.0; SAMPLE_RATE as usize],
            start + Duration::from_secs(1),
        );
        heard.push(
            &vec![0.0; SAMPLE_RATE as usize],
            start + Duration::from_secs(2),
        );
        // When
        let captured = heard.captured_at(start + Duration::from_millis(750));
        // Then
        assert_eq!(captured, start + Duration::from_millis(750));
    }

    #[test]
    fn should_date_a_capture_moment_by_its_stream_position_after_audio_was_lost() {
        // Given one second heard, three seconds that never arrived, then one more second
        let start = Instant::now();
        let mut heard = HeardAudio::new(start);
        heard.push(&vec![0.0; SECOND], start + Duration::from_secs(1));
        heard.push(&vec![0.0; SECOND], start + Duration::from_secs(5));
        // When the moment the last sample was captured is dated
        let dated = heard.dated_at(start + Duration::from_secs(5));
        // Then it lies where that sample stands in the stream
        assert_eq!(dated, start + Duration::from_secs(2));
    }

    #[test]
    fn should_replay_the_speech_before_a_loss_after_audio_was_lost_earlier() {
        // Given one second heard, three seconds that never arrived, then two more seconds
        let start = Instant::now();
        let mut heard = HeardAudio::new(start);
        heard.push(&vec![0.0; SECOND], start + Duration::from_secs(1));
        heard.push(&vec![0.0; 2 * SECOND], start + Duration::from_secs(6));
        // When the service is lost as the last sample is captured and the replay reaches
        // back to the sample captured one second before the loss
        let second_before_loss = heard.dated_at(start + Duration::from_secs(5));
        let (_, samples) = heard.replay_from(second_before_loss);
        // Then the second before the loss is sent again
        assert_eq!(samples.len(), SECOND);
    }

    #[test]
    fn should_start_the_replay_five_seconds_before_a_loss_after_audio_was_lost_earlier() {
        // Given one second heard, three seconds that never arrived, then six more seconds
        let start = Instant::now();
        let held = Mutex::new(HeardAudio::new(start));
        let mut heard = held.lock().unwrap();
        heard.push(&vec![0.0; SECOND], start + Duration::from_secs(1));
        heard.push(&vec![0.0; 6 * SECOND], start + Duration::from_secs(10));
        drop(heard);
        // When the service is lost as the last sample is captured, after a command that
        // ended at the stream date of four seconds
        let lost_at = start + Duration::from_secs(10);
        let timings = super::super::link::LinkTimings::default();
        let from_loss = timings.replay_start(dated(&held, lost_at), None);
        let after_command =
            timings.replay_start(dated(&held, lost_at), Some(start + Duration::from_secs(4)));
        // Then the five seconds before the loss are sent again, or those after the command
        let heard = held.lock().unwrap();
        assert_eq!(heard.replay_from(from_loss).1.len(), 5 * SECOND);
        assert_eq!(heard.replay_from(after_command).1.len(), 3 * SECOND);
    }

    #[test]
    fn should_hold_only_the_last_seconds() {
        // Given twelve seconds heard
        let start = Instant::now();
        let mut heard = HeardAudio::new(start);
        for _ in 0..12 {
            heard.push(&vec![0.0; SAMPLE_RATE as usize], Instant::now());
        }
        // When asked from the start
        let all = heard.since(start);
        // Then only the held seconds come back
        assert_eq!(all.duration_ms(), 10_000);
    }

    #[test]
    fn should_replay_the_held_audio_from_a_moment_with_the_capture_time_of_its_first_sample() {
        // Given twelve seconds heard, ten of them held
        let start = Instant::now();
        let mut heard = HeardAudio::new(start);
        for _ in 0..12 {
            heard.push(&vec![0.0; SAMPLE_RATE as usize], Instant::now());
        }
        // When
        let (first_at, samples) = heard.replay_from(start + Duration::from_secs(9));
        // Then
        assert_eq!(first_at, start + Duration::from_secs(9));
        assert_eq!(samples.len(), 3 * SAMPLE_RATE as usize);
    }

    #[test]
    fn should_replay_from_the_oldest_sample_held_when_asked_for_earlier() {
        // Given twelve seconds heard, ten of them held
        let start = Instant::now();
        let mut heard = HeardAudio::new(start);
        for _ in 0..12 {
            heard.push(&vec![0.0; SAMPLE_RATE as usize], Instant::now());
        }
        // When
        let (first_at, samples) = heard.replay_from(start);
        // Then
        assert_eq!(first_at, start + Duration::from_secs(2));
        assert_eq!(samples.len(), 10 * SAMPLE_RATE as usize);
    }
}
