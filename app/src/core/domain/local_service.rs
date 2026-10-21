//! A local service the app may supervise: what it is, how it starts and the state it is in.

use std::fmt;
use std::path::PathBuf;
use std::time::Duration;

/// The four local services, in the order they stop at quit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ServiceKind {
    /// Holds connections to the copies of the speech recognition service for commands.
    CommandSpotting,
    Dictation,
    Commands,
    Refinement,
}

impl ServiceKind {
    /// The service as the user reads it.
    pub fn name(self) -> &'static str {
        match self {
            ServiceKind::CommandSpotting => "command spotting service",
            ServiceKind::Dictation => "speech recognition service for dictation",
            ServiceKind::Commands => "speech recognition service for commands",
            ServiceKind::Refinement => "text refinement service",
        }
    }
}

/// How one process of a service starts: its program, arguments, port and log file; a
/// `problem` (a program or model not found) fails it without a start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceSpec {
    pub kind: ServiceKind,
    /// The service and, for a copy, which one: "dictation copy 18082".
    pub label: String,
    pub program: PathBuf,
    pub args: Vec<String>,
    pub port: u16,
    pub log_file: PathBuf,
    /// Checked while it runs, and killed when it stops answering; whisper copies are not.
    pub health_checked: bool,
    pub problem: Option<String>,
}

/// What a service is doing, as the app supervises it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceState {
    /// Not decided yet whether it is started or adopted.
    Pending,
    Starting {
        attempt: u32,
    },
    Ready,
    Restarting {
        attempt: u32,
        pause: Duration,
    },
    /// Given up after starts that never answered; started again only on a retry.
    Failed(String),
    /// Answered before the app started it: watched, never started or stopped by the app.
    Adopted,
    /// An adopted service stopped answering: waited for, never replaced.
    StoppedOutside,
    Stopped,
}

impl ServiceState {
    /// Counted ready for dictation.
    pub fn is_ready(&self) -> bool {
        matches!(self, ServiceState::Ready | ServiceState::Adopted)
    }
}

impl fmt::Display for ServiceState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ServiceState::Pending => f.write_str("Pending"),
            ServiceState::Starting { .. } => f.write_str("Starting"),
            ServiceState::Ready => f.write_str("Ready"),
            ServiceState::Restarting { attempt, .. } => write!(f, "Restarting (try {attempt})"),
            ServiceState::Failed(_) => f.write_str("Failed — see log"),
            ServiceState::Adopted => f.write_str("Adopted"),
            ServiceState::StoppedOutside => f.write_str("Stopped outside the app"),
            ServiceState::Stopped => f.write_str("Stopped"),
        }
    }
}

/// One supervised service as the menu and the runtime see it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceStatus {
    pub kind: ServiceKind,
    pub label: String,
    pub port: u16,
    pub state: ServiceState,
    /// Started by the app, so the app may restart and stop it.
    pub owned: bool,
    pub log_file: PathBuf,
}

/// How far the services one configuration needs have come.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServicesProgress {
    pub ready: usize,
    pub total: usize,
    /// The first needed service that failed, and why.
    pub failed: Option<String>,
}

impl ServicesProgress {
    /// Every needed service is ready.
    pub fn all_ready(&self) -> bool {
        self.ready == self.total
    }

    /// The progress of the services on `ports` among `statuses`; a port not supervised yet
    /// counts as not ready.
    pub fn of(ports: &[u16], statuses: &[ServiceStatus]) -> Self {
        let needed: Vec<&ServiceStatus> = ports
            .iter()
            .filter_map(|port| statuses.iter().find(|status| status.port == *port))
            .collect();
        let failed = needed.iter().find_map(|status| match &status.state {
            ServiceState::Failed(reason) => Some(format!("{}: {reason}", status.label)),
            _ => None,
        });
        Self {
            ready: needed
                .iter()
                .filter(|status| status.state.is_ready())
                .count(),
            total: ports.len(),
            failed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status(port: u16, state: ServiceState) -> ServiceStatus {
        ServiceStatus {
            kind: ServiceKind::Dictation,
            label: format!("dictation copy {port}"),
            port,
            state,
            owned: true,
            log_file: PathBuf::from("whisper-server.log"),
        }
    }

    #[test]
    fn should_stop_command_spotting_first_and_refinement_last() {
        let mut kinds = [
            ServiceKind::Refinement,
            ServiceKind::Dictation,
            ServiceKind::CommandSpotting,
            ServiceKind::Commands,
        ];
        kinds.sort();
        assert_eq!(kinds.first(), Some(&ServiceKind::CommandSpotting));
        assert_eq!(kinds.last(), Some(&ServiceKind::Refinement));
    }

    #[test]
    fn should_count_ready_and_adopted_services_of_the_needed_ports() {
        // Given
        let statuses = vec![
            status(1, ServiceState::Ready),
            status(2, ServiceState::Adopted),
            status(3, ServiceState::Starting { attempt: 1 }),
            status(4, ServiceState::Ready),
        ];
        // When
        let progress = ServicesProgress::of(&[1, 2, 3, 5], &statuses);
        // Then
        assert_eq!(
            progress,
            ServicesProgress {
                ready: 2,
                total: 4,
                failed: None
            }
        );
        assert!(!progress.all_ready());
    }

    #[test]
    fn should_tell_the_reason_of_a_failed_needed_service() {
        let statuses = vec![status(1, ServiceState::Failed("no model".into()))];
        let progress = ServicesProgress::of(&[1], &statuses);
        assert_eq!(
            progress.failed.as_deref(),
            Some("dictation copy 1: no model")
        );
    }

    #[test]
    fn should_describe_states_in_words() {
        let restarting = ServiceState::Restarting {
            attempt: 3,
            pause: Duration::from_secs(4),
        };
        assert_eq!(restarting.to_string(), "Restarting (try 3)");
        assert_eq!(
            ServiceState::StoppedOutside.to_string(),
            "Stopped outside the app"
        );
    }
}
