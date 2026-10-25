//! The user's languages and the language the service detects.

use super::*;

/// Answers the requests in turn with `answers` (the last one repeats) and passes each
/// request on, so a test sees what every attempt asked for.
pub(super) fn spawn_answering_stub(
    answers: Vec<&'static str>,
) -> (TranscriptionCfg, mpsc::Receiver<CapturedRequest>) {
    let (tx, rx) = mpsc::channel();
    let mut answered = 0usize;
    let port = serve_each(move |mut stream| {
        let answer = answers[answered.min(answers.len() - 1)];
        if let Some(request) = read_request(&mut stream, &StubReply::ok(answer)) {
            answered += 1;
            let _ = tx.send(request);
        }
    });
    (cfg_for(port), rx)
}

pub(super) const KOREAN_ANSWER: &str = r#"{"text": " 네, 좋아요.", "language": "korean"}"#;

pub(super) const RUSSIAN_ANSWER: &str = r#"{"text": " Да, хорошо.", "language": "russian"}"#;

pub(super) const ENGLISH_ANSWER: &str =
    r#"{"text": " Restart the server.", "language": "english"}"#;

/// Long enough for a second request the transcriber would have sent.
pub(super) const NO_MORE_REQUESTS_WITHIN: Duration = Duration::from_millis(300);

pub(super) fn next_request(requests: &mpsc::Receiver<CapturedRequest>) -> CapturedRequest {
    requests.recv_timeout(Duration::from_secs(5)).unwrap()
}

#[test]
pub(super) fn should_request_a_transcript_in_another_language_again_with_the_first_user_language() {
    // Given
    capturing_logger();
    let (cfg, requests) = spawn_answering_stub(vec![KOREAN_ANSWER, RUSSIAN_ANSWER]);
    // When
    let transcript = transcribe(cfg).unwrap();
    // Then
    assert_eq!(transcript.as_deref(), Some("Да, хорошо."));
    let first = next_request(&requests);
    assert!(!first.body.contains("name=\"language\""));
    assert!(first
        .body
        .contains("name=\"response_format\"\r\n\r\nverbose_json\r\n"));
    let retry = next_request(&requests);
    assert!(retry.body.contains("name=\"language\"\r\n\r\nru\r\n"));
    assert_eq!(logged_by_this_thread("korean"), 1);
}

#[test]
pub(super) fn should_keep_a_transcript_detected_as_one_of_the_user_languages() {
    // Given
    let (cfg, requests) = spawn_answering_stub(vec![ENGLISH_ANSWER, RUSSIAN_ANSWER]);
    // When
    let transcript = transcribe(cfg).unwrap();
    // Then
    assert_eq!(transcript.as_deref(), Some("Restart the server."));
    let first = next_request(&requests);
    assert!(first
        .body
        .contains("name=\"response_format\"\r\n\r\nverbose_json\r\n"));
    assert!(requests.recv_timeout(NO_MORE_REQUESTS_WITHIN).is_err());
}

#[test]
pub(super) fn should_keep_a_transcript_of_a_fixed_language_whatever_the_service_detects() {
    // Given
    let (cfg, requests) = spawn_answering_stub(vec![KOREAN_ANSWER, RUSSIAN_ANSWER]);
    let cfg = TranscriptionCfg { language: "en".into(), ..cfg };
    // When
    let transcript = transcribe(cfg).unwrap();
    // Then
    assert_eq!(transcript.as_deref(), Some("네, 좋아요."));
    next_request(&requests);
    assert!(requests.recv_timeout(NO_MORE_REQUESTS_WITHIN).is_err());
}
