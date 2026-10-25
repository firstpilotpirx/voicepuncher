use std::time::Duration;

use voicepuncher_copy_pool::PoolError;

use crate::adapters::secondary::http::failure::Failure;
use crate::core::domain::{ForeignProgram, ServicePortTaken};

/// Why one attempt brought no answer.
pub(super) struct AttemptFailure {
    pub(super) failure: Failure,
    /// What the service did, for the log.
    pub(super) what: String,
    pub(super) error: String,
    /// Only the wait for a free copy ran out: the request still holds its place in the queue.
    pub(super) waiting: bool,
    /// No copy answers and another program holds the service's own port.
    taken: Option<ServicePortTaken>,
}

impl AttemptFailure {
    fn new(failure: Failure, what: String, error: String) -> Self {
        Self { failure, what, error, waiting: false, taken: None }
    }

    pub(super) fn of(error: &ureq::Error) -> Self {
        let failure = Failure::of(error);
        Self::new(failure, failure.describe(), error.to_string())
    }

    pub(super) fn of_pool(error: &PoolError) -> Self {
        let (failure, what) = match error {
            PoolError::NoFreeCopy { .. } => (Failure::TimedOut, "had no free copy in time"),
            PoolError::NoCopies { .. } => (Failure::Dropped, "accepts no connections"),
        };
        let taken = match error {
            PoolError::NoCopies { foreign: Some(foreign), .. } => {
                Some(ServicePortTaken(ForeignProgram {
                    port: foreign.port,
                    program: foreign.to_string(),
                }))
            }
            _ => None,
        };
        Self {
            waiting: matches!(error, PoolError::NoFreeCopy { .. }),
            taken,
            ..Self::new(failure, what.to_owned(), error.to_string())
        }
    }

    pub(super) fn no_answer(port: u16, deadline: Duration) -> Self {
        let failure = Failure::TimedOut;
        let error = format!("copy {port} sent no answer in {}ms", deadline.as_millis());
        Self::new(failure, failure.describe(), error)
    }

    pub(super) fn unreadable(error: &std::io::Error) -> Self {
        Self::new(
            Failure::Other,
            "sent an unreadable answer".to_owned(),
            error.to_string(),
        )
    }

    /// The error the request gives up with, described by `message`; it carries the program on
    /// the service's port when one holds it.
    pub(super) fn into_error(self, message: String) -> anyhow::Error {
        match self.taken {
            Some(taken) => anyhow::Error::new(taken).context(message),
            None => anyhow::Error::msg(message),
        }
    }
}
