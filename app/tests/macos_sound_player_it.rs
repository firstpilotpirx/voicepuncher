use voicepuncher::adapters::secondary::macos_sound_player::MacosSoundPlayer;
use voicepuncher::core::application::ports::SoundPlayer;

#[test]
fn should_not_panic_when_playing_start_sound_with_valid_name() {
    // Given
    let player = MacosSoundPlayer::new("Glass", "");
    // When / Then — fire-and-forget spawn must not panic
    player.play_start();
}

#[test]
fn should_not_panic_when_playing_stop_sound_with_valid_name() {
    // Given
    let player = MacosSoundPlayer::new("", "Pop");
    // When / Then
    player.play_stop();
}

#[test]
fn should_not_panic_when_both_sounds_are_empty() {
    // Given
    let player = MacosSoundPlayer::new("", "");
    // When / Then
    player.play_start();
    player.play_stop();
}

#[test]
fn should_not_panic_when_sound_name_does_not_exist() {
    // Given — unknown name; afplay will fail but we ignore errors
    let player = MacosSoundPlayer::new("NonExistentSoundXYZ", "");
    // When / Then
    player.play_start();
}
