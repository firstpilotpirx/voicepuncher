//! A text refinement service stand-in: answers the startup model list and chat requests
//! with fixed content and records every chat request body.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

use anyhow::Result;

const SINGLE_MODEL: &str = "stub-model";
const STOP_FINISH_REASON: &str = "stop";
const TRANSCRIPT_OPEN: &str = "<transcript>\n";
const TRANSCRIPT_CLOSE: &str = "\n</transcript>";

#[derive(Debug, Clone)]
pub enum StubBehavior {
    /// Answers every chat request with `content` and `finish_reason`.
    Answer {
        content: String,
        finish_reason: &'static str,
    },
    /// Answers every chat request with its transcript followed by `suffix`.
    AppendToTranscript { suffix: &'static str },
    /// Offers these models; never receives a chat request.
    OfferModels(Vec<&'static str>),
    /// Answers the startup model list, then stops listening.
    StopAfterStartupCheck,
}

#[derive(Debug)]
pub struct RefinementServiceStub {
    pub url: String,
    chat_requests: Arc<Mutex<Vec<String>>>,
}

impl RefinementServiceStub {
    pub fn spawn(behavior: StubBehavior) -> Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let url = format!("http://127.0.0.1:{}/v1", listener.local_addr()?.port());
        let chat_requests = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&chat_requests);
        thread::spawn(move || serve(listener, &behavior, &recorded));
        Ok(Self { url, chat_requests })
    }

    pub fn chat_requests(&self) -> Vec<String> {
        self.chat_requests
            .lock()
            .expect("stub request log is not poisoned")
            .clone()
    }
}

fn serve(listener: TcpListener, behavior: &StubBehavior, recorded: &Mutex<Vec<String>>) {
    for stream in listener.incoming() {
        let Ok(mut stream) = stream else { continue };
        let Some((request_line, body)) = read_request(&mut stream) else {
            continue;
        };
        if request_line.starts_with("GET ") {
            respond(&mut stream, &models_answer(behavior));
            if matches!(behavior, StubBehavior::StopAfterStartupCheck) {
                return;
            }
            continue;
        }
        if let Ok(mut log) = recorded.lock() {
            log.push(body.clone());
        }
        match behavior {
            StubBehavior::Answer {
                content,
                finish_reason,
            } => respond(&mut stream, &chat_answer(content, finish_reason)),
            StubBehavior::AppendToTranscript { suffix } => {
                let content = format!("{}{suffix}", last_transcript(&body));
                respond(&mut stream, &chat_answer(&content, STOP_FINISH_REASON));
            }
            _ => {}
        }
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

fn read_request(stream: &mut TcpStream) -> Option<(String, String)> {
    let mut reader = BufReader::new(stream.try_clone().ok()?);
    let mut request_line = String::new();
    if reader.read_line(&mut request_line).ok()? == 0 {
        return None;
    }
    let mut content_length = 0usize;
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).ok()?;
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            if name.eq_ignore_ascii_case("content-length") {
                content_length = value.trim().parse().ok()?;
            }
        }
    }
    let mut body = vec![0u8; content_length];
    reader.read_exact(&mut body).ok()?;
    Some((request_line, String::from_utf8_lossy(&body).into_owned()))
}

fn respond(stream: &mut TcpStream, body: &str) {
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes());
}
