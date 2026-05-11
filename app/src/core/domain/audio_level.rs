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

pub fn level_to_block(v: f32) -> char {
    const BLOCKS: [char; 9] = [' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    let idx = if v < 0.002 {
        0
    } else if v < 0.004 {
        1
    } else if v < 0.008 {
        2
    } else if v < 0.014 {
        3
    } else if v < 0.022 {
        4
    } else if v < 0.032 {
        5
    } else if v < 0.045 {
        6
    } else if v < 0.060 {
        7
    } else {
        8
    };
    BLOCKS[idx]
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

    #[test]
    fn should_map_zero_to_silence() {
        assert_eq!(level_to_block(0.0), ' ');
    }

    #[test]
    fn should_map_below_silence_threshold_to_space() {
        assert_eq!(level_to_block(0.001), ' ');
    }

    #[test]
    fn should_map_0_003_to_lowest_block() {
        assert_eq!(level_to_block(0.003), '▁');
    }

    #[test]
    fn should_map_0_006_to_second_block() {
        assert_eq!(level_to_block(0.006), '▂');
    }

    #[test]
    fn should_map_0_011_to_third_block() {
        assert_eq!(level_to_block(0.011), '▃');
    }

    #[test]
    fn should_map_0_018_to_fourth_block() {
        assert_eq!(level_to_block(0.018), '▄');
    }

    #[test]
    fn should_map_0_027_to_fifth_block() {
        assert_eq!(level_to_block(0.027), '▅');
    }

    #[test]
    fn should_map_0_038_to_sixth_block() {
        assert_eq!(level_to_block(0.038), '▆');
    }

    #[test]
    fn should_map_0_052_to_seventh_block() {
        assert_eq!(level_to_block(0.052), '▇');
    }

    #[test]
    fn should_map_0_07_to_full_block() {
        assert_eq!(level_to_block(0.07), '█');
    }

    #[test]
    fn should_map_0_5_to_full_block() {
        assert_eq!(level_to_block(0.5), '█');
    }

    #[test]
    fn should_map_1_0_to_full_block() {
        assert_eq!(level_to_block(1.0), '█');
    }
}
