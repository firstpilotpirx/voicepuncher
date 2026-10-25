//! Float to integer conversions of the synthesized speech: sample indexes, values and rates.

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

/// `x` as a `u32`, rounded toward zero; a float cast saturates, so NaN gives 0 and a value
/// outside the range its nearest end.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a float cast saturates by definition"
)]
pub fn saturating_u32(x: impl Into<f64>) -> u32 {
    x.into() as u32
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
