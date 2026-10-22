//! The checks of running utterances that fell due while another check waited for its answer:
//! each waits its turn with the audio it was due for, and is asked once the check before it
//! is answered, so two checks never run side by side and none is skipped.

use std::collections::VecDeque;

use crate::worker::JobKind;

/// A check that fell due while another one waited: its job number, already counted as asked,
/// and the stream samples `from..to` it hears.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct QueuedCheck {
    pub job: u64,
    pub kind: JobKind,
    pub from: usize,
    pub to: usize,
    /// The utterance whose last seconds it searches, when it is a window.
    pub window_of: Option<u64>,
}

/// The checks waiting their turn, and the one asked and not answered yet.
#[derive(Debug, Default)]
pub(super) struct CheckQueue {
    waiting: VecDeque<QueuedCheck>,
    in_flight: Option<u64>,
}

impl CheckQueue {
    /// A window of `utterance` over `from..to` takes the place of the one of the same
    /// utterance still waiting, if any: the job number it takes over.
    pub fn replace_window(&mut self, utterance: u64, from: usize, to: usize) -> Option<u64> {
        let queued = self
            .waiting
            .iter_mut()
            .find(|check| check.window_of == Some(utterance))?;
        queued.from = from;
        queued.to = to;
        Some(queued.job)
    }

    pub fn push(&mut self, check: QueuedCheck) {
        self.waiting.push_back(check);
    }

    /// The check to ask now: the first waiting, once no other one waits for its answer.
    pub fn next(&mut self) -> Option<QueuedCheck> {
        if self.in_flight.is_some() {
            return None;
        }
        self.waiting.pop_front()
    }

    /// Check `job` was asked: the next one waits for its answer.
    pub fn asked(&mut self, job: u64) {
        self.in_flight = Some(job);
    }

    /// Job `job` was answered or given up.
    pub fn answered(&mut self, job: u64) {
        self.in_flight.take_if(|asked| *asked == job);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn window(job: u64, utterance: u64, to: usize) -> QueuedCheck {
        QueuedCheck { job, kind: JobKind::Window, from: to - 100, to, window_of: Some(utterance) }
    }

    fn head(job: u64, to: usize) -> QueuedCheck {
        QueuedCheck { job, kind: JobKind::Head, from: 0, to, window_of: None }
    }

    #[test]
    fn should_ask_the_checks_in_turn_once_each_one_before_is_answered() {
        // Given a check waiting for its answer and two more queued behind it
        let mut queue = CheckQueue::default();
        queue.asked(1);
        queue.push(head(2, 1_000));
        queue.push(window(3, 0, 2_000));
        // When / Then
        assert_eq!(queue.next(), None);
        queue.answered(1);
        assert_eq!(queue.next(), Some(head(2, 1_000)));
        queue.asked(2);
        assert_eq!(queue.next(), None);
        queue.answered(2);
        assert_eq!(queue.next(), Some(window(3, 0, 2_000)));
    }

    #[test]
    fn should_keep_the_audio_range_each_check_was_due_for() {
        // Given
        let mut queue = CheckQueue::default();
        queue.push(head(1, 24_000));
        // When
        let asked = queue.next();
        // Then
        assert_eq!(asked.map(|c| (c.from, c.to)), Some((0, 24_000)));
    }

    #[test]
    fn should_let_a_later_window_take_the_place_of_a_queued_one_of_the_same_utterance() {
        // Given
        let mut queue = CheckQueue::default();
        queue.asked(1);
        queue.push(window(2, 7, 32_000));
        queue.push(head(3, 33_000));
        // When
        let taken_over = queue.replace_window(7, 32_000, 48_000);
        // Then
        assert_eq!(taken_over, Some(2));
        queue.answered(1);
        assert_eq!(queue.next().map(|c| (c.job, c.to)), Some((2, 48_000)));
        assert_eq!(queue.next().map(|c| c.job), Some(3));
    }

    #[test]
    fn should_queue_a_window_of_another_utterance_beside_a_waiting_one() {
        // Given
        let mut queue = CheckQueue::default();
        queue.push(window(2, 7, 32_000));
        // When
        let taken_over = queue.replace_window(8, 40_000, 48_000);
        // Then
        assert_eq!(taken_over, None);
    }

    #[test]
    fn should_not_free_the_turn_for_an_answer_to_another_job() {
        // Given
        let mut queue = CheckQueue::default();
        queue.asked(1);
        queue.push(head(3, 1_000));
        // When
        queue.answered(2);
        // Then
        assert_eq!(queue.next(), None);
    }
}
