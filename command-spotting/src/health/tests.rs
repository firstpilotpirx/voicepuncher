use super::*;

const LATER: Duration = Duration::from_secs(11);

#[test]
fn should_be_healthy_without_connections() {
    assert_eq!(Health::default().stuck(), None);
}

#[test]
fn should_count_a_connection_with_audio_waiting_and_no_progress_as_stuck() {
    // Given
    let health = Health::default();
    let beat = health.register(4);
    // When
    beat.queued();
    // Then
    assert_eq!(health.stuck_at(Instant::now() + LATER), Some(4));
}

#[test]
fn should_not_count_an_idle_connection_as_stuck() {
    // Given
    let health = Health::default();
    let _beat = health.register(4);
    // Then
    assert_eq!(health.stuck_at(Instant::now() + LATER), None);
}

#[test]
fn should_not_count_a_connection_that_keeps_up_as_stuck() {
    // Given
    let health = Health::default();
    let beat = health.register(4);
    // When
    beat.queued();
    beat.processed();
    beat.alive();
    // Then
    assert_eq!(health.stuck_at(Instant::now() + LATER), None);
}

#[test]
fn should_forget_a_closed_connection() {
    // Given
    let health = Health::default();
    health.register(4).queued();
    // When
    health.unregister(4);
    // Then
    assert_eq!(health.stuck_at(Instant::now() + LATER), None);
}

#[test]
fn should_count_a_connection_whose_confirmation_stalled_as_stuck() {
    // Given
    let health = Health::default();
    let beat = health.register(4);
    // When
    beat.stalled(true);
    // Then
    assert_eq!(health.stuck_at(Instant::now()), Some(4));
}

#[test]
fn should_count_a_connection_healthy_again_once_its_stalled_confirmation_ended() {
    // Given
    let health = Health::default();
    let beat = health.register(4);
    beat.stalled(true);
    // When
    beat.stalled(false);
    // Then
    assert_eq!(health.stuck_at(Instant::now()), None);
}

#[test]
fn should_take_audio_sent_faster_than_real_time_while_processing_keeps_up() {
    // Given 30 s of audio arriving at once, as a replay or a test feeds it
    let beat = Health::default().register(4);
    let now = Instant::now();
    // When
    let taken = (0..300).all(|_| beat.queued_at(now));
    // Then
    assert!(taken);
}

#[test]
fn should_drop_audio_once_the_oldest_waiting_audio_waited_past_ten_seconds() {
    // Given
    let beat = Health::default().register(4);
    let now = Instant::now();
    beat.queued_at(now);
    // When
    let within = beat.queued_at(now + Duration::from_secs(10));
    let past = beat.queued_at(now + LATER);
    // Then
    assert!(within);
    assert!(!past);
}

#[test]
fn should_take_audio_again_once_the_waiting_audio_was_processed() {
    // Given
    let beat = Health::default().register(4);
    let now = Instant::now();
    beat.queued_at(now);
    // When
    beat.processed();
    // Then
    assert!(beat.queued_at(now + LATER));
}
