//! Fake command spotting copies of the app's protocol version and of an older one, on the copy
//! ports of an address nothing else listens on.

use std::net::{TcpListener, TcpStream};
use voicepuncher::test_doubles::prelude::*;

use cucumber::{given, then, when};
use tungstenite::Message;
use voicepuncher::adapters::secondary::commandspotting::websocket::client::SpottingClient;
use voicepuncher::adapters::secondary::commandspotting::websocket::test_stream::refuse_as_version_2;
use voicepuncher_copy_pool::{ServiceAddress, COMMAND_SPOTTING_COPY_OFFSET};

use super::super::local;
use crate::IntegrationWorld;

/// The service ports the fake addresses take: their copy ports land below the system's
/// ephemeral ports.
const FAKE_SERVICE_PORTS: std::ops::Range<u16> = 3_000..8_000;
const COMMAND: &str = r#"{"event":"command","commands":["press enter"],"start":0.5,"end":1.0,"placement":"anywhere"}"#;
/// How long the client waits for the command the copy says.
const COMMAND_WITHIN: Duration = Duration::from_secs(3);

/// What one scenario set up: the service's address, the copies' ports and what the client got.
#[derive(Debug, Default)]
pub(in super::super) struct StaleRun {
    url: String,
    copy_ports: Vec<u16>,
    served_by: Option<u16>,
    commands: Vec<SpottingEvent>,
}

/// A fake copy: it refuses the hello as version 2, or takes it and says a command once it got
/// a selection.
fn serve(listener: TcpListener, older: bool) {
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            std::thread::spawn(move || answer(stream, older));
        }
    });
}

fn answer(stream: TcpStream, older: bool) {
    let Ok(mut socket) = tungstenite::accept(stream) else {
        return;
    };
    if older {
        return refuse_as_version_2(&mut socket);
    }
    while let Ok(message) = socket.read() {
        if matches!(&message, Message::Text(text) if text.contains("select"))
            && socket.send(Message::text(COMMAND)).is_err()
        {
            return;
        }
    }
}

/// An address whose own port is free and listeners on its first two command spotting copy ports.
fn free_spotting_copies() -> Result<(String, Vec<TcpListener>)> {
    let start = std::process::id() as usize * 7_919;
    let span = FAKE_SERVICE_PORTS.len();
    for step in 0..span {
        let port = FAKE_SERVICE_PORTS.start + u16::try_from((start + step * 13) % span)?;
        let url = format!("ws://127.0.0.1:{port}");
        if TcpListener::bind(("127.0.0.1", port)).is_err() {
            continue;
        }
        let address = ServiceAddress::parse(&url)?.with_copy_offset(COMMAND_SPOTTING_COPY_OFFSET);
        let listeners: Result<Vec<_>, _> = address.copy_ports()[..2]
            .iter()
            .map(|copy| TcpListener::bind(("127.0.0.1", *copy)))
            .collect();
        if let Ok(listeners) = listeners {
            return Ok((url, listeners));
        }
    }
    bail!("no free command spotting address in {FAKE_SERVICE_PORTS:?}")
}

fn stale(world: &mut IntegrationWorld) -> Result<&mut StaleRun> {
    local(world)
        .stale
        .as_mut()
        .context("no copies were started")
}

#[given("two copies of the command spotting service answer and the first speaks an older protocol version")]
fn first_copy_older(world: &mut IntegrationWorld) -> Result<()> {
    let (url, listeners) = free_spotting_copies()?;
    let mut copy_ports = Vec::new();
    for (index, listener) in listeners.into_iter().enumerate() {
        copy_ports.push(listener.local_addr()?.port());
        serve(listener, index == 0);
    }
    local(world).stale = Some(StaleRun { url, copy_ports, ..StaleRun::default() });
    Ok(())
}

#[when("voice control connects")]
fn voice_control_connects(world: &mut IntegrationWorld) -> Result<()> {
    let run = stale(world)?;
    let mut client = SpottingClient::connect(&run.url, Instant::now())?;
    run.served_by = client.peer_port();
    client.listen_for(&[VoiceCommand::PressEnter])?;
    let deadline = Instant::now() + COMMAND_WITHIN;
    while run.commands.is_empty() && Instant::now() < deadline {
        run.commands.extend(client.read_events()?);
    }
    Ok(())
}

#[then("it is served by the second copy and a spoken command is carried out")]
fn served_by_second(world: &mut IntegrationWorld) -> Result<()> {
    let run = stale(world)?;
    ensure!(
        run.served_by == run.copy_ports.get(1).copied(),
        "served by {:?}, copies {:?}",
        run.served_by,
        run.copy_ports
    );
    ensure!(
        matches!(run.commands.first(), Some(SpottingEvent::CommandsSaid { commands, .. }) if commands == &[VoiceCommand::PressEnter]),
        "{:?}",
        run.commands
    );
    Ok(())
}
