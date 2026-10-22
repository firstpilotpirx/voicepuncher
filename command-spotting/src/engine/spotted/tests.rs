use super::*;

const SAID_END: usize = 32_000;
const UTTERANCE: u64 = 3;

#[test]
fn should_ask_the_check_once_the_next_word_has_begun() {
    // Given
    let mut checks = SpottedChecks::default();
    checks.spotted(SAID_END);
    let due = SAID_END + samples_in(CHECK_AFTER_SECS);
    // When
    let early = checks.take_due(due - 1);
    let on_time = checks.take_due(due);
    // Then
    assert_eq!(early, None);
    assert_eq!(
        on_time,
        Some(DueCheck { said_end: SAID_END, due, round: Round::First, utterance: None })
    );
    assert_eq!(checks.take_due(due + 1), None);
}

#[test]
fn should_keep_the_time_of_the_first_phrase_spotted() {
    // Given two phrases spotted before the first one's check is due
    let mut checks = SpottedChecks::default();
    checks.spotted(SAID_END);
    checks.spotted(SAID_END + samples_in(0.2));
    // When
    let asked = checks.take_due(SAID_END + samples_in(CHECK_AFTER_SECS));
    // Then
    assert_eq!(asked.map(|c| c.said_end), Some(SAID_END));
}

#[test]
fn should_ask_the_second_check_0_3_s_after_the_audio_of_the_first() {
    // Given the first check found no command in audio up to `heard_to`
    let mut checks = SpottedChecks::default();
    let heard_to = SAID_END + samples_in(CHECK_AFTER_SECS);
    checks.found_nothing(SAID_END, heard_to, UTTERANCE);
    let due = heard_to + samples_in(SECOND_CHECK_AFTER_SECS);
    // When
    let early = checks.take_due(due - 1);
    let on_time = checks.take_due(due);
    // Then
    assert_eq!(early, None);
    assert_eq!(
        on_time.map(|c| (c.round, c.utterance)),
        Some((Round::Second, Some(UTTERANCE)))
    );
}

#[test]
fn should_keep_a_waiting_check_when_a_second_round_is_asked() {
    // Given a phrase spotted later than the one whose first check found nothing
    let mut checks = SpottedChecks::default();
    checks.spotted(48_000);
    // When
    checks.found_nothing(16_000, 32_000, 1);
    // Then both are asked, the earlier due first
    let first = checks.take_due(usize::MAX).map(|c| c.round);
    let second = checks.take_due(usize::MAX).map(|c| c.round);
    assert_eq!((first, second), (Some(Round::Second), Some(Round::First)));
    assert_eq!(checks.take_due(usize::MAX), None);
}
