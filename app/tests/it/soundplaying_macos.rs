//! The macOS sound player with the real `afplay`; its other tests sit beside it, in its module.

use voicepuncher::adapters::secondary::soundplaying::macos::MacosSoundPlayer;
use voicepuncher::core::application::SoundPlayer;

#[test]
fn should_not_panic_when_playing_start_sound_with_valid_name() {
    // Given
    let player = MacosSoundPlayer::new("Glass", "", "");
    // When / Then — fire-and-forget spawn must not panic
    player.play_start();
}
