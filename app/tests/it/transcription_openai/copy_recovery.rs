//! Copies that go down, hang or come back.

use super::*;

/// What the client logs when it waits for a copy of a service that accepts no connections.
pub(super) const WAITING_FOR_A_COPY: &str = "no copy accepts connections — waiting";

pub(super) const SHORT_RECOVERY_WAIT: Duration = Duration::from_millis(300);

/// Long enough for the pool to probe a copy that came back: it probes every second.
pub(super) const RECOVERY_WAIT: Duration = Duration::from_secs(5);

/// After the attempts for dropped connections (1.4 s of pauses) are used up.
pub(super) const COMES_BACK_AFTER: Duration = Duration::from_millis(2_500);

#[test]
pub(super) fn should_transcribe_once_a_copy_comes_back_after_every_copy_went_down() {
    // Given
    let (cfg, mut listeners) = free_copies(1);
    let port = listeners[0].local_addr().unwrap().port();
    let copy = voicepuncher_test_steps::loopback::WhisperUntilClosed::on(listeners.remove(0));
    capturing_logger();
    let mut transcriber = OpenAiTranscriber::connect(cfg)
        .unwrap()
        .with_recovery_wait(RECOVERY_WAIT);
    copy.close();
    let comes_back = thread::spawn(move || {
        thread::sleep(COMES_BACK_AFTER);
        spawn_copy(
            TcpListener::bind(("127.0.0.1", port)).unwrap(),
            vec![CopyTurn::Answer],
        )
    });
    // When
    let transcript = transcriber.transcribe(&half_second()).unwrap();
    // Then
    let copy = comes_back.join().unwrap();
    assert_eq!(transcript.unwrap().as_str(), "hello");
    assert_eq!(copy.requests(), 1);
    assert_eq!(logged_by_this_thread(WAITING_FOR_A_COPY), 1);
    assert_eq!(logged_by_this_thread("— giving up"), 0);
}

#[test]
pub(super) fn should_fail_naming_the_copies_when_every_copy_refuses() {
    // Given
    let (cfg, listeners) = free_copies(2);
    let ports: Vec<u16> = listeners
        .iter()
        .map(|l| l.local_addr().unwrap().port())
        .collect();
    let copies: Vec<voicepuncher_test_steps::loopback::WhisperUntilClosed> = listeners
        .into_iter()
        .map(voicepuncher_test_steps::loopback::WhisperUntilClosed::on)
        .collect();
    capturing_logger();
    let mut transcriber = OpenAiTranscriber::connect(cfg)
        .unwrap()
        .with_recovery_wait(SHORT_RECOVERY_WAIT);
    for copy in copies {
        copy.close();
    }
    // When
    let message = format!("{:#}", transcriber.transcribe(&half_second()).unwrap_err());
    // Then
    assert_eq!(
        logged_by_this_thread("WARN speech recognition service"),
        4,
        "not every attempt was used"
    );
    assert_eq!(logged_by_this_thread(WAITING_FOR_A_COPY), 1);
    assert_eq!(logged_by_this_thread("— giving up"), 1);
    for port in ports {
        assert!(message.contains(&port.to_string()), "{message}");
    }
}

#[test]
pub(super) fn should_send_the_attempt_after_a_hung_one_to_another_copy() {
    // Given
    let (cfg, hung, other) = one_hung_copy_of_two();
    // When
    let transcript = transcribe(cfg).unwrap();
    // Then
    assert_eq!(transcript.as_deref(), Some("hello"));
    assert_eq!(hung.requests(), 1, "the hung copy got a second attempt");
    assert_eq!(other.requests(), 1);
}

#[test]
pub(super) fn should_log_a_copy_hung_with_the_time_waited_only_past_twice_its_time_limit() {
    // Given
    let (cfg, mut listeners) = free_copies(2);
    let _other = spawn_copy(listeners.remove(1 - first_tried(2)), vec![CopyTurn::Answer]);
    // The default timeout: the request's own limit (a few seconds for half a second of audio)
    // is far below the timeout the copy's thread reads under, so the copy stays held
    let hung = spawn_copy(listeners.remove(0), vec![CopyTurn::Hang]);
    capturing_logger();
    let hung_line = format!("copy {} hung: no answer in ", hung.port);
    // When
    let started = Instant::now();
    let mut transcriber = OpenAiTranscriber::connect(cfg).unwrap();
    transcriber.transcribe(&half_second()).unwrap();
    // Then
    let limit = attempt_deadline_logged_by_this_thread();
    let logged_at = loop {
        if !logged_by_any_thread(&hung_line).is_empty() {
            break started.elapsed();
        }
        assert!(
            started.elapsed() < limit * 3 + Duration::from_secs(2),
            "copy {} was never logged as hung",
            hung.port
        );
        thread::sleep(Duration::from_millis(50));
    };
    assert!(
        logged_at >= limit * 2,
        "logged hung after {logged_at:?}, before twice its limit {limit:?}"
    );
    let line = logged_by_any_thread(&hung_line).remove(0);
    let waited: u64 = line
        .split(&hung_line)
        .nth(1)
        .and_then(|rest| rest.split(' ').next())
        .and_then(|secs| secs.parse().ok())
        .unwrap_or_else(|| panic!("no time waited in {line:?}"));
    assert!(
        Duration::from_secs(waited + 1) > limit * 2,
        "{line:?} names less than twice the limit {limit:?}"
    );
}

/// The time limit of the first attempt of this thread's request, from its warning.
pub(super) fn attempt_deadline_logged_by_this_thread() -> Duration {
    let me = thread::current().id();
    let lines = capturing_logger().0.lock().unwrap();
    let ms: u64 = lines
        .iter()
        .filter(|(thread, line)| *thread == me && line.contains("attempt 1,"))
        .find_map(|(_, line)| {
            let after = line.split("deadline ").nth(1)?;
            after.split("ms").next()?.parse().ok()
        })
        .expect("the first attempt was logged with its deadline");
    Duration::from_millis(ms)
}

#[test]
pub(super) fn should_transcribe_a_chunk_by_another_copy_when_its_copy_is_killed() {
    // Given
    let (cfg, mut listeners) = free_copies(2);
    let other = spawn_copy(listeners.remove(1 - first_tried(2)), vec![CopyTurn::Answer]);
    let killed = spawn_copy(listeners.remove(0), vec![CopyTurn::Reset]);
    capturing_logger();
    // When
    let transcript = transcribe(cfg).unwrap();
    // Then
    assert_eq!(transcript.as_deref(), Some("hello"));
    assert_eq!(killed.requests(), 1, "the killed copy got a second attempt");
    assert_eq!(
        logged_by_this_thread("WARN speech recognition service dropped the connection"),
        1
    );
    assert_eq!(other.requests(), 1);
}

/// A fake copy that answers every request at once and records the length of each body it
/// got, in the order the requests reached it.
pub(super) fn spawn_recording_copy(listener: TcpListener) -> Arc<Mutex<Vec<usize>>> {
    let bodies = Arc::new(Mutex::new(Vec::new()));
    let recorded = Arc::clone(&bodies);
    thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = stream.unwrap();
            if voicepuncher_test_steps::loopback::answered_as_whisper(&mut stream) {
                continue;
            }
            let reply = StubReply::ok(r#"{"text": "hello"}"#);
            if let Some(request) = read_request(&mut stream, &reply) {
                recorded.lock().unwrap().push(request.body.len());
            }
        }
    });
    bodies
}

/// The body length of a request for `samples` of audio.
pub(super) fn body_length(samples: usize) -> usize {
    let (cfg, requests) = spawn_stub(StubReply::ok(r#"{"text": "hello"}"#));
    let mut transcriber = OpenAiTranscriber::connect(cfg).unwrap();
    transcriber
        .transcribe(&AudioBuffer16k::new(vec![100i16; samples]))
        .unwrap();
    next_request(&requests).body.len()
}

#[test]
pub(super) fn should_transcribe_the_first_of_two_waiting_requests_first_when_its_wait_ran_out() {
    // Given: the only copy busy for longer than a request waits for it, freed just after
    // the first waiting request's wait ran out, while it would pause before asking again
    const WAIT_SECS: u64 = 1;
    const FIRST_ARRIVES: Duration = Duration::from_millis(100);
    const SECOND_ARRIVES: Duration = Duration::from_millis(400);
    const COPY_FREES: Duration = Duration::from_millis(1_200);
    const FIRST_SAMPLES: usize = 8_000;
    const SECOND_SAMPLES: usize = 16_000;
    let (cfg, mut listeners) = free_copies(1);
    let bodies = spawn_recording_copy(listeners.remove(0));
    let cfg = TranscriptionCfg { timeout_secs: WAIT_SECS, ..cfg };
    let pool = voicepuncher_copy_pool::CopyPool::shared(&cfg.base_url).unwrap();
    let busy = pool.lease(Duration::from_secs(WAIT_SECS)).unwrap();
    let started = Instant::now();
    // When
    let requests: Vec<_> = [
        (FIRST_ARRIVES, FIRST_SAMPLES),
        (SECOND_ARRIVES, SECOND_SAMPLES),
    ]
    .into_iter()
    .map(|(arrives, samples)| {
        let cfg = cfg.clone();
        thread::spawn(move || {
            thread::sleep(arrives.saturating_sub(started.elapsed()));
            let mut transcriber = OpenAiTranscriber::connect(cfg).unwrap();
            transcriber
                .transcribe(&AudioBuffer16k::new(vec![100i16; samples]))
                .unwrap()
        })
    })
    .collect();
    thread::sleep(COPY_FREES.saturating_sub(started.elapsed()));
    busy.finished();
    for request in requests {
        assert_eq!(request.join().unwrap().unwrap().as_str(), "hello");
    }
    // Then
    let reached = bodies.lock().unwrap().clone();
    assert_eq!(
        reached,
        [body_length(FIRST_SAMPLES), body_length(SECOND_SAMPLES)],
        "the copy took the second request before the first one"
    );
}
