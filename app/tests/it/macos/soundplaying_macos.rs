//! The macOS sound player with the real `afplay`; its other tests sit beside it, in its module.

use std::time::{Duration, Instant};

use voicepuncher::adapters::secondary::soundplaying::macos::MacosSoundPlayer;
use voicepuncher::core::application::SoundPlayer;

#[test]
fn should_start_the_sound_without_waiting_for_it_to_end() {
    // Given — Glass plays for about a second
    let player = MacosSoundPlayer::new("Glass", "", "");
    let started = Instant::now();
    // When
    player.play_start();
    // Then
    assert!(started.elapsed() < Duration::from_millis(500));
}
