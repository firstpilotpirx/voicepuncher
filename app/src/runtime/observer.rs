//! The runtime state and who is told about it.

use std::fmt;
use std::path::PathBuf;

use crate::core::domain::{HotKey, Permission, RecordingMode};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeState {
    Starting,
    Ready {
        hotkey: HotKey,
        recording_mode: RecordingMode,
        voice_control: bool,
    },
    Restarting,
    /// No pipeline runs until every needed local service answers; `ready` of `total` do.
    StartingServices {
        ready: usize,
        total: usize,
    },
    /// No pipeline runs until every permission is granted; `revoked` were taken away while
    /// the pipeline ran.
    WaitingForPermissions {
        missing: Vec<Permission>,
        revoked: Vec<Permission>,
    },
    /// No service starts until every model file the configuration needs is in `folder`;
    /// `missing` are not there yet.
    WaitingForModels {
        folder: PathBuf,
        missing: Vec<&'static str>,
    },
    Stopped,
    Failed(String),
}

impl fmt::Display for RuntimeState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Starting => write!(f, "starting"),
            Self::Ready { hotkey, voice_control, .. } => write!(
                f,
                "ready (hotkey {hotkey}, voice control {})",
                if *voice_control { "on" } else { "off" }
            ),
            Self::Restarting => write!(f, "restarting"),
            Self::StartingServices { ready, total } => {
                write!(f, "starting services ({ready} of {total} ready)")
            }
            Self::WaitingForPermissions { missing, .. } => {
                let names: Vec<&str> = missing.iter().map(|p| p.name()).collect();
                write!(f, "waiting for permissions: {}", names.join(", "))
            }
            Self::WaitingForModels { folder, missing } => write!(
                f,
                "waiting for models in {}: {}",
                folder.display(),
                missing.join(", ")
            ),
            Self::Stopped => write!(f, "stopped"),
            Self::Failed(reason) => write!(f, "failed: {reason}"),
        }
    }
}

/// Told of every state change, on the runtime thread; a graphical observer dispatches to
/// the main thread itself.
pub trait RuntimeObserver: Send {
    fn state_changed(&self, state: &RuntimeState);
}

/// The observers and the state they last heard; none before the runtime starts.
#[derive(Default)]
pub struct Observers {
    state: Option<RuntimeState>,
    observers: Vec<Box<dyn RuntimeObserver>>,
}

impl Observers {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers `observer` and tells it the current state, when there is one.
    pub fn register(&mut self, observer: Box<dyn RuntimeObserver>) {
        if let Some(state) = &self.state {
            observer.state_changed(state);
        }
        self.observers.push(observer);
    }

    pub fn change(&mut self, state: RuntimeState) {
        for observer in &self.observers {
            observer.state_changed(&state);
        }
        self.state = Some(state);
    }

    pub fn current(&self) -> Option<&RuntimeState> {
        self.state.as_ref()
    }
}

/// The observer every process has: the state goes to the log.
pub struct LogObserver;

impl RuntimeObserver for LogObserver {
    fn state_changed(&self, state: &RuntimeState) {
        match state {
            RuntimeState::Failed(_) => log::error!("runtime: {state}"),
            other => log::info!("runtime: {other}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;

    struct Recording(Arc<Mutex<Vec<RuntimeState>>>);

    impl RuntimeObserver for Recording {
        fn state_changed(&self, state: &RuntimeState) {
            self.0.lock().unwrap().push(state.clone());
        }
    }

    fn recording() -> (Arc<Mutex<Vec<RuntimeState>>>, Box<dyn RuntimeObserver>) {
        let seen = Arc::new(Mutex::new(Vec::new()));
        (Arc::clone(&seen), Box::new(Recording(Arc::clone(&seen))))
    }

    fn ready() -> RuntimeState {
        RuntimeState::Ready {
            hotkey: HotKey::RightOption,
            recording_mode: RecordingMode::Toggle,
            voice_control: false,
        }
    }

    #[test]
    fn should_tell_every_state_in_order() {
        // Given
        let (seen, observer) = recording();
        let mut observers = Observers::new();
        observers.register(observer);
        // When
        observers.change(RuntimeState::Starting);
        observers.change(ready());
        observers.change(RuntimeState::Restarting);
        observers.change(ready());
        observers.change(RuntimeState::Stopped);
        // Then
        assert_eq!(
            *seen.lock().unwrap(),
            vec![
                RuntimeState::Starting,
                ready(),
                RuntimeState::Restarting,
                ready(),
                RuntimeState::Stopped
            ]
        );
    }

    #[test]
    fn should_tell_a_late_observer_the_current_state_at_registration() {
        // Given
        let mut observers = Observers::new();
        observers.change(RuntimeState::Starting);
        observers.change(ready());
        let (seen, observer) = recording();
        // When
        observers.register(observer);
        // Then
        assert_eq!(*seen.lock().unwrap(), vec![ready()]);
    }

    #[test]
    fn should_tell_an_observer_registered_before_the_start_nothing_yet() {
        // Given
        let mut observers = Observers::new();
        let (seen, observer) = recording();
        // When
        observers.register(observer);
        // Then
        assert!(seen.lock().unwrap().is_empty());
        assert_eq!(observers.current(), None);
    }

    #[test]
    fn should_describe_the_wait_with_the_missing_permissions() {
        let state = RuntimeState::WaitingForPermissions {
            missing: vec![Permission::Accessibility, Permission::InputMonitoring],
            revoked: vec![],
        };
        assert_eq!(
            state.to_string(),
            "waiting for permissions: Accessibility, Input Monitoring"
        );
    }

    #[test]
    fn should_describe_ready_with_the_hotkey_and_voice_control() {
        // Given
        let state = RuntimeState::Ready {
            hotkey: HotKey::F19,
            recording_mode: RecordingMode::Hold,
            voice_control: true,
        };
        // When
        let line = state.to_string();
        // Then
        assert_eq!(line, "ready (hotkey F19, voice control on)");
    }
}
