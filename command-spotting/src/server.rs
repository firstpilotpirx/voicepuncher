//! One thread per WebSocket connection; each streams audio in and spotting events out.

use std::net::{TcpListener, TcpStream};
use std::sync::Arc;

use anyhow::Result;
use tungstenite::protocol::frame::coding::CloseCode;
use tungstenite::protocol::CloseFrame;
use tungstenite::{Message, WebSocket};

use crate::engine::Engine;
use crate::protocol::{samples_from_bytes, PhraseSelection};

pub fn serve(listener: TcpListener, engine: Arc<Engine>) -> Result<()> {
    for stream in listener.incoming() {
        let stream = stream?;
        let engine = Arc::clone(&engine);
        std::thread::spawn(move || {
            if let Err(e) = serve_connection(stream, &engine) {
                log::warn!("connection ended: {e:#}");
            }
        });
    }
    Ok(())
}

fn serve_connection(stream: TcpStream, engine: &Engine) -> Result<()> {
    let mut socket = match tungstenite::accept(stream) {
        Ok(socket) => socket,
        Err(e) => {
            log::debug!("not a WebSocket client (a health probe?): {e}");
            return Ok(());
        }
    };
    let mut session = engine.session()?;
    log::info!("app connected");
    loop {
        match socket.read()? {
            Message::Text(text) => {
                let selection: PhraseSelection = serde_json::from_str(text.as_str())?;
                if let Err(unknown) = session.listen_for(&selection.phrases) {
                    return refuse(&mut socket, &unknown.to_string());
                }
                log::info!("listening for {:?}", selection.phrases);
            }
            Message::Binary(bytes) => {
                for event in session.accept(&samples_from_bytes(&bytes)) {
                    socket.send(Message::text(serde_json::to_string(&event)?))?;
                }
            }
            Message::Close(_) => {
                log::info!("app disconnected");
                return Ok(());
            }
            _ => {}
        }
    }
}

fn refuse(socket: &mut WebSocket<TcpStream>, reason: &str) -> Result<()> {
    log::warn!("refusing the connection: {reason}");
    socket.close(Some(CloseFrame {
        code: CloseCode::Policy,
        reason: reason.to_owned().into(),
    }))?;
    while socket.read().is_ok() {}
    Ok(())
}
