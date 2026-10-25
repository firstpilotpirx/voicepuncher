//! The command spotting service's side of voice control: the protocol it speaks with the app,
//! checked against the running service, and how it reads a confirmation's transcript.
//!
//! What the service answered is kept in the world's `stdout`, as the output of the system
//! under test.

use std::net::TcpStream;
use voicepuncher::test_doubles::prelude::*;

use cucumber::{then, when};
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{Message, WebSocket};

use voicepuncher_command_spotting::matcher::PhraseMatcher;
use voicepuncher_command_spotting::protocol::PROTOCOL_VERSION;

use crate::steps::off_thread;
use crate::E2eWorld;

mod confirmations;
pub(crate) mod scheduling;

/// How long the service may take to close a connection it refuses.
const CLOSED_WITHIN: Duration = Duration::from_secs(5);

fn hello(version: u32) -> Message {
    Message::text(format!(r#"{{"type":"hello","version":{version}}}"#))
}

/// Opens a connection to the service, sends `messages` and waits for the service to close
/// it; the close frame's code and reason.
fn closed_after(world: &E2eWorld, messages: Vec<Message>) -> Result<String> {
    let url = world
        .voice
        .command_spotting_url
        .as_deref()
        .context("the command spotting service is not set up")?;
    let (mut socket, _) = tungstenite::connect(url)?;
    if let MaybeTlsStream::Plain(stream) = socket.get_ref() {
        stream.set_read_timeout(Some(Duration::from_millis(100)))?;
    }
    for message in messages {
        socket.send(message)?;
    }
    Ok(close_frame(&mut socket))
}

fn close_frame(socket: &mut WebSocket<MaybeTlsStream<TcpStream>>) -> String {
    let deadline = Instant::now() + CLOSED_WITHIN;
    while Instant::now() < deadline {
        match socket.read() {
            Ok(Message::Close(Some(frame))) => {
                return format!("{:?}: {}", frame.code, frame.reason)
            }
            Ok(Message::Close(None)) => return "closed without a reason".to_owned(),
            Ok(_) => {}
            Err(tungstenite::Error::Io(e)) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(e) => return format!("ended without a close: {e}"),
        }
    }
    format!("still open after {CLOSED_WITHIN:?}")
}

/// Keeps how the service closed a connection that sent `messages`, waiting off the
/// harness's thread.
async fn close_after(world: &mut E2eWorld, messages: Vec<Message>) -> Result<()> {
    off_thread(world, |world| {
        world.app.stdout = closed_after(world, messages)?;
        Ok(())
    })
    .await
}

#[when("a client opens a connection to it with another protocol version")]
async fn other_version(world: &mut E2eWorld) -> Result<()> {
    close_after(world, vec![hello(PROTOCOL_VERSION - 1)]).await
}

#[when("a client connected with the right protocol version sends a message that is not valid")]
async fn malformed(world: &mut E2eWorld) -> Result<()> {
    close_after(
        world,
        vec![hello(PROTOCOL_VERSION), Message::text("{nonsense")],
    )
    .await
}

#[when(
    "a client connected with the right protocol version sends an audio frame whose length is not a whole number of samples"
)]
async fn partial_samples(world: &mut E2eWorld) -> Result<()> {
    close_after(
        world,
        vec![hello(PROTOCOL_VERSION), Message::binary(vec![0u8; 5])],
    )
    .await
}

fn closed_with(world: &E2eWorld, code: &str, fragments: &[&str]) -> Result<()> {
    let closed = &world.app.stdout;
    if !closed.starts_with(code) || !fragments.iter().all(|f| closed.contains(f)) {
        bail!("the service closed the connection as {closed:?}, not {code} naming {fragments:?}");
    }
    Ok(())
}

#[then("the service closes the connection with a reason naming the version it speaks")]
fn names_its_version(world: &mut E2eWorld) -> Result<()> {
    closed_with(world, "Policy", &[&format!("version {PROTOCOL_VERSION}")])
}

#[then("the service closes the connection with a reason saying the message could not be read")]
fn says_malformed(world: &mut E2eWorld) -> Result<()> {
    closed_with(world, "Invalid", &["malformed message"])
}

#[then("the service closes the connection with a reason saying the frame holds partial samples")]
fn says_partial(world: &mut E2eWorld) -> Result<()> {
    closed_with(world, "Invalid", &["5 bytes"])
}

#[when(
    expr = "the speech recognition service for commands hears {string} in one utterance while no dictation runs"
)]
fn hears_in_standby(world: &mut E2eWorld, transcript: String) {
    let phrases: Vec<String> = VoiceCommand::STANDBY
        .iter()
        .map(|c| c.phrase().to_owned())
        .collect();
    world.app.stdout = PhraseMatcher::new(&phrases)
        .phrase_in(&transcript)
        .unwrap_or_default();
}

#[then(expr = "the command spotting service reports {string} once")]
fn reported_once(world: &mut E2eWorld, phrase: String) -> Result<()> {
    if world.app.stdout != phrase {
        bail!(
            "the service took the utterance as {:?}, not {phrase:?} once",
            world.app.stdout
        );
    }
    Ok(())
}
