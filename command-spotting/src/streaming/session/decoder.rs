//! One recognizer decoding an utterance on a thread of its own, so a slow recognizer holds
//! back neither the other nor the gate: audio queues up while it decodes.

use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};

use crate::worker::Wake;

use super::super::ports::{Decoded, RecognizerStream};

enum Job {
    Audio(Vec<f32>),
    Finish,
}

/// What the recognizer held after a decode.
pub struct Update {
    pub decoded: Decoded,
    /// The samples decoded so far.
    pub fed: usize,
    pub took: Duration,
    /// The last update: the stream ended and everything in it is decoded.
    pub finished: bool,
}

pub struct Decoder {
    jobs: Sender<Job>,
    updates: Receiver<Update>,
    fed: usize,
    decoded_to: usize,
    finishing: bool,
    finished: bool,
}

impl Decoder {
    /// Starts decoding `stream` on a thread named after the recognizer; each update calls `wake`.
    pub fn start(
        name: &str,
        stream: Box<dyn RecognizerStream>,
        wake: Option<Wake>,
    ) -> std::io::Result<Self> {
        let (jobs, job_rx) = mpsc::channel();
        let (update_tx, updates) = mpsc::channel();
        std::thread::Builder::new()
            .name(format!("decode-{name}"))
            .spawn(move || run(stream, &job_rx, &update_tx, wake.as_ref()))?;
        Ok(Self { jobs, updates, fed: 0, decoded_to: 0, finishing: false, finished: false })
    }

    pub fn feed(&mut self, samples: &[f32]) {
        self.fed += samples.len();
        let _ = self.jobs.send(Job::Audio(samples.to_vec()));
    }

    /// No more audio comes.
    pub fn finish(&mut self) {
        self.finishing = true;
        let _ = self.jobs.send(Job::Finish);
    }

    /// The updates that came, oldest first, without waiting.
    pub fn arrived(&mut self) -> Vec<Update> {
        let updates: Vec<Update> = self.updates.try_iter().collect();
        updates.iter().for_each(|u| self.note(u));
        updates
    }

    /// The updates that came, oldest first, once every sample fed is decoded (and, once
    /// finishing, the stream ended); fewer when the thread is gone.
    pub fn caught_up(&mut self) -> Vec<Update> {
        let mut updates = self.arrived();
        while !self.done() {
            let Ok(update) = self.updates.recv() else {
                break;
            };
            self.note(&update);
            updates.push(update);
        }
        updates
    }

    const fn done(&self) -> bool {
        self.decoded_to >= self.fed && self.finished == self.finishing
    }

    fn note(&mut self, update: &Update) {
        self.decoded_to = update.fed;
        self.finished |= update.finished;
    }
}

/// The thread: decodes after each job, so every hypothesis on the way is reported as before.
fn run(
    mut stream: Box<dyn RecognizerStream>,
    jobs: &Receiver<Job>,
    updates: &Sender<Update>,
    wake: Option<&Wake>,
) {
    let mut fed = 0;
    while let Ok(job) = jobs.recv() {
        let finished = match job {
            Job::Audio(samples) => {
                fed += samples.len();
                stream.accept(&samples);
                false
            }
            Job::Finish => {
                stream.finish();
                true
            }
        };
        let began = Instant::now();
        stream.decode();
        let took = began.elapsed();
        let update = Update { decoded: stream.decoded(), fed, took, finished };
        if updates.send(update).is_err() || finished {
            return;
        }
        if let Some(wake) = wake {
            wake();
        }
    }
}
