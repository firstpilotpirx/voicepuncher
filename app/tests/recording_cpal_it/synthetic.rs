//! The recorder on a synthetic 48 kHz stream fed through the capture ring and the drain
//! thread, as the microphone's callback would.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use rtrb::{Producer, RingBuffer};
use voicepuncher::adapters::secondary::recording::cpal::capture::{on_frames, CaptureHealth};
use voicepuncher::adapters::secondary::recording::cpal::{CpalAudioRecorder, Source};
use voicepuncher::core::application::chunking::ChunkingCfg;
use voicepuncher::core::application::{AudioRecorder, PipelineInput};

use crate::warnings;

const NATIVE_RATE: u32 = 48_000;
const RATIO: usize = 3;
/// Frames the microphone delivers per callback (10 ms at 48 kHz).
const FRAME: usize = 480;

/// The microphone's side of the ring: what its callback would push.
struct Feeder {
    ring: Producer<f32>,
    health: Arc<CaptureHealth>,
}

impl Feeder {
    fn push(&mut self, samples: &[f32]) {
        on_frames(samples, 1, &mut self.ring, &self.health);
    }
}

/// A recorder on a ring of `ring_samples`, and the feeder of its stream.
fn stand_in(ring_samples: usize) -> (CpalAudioRecorder, Feeder) {
    stand_in_reporting_to(ring_samples, None)
}

/// A recorder that reports a failed microphone, and the feeder of its stream.
fn failing_stand_in() -> (CpalAudioRecorder, Feeder, Receiver<PipelineInput>) {
    let (tx, rx) = mpsc::channel();
    let (recorder, feeder) = stand_in_reporting_to(NATIVE_RATE as usize, Some(tx));
    (recorder, feeder, rx)
}

fn stand_in_reporting_to(
    ring_samples: usize,
    failures: Option<Sender<PipelineInput>>,
) -> (CpalAudioRecorder, Feeder) {
    let (producer, consumer) = RingBuffer::new(ring_samples);
    let health = Arc::new(CaptureHealth::new(Arc::new(AtomicU32::new(0))));
    let source = Mutex::new(Some(Source {
        audio: Box::new(consumer),
        rate: NATIVE_RATE,
        health: Arc::clone(&health),
        started_at: Instant::now(),
        input: None,
    }));
    let mut recorder = CpalAudioRecorder::new(600, Arc::new(AtomicU32::new(0)))
        .with_source(move |_, _| Ok(source.lock().unwrap().take().expect("one start")));
    if let Some(failures) = failures {
        recorder = recorder.with_failure_input(failures);
    }
    (recorder, Feeder { ring: producer, health })
}

fn failure_reason(input: PipelineInput) -> String {
    match input {
        PipelineInput::RecordingFailed(reason) => reason,
        other => panic!("expected a recording failure, got {other:?}"),
    }
}

/// A ramp from 0 to `top` over `samples` samples.
fn ramp(samples: usize, top: f32) -> Vec<f32> {
    (0..samples)
        .map(|i| top * i as f32 / samples as f32)
        .collect()
}

/// Feeds `audio` frame by frame, `speed` times faster than real time.
fn feed(feeder: &mut Feeder, audio: &[f32], speed: u32) {
    let pace = Duration::from_millis(10) / speed;
    for frame in audio.chunks(FRAME) {
        feeder.push(frame);
        std::thread::sleep(pace);
    }
}

/// Spins half the cores until `stop` is set.
fn cpu_hogs(stop: &Arc<AtomicBool>) -> Vec<std::thread::JoinHandle<()>> {
    let cores = std::thread::available_parallelism().map_or(4, usize::from);
    (0..cores.div_ceil(2))
        .map(|_| {
            let stop = Arc::clone(stop);
            std::thread::spawn(move || {
                let mut x = 1u64;
                while !stop.load(Ordering::Relaxed) {
                    x = x.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
                }
                std::hint::black_box(x);
            })
        })
        .collect()
}

#[test]
fn should_keep_every_sample_of_a_long_dictation_in_order_on_a_busy_machine() {
    // Given a minute of rising audio, delivered in real time, and half the cores busy
    let seconds = 60;
    let audio = ramp(seconds * NATIVE_RATE as usize, 0.9);
    let (mut recorder, mut feeder) = stand_in(2 * NATIVE_RATE as usize);
    let stop_hogs = Arc::new(AtomicBool::new(false));
    let hogs = cpu_hogs(&stop_hogs);
    // When
    recorder.start().unwrap();
    feed(&mut feeder, &audio, 1);
    let recording = recorder.stop().unwrap();
    stop_hogs.store(true, Ordering::Relaxed);
    hogs.into_iter().for_each(|hog| hog.join().unwrap());
    // Then the whole minute is there, rising all the way
    assert_eq!(
        recording.samples().len(),
        seconds * NATIVE_RATE as usize / RATIO
    );
    assert_in_order(recording.samples());
    assert_eq!(feeder.health.dropped_samples(), 0);
}

/// A resampled ramp rises all the way; a drop or a swap would show as a fall.
fn assert_in_order(samples: &[i16]) {
    let out_of_order = samples[100..samples.len() - 100]
        .windows(2)
        .position(|pair| pair[1] < pair[0] - 2);
    assert_eq!(out_of_order, None, "the recording is not in order");
}

#[test]
fn should_keep_audio_delivered_while_the_recording_is_held_up() {
    // Given a second of audio delivered before the recorder could take any
    let (mut recorder, mut feeder) = stand_in(2 * NATIVE_RATE as usize);
    let whole = ramp(NATIVE_RATE as usize + FRAME * 10, 0.9);
    let (held_up, after) = whole.split_at(NATIVE_RATE as usize);
    held_up.chunks(FRAME).for_each(|frame| feeder.push(frame));
    // When
    recorder.start().unwrap();
    feed(&mut feeder, after, 1);
    let recording = recorder.stop().unwrap();
    // Then every sample is kept, in order
    assert_eq!(recording.samples().len(), whole.len() / RATIO);
    assert_in_order(recording.samples());
    assert_eq!(feeder.health.dropped_samples(), 0);
}

#[test]
fn should_log_the_audio_dropped_once_at_the_stop() {
    // Given a ring too small for a second delivered at once
    warnings::install();
    let (mut recorder, mut feeder) = stand_in(1_000);
    recorder.start().unwrap();
    std::thread::sleep(Duration::from_millis(50));
    feeder.push(&vec![0.5; NATIVE_RATE as usize]);
    std::thread::sleep(Duration::from_millis(50));
    // and the recording goes on after the overrun
    feed(&mut feeder, &[0.9; FRAME], 1);
    // When
    let before = warnings::count("ms of audio dropped");
    let recording = recorder.stop().unwrap();
    // Then the audio after the overrun is in the recording, and the drop is logged once
    assert_eq!(recording.samples().len(), (1_000 + FRAME) / RATIO);
    assert!(recording.samples().last().copied().unwrap_or(0) > i16::MAX / 2);
    assert_eq!(warnings::count("ms of audio dropped") - before, 1);
    assert!(
        warnings::all()
            .iter()
            .any(|w| w.contains("979 ms of audio dropped")),
        "{:?}",
        warnings::all()
    );
}

#[test]
fn should_end_with_the_streams_last_samples_and_no_trailing_silence() {
    // Given a second of steady sound
    let (mut recorder, mut feeder) = stand_in(2 * NATIVE_RATE as usize);
    recorder.start().unwrap();
    feed(&mut feeder, &vec![0.5; NATIVE_RATE as usize], 4);
    // When
    let recording = recorder.stop().unwrap();
    // Then the recording is exactly a second and loud to its last sample
    let samples = recording.samples();
    assert_eq!(samples.len(), NATIVE_RATE as usize / RATIO);
    let last = samples[samples.len() - 1];
    assert!(last > i16::MAX / 10, "the last sample is {last}");
}

#[test]
fn should_lay_live_chunks_end_to_end_into_the_whole_recording() {
    // Given live chunks of a second without overlap
    let cfg = ChunkingCfg { chunk_threshold_secs: 1, chunk_overlap_secs: 0, vad_lookback_secs: 0 };
    let (mut recorder, mut feeder) = stand_in(2 * NATIVE_RATE as usize);
    recorder.start().unwrap();
    let chunks = recorder.start_live_chunks(cfg).unwrap().unwrap();
    // When three and a half seconds are dictated
    feed(&mut feeder, &ramp(NATIVE_RATE as usize * 7 / 2, 0.9), 4);
    recorder.flush_live_chunks();
    recorder.stop().unwrap();
    let joined: Vec<i16> = chunks
        .try_iter()
        .flat_map(|chunk| chunk.audio.samples().to_vec())
        .collect();
    // Then the chunks are the whole recording
    let whole = recorder.recording().unwrap();
    assert_eq!(joined.len(), whole.samples().len());
    assert_eq!(joined, whole.samples());
}

#[test]
fn should_report_the_device_error_as_a_recording_failure_once() {
    let (mut recorder, mut feeder, failures) = failing_stand_in();
    recorder.start().unwrap();
    feed(&mut feeder, &ramp(NATIVE_RATE as usize / 10, 0.5), 1);
    feeder.health.note_error();
    let reason = failure_reason(failures.recv_timeout(Duration::from_secs(1)).unwrap());
    assert!(reason.contains("error"), "{reason}");
    assert!(failures.recv_timeout(Duration::from_millis(300)).is_err());
    assert_eq!(
        recorder.stop().unwrap().samples().len(),
        NATIVE_RATE as usize / 10 / RATIO
    );
}

#[test]
fn should_report_a_stream_gone_silent_for_a_second_as_a_recording_failure() {
    let (mut recorder, mut feeder, failures) = failing_stand_in();
    recorder.start().unwrap();
    feed(&mut feeder, &ramp(NATIVE_RATE as usize / 5, 0.5), 1);
    // the last frame went in one pace (10 ms) before this moment
    let last_audio = Instant::now();
    assert!(failures.recv_timeout(Duration::from_millis(900)).is_err());
    let reason = failure_reason(failures.recv_timeout(Duration::from_secs(2)).unwrap());
    assert!(last_audio.elapsed() >= Duration::from_millis(980));
    assert!(reason.contains("no audio"), "{reason}");
    recorder.stop().unwrap();
}
