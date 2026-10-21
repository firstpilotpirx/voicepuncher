//! The pipeline's activity as the runtime reads it before a restart.

use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

use crate::core::application::{PipelineActivity, PipelineActivitySink};

/// What the running pipeline last published; Idle before it publishes anything.
#[derive(Clone, Default)]
pub struct ActivityRegister(Arc<AtomicU8>);

impl ActivityRegister {
    pub fn current(&self) -> PipelineActivity {
        match self.0.load(Ordering::SeqCst) {
            1 => PipelineActivity::Recording,
            2 => PipelineActivity::Processing,
            _ => PipelineActivity::Idle,
        }
    }
}

impl PipelineActivitySink for ActivityRegister {
    fn publish(&self, activity: PipelineActivity) {
        let value = match activity {
            PipelineActivity::Idle => 0,
            PipelineActivity::Recording => 1,
            PipelineActivity::Processing => 2,
        };
        self.0.store(value, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
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
}
