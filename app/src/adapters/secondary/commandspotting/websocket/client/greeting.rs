//! The hello that tells which protocol version a copy of the command spotting service speaks.

use std::io::{Read, Write};
use std::time::Instant;

use anyhow::{anyhow, bail, Result};
use tungstenite::protocol::frame::coding::CloseCode;
use tungstenite::{Message, WebSocket};

use super::handshake::handshake;
use super::{CONNECT_WAIT, PROTOCOL_VERSION};

/// Where the service's version stands in its refusal of another one.
const SERVICE_VERSION_MARK: &str = "the service version ";

/// What a copy answers to the app's hello.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Greeting {
    Accepted,
    /// The copy refused the hello: it speaks this protocol version.
    OtherVersion(u32),
}

/// The answer of the copy at `url` to the app's hello; the connection is closed after it.
pub fn protocol_at(url: &str) -> Result<Greeting> {
    let mut socket = handshake(url)?;
    let greeting = greet(&mut socket);
    if greeting.as_ref().is_ok_and(|g| *g == Greeting::Accepted) {
        let _ = socket.close(None);
    }
    greeting
}

/// Says hello in the app's version, then pings: the service answers the ping only after it took
/// the hello, and refuses another version with a policy close that names its own.
pub(super) fn greet<S: Read + Write>(socket: &mut WebSocket<S>) -> Result<Greeting> {
    let hello = serde_json::json!({ "type": "hello", "version": PROTOCOL_VERSION });
    socket.send(Message::text(hello.to_string()))?;
    socket.send(Message::Ping(tungstenite::Bytes::default()))?;
    let deadline = Instant::now() + CONNECT_WAIT;
    while Instant::now() < deadline {
        if let Some(greeting) = answer_to_hello(socket.read()?) {
            return greeting;
        }
    }
    bail!(
        "the service took no hello within {} s",
        CONNECT_WAIT.as_secs()
    )
}

/// What a message of the service says of the hello, or `None` while it says nothing of it.
fn answer_to_hello(message: Message) -> Option<Result<Greeting>> {
    match message {
        Message::Pong(_) => Some(Ok(Greeting::Accepted)),
        Message::Close(Some(frame)) if frame.code == CloseCode::Policy => Some(
            service_version(&frame.reason)
                .map(Greeting::OtherVersion)
                .ok_or_else(|| anyhow!("the service refused the hello: {}", frame.reason)),
        ),
        Message::Close(frame) => Some(Err(anyhow!("the service closed the connection: {frame:?}"))),
        _ => None,
    }
}

/// The version a refusal says the service speaks.
fn service_version(reason: &str) -> Option<u32> {
    let (_, after) = reason.rsplit_once(SERVICE_VERSION_MARK)?;
    after
        .split(|c: char| !c.is_ascii_digit())
        .next()?
        .parse()
        .ok()
}

#[cfg(test)]
mod tests;
