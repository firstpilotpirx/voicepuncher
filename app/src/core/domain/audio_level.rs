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
    const BLOCKS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    let idx = if v < 0.01 {
        0
    } else if v < 0.025 {
        1
    } else if v < 0.05 {
        2
    } else if v < 0.09 {
        3
    } else if v < 0.15 {
        4
    } else if v < 0.22 {
        5
    } else if v < 0.32 {
        6
    } else {
        7
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
    fn should_map_zero_to_lowest_block() {
        assert_eq!(level_to_block(0.0), '▁');
    }

    #[test]
    fn should_map_below_threshold_to_lowest_block() {
        assert_eq!(level_to_block(0.005), '▁');
    }

    #[test]
    fn should_map_0_018_to_second_block() {
        assert_eq!(level_to_block(0.018), '▂');
    }

    #[test]
    fn should_map_0_035_to_third_block() {
        assert_eq!(level_to_block(0.035), '▃');
    }

    #[test]
    fn should_map_0_07_to_fourth_block() {
        assert_eq!(level_to_block(0.07), '▄');
    }

    #[test]
    fn should_map_0_12_to_fifth_block() {
        assert_eq!(level_to_block(0.12), '▅');
    }

    #[test]
    fn should_map_0_18_to_sixth_block() {
        assert_eq!(level_to_block(0.18), '▆');
    }

    #[test]
    fn should_map_0_27_to_seventh_block() {
        assert_eq!(level_to_block(0.27), '▇');
    }

    #[test]
    fn should_map_0_4_to_full_block() {
        assert_eq!(level_to_block(0.4), '█');
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
