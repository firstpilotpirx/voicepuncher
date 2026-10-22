//! The WebSocket handshake with the command spotting service, at its own port or a copy's.

use std::net::TcpStream;

use anyhow::{anyhow, Result};
use tungstenite::stream::MaybeTlsStream;
use tungstenite::WebSocket;
use voicepuncher_copy_pool::{ServiceAddress, COMMAND_SPOTTING_COPY_OFFSET};

use super::CONNECT_WAIT;

/// A WebSocket connection to the service's own port, else to its first copy that
/// completes the handshake; the error of the own port when none does.
pub(super) fn handshake_any(url: &str) -> Result<WebSocket<MaybeTlsStream<TcpStream>>> {
    let own = handshake(url);
    if own.is_ok() {
        return own;
    }
    let copies: Vec<String> = ServiceAddress::parse(url).map_or_else(
        |_| Vec::new(),
        |address| {
            let address = address.with_copy_offset(COMMAND_SPOTTING_COPY_OFFSET);
            address
                .copy_ports()
                .into_iter()
                .map(|port| address.with_port(port))
                .collect()
        },
    );
    copies
        .iter()
        .find_map(|copy| handshake(copy).ok())
        .map_or(own, Ok)
}

/// A WebSocket connection to `url`, given up after `CONNECT_WAIT` at each step.
fn handshake(url: &str) -> Result<WebSocket<MaybeTlsStream<TcpStream>>> {
    let parsed = url::Url::parse(url)?;
    let mut refused = anyhow!("no address for {url}");
    let stream = parsed
        .socket_addrs(|| None)?
        .iter()
        .find_map(|addr| {
            TcpStream::connect_timeout(addr, CONNECT_WAIT)
                .map_err(|e| refused = e.into())
                .ok()
        })
        .ok_or(refused)?;
    stream.set_read_timeout(Some(CONNECT_WAIT))?;
    stream.set_write_timeout(Some(CONNECT_WAIT))?;
    let (socket, _) = tungstenite::client(url, MaybeTlsStream::Plain(stream))
        .map_err(|e| anyhow!("WebSocket handshake: {e}"))?;
    Ok(socket)
}

#[cfg(test)]
mod tests;
