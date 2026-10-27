//! A dictation while voice control listens records the kept microphone, opening no device.

use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::Arc;

use anyhow::anyhow;
use voicepuncher::adapters::secondary::recording::cpal::capture::CapturedAudio;
use voicepuncher::adapters::secondary::recording::cpal::tap::MicTap;
use voicepuncher::adapters::secondary::recording::cpal::CpalAudioRecorder;
use voicepuncher::core::application::AudioRecorder;

/// A recorder on `tap` whose own device opens are counted in `opens` (and fail).
fn recorder_on(tap: &MicTap, opens: &Arc<AtomicUsize>) -> CpalAudioRecorder {
    let counted = Arc::clone(opens);
    CpalAudioRecorder::new(600, Arc::new(AtomicU32::new(0)))
        .with_source(move |_, _| {
            counted.fetch_add(1, Ordering::Relaxed);
            Err(anyhow!("the device was opened"))
        })
        .with_tap(tap.clone())
}

#[test]
fn should_record_the_kept_microphone_without_opening_a_device_while_voice_control_listens() {
    // Given voice control's stream reading the kept microphone through the tap
    let tap = MicTap::default();
    let (kept_tx, kept_rx) = mpsc::channel::<Vec<f32>>();
    let mut stream = tap.tee(Box::new(kept_rx));
    let opens = Arc::new(AtomicUsize::new(0));
    let mut recorder = recorder_on(&tap, &opens);
    // When a dictation records while the stream takes 0.3 s of rising audio
    recorder.start().unwrap();
    let audio: Vec<f32> = (0..4_800).map(|i| i as f32 / 8_000.0).collect();
    for chunk in audio.chunks(320) {
        kept_tx.send(chunk.to_vec()).unwrap();
        assert_eq!(stream.take(), chunk, "the stream gets its audio unchanged");
    }
    std::thread::sleep(std::time::Duration::from_millis(30));
    let recording = recorder.stop().unwrap();
    // Then no device was opened and the recording holds the stream's audio
    assert_eq!(
        opens.load(Ordering::Relaxed),
        0,
        "a second device was opened"
    );
    let samples = recording.samples();
    assert!(
        samples.len() >= 4_700 && samples.len() <= 4_800,
        "{} samples recorded",
        samples.len()
    );
    assert!(
        samples.windows(2).all(|pair| pair[0] <= pair[1]),
        "out of order"
    );
}

#[test]
fn should_open_the_device_when_voice_control_does_not_listen() {
    // Given a tap whose stream has stopped
    let tap = MicTap::default();
    let (_kept_tx, kept_rx) = mpsc::channel::<Vec<f32>>();
    drop(tap.tee(Box::new(kept_rx)));
    let opens = Arc::new(AtomicUsize::new(0));
    let mut recorder = recorder_on(&tap, &opens);
    // When
    let started = recorder.start();
    // Then
    assert!(started.is_err());
    assert_eq!(opens.load(Ordering::Relaxed), 1);
}
