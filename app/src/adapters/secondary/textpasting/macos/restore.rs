//! The clipboard restore a paste leaves pending: owned by the paster, so it ends with it.

use std::sync::mpsc::{self, Sender};
use std::thread::JoinHandle;
use std::time::Duration;

/// A restore waiting out its delay on a thread of its own. Dropping it wakes the thread at once and
/// waits for the restore, so the user's clipboard is back before the paster (or the app) is gone.
pub(super) struct PendingRestore {
    wake: Option<Sender<()>>,
    thread: Option<JoinHandle<()>>,
}

impl PendingRestore {
    /// Runs `restore` after `delay`, or as soon as the pending restore is dropped.
    pub(super) fn after(delay: Duration, restore: impl FnOnce() + Send + 'static) -> Self {
        let (wake, woken) = mpsc::channel::<()>();
        let thread = std::thread::spawn(move || {
            // A wake signal and a closed channel both mean "now"; a timeout means the delay ran out.
            let _ = woken.recv_timeout(delay);
            restore();
        });
        Self { wake: Some(wake), thread: Some(thread) }
    }
}

impl Drop for PendingRestore {
    fn drop(&mut self) {
        drop(self.wake.take());
        if self
            .thread
            .take()
            .is_some_and(|thread| thread.join().is_err())
        {
            log::warn!("clipboard restore panicked");
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::time::Instant;

    use super::*;

    #[test]
    fn should_restore_at_once_when_dropped_before_the_delay() {
        // Given
        let restored = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&restored);
        let pending = PendingRestore::after(Duration::from_secs(60), move || {
            flag.store(true, Ordering::SeqCst);
        });
        let start = Instant::now();
        // When
        drop(pending);
        // Then
        assert!(restored.load(Ordering::SeqCst));
        assert!(start.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn should_restore_after_the_delay_when_kept() {
        // Given
        let restored = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&restored);
        // When
        let _pending = PendingRestore::after(Duration::from_millis(10), move || {
            flag.store(true, Ordering::SeqCst);
        });
        std::thread::sleep(Duration::from_millis(500));
        // Then
        assert!(restored.load(Ordering::SeqCst));
    }
}
