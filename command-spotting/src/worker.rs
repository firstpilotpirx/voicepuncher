//! Runs one connection's confirmations by the speech recognition service on a thread of their
//! own, so a slow or hung request never holds up the audio that keeps arriving.

use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::confirm::{Hear, Heard};

/// Where a confirmation is in its utterance; only a segment's confirmation is tried twice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobKind {
    /// The start of a running utterance, searched for "start".
    Head,
    /// The last seconds of a running utterance.
    Window,
    /// A closed speech segment.
    Segment,
    /// A phrase the keyword spotter found outside any speech segment.
    Unheld,
    /// The last seconds of a closed segment in which no command was heard.
    Tail,
    /// A short segment heard as a near miss of a command, heard again through its steady clip.
    Rehear,
    /// A running utterance in which the keyword spotter heard "start" or "restart".
    Spotted,
}

impl JobKind {
    /// A check of a running utterance: it waits its turn while another one waits for its answer.
    pub fn is_check(self) -> bool {
        matches!(self, JobKind::Head | JobKind::Window | JobKind::Spotted)
    }

    fn attempts(self) -> u32 {
        if self == JobKind::Segment {
            2
        } else {
            1
        }
    }

    fn name(self) -> &'static str {
        match self {
            JobKind::Head => "head",
            JobKind::Window => "window",
            JobKind::Segment => "segment",
            JobKind::Unheld => "unheld",
            JobKind::Tail => "tail",
            JobKind::Rehear => "rehear",
            JobKind::Spotted => "spotted",
        }
    }
}

/// A confirmation's id within its connection, logged as `job=<conn>.<n>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct JobId {
    pub conn: u64,
    pub n: u64,
}

impl fmt::Display for JobId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.conn, self.n)
    }
}

pub struct Job {
    pub id: JobId,
    pub kind: JobKind,
    pub audio: Vec<f32>,
}

/// What the speech recognition service heard for a job; none when every attempt failed.
pub struct Answer {
    pub id: JobId,
    pub heard: Option<Heard>,
}

/// A connection's three confirmation threads: one for the checks of a running utterance, one
/// for second hearings, one for everything else, so a segment's confirmation never waits
/// behind a hung check and a second hearing asked at once never waits behind a segment's.
pub struct ConfirmationWorker {
    jobs: Queue,
    checks: Queue,
    rehears: Queue,
    answers: Receiver<Answer>,
    submitted: AtomicUsize,
    taken: AtomicUsize,
}

/// One confirmation thread: its jobs and how many of them are not answered yet.
struct Queue {
    thread: String,
    jobs: Sender<Job>,
    unanswered: Arc<AtomicUsize>,
}

/// What the worker holds at a moment: by thread, the jobs sent to it and not answered yet;
/// and the jobs whose answers were not taken yet, answered or not.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct WorkerLoad {
    pub unanswered: Vec<(String, usize)>,
    pub untaken: usize,
}

impl ConfirmationWorker {
    /// Starts the threads of connection `conn`; they end when the worker is dropped, once the
    /// request each runs is answered or given up.
    pub fn start(conn: u64, hear: Arc<dyn Hear>, deadline: Duration) -> std::io::Result<Self> {
        let (answer_tx, answers) = mpsc::channel();
        let jobs = spawn(
            format!("conn-{conn}-confirm"),
            hear.clone(),
            deadline,
            answer_tx.clone(),
        )?;
        let checks = spawn(
            format!("conn-{conn}-check"),
            hear.clone(),
            deadline,
            answer_tx.clone(),
        )?;
        let rehears = spawn(format!("conn-{conn}-rehear"), hear, deadline, answer_tx)?;
        Ok(Self {
            jobs,
            checks,
            rehears,
            answers,
            submitted: AtomicUsize::new(0),
            taken: AtomicUsize::new(0),
        })
    }

    pub fn submit(&self, job: Job) {
        log::debug!(
            "job={} kind={} queued audio={:.2}s",
            job.id,
            job.kind.name(),
            seconds(job.audio.len())
        );
        let queue = if job.kind.is_check() {
            &self.checks
        } else if job.kind == JobKind::Rehear {
            &self.rehears
        } else {
            &self.jobs
        };
        self.submitted.fetch_add(1, Ordering::SeqCst);
        queue.unanswered.fetch_add(1, Ordering::SeqCst);
        let _ = queue.jobs.send(job);
    }

    /// The answers that arrived so far, without waiting.
    pub fn answers(&self) -> Vec<Answer> {
        let answers: Vec<Answer> = self.answers.try_iter().collect();
        self.taken.fetch_add(answers.len(), Ordering::SeqCst);
        answers
    }

    /// The next answer, waiting at most `wait`.
    pub fn next_answer(&self, wait: Duration) -> Option<Answer> {
        let answer = self.answers.recv_timeout(wait).ok();
        self.taken
            .fetch_add(usize::from(answer.is_some()), Ordering::SeqCst);
        answer
    }

    /// The jobs not answered yet on each thread and the answers not taken yet.
    pub fn load(&self) -> WorkerLoad {
        WorkerLoad {
            unanswered: [&self.jobs, &self.checks, &self.rehears]
                .iter()
                .map(|q| (q.thread.clone(), q.unanswered.load(Ordering::SeqCst)))
                .collect(),
            untaken: self.submitted.load(Ordering::SeqCst) - self.taken.load(Ordering::SeqCst),
        }
    }
}

/// Starts thread `name`, answering the jobs sent to the returned queue one after another.
fn spawn(
    name: String,
    hear: Arc<dyn Hear>,
    deadline: Duration,
    answers: Sender<Answer>,
) -> std::io::Result<Queue> {
    let (jobs, job_rx) = mpsc::channel::<Job>();
    let unanswered = Arc::new(AtomicUsize::new(0));
    let counted = Arc::clone(&unanswered);
    std::thread::Builder::new()
        .name(name.clone())
        .spawn(move || {
            for job in job_rx {
                let answer = run(hear.as_ref(), &job, deadline);
                // WHY: before the send, so a taken answer is never still counted unanswered
                counted.fetch_sub(1, Ordering::SeqCst);
                if answers.send(answer).is_err() {
                    return;
                }
            }
        })?;
    Ok(Queue {
        thread: name,
        jobs,
        unanswered,
    })
}

fn run(hear: &dyn Hear, job: &Job, deadline: Duration) -> Answer {
    let attempts = job.kind.attempts();
    for attempt in 1..=attempts {
        let started = Instant::now();
        match hear.hear(&job.audio, deadline) {
            Ok(heard) => {
                log_answer(job, attempt, started.elapsed(), &heard);
                return Answer {
                    id: job.id,
                    heard: Some(heard),
                };
            }
            Err(e) => log::warn!(
                "job={} kind={} attempt={attempt}/{attempts} given up after {}ms (deadline {}ms): {e:#}",
                job.id,
                job.kind.name(),
                started.elapsed().as_millis(),
                deadline.as_millis()
            ),
        }
    }
    Answer {
        id: job.id,
        heard: None,
    }
}

/// Every answer at info, with how long it took and what was heard: the trace of a session.
fn log_answer(job: &Job, attempt: u32, took: Duration, heard: &Heard) {
    log::info!(
        "job={} kind={} attempt={attempt} answered in {}ms: {:?}",
        job.id,
        job.kind.name(),
        took.as_millis(),
        heard.text
    );
}

fn seconds(samples: usize) -> f32 {
    samples as f32 / crate::engine::SAMPLE_RATE as f32
}

#[cfg(test)]
mod tests {
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
            Ok(Heard {
                text: self.text.to_owned(),
                words: Vec::new(),
            })
        }
    }

    fn worker_with(failures: u32) -> (ConfirmationWorker, Arc<Scripted>) {
        let hear = Arc::new(Scripted {
            failures,
            calls: AtomicU32::new(0),
            text: " press enter",
        });
        let worker = ConfirmationWorker::start(1, hear.clone(), DEADLINE).unwrap();
        (worker, hear)
    }

    fn job(n: u64, kind: JobKind) -> Job {
        Job {
            id: JobId { conn: 1, n },
            kind,
            audio: vec![0.0; 1_600],
        }
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
            Ok(Heard {
                text: " enter".to_owned(),
                words: Vec::new(),
            })
        }
    }

    #[test]
    fn should_answer_a_segment_while_a_check_hangs() {
        // Given
        let (release, released) = mpsc::channel();
        let hear = Arc::new(HangingCheck {
            released: std::sync::Mutex::new(released),
        });
        let worker = ConfirmationWorker::start(1, hear, DEADLINE).unwrap();
        let check = Job {
            audio: vec![CHECK_SAMPLE; 1_600],
            ..job(1, JobKind::Head)
        };
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
        let (release, released) = mpsc::channel();
        let hear = Arc::new(HangingCheck {
            released: std::sync::Mutex::new(released),
        });
        let worker = ConfirmationWorker::start(1, hear, DEADLINE).unwrap();
        let segment = Job {
            audio: vec![CHECK_SAMPLE; 1_600],
            ..job(1, JobKind::Segment)
        };
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
        let hear = Arc::new(HangingCheck {
            released: std::sync::Mutex::new(released),
        });
        let worker = ConfirmationWorker::start(4, hear, DEADLINE).unwrap();
        let segment = Job {
            audio: vec![CHECK_SAMPLE; 1_600],
            ..job_of(4, 1, JobKind::Segment)
        };
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

    use crate::test_log::captured;

    #[test]
    fn should_name_the_job_its_kind_and_its_attempt_in_every_line_about_it() {
        // Given
        let lines = captured();
        let hear = Arc::new(Scripted {
            failures: 1,
            calls: AtomicU32::new(0),
            text: " enter",
        });
        let worker = ConfirmationWorker::start(7, hear, DEADLINE).unwrap();
        // When
        worker.submit(job_of(7, 3, JobKind::Segment));
        worker.next_answer(WAIT).unwrap();
        // Then
        let about: Vec<String> = lines
            .0
            .lock()
            .unwrap()
            .iter()
            .filter(|l| l.contains("job=7.3"))
            .cloned()
            .collect();
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
        Job {
            id: JobId { conn, n },
            kind,
            audio: vec![0.0; 1_600],
        }
    }

    #[test]
    fn should_count_head_and_window_as_checks_of_a_running_utterance() {
        assert!(JobKind::Head.is_check() && JobKind::Window.is_check());
        assert!(JobKind::Spotted.is_check());
        assert!(!JobKind::Segment.is_check() && !JobKind::Unheld.is_check());
    }
}
