//! The watchdog over the microphone: a device that reports an error or stops delivering
//! audio ends the dictation through the pipeline's input, once.

use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::time::Duration;

use super::capture::CaptureHealth;
use crate::core::application::PipelineInput;

/// How long the device may stay silent once it delivered audio.
pub const SILENCE_LIMIT: Duration = Duration::from_secs(1);
/// How long the device may take to deliver its first audio.
pub const START_LIMIT: Duration = Duration::from_secs(5);

pub struct Watchdog {
    health: Arc<CaptureHealth>,
    /// Taken by the first report, so a failure is reported once.
    failures: Option<Sender<PipelineInput>>,
}

impl Watchdog {
    pub fn new(health: Arc<CaptureHealth>, failures: Option<Sender<PipelineInput>>) -> Self {
        Self { health, failures }
    }

    /// Reports a failed device to the pipeline, the first time it is seen.
    pub fn check(&mut self) {
        let Some(reason) = self.failure() else {
            return;
        };
        if let Some(failures) = self.failures.take() {
            log::error!("recording: {reason}");
            let _ = failures.send(PipelineInput::RecordingFailed(reason));
        }
    }

    fn failure(&self) -> Option<String> {
        if self.health.failed() {
            return Some("the input device reported an error".to_owned());
        }
        let silence = self.health.since_last_callback();
        let limit = if self.health.had_callback() {
            SILENCE_LIMIT
        } else {
            START_LIMIT
        };
        (silence > limit).then(|| {
            format!(
                "the input device delivered no audio for {} ms",
                silence.as_millis()
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicU32;
    use std::sync::mpsc;

    use super::*;

    fn watchdog() -> (Watchdog, Arc<CaptureHealth>, mpsc::Receiver<PipelineInput>) {
        let health = Arc::new(CaptureHealth::new(Arc::new(AtomicU32::new(0))));
        let (tx, rx) = mpsc::channel();
        (Watchdog::new(Arc::clone(&health), Some(tx)), health, rx)
    }

    #[test]
    fn should_report_nothing_while_the_device_is_healthy() {
        let (mut watchdog, _health, rx) = watchdog();
        watchdog.check();
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn should_report_the_error_flag_once() {
        let (mut watchdog, health, rx) = watchdog();
        health.note_error();
        watchdog.check();
        watchdog.check();
        assert!(
            matches!(rx.try_recv(), Ok(PipelineInput::RecordingFailed(r)) if r.contains("error"))
        );
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn should_give_a_fresh_device_the_start_limit() {
        let (watchdog, _health, _rx) = watchdog();
        assert!(watchdog.failure().is_none());
    }
}
