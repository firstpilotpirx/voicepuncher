use super::*;

mod under_load;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use voicepuncher_copy_pool::CopyPool;

/// A fake copy on its own port: answers every request with `answer` and counts them.
fn spawn_copy(answer: &'static str) -> (String, Arc<AtomicUsize>) {
    spawn_slow_copy(answer, Duration::ZERO)
}

/// A fake copy that decodes each request for `decoding` before it answers.
fn spawn_slow_copy(answer: &'static str, decoding: Duration) -> (String, Arc<AtomicUsize>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/v1", listener.local_addr().unwrap());
    (url, serve_copy(listener, answer, decoding))
}

/// Serves every request on `listener` one at a time, with `answer` after `decoding`, as a
/// copy that stalls while it decodes; counts the transcriptions.
fn serve_copy(listener: TcpListener, answer: &'static str, decoding: Duration) -> Arc<AtomicUsize> {
    let requests = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&requests);
    thread::spawn(move || {
        for stream in listener.incoming() {
            answer_request(stream.unwrap(), &[(decoding, answer)], &counter);
        }
    });
    requests
}

/// As `serve_copy`, each request on a thread of its own: a live copy that answers its health
/// check while it decodes, decoding each transcription for its entry of `decodings`, the last
/// one for every later transcription: a copy whose pace changes.
fn serve_live_copy_paced(
    listener: TcpListener,
    answer: &'static str,
    decodings: Vec<Duration>,
) -> Arc<AtomicUsize> {
    serve_live_copy_scripted(
        listener,
        decodings.into_iter().map(|d| (d, answer)).collect(),
    )
}

/// As `serve_live_copy_paced`, the n-th transcription answered after the decoding and with
/// the answer of the n-th entry of `script` (the last one once they run out).
fn serve_live_copy_scripted(
    listener: TcpListener,
    script: Vec<(Duration, &'static str)>,
) -> Arc<AtomicUsize> {
    let requests = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&requests);
    let script = Arc::new(script);
    thread::spawn(move || {
        for stream in listener.incoming() {
            let counter = Arc::clone(&counter);
            let script = Arc::clone(&script);
            thread::spawn(move || answer_request(stream.unwrap(), &script, &counter));
        }
    });
    requests
}

/// Answers a health check at once, and the n-th transcription after the decoding and with
/// the answer of the n-th entry of `script` (the last one once they run out).
fn answer_request(
    mut stream: TcpStream,
    script: &[(Duration, &'static str)],
    counter: &AtomicUsize,
) {
    if voicepuncher_test_steps::loopback::answered_as_whisper(&mut stream) {
        return;
    }
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut request = String::new();
    if reader.read_line(&mut request).unwrap_or(0) == 0 {
        return;
    }
    let length = content_length(&mut reader);
    let reply = if request.starts_with("GET /health") {
        "ok"
    } else {
        let n = counter.fetch_add(1, Ordering::SeqCst);
        reader.read_exact(&mut vec![0u8; length]).unwrap();
        let (decoding, answer) = script[n.min(script.len() - 1)];
        thread::sleep(decoding);
        answer
    };
    let reply = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{reply}",
        reply.len()
    );
    // The client may have closed the connection while the copy decoded.
    let _ = stream.write_all(reply.as_bytes());
}

/// Reads the headers of a request; its body's length.
fn content_length(reader: &mut BufReader<TcpStream>) -> usize {
    let mut length = 0;
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        let line = line.trim_end();
        if line.is_empty() {
            return length;
        }
        match line.split_once(':') {
            Some((name, value)) if name.eq_ignore_ascii_case("content-length") => {
                length = value.trim().parse().unwrap();
            }
            _ => {}
        }
    }
}

/// The service ports a two-copies test may take: `WINDOW_SLOTS` of them, `WINDOW_STEP` apart
/// from `FIRST_WINDOW_BASE`, so the copy ports of two tests never overlap (two tests that
/// pick one slot see it at the bind). Below the ephemeral ports, which macOS hands out in
/// sequence, so no `127.0.0.1:0` of another test lands in a window, and apart from every
/// port the project's services and checks use (design of fix-load-sensitive-confirm-tests).
const FIRST_WINDOW_BASE: u16 = 30_000;
const WINDOW_SLOTS: u16 = 625;
const WINDOW_STEP: u16 = 8;

/// Listeners on the first two copy ports of a service address nothing listens on, in a
/// copy-port window of this test's own.
fn two_copies() -> (String, TcpListener, TcpListener) {
    let first_slot = u16::try_from(std::process::id() % u32::from(WINDOW_SLOTS)).unwrap();
    (0..WINDOW_SLOTS)
        .map(|n| FIRST_WINDOW_BASE + ((first_slot + n) % WINDOW_SLOTS) * WINDOW_STEP)
        .find_map(|base| {
            let url = format!("http://127.0.0.1:{base}/v1");
            let ports = voicepuncher_copy_pool::ServiceAddress::parse(&url)
                .unwrap()
                .copy_ports();
            let first = TcpListener::bind(("127.0.0.1", ports[0])).ok()?;
            let second = TcpListener::bind(("127.0.0.1", ports[1])).ok()?;
            Some((url, first, second))
        })
        .unwrap()
}

#[test]
fn should_give_each_two_copies_test_a_window_of_copy_ports_of_its_own() {
    // Given two tests that serve two copies each, at the same time
    let (first_url, _first_a, _first_b) = two_copies();
    let (second_url, _second_a, _second_b) = two_copies();
    let address = |url: &str| voicepuncher_copy_pool::ServiceAddress::parse(url).unwrap();
    let (first, second) = (address(&first_url), address(&second_url));
    // Then neither pool can find the other's copies
    let overlap: Vec<u16> = first
        .copy_ports()
        .into_iter()
        .filter(|port| second.copy_ports().contains(port))
        .collect();
    assert!(overlap.is_empty(), "{overlap:?}");
    // And no port lies where `127.0.0.1:0` hands ports out, in sequence, to any program
    let ports = [first.port(), second.port()]
        .into_iter()
        .chain(first.copy_ports())
        .chain(second.copy_ports());
    for port in ports {
        assert!(port < EPHEMERAL_PORTS, "{port}");
    }
}

/// Where macOS hands out the ports of `127.0.0.1:0` (`net.inet.ip.portrange.first`).
const EPHEMERAL_PORTS: u16 = 49_152;

#[test]
fn should_send_a_confirmation_refused_by_its_copy_to_the_other_copy() {
    // Given two copies; the one this process tries first stops accepting connections
    let lines = voicepuncher_test_steps::log_lines::captured(log::LevelFilter::Trace);
    let (url, first, second) = two_copies();
    let (down, up) = if std::process::id() % 2 == 0 {
        (first, second)
    } else {
        (second, first)
    };
    let down_port = down.local_addr().unwrap().port();
    let down = voicepuncher_test_steps::loopback::WhisperUntilClosed::on(down);
    let requests = serve_copy(up, PRESS_ENTER, Duration::ZERO);
    let transcriber = UtteranceTranscriber::new(&url).unwrap();
    // The first probe round saw both copies before one went down.
    assert!(CopyPool::shared(&url).unwrap().has_copies());
    down.close();
    // When
    let heard = heard_by(&transcriber);
    // Then
    assert_eq!(heard.text, " press enter");
    assert_eq!(requests.load(Ordering::SeqCst), 1);
    let refused = lines.containing("confirmation refused by copy");
    assert!(
        refused
            .iter()
            .any(|line| line.contains(&down_port.to_string())),
        "{refused:?}"
    );
}

const PRESS_ENTER: &str = r#"{"text":" press enter","segments":[]}"#;
const DEADLINE: Duration = Duration::from_millis(300);
const SAMPLE_RATE: u32 = 16_000;

#[test]
fn should_give_a_confirmation_up_unsent_when_every_copy_stays_busy_past_its_deadline() {
    // Given
    let lines = voicepuncher_test_steps::log_lines::captured(log::LevelFilter::Trace);
    let (url, requests) = spawn_copy(PRESS_ENTER);
    let transcriber = UtteranceTranscriber::new(&url).unwrap();
    let held = CopyPool::shared(&url).unwrap().lease(DEADLINE).unwrap();
    // When
    let started = Instant::now();
    let result = transcriber.transcribe(&[0.0; 1_600], SAMPLE_RATE, DEADLINE);
    // Then
    assert!(result.is_err());
    let given_up = lines.containing("confirmation given up unsent");
    assert!(
        given_up.iter().any(|line| line.contains(&url)),
        "{given_up:?}"
    );
    assert!(
        started.elapsed() < DEADLINE * 2,
        "waited {:?}",
        started.elapsed()
    );
    assert_eq!(requests.load(Ordering::SeqCst), 0);
    drop(held);
}

#[test]
fn should_send_a_confirmation_to_the_copy_that_frees_before_its_deadline() {
    // Given
    let (url, requests) = spawn_copy(PRESS_ENTER);
    let transcriber = UtteranceTranscriber::new(&url).unwrap();
    let held = CopyPool::shared(&url).unwrap().lease(DEADLINE).unwrap();
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(200));
        held.finished();
    });
    // When
    let heard = heard_by(&transcriber);
    // Then
    assert_eq!(heard.text, " press enter");
    assert_eq!(requests.load(Ordering::SeqCst), 1);
}

#[test]
fn should_send_a_confirmation_its_copy_does_not_answer_within_the_budget_to_the_other_copy() {
    // Given two copies; the one this process tries first decodes past the budget
    let lines = voicepuncher_test_steps::log_lines::captured(log::LevelFilter::Trace);
    let (url, first, second) = two_copies();
    let (slow, fast) = if std::process::id() % 2 == 0 {
        (first, second)
    } else {
        (second, first)
    };
    serve_copy(slow, PRESS_ENTER, Duration::from_secs(5));
    let answered = serve_copy(fast, PRESS_ENTER, Duration::ZERO);
    let transcriber = UtteranceTranscriber::new(&url).unwrap();
    // When
    let started = Instant::now();
    let heard = transcriber
        .transcribe(&[0.0; 1_600], SAMPLE_RATE, Duration::from_secs(3))
        .unwrap();
    let took = started.elapsed();
    // Then
    assert_eq!(heard.text, " press enter");
    assert_eq!(answered.load(Ordering::SeqCst), 1);
    assert!(took < Duration::from_millis(1_800), "took {took:?}");
    let cut = lines.containing("confirmation cut short");
    assert!(
        cut.iter().any(|line| line.contains("another copy")),
        "{cut:?}"
    );
}

#[test]
fn should_fail_a_confirmation_when_every_copy_stays_silent_past_its_deadline() {
    // Given
    let (url, _) = spawn_slow_copy(PRESS_ENTER, Duration::from_secs(5));
    let transcriber = UtteranceTranscriber::new(&url).unwrap();
    // When
    let started = Instant::now();
    let result = transcriber.transcribe(&[0.0; 1_600], SAMPLE_RATE, Duration::from_secs(2));
    let took = started.elapsed();
    // Then
    assert!(result.is_err(), "{result:?}");
    assert!(took < Duration::from_millis(2_600), "took {took:?}");
}

#[test]
fn should_hold_only_the_last_samples() {
    // Given
    let mut recent = RecentAudio::new(4);
    // When
    recent.push(&[1.0, 2.0, 3.0]);
    recent.push(&[4.0, 5.0, 6.0]);
    // Then samples 2..6 are held
    assert_eq!(recent.between(0, 10), [3.0, 4.0, 5.0, 6.0]);
    assert_eq!(recent.between(3, 5), [4.0, 5.0]);
    assert!(recent.between(8, 10).is_empty());
}

#[test]
fn should_read_the_words_and_their_times() {
    // Given
    let answer = r#"{"text":" we meet stop","segments":[
        {"start":0.0,"end":1.0,"words":[{"word":" we","start":0.0,"end":0.2},{"word":" meet","start":0.2,"end":0.6}]},
        {"start":1.0,"end":1.4,"words":[{"word":" stop","start":1.0,"end":1.4}]}]}"#;
    // When
    let heard = heard_from(answer).unwrap();
    // Then
    assert_eq!(heard.text, " we meet stop");
    assert_eq!(heard.word_texts(), [" we", " meet", " stop"]);
    assert_eq!((heard.words[2].start, heard.words[2].end), (1.0, 1.4));
}

#[test]
fn should_join_the_pieces_of_a_word() {
    // Given
    let answer = r#"{"text":" top ender.","segments":[{"words":[
        {"word":" top","start":0.0,"end":0.3},{"word":" end","start":0.3,"end":0.5},
        {"word":"er","start":0.5,"end":0.7},{"word":".","start":0.7,"end":0.7}]}]}"#;
    // When
    let heard = heard_from(answer).unwrap();
    // Then
    assert_eq!(heard.word_texts(), [" top", " ender."]);
    assert_eq!((heard.words[1].start, heard.words[1].end), (0.3, 0.7));
}

/// What `transcriber` hears in a tenth of a second of silence, within 2 s.
fn heard_by(transcriber: &UtteranceTranscriber) -> Heard {
    transcriber
        .transcribe(&[0.0; 1_600], SAMPLE_RATE, Duration::from_secs(2))
        .unwrap()
}
