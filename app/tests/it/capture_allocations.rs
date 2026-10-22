//! The microphone input callback runs on the audio thread: it allocates nothing.

use std::sync::atomic::AtomicU32;
use std::sync::Arc;

use assert_no_alloc::{assert_no_alloc, AllocDisabler};
use rtrb::RingBuffer;
use voicepuncher::adapters::secondary::recording::cpal::capture::{on_frames, CaptureHealth};

#[global_allocator]
static ALLOCATOR: AllocDisabler = AllocDisabler;

#[test]
fn should_allocate_nothing_for_mono_frames() {
    let (mut ring, _consumer) = RingBuffer::new(4_096);
    let health = CaptureHealth::new(Arc::new(AtomicU32::new(0)));
    let frames = vec![0.25_f32; 1_024];
    assert_no_alloc(|| on_frames(&frames, 1, &mut ring, &health));
}

#[test]
fn should_allocate_nothing_for_stereo_frames() {
    let (mut ring, _consumer) = RingBuffer::new(4_096);
    let health = CaptureHealth::new(Arc::new(AtomicU32::new(0)));
    let frames = vec![0.25_f32; 2_048];
    assert_no_alloc(|| on_frames(&frames, 2, &mut ring, &health));
}

#[test]
fn should_allocate_nothing_when_the_ring_is_full() {
    let (mut ring, _consumer) = RingBuffer::new(16);
    let health = CaptureHealth::new(Arc::new(AtomicU32::new(0)));
    let frames = vec![0.25_f32; 1_024];
    assert_no_alloc(|| on_frames(&frames, 1, &mut ring, &health));
    assert_eq!(health.dropped_samples(), 1_024 - 16);
}
