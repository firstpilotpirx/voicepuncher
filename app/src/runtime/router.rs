//! Hotkey signals reach the running pipeline through a router the runtime points at the
//! current pipeline; while no pipeline runs, signals are dropped.

use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use crate::core::application::PipelineInput;
use crate::core::domain::PttSignal;

#[derive(Clone, Default)]
pub struct SignalRouter {
    target: Arc<Mutex<Option<Sender<PipelineInput>>>>,
}

impl SignalRouter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Signals go to `pipeline` from now on.
    pub fn route_to(&self, pipeline: Sender<PipelineInput>) {
        *self.lock() = Some(pipeline);
    }

    /// Signals are dropped from now on, until the next `route_to`.
    pub fn detach(&self) {
        *self.lock() = None;
    }

    pub fn forward(&self, signal: PttSignal) {
        let target = self.lock().clone();
        match target {
            Some(pipeline) => {
                if pipeline.send(PipelineInput::Hotkey(signal)).is_err() {
                    log::debug!("hotkey {signal:?} dropped: the pipeline is gone");
                }
            }
            None => log::debug!("hotkey {signal:?} dropped: no pipeline runs"),
        }
    }

    /// Forwards every signal from `hotkeys` until the listener ends.
    pub fn spawn_forwarding(&self, hotkeys: Receiver<PttSignal>) -> JoinHandle<()> {
        let router = self.clone();
        std::thread::Builder::new()
            .name("hotkey-router".into())
            .spawn(move || {
                for signal in hotkeys {
                    router.forward(signal);
                }
            })
            .expect("failed to spawn hotkey router thread")
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Option<Sender<PipelineInput>>> {
        self.target
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use super::*;

    #[test]
    fn should_forward_a_signal_to_the_routed_pipeline() {
        // Given
        let router = SignalRouter::new();
        let (tx, rx) = mpsc::channel();
        router.route_to(tx);
        // When
        router.forward(PttSignal::Pressed);
        // Then
        assert_eq!(rx.try_recv(), Ok(PipelineInput::Hotkey(PttSignal::Pressed)));
    }

    #[test]
    fn should_drop_a_signal_while_no_pipeline_is_routed() {
        // Given
        let router = SignalRouter::new();
        let (tx, rx) = mpsc::channel();
        router.route_to(tx);
        router.detach();
        // When
        router.forward(PttSignal::Pressed);
        // Then
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn should_forward_to_the_new_pipeline_after_a_swap() {
        // Given
        let router = SignalRouter::new();
        let (old_tx, old_rx) = mpsc::channel();
        let (new_tx, new_rx) = mpsc::channel();
        router.route_to(old_tx);
        router.route_to(new_tx);
        // When
        router.forward(PttSignal::Released);
        // Then
        assert!(old_rx.try_recv().is_err());
        assert_eq!(
            new_rx.try_recv(),
            Ok(PipelineInput::Hotkey(PttSignal::Released))
        );
    }

    #[test]
    fn should_forward_from_the_listener_channel_on_its_thread() {
        // Given
        let router = SignalRouter::new();
        let (pipeline_tx, pipeline_rx) = mpsc::channel();
        router.route_to(pipeline_tx);
        let (hotkey_tx, hotkey_rx) = mpsc::channel();
        let forwarder = router.spawn_forwarding(hotkey_rx);
        // When
        hotkey_tx.send(PttSignal::Cancel).unwrap();
        drop(hotkey_tx);
        forwarder.join().unwrap();
        // Then
        assert_eq!(
            pipeline_rx.try_recv(),
            Ok(PipelineInput::Hotkey(PttSignal::Cancel))
        );
    }
}
