//! The permissions the runtime last waited for, kept for the windows that show them when
//! they open.

use std::sync::{Arc, Mutex, PoisonError};

use crate::core::domain::Permission;

use crate::runtime::{RuntimeObserver, RuntimeState};

/// The permissions missing in the runtime's last state; none unless it waits for them.
#[derive(Clone, Default)]
pub struct PermissionsWait {
    missing: Arc<Mutex<Vec<Permission>>>,
}

impl PermissionsWait {
    pub fn new() -> Self {
        Self::default()
    }

    /// The permissions missing now.
    pub fn missing(&self) -> Vec<Permission> {
        self.missing
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// The observer that keeps this wait current.
    pub fn observer(&self) -> Box<dyn RuntimeObserver> {
        Box::new(self.clone())
    }
}

impl RuntimeObserver for PermissionsWait {
    fn state_changed(&self, state: &RuntimeState) {
        let missing = match state {
            RuntimeState::WaitingForPermissions { missing, .. } => missing.clone(),
            _ => Vec::new(),
        };
        *self.missing.lock().unwrap_or_else(PoisonError::into_inner) = missing;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_keep_the_missing_permissions_of_the_last_state() {
        // Given
        let wait = PermissionsWait::new();
        let observer = wait.observer();
        // When
        observer.state_changed(&RuntimeState::WaitingForPermissions {
            missing: vec![Permission::Accessibility],
            revoked: vec![],
        });
        let waiting = wait.missing();
        observer.state_changed(&RuntimeState::Starting);
        // Then
        assert_eq!(waiting, [Permission::Accessibility]);
        assert!(wait.missing().is_empty());
    }
}
