//! One recognizer decoding an utterance on a thread of its own, so a slow recognizer holds
//! back neither the other nor the gate: audio queues up while it decodes.

use std::collections::VecDeque;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};

use crate::streaming::Wake;

use super::super::ports::{Decoded, RecognizerStream};

enum Job {
    Audio { samples: Vec<f32>, fed_at: f32 },
    Finish { fed_at: f32 },
}

/// When the session sees a decode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Delivery {
    /// As it arrives from the recognizer's thread: the service and the real-time bench.
    #[default]
    AsDecoded,
    /// At the stream time it would have ended when fed in real time: its measured decode
    /// time, queued behind the decode before it. The fast bench, which feeds audio faster
    /// than real time and still measures the service's delays.
    Replayed,
}

/// How the recognizers' threads decode: whom they wake and when their decodes are seen.
#[derive(Clone, Default)]
pub struct Decoding {
    /// Called when a recognizer decoded more, so its events go out without waiting for audio.
    pub wake: Option<Wake>,
    pub delivery: Delivery,
}

/// What the recognizer held after a decode.
pub struct Update {
    pub decoded: Decoded,
    /// The samples decoded so far.
    pub fed: usize,
    /// The stream time the last of them was fed at.
    pub fed_at: f32,
    pub took: Duration,
    /// The last update: the stream ended and everything in it is decoded.
    pub finished: bool,
}

/// How far a decoder got: what it decoded, and whether it ended its stream.
#[derive(Default)]
struct Progress {
    decoded_to: usize,
    finished: bool,
}

impl Progress {
    fn note(&mut self, update: &Update) {
        self.decoded_to = update.fed;
        self.finished |= update.finished;
    }
}

pub struct Decoder {
    jobs: Sender<Job>,
    updates: Receiver<Update>,
    delivery: Delivery,
    fed: usize,
    finishing: bool,
    /// What the thread decoded.
    received: Progress,
    /// What the session saw of it.
    seen: Progress,
    /// Updates received and not yet seen, each with the stream time it becomes visible at.
    pending: VecDeque<(f32, Update)>,
    /// When the last update received becomes visible.
    visible_until: f32,
}

impl Decoder {
    /// Starts decoding `stream` on a thread named after the recognizer.
    pub fn start(
        name: &str,
        stream: Box<dyn RecognizerStream>,
        decoding: &Decoding,
    ) -> std::io::Result<Self> {
        let (jobs, job_rx) = mpsc::channel();
        let (update_tx, updates) = mpsc::channel();
        let wake = decoding.wake.clone();
        std::thread::Builder::new()
            .name(format!("decode-{name}"))
            .spawn(move || run(stream, &job_rx, &update_tx, wake.as_ref()))?;
        Ok(Self {
            jobs,
            updates,
            delivery: decoding.delivery,
            fed: 0,
            finishing: false,
            received: Progress::default(),
            seen: Progress::default(),
            pending: VecDeque::new(),
            visible_until: 0.0,
        })
    }

    /// Feeds `samples`, the stream at `now`.
    pub fn feed(&mut self, samples: &[f32], now: f32) {
        self.fed += samples.len();
        let _ = self
            .jobs
            .send(Job::Audio { samples: samples.to_vec(), fed_at: now });
    }

    /// No more audio comes, the stream at `now`.
    pub fn finish(&mut self, now: f32) {
        self.finishing = true;
        let _ = self.jobs.send(Job::Finish { fed_at: now });
    }

    /// The updates seen by `now` in the stream, oldest first: as they came, or, replayed,
    /// those whose decode would have ended by `now`, once the thread decoded everything fed.
    pub fn arrived(&mut self, now: f32) -> Vec<Update> {
        match self.delivery {
            Delivery::AsDecoded => {
                let updates: Vec<Update> = self.updates.try_iter().collect();
                updates.into_iter().for_each(|u| self.queue(u));
            }
            Delivery::Replayed => self.receive_all(),
        }
        self.release_by(now)
    }

    /// Every update, oldest first, once every sample fed is decoded (and, once finishing,
    /// the stream ended); fewer when the thread is gone.
    pub fn caught_up(&mut self) -> Vec<Update> {
        self.receive_all();
        self.release_by(f32::INFINITY)
    }

    /// Takes updates from the thread until it decoded everything fed.
    fn receive_all(&mut self) {
        self.updates
            .try_iter()
            .collect::<Vec<Update>>()
            .into_iter()
            .for_each(|u| self.queue(u));
        while !self.received_all() {
            let Ok(update) = self.updates.recv() else {
                break;
            };
            self.queue(update);
        }
    }

    const fn received_all(&self) -> bool {
        self.received.decoded_to >= self.fed && self.received.finished == self.finishing
    }

    /// Queues `update` with the stream time it becomes visible at: at once as decoded, or
    /// its decode time after it was fed or the update before it became visible.
    fn queue(&mut self, update: Update) {
        let visible_at = match self.delivery {
            Delivery::AsDecoded => f32::NEG_INFINITY,
            Delivery::Replayed => self.visible_until.max(update.fed_at) + update.took.as_secs_f32(),
        };
        self.visible_until = visible_at;
        self.received.note(&update);
        self.pending.push_back((visible_at, update));
    }

    fn release_by(&mut self, now: f32) -> Vec<Update> {
        let mut updates = Vec::new();
        while self.pending.front().is_some_and(|(at, _)| *at <= now) {
            if let Some((_, update)) = self.pending.pop_front() {
                self.seen.note(&update);
                updates.push(update);
            }
        }
        updates
    }

    /// Whether the stream ended and everything in it is decoded and seen.
    pub const fn finished(&self) -> bool {
        self.seen.finished
    }
}

/// The thread: decodes after each job, so every hypothesis on the way is reported as before.
fn run(
    mut stream: Box<dyn RecognizerStream>,
    jobs: &Receiver<Job>,
    updates: &Sender<Update>,
    wake: Option<&Wake>,
) {
    interactive();
    let mut fed = 0;
    while let Ok(job) = jobs.recv() {
        let (finished, fed_at) = match job {
            Job::Audio { samples, fed_at } => {
                fed += samples.len();
                stream.accept(&samples);
                (false, fed_at)
            }
            Job::Finish { fed_at } => {
                stream.finish();
                (true, fed_at)
            }
        };
        let began = Instant::now();
        stream.decode();
        let took = began.elapsed();
        let update = Update { decoded: stream.decoded(), fed, fed_at, took, finished };
        if updates.send(update).is_err() {
            return;
        }
        if let Some(wake) = wake {
            wake();
        }
        if finished {
            return;
        }
    }
}

/// Runs this thread at the user-interactive service class, so other work on a busy machine
/// waits for a command rather than the other way round.
fn interactive() {
    // SAFETY: sets the calling thread's own class; no pointer or shared state is passed.
    let failed = unsafe {
        libc::pthread_set_qos_class_self_np(libc::qos_class_t::QOS_CLASS_USER_INTERACTIVE, 0)
    };
    if failed != 0 {
        log::warn!("the decoding thread keeps its service class: error {failed}");
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;

    /// A stream that notes the service class of the thread it decodes on.
    struct ClassNoting(Arc<Mutex<Option<libc::qos_class_t>>>);

    impl RecognizerStream for ClassNoting {
        fn accept(&mut self, _samples: &[f32]) {}
        fn finish(&mut self) {}
        fn decode(&mut self) {
            let mut class = libc::qos_class_t::QOS_CLASS_UNSPECIFIED;
            let mut relative = 0;
            // SAFETY: returns the calling thread's own handle; no argument.
            let thread = unsafe { libc::pthread_self() };
            // SAFETY: reads that live thread's class into the two locals above.
            unsafe { libc::pthread_get_qos_class_np(thread, &raw mut class, &raw mut relative) };
            *self.0.lock().expect("not poisoned") = Some(class);
        }
        fn decoded(&self) -> Decoded {
            Decoded::default()
        }
    }

    #[test]
    fn should_decode_at_the_user_interactive_service_class() {
        // Given
        let class = Arc::new(Mutex::new(None));
        let stream = Box::new(ClassNoting(Arc::clone(&class)));
        let mut decoder = Decoder::start("en", stream, &Decoding::default()).expect("a thread");
        // When
        decoder.feed(&[0.0; 160], 0.01);
        decoder.finish(0.01);
        decoder.caught_up();
        // Then
        assert!(matches!(
            *class.lock().expect("not poisoned"),
            Some(libc::qos_class_t::QOS_CLASS_USER_INTERACTIVE)
        ));
    }
}
