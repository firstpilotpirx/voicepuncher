//! The WebSocket handshake with the command spotting service, at its own port or a copy's.

use std::net::TcpStream;

use anyhow::{anyhow, Result};
use tungstenite::stream::MaybeTlsStream;
use tungstenite::WebSocket;
use voicepuncher_copy_pool::{ServiceAddress, COMMAND_SPOTTING_COPY_OFFSET};

use super::greeting::{greet, Greeting};
use super::{CONNECT_WAIT, PROTOCOL_VERSION};

/// A connection that took the app's hello, to the service's own port, else to its first copy
/// that takes it; a copy that refuses the hello for its protocol version is passed over and
/// its port handed to `refused`. The error of the own port when none takes it.
pub(super) fn greeted_any(
    url: &str,
    refused: &dyn Fn(u16),
) -> Result<WebSocket<MaybeTlsStream<TcpStream>>> {
    let own = greeted(url, refused);
    if own.is_ok() {
        return own;
    }
    copy_urls(url)
        .iter()
        .find_map(|copy| greeted(copy, refused).ok())
        .map_or(own, Ok)
}

/// The addresses of the service's copies.
fn copy_urls(url: &str) -> Vec<String> {
    ServiceAddress::parse(url).map_or_else(
        |_| Vec::new(),
        |address| {
            let address = address.with_copy_offset(COMMAND_SPOTTING_COPY_OFFSET);
            address
                .copy_ports()
                .into_iter()
                .map(|port| address.with_port(port))
                .collect()
        },
    )
}

/// A connection to `url` that took the app's hello.
fn greeted(url: &str, refused: &dyn Fn(u16)) -> Result<WebSocket<MaybeTlsStream<TcpStream>>> {
    let mut socket = handshake(url)?;
    match greet(&mut socket)? {
        Greeting::Accepted => Ok(socket),
        Greeting::OtherVersion(version) => {
            let port = url::Url::parse(url)?.port().unwrap_or_default();
            log::warn!(
                "command spotting: the copy at port {port} speaks protocol version {version}, the app version {PROTOCOL_VERSION} — passed over"
            );
            refused(port);
            Err(anyhow!(
                "the copy at port {port} speaks protocol version {version}"
            ))
        }
    }
}

/// A WebSocket connection to `url`, given up after `CONNECT_WAIT` at each step.
pub(super) fn handshake(url: &str) -> Result<WebSocket<MaybeTlsStream<TcpStream>>> {
    let stream = connect(&url::Url::parse(url)?)?;
    stream.set_read_timeout(Some(CONNECT_WAIT))?;
    stream.set_write_timeout(Some(CONNECT_WAIT))?;
    let (socket, _) = tungstenite::client(url, MaybeTlsStream::Plain(stream))
        .map_err(|e| anyhow!("WebSocket handshake: {e}"))?;
    Ok(socket)
}

/// A TCP connection to the first address of `url` that takes one within `CONNECT_WAIT`.
fn connect(url: &url::Url) -> Result<TcpStream> {
    let mut refused = anyhow!("no address for {url}");
    url.socket_addrs(|| None)?
        .iter()
        .find_map(|addr| {
            TcpStream::connect_timeout(addr, CONNECT_WAIT)
                .map_err(|e| refused = e.into())
                .ok()
        })
        .ok_or(refused)
}

#[cfg(test)]
mod tests;
