use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};

use crate::core::application::audio_recorder::AudioRecorder;
use crate::core::domain::AudioBuffer16k;

pub struct FileSystemAudioRecorder {
    path: PathBuf,
}

impl FileSystemAudioRecorder {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn load(path: &Path) -> Result<AudioBuffer16k> {
        let mut reader = hound::WavReader::open(path)
            .map_err(|e| anyhow!("failed to open WAV file {path:?}: {e}"))?;
        let spec = reader.spec();
        if spec.sample_rate != AudioBuffer16k::SAMPLE_RATE_HZ {
            anyhow::bail!(
                "WAV sample rate is {} Hz; expected {} Hz",
                spec.sample_rate,
                AudioBuffer16k::SAMPLE_RATE_HZ
            );
        }
        if spec.channels != 1 {
            anyhow::bail!(
                "WAV must be mono (1 channel); got {} channels",
                spec.channels
            );
        }
        if spec.bits_per_sample != 16 {
            anyhow::bail!("WAV must be 16-bit PCM; got {}-bit", spec.bits_per_sample);
        }
        let samples: Result<Vec<i16>, _> = reader.samples::<i16>().collect();
        let samples = samples.map_err(|e| anyhow!("failed to read WAV samples: {e}"))?;
        Ok(AudioBuffer16k::new(samples))
    }
}

impl AudioRecorder for FileSystemAudioRecorder {
    fn start(&mut self) -> Result<()> {
        Ok(())
    }

    fn stop(&mut self) -> Result<AudioBuffer16k> {
        Self::load(&self.path)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

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
    fn should_fail_to_load_non_existent_file() {
        // Given
        let path = PathBuf::from("/nonexistent/audio.wav");
        // When
        let result = FileSystemAudioRecorder::load(&path);
        // Then
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string().to_lowercase();
        assert!(
            msg.contains("failed to open") || msg.contains("nonexistent"),
            "unexpected error: {msg}"
        );
    }

    #[test]
    fn should_fail_to_load_wav_with_wrong_sample_rate() {
        // Given
        let tmp = tempfile::NamedTempFile::new().unwrap();
        write_wav(tmp.path(), 44_100, 1, 16, &[0i16; 100]);
        // When
        let result = FileSystemAudioRecorder::load(tmp.path());
        // Then
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("44100") || msg.contains("sample rate"),
            "unexpected error: {msg}"
        );
    }

    #[test]
    fn should_fail_to_load_stereo_wav() {
        // Given
        let tmp = tempfile::NamedTempFile::new().unwrap();
        write_wav(
            tmp.path(),
            AudioBuffer16k::SAMPLE_RATE_HZ,
            2,
            16,
            &[0i16; 100],
        );
        // When
        let result = FileSystemAudioRecorder::load(tmp.path());
        // Then
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("2") || msg.contains("channel") || msg.contains("mono"),
            "unexpected error: {msg}"
        );
    }

    #[test]
    fn should_load_valid_16k_mono_16bit_wav() {
        // Given
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let samples: Vec<i16> = (0..160).map(|i| (i * 100) as i16).collect();
        write_wav(tmp.path(), AudioBuffer16k::SAMPLE_RATE_HZ, 1, 16, &samples);
        // When
        let result = FileSystemAudioRecorder::load(tmp.path());
        // Then
        assert!(result.is_ok());
        let buf = result.unwrap();
        assert_eq!(buf.samples().len(), samples.len());
    }

    #[test]
    fn should_load_via_recorder_start_stop_cycle() {
        // Given
        let tmp = tempfile::NamedTempFile::new().unwrap();
        write_wav(
            tmp.path(),
            AudioBuffer16k::SAMPLE_RATE_HZ,
            1,
            16,
            &[1000i16; 80],
        );
        let mut recorder = FileSystemAudioRecorder::new(tmp.path().to_path_buf());
        recorder.start().unwrap();
        // When
        let audio = recorder.stop().unwrap();
        // Then
        assert_eq!(audio.samples().len(), 80);
    }
}
