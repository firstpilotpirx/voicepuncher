use std::sync::atomic::{AtomicU32, Ordering};

use anyhow::{anyhow, Result};

use super::*;

const DEADLINE: Duration = Duration::from_millis(200);
const WAIT: Duration = Duration::from_secs(2);

/// Fails the first `failures` requests, then hears `text`.
struct Scripted {
    failures: u32,
    calls: AtomicU32,
    text: &'static str,
}

impl Hear for Scripted {
    fn hear(&self, _: &[f32], _: Duration) -> Result<Heard> {
        let call = self.calls.fetch_add(1, Ordering::SeqCst);
        if call < self.failures {
            return Err(anyhow!("timed out reading response"));
        }
        Ok(Heard { text: self.text.to_owned(), words: Vec::new() })
    }
}

fn worker_with(failures: u32) -> (ConfirmationWorker, Arc<Scripted>) {
    let hear = Arc::new(Scripted { failures, calls: AtomicU32::new(0), text: " press enter" });
    let worker = ConfirmationWorker::start(1, hear.clone(), DEADLINE).unwrap();
    (worker, hear)
}

fn job(n: u64, kind: JobKind) -> Job {
    Job { id: JobId { conn: 1, n }, kind, audio: vec![0.0; 1_600] }
}

#[test]
fn should_try_a_segment_once_more_when_its_first_attempt_fails() {
    // Given
    let (worker, hear) = worker_with(1);
    // When
    worker.submit(job(1, JobKind::Segment));
    let answer = worker.next_answer(WAIT).unwrap();
    // Then
    assert_eq!(answer.heard.unwrap().text, " press enter");
    assert_eq!(hear.calls.load(Ordering::SeqCst), 2);
}

#[test]
fn should_give_up_a_check_after_one_failed_attempt() {
    // Given
    let (worker, hear) = worker_with(1);
    // When
    worker.submit(job(1, JobKind::Head));
    let answer = worker.next_answer(WAIT).unwrap();
    // Then
    assert!(answer.heard.is_none());
    assert_eq!(hear.calls.load(Ordering::SeqCst), 1);
}

#[test]
fn should_answer_confirmations_other_than_checks_in_the_order_they_were_submitted() {
    // Given
    let (worker, _) = worker_with(0);
    // When
    for (n, kind) in [
        (1, JobKind::Tail),
        (2, JobKind::Segment),
        (3, JobKind::Unheld),
    ] {
        worker.submit(job(n, kind));
    }
    let order: Vec<u64> = (0..3)
        .map(|_| worker.next_answer(WAIT).unwrap().id.n)
        .collect();
    // Then
    assert_eq!(order, [1, 2, 3]);
}

/// Hangs on a check's audio until the test ends, and hears every other clip at once.
struct HangingCheck {
    released: std::sync::Mutex<Receiver<()>>,
}

const CHECK_SAMPLE: f32 = 1.0;
const HANG: Duration = Duration::from_secs(10);

impl Hear for HangingCheck {
    fn hear(&self, audio: &[f32], _: Duration) -> Result<Heard> {
        if audio.first() == Some(&CHECK_SAMPLE) {
            let _ = self.released.lock().unwrap().recv_timeout(HANG);
            return Err(anyhow!("timed out reading response"));
        }
        Ok(Heard { text: " enter".to_owned(), words: Vec::new() })
    }
}

#[test]
fn should_answer_a_segment_while_a_check_hangs() {
    // Given
    let (worker, release) = worker_with_hanging_check();
    let check = Job { audio: vec![CHECK_SAMPLE; 1_600], ..job(1, JobKind::Head) };
    // When
    worker.submit(check);
    worker.submit(job(2, JobKind::Segment));
    let first = worker.next_answer(WAIT);
    drop(release);
    // Then
    assert_eq!(first.map(|a| a.id.n), Some(2));
}

#[test]
fn should_answer_a_second_hearing_while_a_segment_hangs() {
    // Given
    let (worker, release) = worker_with_hanging_check();
    let segment = Job { audio: vec![CHECK_SAMPLE; 1_600], ..job(1, JobKind::Segment) };
    // When
    worker.submit(segment);
    worker.submit(job(2, JobKind::Rehear));
    let first = worker.next_answer(WAIT);
    drop(release);
    // Then
    assert_eq!(first.map(|a| a.id.n), Some(2));
}

#[test]
fn should_count_a_job_unanswered_on_its_thread_and_untaken_until_its_answer_is_taken() {
    // Given a segment whose hearing hangs until released
    let (release, released) = mpsc::channel();
    let hear = Arc::new(HangingCheck { released: std::sync::Mutex::new(released) });
    let worker = ConfirmationWorker::start(4, hear, DEADLINE).unwrap();
    let segment = Job { audio: vec![CHECK_SAMPLE; 1_600], ..job_of(4, 1, JobKind::Segment) };
    // When
    worker.submit(segment);
    let held = worker.load();
    drop(release);
    let answer = worker.next_answer(WAIT);
    // Then
    assert!(answer.is_some());
    assert_eq!(held.untaken, 1);
    assert!(held.unanswered.contains(&("conn-4-confirm".to_owned(), 1)));
    let after = worker.load();
    assert_eq!(after.untaken, 0);
    assert!(after.unanswered.iter().all(|(_, n)| *n == 0), "{after:?}");
}

#[test]
fn should_name_the_job_its_kind_and_its_attempt_in_every_line_about_it() {
    // Given
    let lines = voicepuncher_test_steps::log_lines::captured(log::LevelFilter::Trace);
    let hear = Arc::new(Scripted { failures: 1, calls: AtomicU32::new(0), text: " enter" });
    let worker = ConfirmationWorker::start(7, hear, DEADLINE).unwrap();
    // When
    worker.submit(job_of(7, 3, JobKind::Segment));
    worker.next_answer(WAIT).unwrap();
    // Then
    let about = lines.containing("job=7.3");
    assert!(about.iter().any(|l| l.contains("queued")), "{about:?}");
    assert!(
        about
            .iter()
            .any(|l| l.contains("kind=segment attempt=1/2 given up")),
        "{about:?}"
    );
    assert!(
        about
            .iter()
            .any(|l| l.contains("kind=segment attempt=2 answered in")),
        "{about:?}"
    );
    assert!(
        about.iter().any(|l| l.starts_with("[conn-7-confirm]")),
        "the worker thread is not named after its connection: {about:?}"
    );
}

fn job_of(conn: u64, n: u64, kind: JobKind) -> Job {
    Job { id: JobId { conn, n }, kind, audio: vec![0.0; 1_600] }
}

#[test]
fn should_count_head_and_window_as_checks_of_a_running_utterance() {
    assert!(JobKind::Head.is_check() && JobKind::Window.is_check());
    assert!(JobKind::Spotted.is_check());
    assert!(!JobKind::Segment.is_check() && !JobKind::Unheld.is_check());
}

#[test]
fn should_fail_a_job_at_once_when_its_stalled_thread_already_has_its_most_unanswered_jobs() {
    // Given a thread whose running job hangs, with 4 jobs unanswered
    let lines = voicepuncher_test_steps::log_lines::captured(log::LevelFilter::Trace);
    let (worker, release) = worker_with_hanging_check();
    for n in 1..=4 {
        worker.submit(Job { audio: vec![CHECK_SAMPLE; 1_600], ..job(n, JobKind::Segment) });
    }
    std::thread::sleep(Duration::from_millis(100));
    // When one more job comes once the running one ran past its attempts
    let stalled = Instant::now() + Duration::from_secs(11);
    worker.submit_at(
        Job { audio: vec![CHECK_SAMPLE; 1_600], ..job(5, JobKind::Segment) },
        stalled,
    );
    let first = worker.next_answer(WAIT);
    drop(release);
    // Then
    let first = first.unwrap();
    assert_eq!(first.id.n, 5);
    assert!(first.heard.is_none());
    let logged = lines.containing("failed at once");
    assert!(
        logged.iter().any(|l| l.contains("4 unanswered jobs")),
        "{logged:?}"
    );
}

/// A copy under load: every confirmation is heard, a little late.
struct Slow;

impl Hear for Slow {
    fn hear(&self, _: &[f32], _: Duration) -> Result<Heard> {
        std::thread::sleep(Duration::from_millis(30));
        Ok(Heard { text: " restart".to_owned(), words: Vec::new() })
    }
}

#[test]
fn should_queue_jobs_past_the_bound_while_the_thread_keeps_answering() {
    // Given
    let worker = ConfirmationWorker::start(1, Arc::new(Slow), DEADLINE).unwrap();
    // When
    for n in 1..=8 {
        worker.submit(job(n, JobKind::Spotted));
    }
    let answers: Vec<Answer> = (1..=8).filter_map(|_| worker.next_answer(WAIT)).collect();
    // Then
    assert_eq!(answers.len(), 8);
    let refused = answers.iter().filter(|a| a.heard.is_none()).count();
    assert_eq!(refused, 0, "jobs refused while the thread keeps answering");
}

#[test]
fn should_count_a_job_running_past_its_attempts_as_stalled() {
    // Given
    let (worker, release) = worker_with_hanging_check();
    // When
    worker.submit(Job { audio: vec![CHECK_SAMPLE; 1_600], ..job(1, JobKind::Head) });
    std::thread::sleep(Duration::from_millis(100));
    let now = worker.stalled_at(Instant::now());
    let later = worker.stalled_at(Instant::now() + Duration::from_secs(11));
    drop(release);
    // Then
    assert_eq!(now, None);
    assert!(
        later.is_some_and(|ran| ran > Duration::from_secs(11)),
        "{later:?}"
    );
}

#[test]
fn should_bound_a_job_by_its_attempts_and_never_below_ten_seconds() {
    assert_eq!(
        stall_bound(2, Duration::from_secs(3)),
        Duration::from_secs(10)
    );
    assert_eq!(
        stall_bound(2, Duration::from_secs(5)),
        Duration::from_secs(12)
    );
}

/// A worker whose checks hang until the returned sender is dropped.
fn worker_with_hanging_check() -> (ConfirmationWorker, mpsc::Sender<()>) {
    let (release, released) = mpsc::channel();
    let hear = Arc::new(HangingCheck { released: std::sync::Mutex::new(released) });
    (
        ConfirmationWorker::start(1, hear, DEADLINE).unwrap(),
        release,
    )
}
