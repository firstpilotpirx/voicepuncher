//! A program on a service port that is no speech recognition service.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// An HTTP/2 SETTINGS frame: all such a program answers.
const SETTINGS_FRAME: [u8; 15] = [0, 0, 6, 4, 0, 0, 0, 0, 0, 0, 5, 0, 0, 64, 0];

/// A program that speaks HTTP/2 only, as the `go run` server that held 8081 did: it answers
/// every connection with an HTTP/2 SETTINGS frame and counts the transcription requests it got.
#[derive(Debug)]
pub struct Http2Only {
    transcriptions: Arc<AtomicUsize>,
}

impl Http2Only {
    pub fn on(listener: TcpListener) -> Self {
        let transcriptions = Arc::new(AtomicUsize::new(0));
        let counted = Arc::clone(&transcriptions);
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                answer_with_settings(stream, &counted);
            }
        });
        Self { transcriptions }
    }

    /// The transcription requests (`POST`) it got.
    pub fn transcriptions(&self) -> usize {
        self.transcriptions.load(Ordering::SeqCst)
    }
}

/// Answers one connection with the SETTINGS frame, counting a transcription request.
fn answer_with_settings(mut stream: TcpStream, transcriptions: &AtomicUsize) {
    let mut request = [0u8; 4096];
    let read = stream.read(&mut request).unwrap_or(0);
    if request[..read].starts_with(b"POST") {
        transcriptions.fetch_add(1, Ordering::SeqCst);
    }
    let _ = stream.write_all(&SETTINGS_FRAME);
}
