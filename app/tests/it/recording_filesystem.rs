use std::path::PathBuf;

use voicepuncher::adapters::secondary::recording::filesystem::FileSystemAudioRecorder;
use voicepuncher::core::application::AudioRecorder;
use voicepuncher::core::domain::AudioBuffer16k;

fn write_wav(
    path: &std::path::Path,
    sample_rate: u32,
    channels: u16,
    bits_per_sample: u16,
    samples: &[i16],
) {
    let spec = hound::WavSpec {
        channels,
        sample_rate,
        bits_per_sample,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).unwrap();
    for &s in samples {
        writer.write_sample(s).unwrap();
    }
    writer.finalize().unwrap();
}

#[test]
fn should_fail_when_path_is_non_existent() {
    // Given
    let mut recorder = FileSystemAudioRecorder::new(PathBuf::from("/nonexistent/audio.wav"));
    recorder.start().unwrap();
    // When
    let result = recorder.stop();
    // Then
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string().to_lowercase();
    assert!(
        msg.contains("failed to open") || msg.contains("nonexistent"),
        "unexpected error: {msg}"
    );
}

#[test]
fn should_fail_when_wav_has_wrong_sample_rate() {
    // Given — 44100 Hz instead of the required 16000 Hz
    let tmp = tempfile::NamedTempFile::new().unwrap();
    write_wav(tmp.path(), 44_100, 1, 16, &[0i16; 100]);
    let mut recorder = FileSystemAudioRecorder::new(tmp.path().to_path_buf());
    recorder.start().unwrap();
    // When
    let result = recorder.stop();
    // Then
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("44100") || msg.contains("sample rate"),
        "unexpected error: {msg}"
    );
}

#[test]
fn should_fail_when_wav_is_stereo() {
    // Given — 2 channels instead of required mono
    let tmp = tempfile::NamedTempFile::new().unwrap();
    write_wav(
        tmp.path(),
        AudioBuffer16k::SAMPLE_RATE_HZ,
        2,
        16,
        &[0i16; 100],
    );
    let mut recorder = FileSystemAudioRecorder::new(tmp.path().to_path_buf());
    recorder.start().unwrap();
    // When
    let result = recorder.stop();
    // Then
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains('2') || msg.contains("channel") || msg.contains("mono"),
        "unexpected error: {msg}"
    );
}

#[test]
fn should_load_valid_16k_mono_16bit_wav() {
    // Given
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let samples: Vec<i16> = (0i16..160).collect();
    write_wav(tmp.path(), AudioBuffer16k::SAMPLE_RATE_HZ, 1, 16, &samples);
    let mut recorder = FileSystemAudioRecorder::new(tmp.path().to_path_buf());
    recorder.start().unwrap();
    // When
    let audio = recorder.stop().unwrap();
    // Then
    assert_eq!(audio.samples().len(), samples.len());
    assert!(!audio.is_empty());
}
