//! The asking of confirmations: reserving a job, cutting its clip, and taking the answers.

use std::ops::Range;
use std::sync::Arc;
use std::time::Instant;

use crate::protocol::ServerEvent;
use crate::worker::{ConfirmationWorker, Job, JobId, JobKind, WorkerLoad};

use super::super::queued::QueuedCheck;
use super::super::{seconds, Session};

use super::{AskedJob, JobContext, SETTLE_DEADLINES};

impl Session<'_> {
    /// The events whose confirmations were answered since the last call, without waiting.
    pub(in crate::engine) fn take_answers(&mut self) -> Vec<ServerEvent> {
        let answers = self
            .worker
            .as_ref()
            .map(crate::worker::ConfirmationWorker::answers)
            .unwrap_or_default();
        let events = answers.into_iter().flat_map(|a| self.apply(a)).collect();
        self.ask_queued_check();
        events
    }

    /// The events of the confirmations answered since the last call, without new audio.
    pub fn poll(&mut self) -> Vec<ServerEvent> {
        let events = self.take_answers();
        self.recorded(events, false)
    }

    /// The events still waiting for confirmations, once every one is answered or given up.
    pub fn settle(&mut self) -> Vec<ServerEvent> {
        let wait = self.engine.deadline * SETTLE_DEADLINES;
        let mut events = Vec::new();
        while self.jobs.outbox.pending() > 0 {
            let Some(answer) = self.worker.as_ref().and_then(|w| w.next_answer(wait)) else {
                break;
            };
            events.extend(self.apply(answer));
            self.ask_queued_check();
        }
        self.recorded(events, true)
    }

    /// Asks for `synced` with `id` once every event for the audio accepted so far is out.
    pub fn sync(&mut self, id: u64) -> Vec<ServerEvent> {
        let mut events = self.take_answers();
        events.extend(self.jobs.outbox.sync(id));
        self.recorded(events, false)
    }

    /// The confirmations on each thread of the worker not answered yet, and those whose
    /// answers were not taken yet; nothing without a worker.
    pub fn confirmation_load(&self) -> WorkerLoad {
        self.worker
            .as_ref()
            .map(ConfirmationWorker::load)
            .unwrap_or_default()
    }

    /// Confirmations asked for and not answered yet.
    pub fn pending_confirmations(&self) -> usize {
        self.jobs.outbox.pending()
    }

    /// Asks the worker what was said from sample `from` to `to`; none without a worker.
    pub(in crate::engine) fn ask(
        &mut self,
        kind: JobKind,
        from: usize,
        to: usize,
        context: JobContext,
    ) -> Option<u64> {
        self.worker.as_ref()?;
        let audio = self.recent.between(from, to);
        self.ask_clip(kind, audio, from..to, context)
    }

    /// Asks the worker what was said in `audio`, the samples `span` of the stream.
    pub(super) fn ask_clip(
        &mut self,
        kind: JobKind,
        audio: Vec<f32>,
        span: Range<usize>,
        context: JobContext,
    ) -> Option<u64> {
        let n = self.reserve(kind, context)?;
        self.submit(n, kind, audio, span);
        Some(n)
    }

    /// A job number for a confirmation with `context`, counted as asked from now; none
    /// without a worker.
    pub(super) fn reserve(&mut self, kind: JobKind, context: JobContext) -> Option<u64> {
        self.worker.as_ref()?;
        let n = self.jobs.next_job;
        self.jobs.next_job += 1;
        if let Some(utterance) = context.utterance() {
            self.mark(utterance, |r| r.jobs += 1);
        }
        self.jobs.outbox.asked(n, kind.is_check());
        self.jobs.asked.insert(
            n,
            AskedJob {
                context,
                asked_at: self.detector.samples_seen,
                asked: Instant::now(),
                matcher: Arc::clone(&self.matcher),
            },
        );
        Some(n)
    }

    /// Sends job `n` to the worker: what was said in `audio`, the samples `span` of the
    /// stream.
    pub(super) fn submit(&mut self, n: u64, kind: JobKind, audio: Vec<f32>, span: Range<usize>) {
        log::debug!(
            "job={} {kind:?} asks for {:.2}–{:.2}s",
            JobId { conn: self.conn, n },
            seconds(span.start),
            seconds(span.end)
        );
        if let Some(asked) = self.jobs.asked.get_mut(&n) {
            asked.asked = Instant::now();
        }
        self.record_asked(n, kind, &span);
        if let Some(worker) = self.worker.as_ref() {
            worker.submit(Job { id: JobId { conn: self.conn, n }, kind, audio });
        }
    }

    /// Asks a check of a running utterance about the stream from `from` to `to`, or queues it
    /// with that audio while another check waits; a window takes the place of a queued window
    /// of the same utterance.
    pub(in crate::engine) fn ask_check(
        &mut self,
        kind: JobKind,
        from: usize,
        to: usize,
        context: JobContext,
    ) {
        if !self.jobs.outbox.check_busy() {
            if let Some(n) = self.ask(kind, from, to, context) {
                self.jobs.checks.asked(n);
            }
            return;
        }
        let window_of = match context {
            JobContext::Window { utterance, .. } => Some(utterance),
            _ => None,
        };
        let taken_over = window_of.and_then(|u| self.jobs.checks.replace_window(u, from, to));
        log::debug!(
            "conn={} {kind:?} for {:.2}–{:.2}s waits its turn: a check still waits",
            self.conn,
            seconds(from),
            seconds(to)
        );
        if let Some(asked) = taken_over.and_then(|n| self.jobs.asked.get_mut(&n)) {
            asked.context = context;
            asked.asked_at = self.detector.samples_seen;
            return;
        }
        if let Some(job) = self.reserve(kind, context) {
            self.jobs
                .checks
                .push(QueuedCheck { job, kind, from, to, window_of });
        }
    }

    /// Asks the check whose turn it is, once no other one waits for its answer.
    pub(in crate::engine) fn ask_queued_check(&mut self) {
        let Some(QueuedCheck { job, kind, from, to, .. }) = self.jobs.checks.next() else {
            return;
        };
        let audio = self.recent.between(from, to);
        self.submit(job, kind, audio, from..to);
        self.jobs.checks.asked(job);
    }
}
