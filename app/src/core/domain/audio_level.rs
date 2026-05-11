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

pub const PILL_COUNT: usize = 10;

pub fn level_to_pills(v: f32) -> String {
    let active = if v < 0.006 {
        0
    } else if v < 0.012 {
        1
    } else if v < 0.018 {
        2
    } else if v < 0.024 {
        3
    } else if v < 0.030 {
        4
    } else if v < 0.036 {
        5
    } else if v < 0.042 {
        6
    } else if v < 0.048 {
        7
    } else if v < 0.054 {
        8
    } else if v < 0.060 {
        9
    } else {
        PILL_COUNT
    };
    (0..PILL_COUNT)
        .map(|i| if i < active { '┃' } else { '┊' })
        .collect()
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
    fn should_render_ten_pills() {
        assert_eq!(level_to_pills(0.0).chars().count(), PILL_COUNT);
        assert_eq!(level_to_pills(1.0).chars().count(), PILL_COUNT);
    }

    #[test]
    fn should_render_all_empty_for_silence() {
        assert_eq!(level_to_pills(0.0), "┊┊┊┊┊┊┊┊┊┊");
    }

    #[test]
    fn should_render_all_empty_below_threshold() {
        assert_eq!(level_to_pills(0.005), "┊┊┊┊┊┊┊┊┊┊");
    }

    #[test]
    fn should_light_one_pill_at_0_009() {
        assert_eq!(level_to_pills(0.009), "┃┊┊┊┊┊┊┊┊┊");
    }

    #[test]
    fn should_light_two_pills_at_0_015() {
        assert_eq!(level_to_pills(0.015), "┃┃┊┊┊┊┊┊┊┊");
    }

    #[test]
    fn should_light_three_pills_at_0_021() {
        assert_eq!(level_to_pills(0.021), "┃┃┃┊┊┊┊┊┊┊");
    }

    #[test]
    fn should_light_four_pills_at_0_027() {
        assert_eq!(level_to_pills(0.027), "┃┃┃┃┊┊┊┊┊┊");
    }

    #[test]
    fn should_light_five_pills_at_0_033() {
        assert_eq!(level_to_pills(0.033), "┃┃┃┃┃┊┊┊┊┊");
    }

    #[test]
    fn should_light_six_pills_at_0_039() {
        assert_eq!(level_to_pills(0.039), "┃┃┃┃┃┃┊┊┊┊");
    }

    #[test]
    fn should_light_seven_pills_at_0_045() {
        assert_eq!(level_to_pills(0.045), "┃┃┃┃┃┃┃┊┊┊");
    }

    #[test]
    fn should_light_eight_pills_at_0_051() {
        assert_eq!(level_to_pills(0.051), "┃┃┃┃┃┃┃┃┊┊");
    }

    #[test]
    fn should_light_nine_pills_at_0_057() {
        assert_eq!(level_to_pills(0.057), "┃┃┃┃┃┃┃┃┃┊");
    }

    #[test]
    fn should_saturate_at_0_060() {
        assert_eq!(level_to_pills(0.060), "┃┃┃┃┃┃┃┃┃┃");
    }

    #[test]
    fn should_saturate_at_0_5() {
        assert_eq!(level_to_pills(0.5), "┃┃┃┃┃┃┃┃┃┃");
    }

    #[test]
    fn should_saturate_at_1_0() {
        assert_eq!(level_to_pills(1.0), "┃┃┃┃┃┃┃┃┃┃");
    }
}
