//! A handshake that never completes and a copy that goes.

use super::*;

#[test]
pub(super) fn should_give_up_connecting_to_a_service_that_never_completes_the_handshake() {
    // Given a port that accepts connections but whose service never answers
    let (listener, url) = free_listener();
    // When
    let started = Instant::now();
    let error = voicepuncher::adapters::secondary::commandspotting::websocket::client::probe(&url)
        .unwrap_err();
    // Then
    assert!(
        started.elapsed() < BACK_WITHIN,
        "held {:?}: {error:#}",
        started.elapsed()
    );
    drop(listener);
}

#[test]
pub(super) fn should_move_to_another_copy_at_the_first_attempt_when_the_connected_copy_goes() {
    // Given two copies: the first serves one connection and goes, the second stays
    lines();
    let (first, second, url) = two_free_copies();
    serve_one_connection_then_go(first);
    let connections = stub_service_that_stays(second);
    let (stream, audio, inputs) = start_stream(&url);
    // When
    let outage = outage_inputs(&audio, &inputs, &OnceLock::new());
    // Then
    assert_eq!(
        outage,
        [PipelineInput::SpottingLost, PipelineInput::SpottingBack],
        "{:#?}",
        logged(&url)
    );
    let failed: Vec<String> = logged(&url)
        .into_iter()
        .filter(|line| line.contains("failed"))
        .collect();
    assert!(failed.is_empty(), "{failed:#?}");
    wait_for_record(&connections, |all| {
        all.first()
            .is_some_and(|c| c.selections.iter().any(|s| s.contains("press enter")))
    });
    stream.stop();
}
