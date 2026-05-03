use std::path::PathBuf;

use voicepuncher::adapters::secondary::file_system_audio_recorder::FileSystemAudioRecorder;
use voicepuncher::core::application::ports::AudioRecorder;

#[test]
fn should_skip_gracefully_when_test_wav_absent() {
    // Given
    let path = match std::env::var("VOICEPUNCHER_TEST_WAV") {
        Ok(p) => p,
        Err(_) => {
            eprintln!("GATED: VOICEPUNCHER_TEST_WAV not set, skipping");
            return;
        }
    };
    // When
    let mut recorder = FileSystemAudioRecorder::new(PathBuf::from(path));
    let result = recorder.start();
    // Then
    assert!(result.is_ok());
}

#[test]
fn should_load_wav_and_return_audio_buffer() {
    // Given
    let path = match std::env::var("VOICEPUNCHER_TEST_WAV") {
        Ok(p) => p,
        Err(_) => {
            eprintln!("GATED: VOICEPUNCHER_TEST_WAV not set, skipping");
            return;
        }
    };
    let mut recorder = FileSystemAudioRecorder::new(PathBuf::from(path));
    recorder.start().expect("start should succeed");
    // When
    let audio = recorder.stop().expect("stop should return audio buffer");
    // Then
    assert!(!audio.is_empty());
}
