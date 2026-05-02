use voicepuncher::adapters::secondary::cpal_audio_recorder::CpalAudioRecorder;
use voicepuncher::core::application::ports::AudioRecorder;

#[test]
fn should_record_and_resample_when_real_mic_available() {
    // Given
    if std::env::var("VOICEPUNCHER_REAL_MIC").is_err() {
        eprintln!("GATED: VOICEPUNCHER_REAL_MIC=1 not set, skipping mic IT");
        return;
    }
    // When
    let mut recorder = CpalAudioRecorder::new(None);
    recorder.start().expect("start should succeed");
    std::thread::sleep(std::time::Duration::from_millis(500));
    let audio = recorder.stop().expect("stop should succeed");
    // Then
    assert!(
        !audio.samples().is_empty(),
        "expected non-empty audio buffer"
    );
    assert!(
        audio.duration_ms() > 300,
        "expected > 300ms of audio, got {}ms",
        audio.duration_ms()
    );
}
