//! How the transcriber reads the service's answers, errors and dropped connections.

use super::*;

#[test]
pub(super) fn should_post_multipart_request_and_return_transcript() {
    // Given
    let (cfg, requests) = spawn_stub(StubReply::ok(r#"{"text": " hello world\n"}"#));
    let cfg = TranscriptionCfg { language: "en".into(), ..cfg };
    // When
    let transcript = transcribe(cfg).unwrap();
    // Then
    assert_eq!(transcript.as_deref(), Some("hello world"));
    let request = requests.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(
        request.request_line,
        "POST /v1/audio/transcriptions HTTP/1.1"
    );
    assert!(request
        .content_type
        .starts_with("multipart/form-data; boundary="));
    for field in ["model", "language", "temperature", "response_format"] {
        assert!(
            request.body.contains(&format!("name=\"{field}\"")),
            "missing {field}"
        );
    }
    assert!(request.body.contains("name=\"language\"\r\n\r\nen\r\n"));
    assert!(request.body.contains("filename=\"audio.wav\""));
    assert!(request.body.contains("RIFF"));
}

#[test]
pub(super) fn should_omit_language_when_auto() {
    // Given
    let (cfg, requests) = spawn_stub(StubReply::ok(r#"{"text": "привет"}"#));
    // When
    transcribe(cfg).unwrap();
    // Then
    let request = requests.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(!request.body.contains("name=\"language\""));
}

#[test]
pub(super) fn should_join_word_split_across_segments() {
    // Given
    let (cfg, _requests) = spawn_stub(StubReply::ok(r#"{"text": " наход\nившаяся у власти"}"#));
    // When
    let transcript = transcribe(cfg).unwrap();
    // Then
    assert_eq!(transcript.as_deref(), Some("находившаяся у власти"));
}

#[test]
pub(super) fn should_discard_known_hallucination() {
    // Given
    let (cfg, _requests) = spawn_stub(StubReply::ok(
        r#"{"text": " Субтитры создавал DimaTorzok"}"#,
    ));
    // When
    let transcript = transcribe(cfg).unwrap();
    // Then
    assert_eq!(transcript, None);
}

#[test]
pub(super) fn should_keep_speech_before_a_trailing_hallucination() {
    // Given
    let (cfg, _requests) = spawn_stub(StubReply::ok(
        r#"{"text": " I restarted the service.\n We split the module into several small parts.\n Thanks for watching.\n"}"#,
    ));
    // When
    let transcript = transcribe(cfg).unwrap();
    // Then
    assert_eq!(
        transcript.as_deref(),
        Some("I restarted the service. We split the module into several small parts.")
    );
}

#[test]
pub(super) fn should_remove_a_marker_inside_speech() {
    // Given
    let (cfg, _requests) = spawn_stub(StubReply::ok(
        r#"{"text": " the index\n [BLANK_AUDIO]\n was never built.\n"}"#,
    ));
    // When
    let transcript = transcribe(cfg).unwrap();
    // Then
    assert_eq!(transcript.as_deref(), Some("the index was never built."));
}

#[test]
pub(super) fn should_return_no_transcript_when_service_hears_no_speech() {
    // Given
    let (cfg, _requests) = spawn_stub(StubReply::ok(r#"{"text": ""}"#));
    // When
    let transcript = transcribe(cfg).unwrap();
    // Then
    assert_eq!(transcript, None);
}

#[test]
pub(super) fn should_return_no_transcript_when_service_answers_only_whitespace() {
    // Given
    let (cfg, _requests) = spawn_stub(StubReply::ok(r#"{"text": " \n"}"#));
    // When
    let transcript = transcribe(cfg).unwrap();
    // Then
    assert_eq!(transcript, None);
}

#[test]
pub(super) fn should_fail_on_server_error() {
    // Given
    let (cfg, _requests) =
        spawn_stub(StubReply { status: 500, ..StubReply::ok(r#"{"error": "boom"}"#) });
    // When / Then
    assert!(transcribe(cfg).is_err());
}

#[test]
pub(super) fn should_fail_on_malformed_json() {
    // Given
    let (cfg, _requests) = spawn_stub(StubReply::ok("not json"));
    // When / Then
    let message = format!("{:#}", transcribe(cfg).unwrap_err());
    assert!(
        message.contains("unexpected speech recognition response"),
        "{message}"
    );
}

#[test]
pub(super) fn should_fail_when_service_answers_after_timeout() {
    // Given
    let (cfg, _requests) = spawn_stub(StubReply {
        delay: Duration::from_secs(3),
        ..StubReply::ok(r#"{"text": "late"}"#)
    });
    let cfg = TranscriptionCfg { timeout_secs: 1, ..cfg };
    // When / Then
    assert!(transcribe(cfg).is_err());
}

#[test]
pub(super) fn should_retry_when_service_drops_connection() {
    // Given
    capturing_logger();
    let (cfg, attempts) = spawn_dropping_stub(2, StubReply::ok(r#"{"text": "hello"}"#));
    // When
    let transcript = transcribe(cfg).unwrap();
    // Then
    assert_eq!(transcript.as_deref(), Some("hello"));
    assert_eq!(attempts.load(Ordering::SeqCst), 3);
    assert_eq!(
        logged_by_this_thread("WARN speech recognition service dropped the connection"),
        2
    );
    assert_eq!(logged_by_this_thread(WAITING_FOR_A_COPY), 0);
}

#[test]
pub(super) fn should_wait_30_seconds_for_a_copy_to_come_back_by_default() {
    assert_eq!(
        voicepuncher::adapters::secondary::transcription::openai::RECOVERY_WAIT,
        Duration::from_secs(30)
    );
}

#[test]
pub(super) fn should_give_up_after_four_attempts_when_service_keeps_dropping() {
    // Given
    let (cfg, attempts) = spawn_dropping_stub(usize::MAX, StubReply::ok(r#"{"text": "x"}"#));
    // When
    let result = transcribe(cfg);
    // Then
    assert!(result.is_err());
    assert_eq!(attempts.load(Ordering::SeqCst), 4);
}

#[test]
pub(super) fn should_send_a_recording_answered_with_errors_three_times() {
    // Given
    let (cfg, attempts) = spawn_dropping_stub(
        0,
        StubReply { status: 500, ..StubReply::ok(r#"{"error": "boom"}"#) },
    );
    // When
    let result = transcribe(cfg);
    // Then
    assert!(result.is_err());
    assert_eq!(attempts.load(Ordering::SeqCst), 3);
}

#[test]
pub(super) fn should_not_retry_a_request_the_service_cannot_take() {
    // Given
    let (cfg, attempts) = spawn_dropping_stub(
        0,
        StubReply { status: 400, ..StubReply::ok(r#"{"error": "bad request"}"#) },
    );
    // When
    let result = transcribe(cfg);
    // Then
    assert!(result.is_err());
    assert_eq!(attempts.load(Ordering::SeqCst), 1);
}
