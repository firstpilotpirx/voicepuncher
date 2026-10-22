//! The one timer thread behind the dictation sign's self-hiding badges.

use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use super::pointer_sign::PointerSignView;
use crate::adapters::locking::lock;

/// Runs `hide(generation)` once a badge's lifetime ran out; a later badge replaces the pending one.
/// Dropping the timer cancels what is pending and joins its thread.
pub(super) struct SelfHideTimer {
    requests: Option<Sender<(Duration, u64)>>,
    thread: Option<JoinHandle<()>>,
}

impl SelfHideTimer {
    pub(super) fn new(hide: impl Fn(u64) + Send + 'static) -> Self {
        let (requests, received) = mpsc::channel::<(Duration, u64)>();
        let thread = std::thread::spawn(move || run(&received, &hide));
        Self { requests: Some(requests), thread: Some(thread) }
    }

    /// A timer that hides `sign` when the badge it was asked about is still the one shown.
    pub(super) fn hiding<S: PointerSignView>(sign: Arc<S>, generation: Arc<Mutex<u64>>) -> Self {
        Self::new(hide_if_still_shown(sign, generation))
    }

    /// Hides the badge of `generation` after `lifetime`, unless the timer is dropped first.
    pub(super) fn hide_after(&self, lifetime: Duration, generation: u64) {
        if let Some(requests) = &self.requests {
            let _ = requests.send((lifetime, generation));
        }
    }
}

/// The timer thread: waits for the next request, or for the pending badge's deadline, until the
/// timer is dropped.
fn run(received: &Receiver<(Duration, u64)>, hide: &impl Fn(u64)) {
    let mut pending: Option<(Instant, u64)> = None;
    loop {
        match next_request(received, pending) {
            Ok((lifetime, generation)) => pending = Some((Instant::now() + lifetime, generation)),
            Err(RecvTimeoutError::Timeout) => {
                if let Some((_, generation)) = pending.take() {
                    hide(generation);
                }
            }
            Err(RecvTimeoutError::Disconnected) => return,
        }
    }
}

fn next_request(
    received: &Receiver<(Duration, u64)>,
    pending: Option<(Instant, u64)>,
) -> Result<(Duration, u64), RecvTimeoutError> {
    match pending {
        None => received.recv().map_err(|_| RecvTimeoutError::Disconnected),
        Some((deadline, _)) => {
            received.recv_timeout(deadline.saturating_duration_since(Instant::now()))
        }
    }
}

/// Hides the sign when the badge of `shown` is still the one it shows.
fn hide_if_still_shown<S: PointerSignView>(
    sign: Arc<S>,
    generation: Arc<Mutex<u64>>,
) -> impl Fn(u64) + Send + 'static {
    move |shown| {
        if *lock(&generation) == shown {
            sign.hide();
        }
    }
}

impl Drop for SelfHideTimer {
    fn drop(&mut self) {
        drop(self.requests.take());
        if self
            .thread
            .take()
            .is_some_and(|thread| thread.join().is_err())
        {
            log::warn!("dictation sign timer panicked");
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc::RecvTimeoutError;

    use super::*;

    #[test]
    fn should_cancel_a_pending_self_hide_when_the_timer_drops() {
        // Given
        let (hidden, hides) = mpsc::channel();
        let timer = SelfHideTimer::new(move |generation| hidden.send(generation).unwrap());
        timer.hide_after(Duration::from_millis(30), 1);
        // When
        drop(timer);
        // Then — the thread ended without hiding
        assert_eq!(
            hides.recv_timeout(Duration::from_millis(300)),
            Err(RecvTimeoutError::Disconnected)
        );
    }

    #[test]
    fn should_hide_only_the_latest_badge() {
        // Given
        let (hidden, hides) = mpsc::channel();
        let timer = SelfHideTimer::new(move |generation| hidden.send(generation).unwrap());
        // When
        timer.hide_after(Duration::from_millis(30), 1);
        timer.hide_after(Duration::from_millis(30), 2);
        // Then
        assert_eq!(hides.recv_timeout(Duration::from_secs(2)), Ok(2));
        drop(timer);
        assert!(hides.recv_timeout(Duration::from_millis(100)).is_err());
    }
}
