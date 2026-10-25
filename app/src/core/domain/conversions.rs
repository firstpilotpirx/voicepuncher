//! Conversions of sample counts, sample values, indexes and durations that a cast would
//! leave unchecked.

/// `x` as a `usize`, rounded toward zero; a float cast saturates, so NaN gives 0 and a value
/// outside the range its nearest end.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a float cast saturates by definition"
)]
pub fn saturating_usize(x: impl Into<f64>) -> usize {
    x.into() as usize
}

/// `x` as a `u64`, rounded toward zero; a float cast saturates, so NaN gives 0 and a value
/// outside the range its nearest end.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a float cast saturates by definition"
)]
pub fn saturating_u64(x: impl Into<f64>) -> u64 {
    x.into() as u64
}

/// `x` as a `i16`, rounded toward zero; a float cast saturates, so NaN gives 0 and a value
/// outside the range its nearest end.
#[expect(
    clippy::cast_possible_truncation,
    reason = "a float cast saturates by definition"
)]
pub fn saturating_i16(x: impl Into<f64>) -> i16 {
    x.into() as i16
}

/// `x` as an `f32`, rounded to the nearest; a value outside its range gives an infinity.
#[expect(
    clippy::cast_possible_truncation,
    reason = "a float narrowing rounds by definition"
)]
pub fn narrow_f32(x: f64) -> f32 {
    x as f32
}

/// `index` as the `isize` an `AppKit` call takes; an index stays below `isize::MAX`, as every
/// allocation does.
pub fn index_isize(index: usize) -> isize {
    isize::try_from(index).unwrap_or(isize::MAX)
}

/// `duration` in whole milliseconds, `u64::MAX` past it.
pub fn millis_u64(duration: std::time::Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_saturate_float_casts_at_the_ends_of_the_range() {
        assert_eq!(saturating_usize(-1.0), 0);
        assert_eq!(saturating_usize(f64::NAN), 0);
        assert_eq!(saturating_u64(2.9), 2);
        assert_eq!(saturating_i16(40_000.0), i16::MAX);
        assert_eq!(saturating_i16(-40_000.0), i16::MIN);
    }

    #[test]
    fn should_narrow_indexes_and_durations_without_wrapping() {
        assert!((narrow_f32(0.1) - 0.1_f32).abs() < f32::EPSILON);
        assert_eq!(index_isize(usize::MAX), isize::MAX);
        assert_eq!(millis_u64(std::time::Duration::from_millis(1_500)), 1_500);
        assert_eq!(millis_u64(std::time::Duration::MAX), u64::MAX);
    }
}
