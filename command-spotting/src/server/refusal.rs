//! Closing a connection whose app breaks the protocol, with the reason.

use std::fmt;
use std::net::TcpStream;

use anyhow::Result;
use tungstenite::protocol::frame::coding::CloseCode;
use tungstenite::protocol::CloseFrame;
use tungstenite::WebSocket;

use super::Input;
use crate::protocol::PROTOCOL_VERSION;

/// The longest reason a close frame carries.
const MOST_REASON_BYTES: usize = 123;

/// Why the service closes a connection, and the close code that says it.
#[derive(Debug)]
pub(super) struct Refusal {
    code: CloseCode,
    reason: String,
}

impl Refusal {
    /// The app asks for what the service does not do: another protocol version, a phrase
    /// it does not know.
    pub(super) fn policy(reason: String) -> Self {
        Self { code: CloseCode::Policy, reason }
    }

    /// The app sent a message the protocol has no meaning for.
    pub(super) fn invalid(reason: String) -> Self {
        Self { code: CloseCode::Invalid, reason }
    }

    /// The app said nothing of its protocol version first: an app of version 1 says none.
    pub(super) fn no_hello() -> Self {
        Self::policy(format!(
            "no hello first: the app speaks protocol version 1, the service version {PROTOCOL_VERSION}"
        ))
    }
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.reason)
    }
}

impl std::error::Error for Refusal {}

/// Lets the app's messages through once it said hello in this service's protocol version.
pub(super) fn greet(input: Option<Input>, greeted: &mut bool) -> Result<Option<Input>> {
    match input {
        Some(Input::Hello(PROTOCOL_VERSION)) => {
            *greeted = true;
            Ok(None)
        }
        Some(Input::Hello(version)) => Err(Refusal::policy(format!(
            "the app speaks protocol version {version}, the service version {PROTOCOL_VERSION}"
        ))
        .into()),
        Some(_) if !*greeted => Err(Refusal::no_hello().into()),
        input => Ok(input),
    }
}

/// Closes the connection with the reason of a refusal; any other error ends it as it is.
pub(super) fn refuse_or_end(
    socket: &mut WebSocket<TcpStream>,
    conn: u64,
    error: anyhow::Error,
) -> Result<()> {
    match error.downcast::<Refusal>() {
        Ok(refusal) => refuse(socket, conn, refusal),
        Err(error) => Err(error),
    }
}

pub(super) fn refuse(socket: &mut WebSocket<TcpStream>, conn: u64, refusal: Refusal) -> Result<()> {
    log::warn!("conn={conn} refusing the connection: {refusal}");
    let mut end = refusal.reason.len().min(MOST_REASON_BYTES);
    while !refusal.reason.is_char_boundary(end) {
        end -= 1;
    }
    socket.close(Some(CloseFrame {
        code: refusal.code,
        reason: refusal.reason[..end].to_owned().into(),
    }))?;
    while socket.read().is_ok() {}
    Ok(())
}
