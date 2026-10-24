use std::sync::atomic::{AtomicU32, AtomicUsize};

use rtrb::{Producer, RingBuffer};

use super::*;

/// Whether `sample` is `value`, as the test sent it.
fn is(sample: f32, value: f32) -> bool {
    (sample - value).abs() < f32::EPSILON
}
use crate::adapters::secondary::recording::cpal::capture::on_frames;
use crate::core::application::pipeline::tests::doubles::captured_log;

/// How long a test waits for the keeper to act.
const WAIT: Duration = Duration::from_secs(2);

const FAST: KeeperTimings = KeeperTimings {
    stall: Duration::from_millis(100),
    retry: Duration::from_millis(50),
    tick: Duration::from_millis(5),
};

/// A microphone the test feeds by hand.
struct FakeMic {
    ring: Producer<f32>,
    health: Arc<CaptureHealth>,
    started_at: Instant,
}

impl FakeMic {
    fn deliver(&mut self, samples: &[f32]) {
        on_frames(samples, 1, &mut self.ring, &self.health);
    }
}

/// An opener that fails where `plan` says false, and hands every microphone it opens to
/// the test; it counts its calls.
fn opener(plan: Vec<bool>) -> (MicOpener, Receiver<FakeMic>, Arc<AtomicUsize>) {
    let (mics_tx, mics_rx) = mpsc::channel();
    let calls = Arc::new(AtomicUsize::new(0));
    let counted = Arc::clone(&calls);
    let open = move || {
        let call = counted.fetch_add(1, Ordering::SeqCst);
        if !plan.get(call).copied().unwrap_or(true) {
            return Err(anyhow!("the device is busy"));
        }
        let (ring, consumer) = RingBuffer::new(16_000);
        let health = Arc::new(CaptureHealth::new(Arc::new(AtomicU32::new(0))));
        let started_at = Instant::now();
        let _ = mics_tx.send(FakeMic { ring, health: Arc::clone(&health), started_at });
        Ok(OpenMic {
            audio: Box::new(consumer),
            health,
            rate: AudioBuffer16k::SAMPLE_RATE_HZ,
            started_at,
            device: Box::new(()),
        })
    };
    (Box::new(open), mics_rx, calls)
}

/// Every sample sent until one equals `wanted` (included); empty when none came within the wait.
fn audio_until(audio: &Receiver<Vec<f32>>, wanted: f32) -> Vec<f32> {
    let deadline = Instant::now() + WAIT;
    let mut taken = Vec::new();
    while Instant::now() < deadline {
        if let Ok(chunk) = audio.recv_timeout(Duration::from_millis(10)) {
            taken.extend_from_slice(&chunk);
            if taken.iter().any(|&s| is(s, wanted)) {
                return taken;
            }
        }
    }
    Vec::new()
}

#[test]
fn should_open_a_stalled_microphone_again_and_send_its_audio() {
    // Given — a microphone that delivered audio once
    let log = captured_log();
    let (open, mics, calls) = opener(vec![true, true]);
    let (_keeper, audio, _) = KeptMic::start(open, FAST).unwrap();
    let mut first = mics.recv().unwrap();
    first.deliver(&[0.5; 160]);
    // When — it delivers nothing more
    let mut second = mics
        .recv_timeout(WAIT)
        .expect("the microphone was not opened again");
    second.deliver(&[0.25; 16]);
    // Then
    assert!(audio_until(&audio, 0.25).iter().any(|&s| is(s, 0.5)));
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    let stalled = "command spotting: microphone delivered no audio";
    assert_eq!(log.lines_starting_with(stalled).len(), 1);
    assert_eq!(
        log.lines_starting_with("command spotting: microphone open again")
            .len(),
        1
    );
}

#[test]
fn should_open_a_microphone_that_reported_an_error_again() {
    // Given — a stall limit the test never reaches
    let (open, mics, _) = opener(vec![true, true]);
    let timings = KeeperTimings { stall: Duration::from_secs(60), ..FAST };
    let (_keeper, _audio, _) = KeptMic::start(open, timings).unwrap();
    let mut first = mics.recv().unwrap();
    first.deliver(&[0.5; 160]);
    // When
    first.health.note_error();
    // Then
    assert!(
        mics.recv_timeout(WAIT).is_ok(),
        "the microphone was not opened again"
    );
}

#[test]
fn should_retry_a_failed_reopen_until_it_succeeds() {
    // Given — a microphone that stalls, then two failed attempts to open it again
    let log = captured_log();
    let (open, mics, calls) = opener(vec![true, false, false, true]);
    let (_keeper, _audio, _) = KeptMic::start(open, FAST).unwrap();
    let mut first = mics.recv().unwrap();
    first.deliver(&[0.5; 160]);
    // When
    let second = mics.recv_timeout(WAIT);
    // Then
    assert!(second.is_ok(), "the keeper gave up");
    assert_eq!(calls.load(Ordering::SeqCst), 4);
    let failed = "command spotting: cannot open the microphone again";
    assert_eq!(log.lines_starting_with(failed).len(), 1, "logged once");
}

#[test]
fn should_wait_two_seconds_and_retry_every_second_by_default() {
    let timings = KeeperTimings::default();
    assert_eq!(timings.stall, Duration::from_secs(2));
    assert_eq!(timings.retry, Duration::from_secs(1));
}

#[test]
fn should_give_a_microphone_five_seconds_for_its_first_audio() {
    // Given — a microphone that never delivered audio
    let health = CaptureHealth::new(Arc::new(AtomicU32::new(0)));
    // When — it is judged by a limit shorter than the start limit
    let verdict = stall(&health, Duration::ZERO);
    // Then
    assert!(verdict.is_none(), "{verdict:?}");
    assert_eq!(START_LIMIT, Duration::from_secs(5));
}

#[test]
fn should_send_the_silent_time_as_silence() {
    // Given — a microphone that delivered 160 samples, then stalled
    let (open, mics, _) = opener(vec![true, true]);
    let (_keeper, audio, started_at) = KeptMic::start(open, FAST).unwrap();
    let mut first = mics.recv().unwrap();
    first.deliver(&[0.5; 160]);
    let mut second = mics.recv_timeout(WAIT).unwrap();
    // When
    second.deliver(&[0.25; 16]);
    // Then — the reopened audio sits at its capture time in the stream
    let taken = audio_until(&audio, 0.25);
    let at = taken.iter().position(|&s| is(s, 0.25)).unwrap();
    let due = second.started_at.duration_since(started_at).as_secs_f64() * 16_000.0;
    assert!(
        (f64::from(u32::try_from(at).unwrap()) - due).abs() <= 16.0,
        "at {at}, due {due}"
    );
    assert!(taken[160..at].iter().all(|&s| is(s, 0.0)));
}
