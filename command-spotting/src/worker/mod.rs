//! Runs one connection's confirmations by the speech recognition service on a thread of their
//! own, so a slow or hung request never holds up the audio that keeps arriving.

use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::confirm::{Hear, Heard};

/// The most jobs a confirmation thread holds unanswered while its running job is stalled;
/// past it a job fails at once. A thread that keeps answering, however slowly, queues them.
const MOST_UNANSWERED: usize = 4;
/// A job running this long cannot be a slow copy, whatever its attempts: the thread hangs.
const STALL_FLOOR: Duration = Duration::from_secs(10);
/// What a job may take beyond its attempts' deadlines before it counts as stalled.
const STALL_MARGIN: Duration = Duration::from_secs(2);

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
        matches!(self, Self::Head | Self::Window | Self::Spotted)
    }

    fn attempts(self) -> u32 {
        if self == Self::Segment {
            2
        } else {
            1
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Head => "head",
            Self::Window => "window",
            Self::Segment => "segment",
            Self::Unheld => "unheld",
            Self::Tail => "tail",
            Self::Rehear => "rehear",
            Self::Spotted => "spotted",
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

/// A connection's three confirmation threads.
///
/// One for the checks of a running utterance, one for second hearings, one for everything else, so
/// a segment's confirmation never waits behind a hung check and a second hearing asked at once
/// never waits behind a segment's.
pub struct ConfirmationWorker {
    jobs: Queue,
    checks: Queue,
    rehears: Queue,
    answers: Receiver<Answer>,
    /// Answers a job refused at once, as its thread runs them.
    refused: Sender<Answer>,
    submitted: AtomicUsize,
    taken: AtomicUsize,
}

/// One confirmation thread: its jobs and how many of them are not answered yet.
struct Queue {
    thread: String,
    jobs: Sender<Job>,
    unanswered: Arc<AtomicUsize>,
    /// When the running job started, and how long it may run before it counts as stalled.
    running: Arc<Mutex<Option<(Instant, Duration)>>>,
}

impl Queue {
    /// How long the running job has run, when it has run past its bound by `now`.
    fn ran_past_bound(&self, now: Instant) -> Option<Duration> {
        let (started, bound) = (*self.running.lock().ok()?)?;
        let ran = now.saturating_duration_since(started);
        (ran > bound).then_some(ran)
    }

    fn stalled_at(&self, now: Instant) -> bool {
        self.ran_past_bound(now).is_some()
    }
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
        let rehears = spawn(
            format!("conn-{conn}-rehear"),
            hear,
            deadline,
            answer_tx.clone(),
        )?;
        Ok(Self {
            jobs,
            checks,
            rehears,
            answers,
            refused: answer_tx,
            submitted: AtomicUsize::new(0),
            taken: AtomicUsize::new(0),
        })
    }

    pub fn submit(&self, job: Job) {
        self.submit_at(job, Instant::now());
    }

    fn submit_at(&self, job: Job, now: Instant) {
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
        if queue.unanswered.load(Ordering::SeqCst) >= MOST_UNANSWERED && queue.stalled_at(now) {
            log::warn!(
                "job={} kind={} failed at once: {} is stalled with {MOST_UNANSWERED} unanswered jobs",
                job.id,
                job.kind.name(),
                queue.thread
            );
            let _ = self.refused.send(Answer { id: job.id, heard: None });
            return;
        }
        queue.unanswered.fetch_add(1, Ordering::SeqCst);
        if let Err(lost) = queue.jobs.send(job) {
            queue.unanswered.fetch_sub(1, Ordering::SeqCst);
            log::warn!("job={} lost: {} has ended", lost.0.id, queue.thread);
            let _ = self.refused.send(Answer { id: lost.0.id, heard: None });
        }
    }

    /// How long a job has run, when one has run past its bound by `now`: its thread hangs.
    pub fn stalled_at(&self, now: Instant) -> Option<Duration> {
        [&self.jobs, &self.checks, &self.rehears]
            .iter()
            .filter_map(|q| q.ran_past_bound(now))
            .max()
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
    let running = Arc::new(Mutex::new(None));
    let started = Arc::clone(&running);
    std::thread::Builder::new()
        .name(name.clone())
        .spawn(move || {
            for job in job_rx {
                let bound = stall_bound(job.kind.attempts(), deadline);
                set(&started, Some((Instant::now(), bound)));
                let answer = run(hear.as_ref(), &job, deadline);
                set(&started, None);
                // WHY: before the send, so a taken answer is never still counted unanswered
                counted.fetch_sub(1, Ordering::SeqCst);
                if answers.send(answer).is_err() {
                    return;
                }
            }
        })?;
    Ok(Queue { thread: name, jobs, unanswered, running })
}

fn set<T>(slot: &Mutex<T>, value: T) {
    if let Ok(mut slot) = slot.lock() {
        *slot = value;
    }
}

/// How long a job of `attempts` attempts, each given up at `deadline`, may run at most.
fn stall_bound(attempts: u32, deadline: Duration) -> Duration {
    (deadline * attempts + STALL_MARGIN).max(STALL_FLOOR)
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
    Answer { id: job.id, heard: None }
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
    samples as f32 / crate::audio::SAMPLE_RATE as f32
}

#[cfg(test)]
mod tests;
