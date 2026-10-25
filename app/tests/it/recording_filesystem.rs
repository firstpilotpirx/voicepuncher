use std::path::{Path, PathBuf};

use voicepuncher::adapters::secondary::recording::filesystem::FileSystemAudioRecorder;
use voicepuncher::core::application::AudioRecorder;
use voicepuncher::core::domain::AudioBuffer16k;

fn write_wav(path: &Path, sample_rate: u32, channels: u16, samples: &[i16]) {
    let spec = hound::WavSpec {
        channels,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).unwrap();
    for &s in samples {
        writer.write_sample(s).unwrap();
    }
    writer.finalize().unwrap();
}

/// What a recorder reading `path` hands over at the stop of one recording.
fn record_from(path: &Path) -> anyhow::Result<AudioBuffer16k> {
    let mut recorder = FileSystemAudioRecorder::new(path.to_path_buf());
    recorder.start().unwrap();
    recorder.stop()
}

/// The error a recorder reading a WAV file of `sample_rate` and `channels` fails with.
fn error_reading_wav(sample_rate: u32, channels: u16) -> String {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    write_wav(tmp.path(), sample_rate, channels, &[0i16; 100]);
    record_from(tmp.path()).unwrap_err().to_string()
}

#[test]
fn should_fail_when_path_is_non_existent() {
    // Given
    let path = PathBuf::from("/nonexistent/audio.wav");
    // When
    let result = record_from(&path);
    // Then
    let msg = result.unwrap_err().to_string().to_lowercase();
    assert!(
        msg.contains("failed to open") || msg.contains("nonexistent"),
        "unexpected error: {msg}"
    );
}

#[test]
fn should_fail_when_wav_has_wrong_sample_rate() {
    // Given — 44100 Hz instead of the required 16000 Hz
    let rate = 44_100;
    // When
    let msg = error_reading_wav(rate, 1);
    // Then
    assert!(
        msg.contains("44100") || msg.contains("sample rate"),
        "unexpected error: {msg}"
    );
}

#[test]
fn should_fail_when_wav_is_stereo() {
    // Given — 2 channels instead of required mono
    let channels = 2;
    // When
    let msg = error_reading_wav(AudioBuffer16k::SAMPLE_RATE_HZ, channels);
    // Then
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
    write_wav(tmp.path(), AudioBuffer16k::SAMPLE_RATE_HZ, 1, &samples);
    // When
    let audio = record_from(tmp.path()).unwrap();
    // Then
    assert_eq!(audio.samples().len(), samples.len());
    assert!(!audio.is_empty());
}
