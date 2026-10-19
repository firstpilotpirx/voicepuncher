#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AudioLevel(f32);

impl AudioLevel {
    pub const SILENT: Self = Self(0.0);

    pub fn from_rms(rms: f32) -> Self {
        let clamped = if rms.is_nan() || rms.is_infinite() {
            0.0
        } else {
            rms.clamp(0.0, 1.0)
        };
        Self(clamped)
    }

    pub fn value(self) -> f32 {
        self.0
    }

    pub fn to_bits(self) -> u32 {
        self.0.to_bits()
    }

    pub fn from_bits(bits: u32) -> Self {
        Self::from_rms(f32::from_bits(bits))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_clamp_nan_to_silent() {
        assert_eq!(AudioLevel::from_rms(f32::NAN).value(), 0.0);
    }

    #[test]
    fn should_clamp_positive_infinity_to_silent() {
        assert_eq!(AudioLevel::from_rms(f32::INFINITY).value(), 0.0);
    }

    #[test]
    fn should_clamp_negative_infinity_to_silent() {
        assert_eq!(AudioLevel::from_rms(f32::NEG_INFINITY).value(), 0.0);
    }

    #[test]
    fn should_clamp_negative_rms_to_zero() {
        assert_eq!(AudioLevel::from_rms(-0.5).value(), 0.0);
    }

    #[test]
    fn should_clamp_rms_above_one_to_one() {
        assert_eq!(AudioLevel::from_rms(1.5).value(), 1.0);
    }

    #[test]
    fn should_round_trip_bits_for_zero() {
        let level = AudioLevel::from_rms(0.0);
        assert_eq!(
            AudioLevel::from_bits(level.to_bits()).value(),
            level.value()
        );
    }

    #[test]
    fn should_round_trip_bits_for_half() {
        let level = AudioLevel::from_rms(0.5);
        assert_eq!(
            AudioLevel::from_bits(level.to_bits()).value(),
            level.value()
        );
    }

    #[test]
    fn should_round_trip_bits_for_one() {
        let level = AudioLevel::from_rms(1.0);
        assert_eq!(
            AudioLevel::from_bits(level.to_bits()).value(),
            level.value()
        );
    }
}
