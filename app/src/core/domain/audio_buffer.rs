#[derive(Debug, Clone)]
pub struct AudioBuffer16k(Vec<i16>);

impl AudioBuffer16k {
    pub const SAMPLE_RATE_HZ: u32 = 16_000;

    pub fn new(samples: Vec<i16>) -> Self {
        Self(samples)
    }

    pub fn samples(&self) -> &[i16] {
        &self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn duration_ms(&self) -> u64 {
        (self.0.len() as u64 * 1000) / Self::SAMPLE_RATE_HZ as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_expose_sample_rate_constant_16khz() {
        // Given
        let rate = AudioBuffer16k::SAMPLE_RATE_HZ;
        // When
        // (constant access only)
        // Then
        assert_eq!(rate, 16_000);
    }

    #[test]
    fn should_compute_duration_for_one_second_of_samples() {
        // Given
        let buffer = AudioBuffer16k::new(vec![0; 16_000]);
        // When
        let duration = buffer.duration_ms();
        // Then
        assert_eq!(duration, 1000);
    }

    #[test]
    fn should_report_empty_for_zero_samples() {
        // Given
        let buffer = AudioBuffer16k::new(vec![]);
        // When
        let empty = buffer.is_empty();
        let duration = buffer.duration_ms();
        // Then
        assert!(empty);
        assert_eq!(duration, 0);
    }

    #[test]
    fn should_expose_samples_slice() {
        // Given
        let buffer = AudioBuffer16k::new(vec![1, 2, 3]);
        // When
        let samples = buffer.samples();
        // Then
        assert_eq!(samples, &[1_i16, 2, 3]);
    }
}
