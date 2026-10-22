//! What went wrong with a request to a local service, so the clients of the speech
//! recognition and text refinement services retry what a new attempt can fix.

use std::io;

/// Why a request to a local service failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Failure {
    /// The service refused or dropped the connection before it answered.
    Dropped,
    /// The service did not answer within the request's deadline.
    TimedOut,
    /// The service answered with an error a later attempt may not get (5xx, 429).
    Busy(u16),
    /// Anything a new attempt will not change: another error answer, a broken request.
    Other,
}

impl Failure {
    pub fn of(error: &ureq::Error) -> Self {
        match error {
            ureq::Error::Status(code, _) if *code >= 500 || *code == TOO_MANY_REQUESTS => {
                Self::Busy(*code)
            }
            ureq::Error::Status(..) => Self::Other,
            ureq::Error::Transport(_) if is_dropped_connection(error) => Self::Dropped,
            ureq::Error::Transport(transport) if io_errors(transport).any(is_timeout) => {
                Self::TimedOut
            }
            ureq::Error::Transport(_) => Self::Other,
        }
    }

    pub fn describe(self) -> String {
        match self {
            Self::Dropped => "dropped the connection".to_owned(),
            Self::TimedOut => "did not answer in time".to_owned(),
            Self::Busy(code) => format!("answered with an error (HTTP {code})"),
            Self::Other => "failed".to_owned(),
        }
    }
}

const TOO_MANY_REQUESTS: u16 = 429;

/// True when the service refused or dropped the connection before it answered; an
/// error answer or a timeout is not a dropped connection.
pub fn is_dropped_connection(error: &ureq::Error) -> bool {
    let ureq::Error::Transport(transport) = error else {
        return false;
    };
    transport.kind() == ureq::ErrorKind::ConnectionFailed || io_errors(transport).any(is_dropped_io)
}

/// Every I/O error in the chain: ureq may rewrap the OS error.
fn io_errors(transport: &ureq::Transport) -> impl Iterator<Item = &io::Error> {
    std::iter::successors(std::error::Error::source(transport), |cause| cause.source())
        .filter_map(|cause| cause.downcast_ref::<io::Error>())
}

fn is_timeout(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock
    )
}

/// macOS reports a socket call on a connection the peer already reset as EINVAL; an
/// invalid-input error raised by a library has no OS code and is not a drop.
pub fn is_dropped_io(error: &io::Error) -> bool {
    match error.kind() {
        io::ErrorKind::ConnectionRefused
        | io::ErrorKind::ConnectionReset
        | io::ErrorKind::ConnectionAborted
        | io::ErrorKind::BrokenPipe
        | io::ErrorKind::UnexpectedEof => true,
        io::ErrorKind::InvalidInput => error.raw_os_error().is_some(),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status(code: u16) -> ureq::Error {
        ureq::Error::Status(code, ureq::Response::new(code, "status", "body").unwrap())
    }

    fn transport(error: io::Error) -> ureq::Error {
        ureq::Error::from(error)
    }

    #[test]
    fn should_treat_reset_refused_and_closed_connections_as_dropped() {
        for kind in [
            io::ErrorKind::ConnectionRefused,
            io::ErrorKind::ConnectionReset,
            io::ErrorKind::ConnectionAborted,
            io::ErrorKind::BrokenPipe,
            io::ErrorKind::UnexpectedEof,
        ] {
            assert!(is_dropped_io(&io::Error::from(kind)), "{kind:?}");
        }
    }

    #[test]
    fn should_not_treat_timeout_as_dropped() {
        assert!(!is_dropped_io(&io::Error::from(io::ErrorKind::TimedOut)));
        assert!(!is_dropped_io(&io::Error::from(io::ErrorKind::WouldBlock)));
    }

    #[test]
    fn should_treat_os_invalid_argument_as_dropped() {
        // macOS reports a connection the service already reset as EINVAL (os error 22)
        let einval = io::Error::from_raw_os_error(22);
        assert_eq!(einval.kind(), io::ErrorKind::InvalidInput);
        assert!(is_dropped_io(&einval));
    }

    #[test]
    fn should_treat_os_invalid_argument_wrapped_by_http_client_as_dropped() {
        // ureq rewraps a status-line read error in an io::Error without the OS code
        let einval = io::Error::from_raw_os_error(22);
        let rewrapped = io::Error::new(einval.kind(), ureq::Error::from(einval));
        assert!(is_dropped_connection(&ureq::Error::from(rewrapped)));
    }

    #[test]
    fn should_not_treat_library_invalid_input_as_dropped() {
        let invalid = io::Error::new(io::ErrorKind::InvalidInput, "bad header");
        assert!(!is_dropped_io(&invalid));
    }

    #[test]
    fn should_classify_a_timeout_as_timed_out() {
        let timeout = io::Error::new(io::ErrorKind::TimedOut, "timed out reading response");
        assert_eq!(Failure::of(&transport(timeout)), Failure::TimedOut);
    }

    #[test]
    fn should_classify_server_errors_and_too_many_requests_as_busy() {
        assert_eq!(Failure::of(&status(500)), Failure::Busy(500));
        assert_eq!(Failure::of(&status(503)), Failure::Busy(503));
        assert_eq!(Failure::of(&status(429)), Failure::Busy(429));
    }

    #[test]
    fn should_not_retry_other_error_answers() {
        assert_eq!(Failure::of(&status(400)), Failure::Other);
        assert_eq!(Failure::of(&status(404)), Failure::Other);
    }

    #[test]
    fn should_classify_a_reset_as_dropped() {
        let reset = io::Error::from(io::ErrorKind::ConnectionReset);
        assert_eq!(Failure::of(&transport(reset)), Failure::Dropped);
    }
}
