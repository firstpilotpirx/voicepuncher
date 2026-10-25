//! Outages of the command spotting service and what the stream sends once it is back.

use super::*;
use voicepuncher::core::domain::conversions::saturating_usize;

#[test]
pub(super) fn should_reach_for_the_service_again_after_it_closes_the_connection() {
    // Given
    let (listener, url) = free_listener();
    let connections = stub_service(listener, Duration::ZERO);
    let (stream, audio, inputs) = start_stream(&url);
    // When
    let lost = feed_until(
        &audio,
        &inputs,
        0.0,
        &PipelineInput::SpottingLost,
        BACK_WITHIN,
    );
    let back = feed_until(
        &audio,
        &inputs,
        0.0,
        &PipelineInput::SpottingBack,
        BACK_WITHIN,
    );
    // Then
    assert!(lost && back, "lost: {lost}, back: {back}");
    wait_for_record(&connections, |all| {
        all.get(1)
            .is_some_and(|c| c.selections.iter().any(|s| s.contains("press enter")))
    });
    let selections = &connections.lock().unwrap()[1].selections;
    assert!(
        selections.iter().any(|s| s.contains("press enter")),
        "{selections:?}"
    );
    stream.stop();
}

/// What the stream sends while the service stays away: at least the long outage, and until
/// the client failed its tenth attempt to reach `url`.
fn inputs_while_away(
    url: &str,
    audio: &Sender<Vec<f32>>,
    inputs: &Receiver<PipelineInput>,
) -> Vec<PipelineInput> {
    let tenth_failed = || {
        logged(&format!("to reach {url} failed"))
            .iter()
            .any(|l| l.contains("attempt 10 "))
    };
    let away_since = Instant::now();
    let mut while_away = Vec::new();
    while (away_since.elapsed() < LONG_OUTAGE || !tenth_failed())
        && away_since.elapsed() < TENTH_ATTEMPT_WITHIN
    {
        audio.send(microphone(0.0)).unwrap();
        if let Ok(input) = inputs.recv_timeout(Duration::from_millis(100)) {
            while_away.push(input);
        }
    }
    while_away
}

/// The client logged its failed attempts to reach `url`, the first to the tenth, each with
/// its session.
fn assert_attempted_from_first_to_tenth(url: &str) {
    let attempts = logged(&format!("to reach {url} failed"));
    assert!(
        attempts.iter().any(|l| l.contains("attempt 1 ")),
        "{attempts:?}"
    );
    assert!(
        attempts.iter().any(|l| l.contains("attempt 10 ")),
        "{attempts:?}"
    );
    assert!(
        attempts.iter().all(|l| l.contains("session=")),
        "{attempts:?}"
    );
}

#[test]
pub(super) fn should_be_back_within_seconds_once_the_service_answers_after_a_long_outage() {
    // Given the service goes away and nothing listens on its port for a while
    lines();
    let (listener, url) = free_listener();
    let port_addr = listener.local_addr().unwrap();
    serve_one_connection_then_go(listener);
    let (stream, audio, inputs) = start_fast_stream(&url);
    let lost = feed_until(
        &audio,
        &inputs,
        0.0,
        &PipelineInput::SpottingLost,
        BACK_WITHIN,
    );
    let while_away = inputs_while_away(&url, &audio, &inputs);
    // When the service answers on its port again, once the client failed its tenth attempt
    let _connections = stub_service(TcpListener::bind(port_addr).unwrap(), Duration::ZERO);
    let started = Instant::now();
    let back = feed_until(
        &audio,
        &inputs,
        0.0,
        &PipelineInput::SpottingBack,
        BACK_WITHIN,
    );
    // Then
    let client_log = logged(&url);
    assert!(lost, "the loss was not reported: {client_log:#?}");
    assert_eq!(
        while_away,
        [PipelineInput::SpottingStillAway],
        "only the long outage comes while away: {client_log:#?}"
    );
    assert!(
        back,
        "not back within {BACK_WITHIN:?}, waited {:?}: {client_log:#?}",
        started.elapsed()
    );
    assert_attempted_from_first_to_tenth(&url);
    stream.stop();
}

#[test]
pub(super) fn should_report_once_that_the_service_is_still_away_past_the_replay() {
    // Given the service is away for longer than the replay reaches back
    lines();
    let (listener, url) = free_listener();
    let (_connections, back_at) = stub_service_back_at(listener, OUTAGE_PAST_REPLAY);
    let (stream, audio, inputs) = start_stream(&url);
    // When
    let outage = outage_inputs(&audio, &inputs, &back_at);
    // Then
    assert_eq!(
        outage,
        [
            PipelineInput::SpottingLost,
            PipelineInput::SpottingStillAway,
            PipelineInput::SpottingBack
        ],
        "{:#?}",
        logged(&url)
    );
    stream.stop();
}

#[test]
pub(super) fn should_not_report_a_service_back_within_the_replay_as_still_away() {
    // Given the service is away for less than the replay reaches back
    lines();
    let (listener, url) = free_listener();
    let (_connections, back_at) = stub_service_back_at(listener, SHORT_OUTAGE);
    let (stream, audio, inputs) = start_stream(&url);
    // When
    let outage = outage_inputs(&audio, &inputs, &back_at);
    // Then
    assert_eq!(
        outage,
        [PipelineInput::SpottingLost, PipelineInput::SpottingBack],
        "{:#?}",
        logged(&url)
    );
    stream.stop();
}

/// The inputs other than spotting events, feeding silence until the service is back, at
/// most until `BACK_WITHIN` after the stub answers again (`back_at`).
pub(super) fn outage_inputs(
    audio: &Sender<Vec<f32>>,
    inputs: &Receiver<PipelineInput>,
    back_at: &OnceLock<Instant>,
) -> Vec<PipelineInput> {
    let started = Instant::now();
    let waiting = || match back_at.get() {
        Some(at) => Instant::now() < *at + BACK_WITHIN,
        // The client has not connected yet: its first connection starts the outage.
        None => started.elapsed() < BACK_WITHIN,
    };
    let mut got = Vec::new();
    while waiting() && got.last() != Some(&PipelineInput::SpottingBack) {
        audio.send(microphone(0.0)).unwrap();
        match inputs.recv_timeout(Duration::from_millis(100)) {
            Ok(PipelineInput::Spotting(_)) | Err(_) => {}
            Ok(input) => got.push(input),
        }
    }
    got
}

#[test]
pub(super) fn should_send_the_service_that_is_back_the_speech_heard_while_it_was_away() {
    // Given the service is away for a few seconds after the first connection
    let (listener, url) = free_listener();
    let connections = stub_service(listener, SHORT_OUTAGE);
    let audio_start = Instant::now();
    let (stream, audio, inputs) = start_stream_from(&url, audio_start);
    // When speech is heard between the loss and the return
    let lost = feed_until(
        &audio,
        &inputs,
        0.0,
        &PipelineInput::SpottingLost,
        BACK_WITHIN,
    );
    let back = feed_until(
        &audio,
        &inputs,
        MARK,
        &PipelineInput::SpottingBack,
        SHORT_OUTAGE + BACK_WITHIN,
    );
    let speech_start = next_speech_start(&audio, &inputs);
    // Then the speech reached the new connection, its events placed where it was said
    assert!(lost && back, "lost: {lost}, back: {back}");
    let outage_samples = saturating_usize((SHORT_OUTAGE.as_secs_f32() - 1.0) * SAMPLE_RATE_HZ);
    wait_for_record(&connections, |all| {
        all.last().is_some_and(|c| marked(c) > outage_samples)
    });
    let marked = marked(connections.lock().unwrap().last().unwrap());
    assert!(
        marked > outage_samples,
        "the speech heard while away was not sent: {marked} samples"
    );
    let at = speech_start.expect("the new connection's event reached the app");
    assert!(
        at.saturating_duration_since(audio_start) < EVENT_TIME_TOLERANCE,
        "an event at the first sample sent again is placed {:?} after the capture of that sample",
        at.saturating_duration_since(audio_start)
    );
    stream.stop();
}
