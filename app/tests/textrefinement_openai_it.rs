// Integration tests for OpenAiTextRefiner against an in-test HTTP stub.
// One test is gated on VOICEPUNCHER_REFINEMENT_URL and refines a sentence through a
// real text refinement service.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use voicepuncher::adapters::secondary::textrefinement::openai::OpenAiTextRefiner;
use voicepuncher::config::RefinementCfg;
use voicepuncher::core::application::TextRefiner;
use voicepuncher::core::domain::RawTranscript;

const MODELS: &str = r#"{"object":"list","data":[{"id":"qwen"}]}"#;
const TRANSCRIPT: &str = "the docker image builds fine on staging";
const ANSWER: &str = r#"{"choices":[{"message":{"role":"assistant","content":"The Docker image builds fine on staging."},"finish_reason":"stop"}]}"#;
const RECV_TIMEOUT: Duration = Duration::from_secs(5);

struct StubReply {
    status: u16,
    body: &'static str,
    delay: Duration,
}

impl StubReply {
    fn ok(body: &'static str) -> Self {
        Self {
            status: 200,
            body,
            delay: Duration::ZERO,
        }
    }
}

struct CapturedRequest {
    request_line: String,
    authorization: Option<String>,
    body: String,
}

/// Answers the startup model list, then serves exactly one chat request with `reply`.
fn spawn_stub(reply: StubReply) -> (RefinementCfg, mpsc::Receiver<CapturedRequest>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = stream.unwrap();
            let Some(request) = read_request(&mut stream) else {
                continue;
            };
            if request.request_line.starts_with("GET ") {
                respond(&mut stream, 200, MODELS);
                continue;
            }
            thread::sleep(reply.delay);
            respond(&mut stream, reply.status, reply.body);
            let _ = tx.send(request);
            return;
        }
    });
    let cfg = RefinementCfg {
        base_url: format!("http://127.0.0.1:{port}/v1"),
        ..RefinementCfg::default()
    };
    (cfg, rx)
}

fn read_request(stream: &mut TcpStream) -> Option<CapturedRequest> {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut request_line = String::new();
    if reader.read_line(&mut request_line).ok()? == 0 {
        return None;
    }
    let (mut content_length, mut authorization) = (0usize, None);
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).ok()?;
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        let (name, value) = line.split_once(':')?;
        match name.to_ascii_lowercase().as_str() {
            "content-length" => content_length = value.trim().parse().ok()?,
            "authorization" => authorization = Some(value.trim().to_owned()),
            _ => {}
        }
    }
    let mut body = vec![0u8; content_length];
    reader.read_exact(&mut body).ok()?;
    Some(CapturedRequest {
        request_line: request_line.trim_end().to_owned(),
        authorization,
        body: String::from_utf8_lossy(&body).into_owned(),
    })
}

fn respond(stream: &mut TcpStream, status: u16, body: &str) {
    let response = format!(
        "HTTP/1.1 {status} STUB\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes());
}

fn refine(cfg: RefinementCfg, transcript: &str) -> anyhow::Result<String> {
    let mut refiner = OpenAiTextRefiner::connect(cfg)?;
    let cleaned = refiner.refine(&RawTranscript::new(transcript.to_owned())?)?;
    Ok(cleaned.as_str().to_owned())
}

#[test]
fn should_post_chat_request_and_return_cleaned_text() {
    // Given
    let (cfg, requests) = spawn_stub(StubReply::ok(ANSWER));
    let cfg = RefinementCfg {
        model: "qwen".into(),
        ..cfg
    };
    // When
    let cleaned = refine(cfg, TRANSCRIPT).unwrap();
    // Then
    assert_eq!(cleaned, "The Docker image builds fine on staging.");
    let request = requests.recv_timeout(RECV_TIMEOUT).unwrap();
    assert_eq!(request.request_line, "POST /v1/chat/completions HTTP/1.1");
    let body: serde_json::Value = serde_json::from_str(&request.body).unwrap();
    assert_eq!(body["model"], "qwen");
    assert_eq!(
        body["messages"][1]["content"],
        format!("<transcript>\n{TRANSCRIPT}\n</transcript>")
    );
    assert_eq!(request.authorization, None);
}

#[test]
fn should_send_api_key_as_bearer_token() {
    // Given
    let (cfg, requests) = spawn_stub(StubReply::ok(ANSWER));
    let cfg = RefinementCfg {
        api_key: "secret".into(),
        ..cfg
    };
    // When
    refine(cfg, TRANSCRIPT).unwrap();
    // Then
    let request = requests.recv_timeout(RECV_TIMEOUT).unwrap();
    assert_eq!(request.authorization.as_deref(), Some("Bearer secret"));
}

#[test]
fn should_fail_on_server_error() {
    // Given
    let (cfg, _requests) = spawn_stub(StubReply {
        status: 500,
        ..StubReply::ok(r#"{"error": "boom"}"#)
    });
    // When / Then
    assert!(refine(cfg, TRANSCRIPT).is_err());
}

#[test]
fn should_fail_on_malformed_json() {
    // Given
    let (cfg, _requests) = spawn_stub(StubReply::ok("not json"));
    // When / Then
    let message = format!("{:#}", refine(cfg, TRANSCRIPT).unwrap_err());
    assert!(
        message.contains("unexpected text refinement response"),
        "{message}"
    );
}

#[test]
fn should_fail_when_service_answers_after_timeout() {
    // Given
    let (cfg, _requests) = spawn_stub(StubReply {
        delay: Duration::from_secs(3),
        ..StubReply::ok(ANSWER)
    });
    let cfg = RefinementCfg {
        timeout_secs: 1,
        ..cfg
    };
    // When / Then
    assert!(refine(cfg, TRANSCRIPT).is_err());
}

#[test]
fn should_refine_sentence_through_real_service() {
    // Given — gated on a running text refinement service
    let Ok(base_url) = std::env::var("VOICEPUNCHER_REFINEMENT_URL") else {
        eprintln!("GATED: VOICEPUNCHER_REFINEMENT_URL not set, skipping");
        return;
    };
    let cfg = RefinementCfg {
        base_url,
        ..RefinementCfg::default()
    };
    // When
    let cleaned = refine(cfg, TRANSCRIPT).unwrap();
    // Then
    let cleaned = cleaned.to_lowercase();
    for word in ["docker", "staging"] {
        assert!(cleaned.contains(word), "expected '{word}' in '{cleaned}'");
    }
}
