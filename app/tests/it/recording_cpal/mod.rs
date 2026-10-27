mod kept_mic;
mod synthetic;

use std::sync::atomic::AtomicU32;
use std::sync::Arc;

use cpal::traits::{DeviceTrait, HostTrait};

use voicepuncher::adapters::secondary::recording::cpal::devices::{open, CpalInputDevices};
use voicepuncher::adapters::secondary::recording::cpal::CpalAudioRecorder;
use voicepuncher::core::application::{AudioRecorder, InputDevices};
use voicepuncher_test_steps::required_env;

#[test]
fn should_return_err_when_stop_called_without_start() {
    // Given
    let mut recorder = CpalAudioRecorder::new(600, Arc::new(AtomicU32::new(0)));
    // When
    let result = recorder.stop();
    // Then
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("recorder was not started"),
        "expected 'recorder was not started' in error message"
    );
}

#[test]
fn should_name_every_input_device_of_the_mac_once() {
    // When
    let names = CpalInputDevices.names();
    // Then
    let mut unique = names.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), names.len(), "{names:?}");
    assert!(names.iter().all(|name| !name.is_empty()), "{names:?}");
}

fn name(device: &cpal::Device) -> Option<String> {
    device.description().ok().map(|d| d.name().to_owned())
}

/// The warnings logged by the test process.
mod warnings {
    use voicepuncher_test_steps::log_lines::{captured, LogLines};

    fn lines() -> &'static LogLines {
        captured(log::LevelFilter::Warn)
    }

    /// Captures the warnings of the whole test process, once.
    pub fn install() {
        lines();
    }

    pub fn all() -> Vec<String> {
        lines().starting_with("")
    }

    pub fn count(text: &str) -> usize {
        lines().containing(text).len()
    }
}

mod needs_desktop {
    use super::*;

    #[test]
    fn should_record_and_resample_when_real_mic_available() {
        // Given
        required_env("VOICEPUNCHER_DESKTOP_IT");
        // When
        let mut recorder = CpalAudioRecorder::new(600, Arc::new(AtomicU32::new(0)));
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

    #[test]
    fn should_record_from_the_default_device_and_log_a_named_device_that_is_not_connected() {
        // Given
        required_env("VOICEPUNCHER_DESKTOP_IT");
        let default = cpal::default_host()
            .default_input_device()
            .expect("the Mac running the desktop tests has a default input device");
        warnings::install();
        let missing = "No Such Microphone";
        // When
        let device = open(missing).expect("the default device opens");
        // Then
        assert_eq!(name(&device), name(&default));
        let warnings = warnings::all();
        assert!(
            warnings.iter().any(|warning| warning.contains(missing)),
            "no warning names {missing:?}: {warnings:?}"
        );
    }
}
