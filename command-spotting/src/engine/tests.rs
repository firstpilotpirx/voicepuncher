use super::*;

const TOLERANCE: f32 = 1e-4;

fn assert_close(actual: (f32, f32), expected: (f32, f32)) {
    assert!(
        (actual.0 - expected.0).abs() < TOLERANCE && (actual.1 - expected.1).abs() < TOLERANCE,
        "got {actual:?}, expected {expected:?}"
    );
}

#[test]
fn should_end_the_phrase_where_the_spotter_has_decoded_to() {
    // Given token times of a phrase that took 0.6 s between its first and last token
    let token_times = [0.32, 0.5, 0.92];
    // When
    let span = phrase_span(27.4, &token_times);
    // Then
    let end = 27.4 - DETECTION_LAG_SECS;
    let duration = 0.6 + FIRST_TOKEN_SECS + LAST_TOKEN_SECS;
    assert_close(span, (end - duration, end));
}

#[test]
fn should_place_a_phrase_late_in_a_long_stream_whatever_its_token_times_count_from() {
    // Given the same phrase reported with token times from two different origins
    let early = phrase_span(60.0, &[0.3, 0.9]);
    let late = phrase_span(60.0, &[45.3, 45.9]);
    // Then both sit at the end of the audio fed so far
    assert_close(early, late);
    assert!(early.0 > 58.0, "{early:?}");
}

#[test]
fn should_not_count_a_stray_first_token_heard_in_noise() {
    // Given the first token fired in noise half a second before the rest of the phrase
    let with_stray = phrase_span(27.0, &[0.32, 0.8, 0.88, 1.08]);
    let without = phrase_span(27.0, &[0.8, 0.88, 1.08]);
    // Then
    assert_close(with_stray, without);
}

#[test]
fn should_keep_a_quiet_first_word_a_little_apart_from_the_next() {
    // Given a quiet "start" decoded 0.32 s before the first token of "dictation"
    let token_times = [1.32, 1.64, 1.72, 1.8, 1.92];
    // When
    let span = phrase_span(2.4, &token_times);
    // Then the phrase starts at the first token
    let duration = 0.6 + FIRST_TOKEN_SECS + LAST_TOKEN_SECS;
    assert_close(
        span,
        (
            2.4 - DETECTION_LAG_SECS - duration,
            2.4 - DETECTION_LAG_SECS,
        ),
    );
}

#[test]
fn should_raise_the_detector_input_by_the_detector_gain() {
    // When
    let heard = raised(&[0.002, -0.25, 0.0]);
    // Then
    assert_eq!(heard, [0.003, -0.375, 0.0]);
}

#[test]
fn should_clamp_the_raised_detector_input_to_full_scale() {
    // When
    let heard = raised(&[0.8, -0.7]);
    // Then
    assert_eq!(heard, [1.0, -1.0]);
}

#[test]
fn should_not_place_a_phrase_before_the_stream_started() {
    // When
    let (start, _) = phrase_span(0.5, &[0.1, 0.8]);
    // Then
    assert!((start - 0.0).abs() < 1e-6, "{} is not {}", start, 0.0);
}

#[test]
fn should_give_a_phrase_without_token_times_the_token_tails_only() {
    // When
    let (start, end) = phrase_span(10.0, &[]);
    // Then
    assert!((end - start - (FIRST_TOKEN_SECS + LAST_TOKEN_SECS)).abs() < TOLERANCE);
}
