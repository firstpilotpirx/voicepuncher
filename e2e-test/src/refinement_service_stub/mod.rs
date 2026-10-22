//! A text refinement service stand-in: answers the startup model list and chat requests
//! with fixed content and records every chat request body.

use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread;

use anyhow::Result;

mod serve;
#[cfg(test)]
mod tests;

use serve::serve;

const STOP_FINISH_REASON: &str = "stop";

#[derive(Debug, Clone)]
pub enum StubBehavior {
    /// Answers every chat request with `content` and `finish_reason`.
    Answer { content: String, finish_reason: &'static str },
    /// Answers every chat request with its transcript followed by `suffix`.
    AppendToTranscript { suffix: &'static str },
    /// Answers every chat request with its transcript, the last letter of every Cyrillic
    /// word of three or more letters replaced, starting with a capital and ending with a period.
    CorrectEndings,
    /// Offers these models; never receives a chat request.
    OfferModels(Vec<&'static str>),
    /// Answers the startup model list, then stops listening.
    StopAfterStartupCheck,
}

/// What one connection to the stub became.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StubEvent {
    /// The startup model-list request.
    Models,
    /// A chat request with its body.
    Chat { body: String },
    /// A connection opened and closed without a byte: the app probing whether the service
    /// listens (the copy pool does so before every request).
    Probe,
    /// A connection dropped before it was answered, and why.
    Dropped { reason: String },
}

type EventLog = Arc<Mutex<Vec<StubEvent>>>;

#[derive(Debug)]
pub struct RefinementServiceStub {
    pub url: String,
    events: EventLog,
}

impl RefinementServiceStub {
    pub fn spawn(behavior: StubBehavior) -> Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let url = format!("http://127.0.0.1:{}/v1", listener.local_addr()?.port());
        let events: EventLog = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&events);
        thread::spawn(move || serve(&listener, &behavior, &recorded));
        Ok(Self { url, events })
    }

    /// Every connection the stub saw, in order.
    pub fn events(&self) -> Vec<StubEvent> {
        self.events
            .lock()
            .map(|events| events.clone())
            .unwrap_or_default()
    }

    /// The bodies of the chat requests, in order.
    pub fn chat_requests(&self) -> Vec<String> {
        self.events()
            .into_iter()
            .filter_map(|event| match event {
                StubEvent::Chat { body } => Some(body),
                _ => None,
            })
            .collect()
    }
}

/// Why a transcription with refinement ended with other than one chat request: the
/// request count, every connection the stub saw in order, then the app's own log.
pub fn missing_request_report(events: &[StubEvent], stderr: &str) -> String {
    let chat_requests = events
        .iter()
        .filter(|event| matches!(event, StubEvent::Chat { .. }))
        .count();
    let connections = if events.is_empty() {
        "the service saw no connection".to_owned()
    } else {
        let listed: Vec<String> = events
            .iter()
            .enumerate()
            .map(|(index, event)| format!("{}. {}", index + 1, event.describe()))
            .collect();
        format!(
            "the service saw {} connections: {}",
            events.len(),
            listed.join("; ")
        )
    };
    format!(
        "expected one request to the text refinement service, got {chat_requests}; \
         {connections}; stderr: {stderr}"
    )
}

impl StubEvent {
    fn describe(&self) -> String {
        match self {
            Self::Models => "the model list was requested".to_owned(),
            Self::Chat { body } => format!("a chat request arrived: {body}"),
            Self::Probe => "a connection was opened and closed without a request".to_owned(),
            Self::Dropped { reason } => format!("a connection was dropped: {reason}"),
        }
    }
}
