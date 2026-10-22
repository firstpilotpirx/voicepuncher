//! The stub's connection loop: one request per connection, answered by its behavior.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Mutex;

use super::{StubBehavior, StubEvent, STOP_FINISH_REASON};

const SINGLE_MODEL: &str = "stub-model";
const TRANSCRIPT_OPEN: &str = "<transcript>\n";
const TRANSCRIPT_CLOSE: &str = "\n</transcript>";

fn record(log: &Mutex<Vec<StubEvent>>, event: StubEvent) {
    if let Ok(mut events) = log.lock() {
        events.push(event);
    }
}

pub(super) fn serve(
    listener: &TcpListener,
    behavior: &StubBehavior,
    recorded: &Mutex<Vec<StubEvent>>,
) {
    for stream in listener.incoming() {
        let Ok(mut stream) = stream else { continue };
        let (request_line, body) = match read_request(&stream) {
            Ok(request) => request,
            Err(Unanswered::Probe) => {
                record(recorded, StubEvent::Probe);
                continue;
            }
            Err(Unanswered::Dropped(reason)) => {
                record(recorded, StubEvent::Dropped { reason });
                continue;
            }
        };
        if request_line.starts_with("GET ") {
            record(recorded, StubEvent::Models);
            respond(&mut stream, &models_answer(behavior));
            if matches!(behavior, StubBehavior::StopAfterStartupCheck) {
                return;
            }
            continue;
        }
        record(recorded, StubEvent::Chat { body: body.clone() });
        answer_chat(&mut stream, behavior, &body);
    }
}

/// Answers a chat request as the behavior says; the behaviors that never expect one leave
/// the connection unanswered.
fn answer_chat(stream: &mut TcpStream, behavior: &StubBehavior, body: &str) {
    match behavior {
        StubBehavior::Answer { content, finish_reason } => {
            respond(stream, &chat_answer(content, finish_reason));
        }
        StubBehavior::AppendToTranscript { suffix } => {
            let content = format!("{}{suffix}", last_transcript(body));
            respond(stream, &chat_answer(&content, STOP_FINISH_REASON));
        }
        StubBehavior::CorrectEndings => {
            let content = with_endings_changed(&last_transcript(body));
            respond(stream, &chat_answer(&content, STOP_FINISH_REASON));
        }
        _ => {}
    }
}

/// The delimited transcript of the request's last message.
fn last_transcript(body: &str) -> String {
    let request: serde_json::Value = serde_json::from_str(body).unwrap_or_default();
    let last = request["messages"]
        .as_array()
        .and_then(|messages| messages.last())
        .and_then(|message| message["content"].as_str())
        .unwrap_or_default();
    last.trim_start_matches(TRANSCRIPT_OPEN)
        .trim_end_matches(TRANSCRIPT_CLOSE)
        .to_owned()
}

/// `transcript` with every Cyrillic word of three or more letters ending in another letter,
/// a capital first letter and a final period.
fn with_endings_changed(transcript: &str) -> String {
    let mut text = String::new();
    let mut word = String::new();
    for c in transcript.chars().chain(std::iter::once(' ')) {
        if c.is_alphabetic() {
            word.push(c);
            continue;
        }
        text.push_str(&changed_ending(&word));
        word.clear();
        text.push(c);
    }
    let text = text.trim().trim_end_matches(['.', '!', '?']);
    let mut chars = text.chars();
    let capitalized: String = chars
        .next()
        .map(|first| first.to_uppercase().chain(chars).collect())
        .unwrap_or_default();
    format!("{capitalized}.")
}

fn changed_ending(word: &str) -> String {
    let letters: Vec<char> = word.chars().collect();
    let cyrillic = letters.iter().all(|c| matches!(c, '\u{0400}'..='\u{04FF}'));
    match letters.split_last() {
        Some((last, stem)) if cyrillic && letters.len() >= 3 => {
            let ending = if *last == 'а' { 'у' } else { 'а' };
            stem.iter().chain(std::iter::once(&ending)).collect()
        }
        _ => word.to_owned(),
    }
}

fn models_answer(behavior: &StubBehavior) -> String {
    let ids = match behavior {
        StubBehavior::OfferModels(ids) => ids.clone(),
        _ => vec![SINGLE_MODEL],
    };
    let data: Vec<serde_json::Value> = ids
        .iter()
        .map(|id| serde_json::json!({ "id": id }))
        .collect();
    serde_json::json!({ "object": "list", "data": data }).to_string()
}

fn chat_answer(content: &str, finish_reason: &str) -> String {
    serde_json::json!({
        "choices": [{
            "message": { "role": "assistant", "content": content },
            "finish_reason": finish_reason
        }]
    })
    .to_string()
}

/// A connection that carried no request to answer.
enum Unanswered {
    /// Closed without a byte: a liveness probe.
    Probe,
    /// Dropped mid-request, and why.
    Dropped(String),
}

impl From<String> for Unanswered {
    fn from(reason: String) -> Self {
        Self::Dropped(reason)
    }
}

/// The request line and body of one request, or why there is none.
fn read_request(stream: &TcpStream) -> Result<(String, String), Unanswered> {
    let cloned = stream
        .try_clone()
        .map_err(|e| format!("the connection cannot be read: {e}"))?;
    let mut reader = BufReader::new(cloned);
    let mut request_line = String::new();
    let read = reader
        .read_line(&mut request_line)
        .map_err(|e| format!("the request line failed to read: {e}"))?;
    if read == 0 {
        return Err(Unanswered::Probe);
    }
    let content_length = read_content_length(&mut reader)?;
    let mut body = vec![0u8; content_length];
    reader
        .read_exact(&mut body)
        .map_err(|e| format!("the body of {content_length} bytes failed to read: {e}"))?;
    Ok((request_line, String::from_utf8_lossy(&body).into_owned()))
}

/// The content length of the headers `reader` holds, 0 without one.
fn read_content_length(reader: &mut BufReader<TcpStream>) -> Result<usize, String> {
    let mut content_length = 0usize;
    loop {
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .map_err(|e| format!("a header failed to read: {e}"))?;
        let line = line.trim_end();
        if line.is_empty() {
            return Ok(content_length);
        }
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        if name.eq_ignore_ascii_case("content-length") {
            content_length = value
                .trim()
                .parse()
                .map_err(|_| format!("bad content length {:?}", value.trim()))?;
        }
    }
}

fn respond(stream: &mut TcpStream, body: &str) {
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes());
}
