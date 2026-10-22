//! The pipeline's activity as the runtime reads it before a restart.

use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use crate::core::application::{PipelineActivity, PipelineActivitySink};

/// What the running pipeline last published; Idle before it publishes anything. A restart
/// waits on it until the dictation in progress has ended.
#[derive(Clone, Default)]
pub struct ActivityRegister(Arc<(Mutex<PipelineActivity>, Condvar)>);

impl ActivityRegister {
    pub fn current(&self) -> PipelineActivity {
        *self.activity()
    }

    /// Waits until Idle is published, at most `bound`; the activity at the end of the wait.
    pub fn wait_idle(&self, bound: Duration) -> PipelineActivity {
        let busy = |activity: &mut PipelineActivity| *activity != PipelineActivity::Idle;
        let (activity, _) = self
            .0
             .1
            .wait_timeout_while(self.activity(), bound, busy)
            .unwrap_or_else(PoisonError::into_inner);
        *activity
    }

    fn activity(&self) -> MutexGuard<'_, PipelineActivity> {
        self.0 .0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl PipelineActivitySink for ActivityRegister {
    fn publish(&self, activity: PipelineActivity) {
        *self.activity() = activity;
        self.0 .1.notify_all();
    }
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use super::*;

    #[test]
    fn should_read_idle_before_anything_was_published() {
        // Given
        let register = ActivityRegister::default();
        // When
        let activity = register.current();
        // Then
        assert_eq!(activity, PipelineActivity::Idle);
    }

    #[test]
    fn should_read_what_a_clone_published_last() {
        // Given
        let register = ActivityRegister::default();
        let sink = register.clone();
        // When
        sink.publish(PipelineActivity::Recording);
        sink.publish(PipelineActivity::Processing);
        // Then
        assert_eq!(register.current(), PipelineActivity::Processing);
    }

    #[test]
    fn should_return_from_the_wait_as_soon_as_idle_is_published() {
        // Given a pipeline that ends its dictation a moment after the wait starts
        let register = ActivityRegister::default();
        register.publish(PipelineActivity::Processing);
        let sink = register.clone();
        let pipeline = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(30));
            sink.publish(PipelineActivity::Idle);
        });
        // When
        let started = Instant::now();
        let activity = register.wait_idle(Duration::from_secs(5));
        // Then the wait ended with the publication, long before its bound
        assert!(pipeline.join().is_ok());
        assert_eq!(activity, PipelineActivity::Idle);
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn should_return_from_the_wait_after_its_bound_while_the_dictation_goes_on() {
        // Given
        let register = ActivityRegister::default();
        register.publish(PipelineActivity::Recording);
        // When
        let started = Instant::now();
        let activity = register.wait_idle(Duration::from_millis(40));
        // Then
        assert_eq!(activity, PipelineActivity::Recording);
        assert!(started.elapsed() >= Duration::from_millis(40));
    }
}
