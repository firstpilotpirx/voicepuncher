//! A copy of the speech recognition service for commands over HTTP, as a copy on a loaded
//! machine: it hears each request's audio with a `ScriptedHearing` and answers after a
//! decoding of wall time, while it answers its health check at once.

use std::io::{BufRead, BufReader, Cursor, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use voicepuncher_command_spotting::confirm::Hear;

use crate::scripted_hearing::ScriptedHearing;

const HEALTH_PREFIX: &str = "GET /health";
const WAV_PART_START: &[u8] = b"Content-Type: audio/wav\r\n\r\n";
const PART_END: &[u8] = b"\r\n--";
const HEALTH_ANSWER: &str = "ok";

/// Serves `hearing` on a port of its own, each answer `decoding` after its request came in;
/// the service address to confirm with.
pub fn serve(hearing: ScriptedHearing, decoding: Duration) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/v1", listener.local_addr().unwrap());
    let hearing =
        Arc::new(ScriptedHearing { stream: as_heard_over_wav(&hearing.stream), ..hearing });
    thread::spawn(move || {
        for stream in listener.incoming() {
            let hearing = Arc::clone(&hearing);
            thread::spawn(move || answer(stream.unwrap(), &hearing, decoding));
        }
    });
    url
}

/// `samples` as the copy reads them back from the 16-bit WAV the client sends, so the
/// scripted hearing finds the clips in its stream.
fn as_heard_over_wav(samples: &[f32]) -> Vec<f32> {
    samples
        .iter()
        .map(|s| f32::from((s.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16) / f32::from(i16::MAX))
        .collect()
}

fn answer(mut stream: TcpStream, hearing: &ScriptedHearing, decoding: Duration) {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let Some((request, length)) = head(&mut reader) else {
        return;
    };
    let reply = if request.starts_with(HEALTH_PREFIX) {
        HEALTH_ANSWER.to_owned()
    } else {
        let mut body = vec![0u8; length];
        reader.read_exact(&mut body).unwrap();
        thread::sleep(decoding);
        verbose_json(hearing, &wav_samples(&body))
    };
    let reply = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{reply}",
        reply.len()
    );
    // The client may have closed the connection while the copy decoded.
    let _ = stream.write_all(reply.as_bytes());
}

/// The request line and the body's length from the request's head, none when the
/// connection was closed before a request came.
fn head(reader: &mut BufReader<TcpStream>) -> Option<(String, usize)> {
    let mut lines = Vec::new();
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).unwrap_or(0) == 0 {
            return None;
        }
        if line.trim_end().is_empty() {
            break;
        }
        lines.push(line);
    }
    let length = lines
        .iter()
        .filter_map(|line| line.split_once(':'))
        .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
        .map_or(0, |(_, value)| value.trim().parse().unwrap());
    Some((lines.first().cloned().unwrap_or_default(), length))
}

/// The samples of the WAV part of a multipart `body`.
fn wav_samples(body: &[u8]) -> Vec<f32> {
    let start = find(body, WAV_PART_START).expect("a WAV part") + WAV_PART_START.len();
    let end = start + find(&body[start..], PART_END).expect("the part's end");
    hound::WavReader::new(Cursor::new(&body[start..end]))
        .unwrap()
        .samples::<i16>()
        .map(|s| f32::from(s.unwrap()) / f32::from(i16::MAX))
        .collect()
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

/// What `hearing` hears in `clip`, as the service's verbose JSON with word times.
fn verbose_json(hearing: &ScriptedHearing, clip: &[f32]) -> String {
    let heard = hearing.hear(clip, Duration::ZERO).unwrap();
    let words: Vec<serde_json::Value> = heard
        .words
        .iter()
        .map(|w| serde_json::json!({ "word": w.text, "start": w.start, "end": w.end }))
        .collect();
    serde_json::json!({ "text": heard.text, "segments": [{ "words": words }] }).to_string()
}
