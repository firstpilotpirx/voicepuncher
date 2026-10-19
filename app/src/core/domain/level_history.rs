use std::time::Duration;

use super::AudioLevel;

/// Time one column of the level waveform covers.
pub const COLUMN_PERIOD: Duration = Duration::from_millis(100);
/// Distance between neighbouring columns, in points.
pub const COLUMN_PITCH_PT: f32 = 6.0;
/// Height of a silent column (a dot), in points.
pub const MIN_HEIGHT_PT: f32 = 2.0;
/// Height of a column at full loudness, in points.
pub const MAX_HEIGHT_PT: f32 = 28.0;
/// Columns of a waveform that has the pill to itself: the last 3 s.
pub const FULL_CAPACITY: usize = 30;
/// Columns of a waveform that shares the pill with the commands-unavailable sign.
pub const CAPACITY_BESIDE_SIGN: usize = 22;

const RISE_RATE: f32 = 0.35;
const FALL_RATE: f32 = 0.12;
const QUIET_DBFS: f32 = -50.0;
const LOUD_DBFS: f32 = -10.0;

/// Recent loudness of the microphone as a strip of columns, one per
/// [`COLUMN_PERIOD`], that slides left as time passes: the model behind the
/// level waveform. Times are offsets from any fixed origin chosen by the caller.
#[derive(Clone, Debug)]
pub struct LevelHistory {
    columns: Vec<f32>,
    oldest: usize,
    level: f32,
    window_peak: f32,
    window_start: Duration,
}

const DECIBELS_PER_DECADE: f32 = 20.0;

impl LevelHistory {
    /// A history of `capacity` silent columns whose first column period
    /// starts at `start`. Allocates once; nothing later allocates.
    pub fn new(capacity: usize, start: Duration) -> Self {
        Self {
            columns: vec![0.0; capacity],
            oldest: 0,
            level: 0.0,
            window_peak: 0.0,
            window_start: start,
        }
    }

    /// Takes one microphone level heard at `at` into the smoothed level,
    /// first closing every column period that ended before it.
    pub fn record(&mut self, level: AudioLevel, at: Duration) {
        self.advance_to(at);
        let target = loudness(level);
        let rate = if target > self.level {
            RISE_RATE
        } else {
            FALL_RATE
        };
        self.level += (target - self.level) * rate;
        self.window_peak = self.window_peak.max(self.level);
    }

    /// Closes every column period that ended by `at`: each becomes the
    /// newest column and the oldest one is dropped.
    pub fn advance_to(&mut self, at: Duration) {
        let elapsed = at.saturating_sub(self.window_start);
        let period_nanos = COLUMN_PERIOD.as_nanos();
        let ended_periods = elapsed.as_nanos() / period_nanos;
        if ended_periods == 0 {
            return;
        }
        let new_columns = usize::try_from(ended_periods)
            .map_or(self.columns.len(), |ended| ended.min(self.columns.len()));
        for _ in 0..new_columns {
            self.push(self.window_peak);
            self.window_peak = self.level;
        }
        let into_current_period = (elapsed.as_nanos() % period_nanos) as u64;
        self.window_start = at - Duration::from_nanos(into_current_period);
    }

    /// The smoothed loudness, in [0, 1].
    pub fn level(&self) -> f32 {
        self.level
    }

    /// Height of every column in points, oldest (leftmost) first.
    pub fn column_heights(&self) -> impl Iterator<Item = f32> + '_ {
        let (newer, older) = self.columns.split_at(self.oldest);
        older
            .iter()
            .chain(newer)
            .map(|value| (value * MAX_HEIGHT_PT).max(MIN_HEIGHT_PT))
    }

    /// How far left the strip has slid at `at` since its newest column was
    /// added, in points: one pitch per column period, never more than one.
    pub fn slide_offset(&self, at: Duration) -> f32 {
        let elapsed = at.saturating_sub(self.window_start);
        let progress = elapsed.as_secs_f32() / COLUMN_PERIOD.as_secs_f32();
        progress.min(1.0) * COLUMN_PITCH_PT
    }

    fn push(&mut self, value: f32) {
        if let Some(slot) = self.columns.get_mut(self.oldest) {
            *slot = value;
            self.oldest = (self.oldest + 1) % self.columns.len();
        }
    }
}

/// The level in dBFS mapped linearly from [`QUIET_DBFS`] (0) to
/// [`LOUD_DBFS`] (1), clamped.
fn loudness(level: AudioLevel) -> f32 {
    let rms = level.value();
    if rms <= 0.0 {
        return 0.0;
    }
    let dbfs = DECIBELS_PER_DECADE * rms.log10();
    ((dbfs - QUIET_DBFS) / (LOUD_DBFS - QUIET_DBFS)).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const UPDATE_PERIOD: Duration = Duration::from_millis(20);
    const SPEECH_RMS: f32 = 0.3;
    const SILENCE_RMS: f32 = 0.001;

    fn feed(history: &mut LevelHistory, rms: f32, from: Duration, to: Duration) {
        let mut at = from;
        while at < to {
            history.record(AudioLevel::from_rms(rms), at);
            at += UPDATE_PERIOD;
        }
        history.advance_to(to);
    }

    fn secs(seconds: f32) -> Duration {
        Duration::from_secs_f32(seconds)
    }

    fn heights(history: &LevelHistory) -> Vec<f32> {
        history.column_heights().collect()
    }

    #[test]
    fn should_draw_a_new_history_as_all_dots() {
        // Given / When
        let history = LevelHistory::new(FULL_CAPACITY, Duration::ZERO);
        // Then
        assert_eq!(heights(&history), vec![MIN_HEIGHT_PT; FULL_CAPACITY]);
    }

    #[test]
    fn should_hold_as_many_columns_as_its_capacity() {
        // Given / When
        let history = LevelHistory::new(CAPACITY_BESIDE_SIGN, Duration::ZERO);
        // Then
        assert_eq!(history.column_heights().count(), CAPACITY_BESIDE_SIGN);
    }

    #[test]
    fn should_draw_speech_as_tall_columns() {
        // Given
        let mut history = LevelHistory::new(FULL_CAPACITY, Duration::ZERO);
        // When
        feed(&mut history, SPEECH_RMS, Duration::ZERO, secs(1.0));
        // Then
        let newest = heights(&history)[FULL_CAPACITY - 1];
        assert!(newest > 0.9 * MAX_HEIGHT_PT, "newest column {newest}");
    }

    #[test]
    fn should_draw_silence_as_dots() {
        // Given
        let mut history = LevelHistory::new(FULL_CAPACITY, Duration::ZERO);
        // When
        feed(&mut history, SILENCE_RMS, Duration::ZERO, secs(1.0));
        // Then
        assert_eq!(heights(&history), vec![MIN_HEIGHT_PT; FULL_CAPACITY]);
    }

    #[test]
    fn should_still_show_speech_from_less_than_three_seconds_ago() {
        // Given
        let mut history = LevelHistory::new(FULL_CAPACITY, Duration::ZERO);
        feed(&mut history, SPEECH_RMS, Duration::ZERO, secs(1.0));
        // When
        feed(&mut history, SILENCE_RMS, secs(1.0), secs(3.5));
        // Then
        let oldest = heights(&history)[0];
        assert!(oldest > 0.9 * MAX_HEIGHT_PT, "oldest column {oldest}");
    }

    #[test]
    fn should_forget_speech_older_than_three_seconds() {
        // Given
        let mut history = LevelHistory::new(FULL_CAPACITY, Duration::ZERO);
        feed(&mut history, SPEECH_RMS, Duration::ZERO, secs(1.0));
        // When
        feed(&mut history, SILENCE_RMS, secs(1.0), secs(4.5));
        // Then
        assert_eq!(heights(&history), vec![MIN_HEIGHT_PT; FULL_CAPACITY]);
    }

    #[test]
    fn should_rise_fast_towards_a_louder_level() {
        // Given
        let mut history = LevelHistory::new(FULL_CAPACITY, Duration::ZERO);
        // When
        history.record(AudioLevel::from_rms(1.0), Duration::ZERO);
        // Then
        assert!((history.level() - RISE_RATE).abs() < 1e-6);
    }

    #[test]
    fn should_fall_slower_towards_a_quieter_level() {
        // Given
        let mut history = LevelHistory::new(FULL_CAPACITY, Duration::ZERO);
        feed(&mut history, 1.0, Duration::ZERO, secs(1.0));
        let before = history.level();
        // When
        history.record(AudioLevel::SILENT, secs(1.0));
        // Then
        assert!((history.level() - before * (1.0 - FALL_RATE)).abs() < 1e-6);
    }

    #[test]
    fn should_keep_the_peak_of_its_period_in_a_column() {
        // Given
        let mut history = LevelHistory::new(FULL_CAPACITY, Duration::ZERO);
        history.record(AudioLevel::from_rms(1.0), Duration::ZERO);
        history.record(AudioLevel::SILENT, secs(0.05));
        // When
        history.advance_to(COLUMN_PERIOD);
        // Then
        let newest = heights(&history)[FULL_CAPACITY - 1];
        assert!((newest - RISE_RATE * MAX_HEIGHT_PT).abs() < 1e-4);
    }

    #[test]
    fn should_slide_one_pitch_per_column_period() {
        // Given
        let history = LevelHistory::new(FULL_CAPACITY, Duration::ZERO);
        // When
        let halfway = history.slide_offset(COLUMN_PERIOD / 2);
        // Then
        assert!((halfway - COLUMN_PITCH_PT / 2.0).abs() < 1e-4);
    }

    #[test]
    fn should_restart_the_slide_when_a_column_is_added() {
        // Given
        let mut history = LevelHistory::new(FULL_CAPACITY, Duration::ZERO);
        // When
        history.advance_to(COLUMN_PERIOD);
        // Then
        assert_eq!(history.slide_offset(COLUMN_PERIOD), 0.0);
    }

    #[test]
    fn should_slide_no_further_than_one_pitch_before_a_column_is_added() {
        // Given
        let history = LevelHistory::new(FULL_CAPACITY, Duration::ZERO);
        // When
        let offset = history.slide_offset(COLUMN_PERIOD * 3);
        // Then
        assert_eq!(offset, COLUMN_PITCH_PT);
    }

    #[test]
    fn should_map_minus_fifty_dbfs_and_below_to_no_loudness() {
        assert_eq!(loudness(AudioLevel::from_rms(0.003)), 0.0);
        assert_eq!(loudness(AudioLevel::SILENT), 0.0);
    }

    #[test]
    fn should_map_minus_ten_dbfs_and_above_to_full_loudness() {
        assert_eq!(loudness(AudioLevel::from_rms(0.316_3)), 1.0);
        assert_eq!(loudness(AudioLevel::from_rms(1.0)), 1.0);
    }

    #[test]
    fn should_map_dbfs_linearly_between_the_bounds() {
        let minus_forty_dbfs = AudioLevel::from_rms(0.01);
        assert!((loudness(minus_forty_dbfs) - 0.25).abs() < 1e-4);
    }
}
