//! Spreading requests over the service's copies.

use super::*;

/// What one fake copy does with one request.
#[derive(Clone, Copy)]
pub(super) enum CopyTurn {
    /// Answers "hello" after `COPY_WORK`.
    Answer,
    /// Reads the request and never answers it.
    Hang,
    /// Resets the connection as the request reaches it, as a killed copy does.
    Reset,
}

/// How long a fake copy works on a request it answers.
pub(super) const COPY_WORK: Duration = Duration::from_millis(300);

/// A fake copy of the service: counts the requests it got and how many it held at once.
pub(super) struct FakeCopy {
    pub(crate) port: u16,
    requests: Arc<AtomicUsize>,
    most_at_once: Arc<AtomicUsize>,
}

impl FakeCopy {
    pub(crate) fn requests(&self) -> usize {
        self.requests.load(Ordering::SeqCst)
    }

    fn most_at_once(&self) -> usize {
        self.most_at_once.load(Ordering::SeqCst)
    }
}

/// Listeners on the first `n` copy ports of a service address nothing listens on.
pub(super) fn free_copies(n: usize) -> (TranscriptionCfg, Vec<TcpListener>) {
    loop {
        let base = reserve_port();
        let base_url = format!("http://127.0.0.1:{base}/v1");
        let ports = ServiceAddress::parse(&base_url).unwrap().copy_ports();
        let listeners: Result<Vec<_>, _> = ports[..n]
            .iter()
            .map(|port| TcpListener::bind(("127.0.0.1", *port)))
            .collect();
        if let Ok(listeners) = listeners {
            let cfg = TranscriptionCfg { base_url, ..TranscriptionCfg::default() };
            return (cfg, listeners);
        }
    }
}

/// The service ports the fake copies' addresses take: their copies land on 21001–28000, clear of
/// the system's ephemeral ports (49152+), which the stubs of the other tests take and whose
/// pools go on probing ports they freed, and clear of the user's copies (18082–18089) and the
/// test services (28080–28085, 38082–38089).
pub(super) const FAKE_SERVICE_PORTS: std::ops::Range<u16> = 11_000..18_000;

/// A port of `FAKE_SERVICE_PORTS` bound without listening for the life of the test process.
pub(super) fn reserve_port() -> u16 {
    crate::reserved_ports::reserve_port(FAKE_SERVICE_PORTS)
}

/// The index of the copy this process tries first among `n` copies.
pub(super) fn first_tried(n: usize) -> usize {
    std::process::id() as usize % n
}

/// What a fake copy counts: every request, those in flight, and the most at once.
#[derive(Clone, Default)]
struct CopyCounters {
    requests: Arc<AtomicUsize>,
    most_at_once: Arc<AtomicUsize>,
    in_flight: Arc<AtomicUsize>,
}

/// Serves one connection as the turn of `script` it comes at says; a connection that sends
/// nothing (the pool's probe) is not counted.
fn serve_copy_request(mut stream: TcpStream, script: &[CopyTurn], counters: &CopyCounters) {
    if !voicepuncher_test_steps::loopback::is_request(&mut stream) {
        return;
    }
    let n = counters.requests.fetch_add(1, Ordering::SeqCst);
    let now = counters.in_flight.fetch_add(1, Ordering::SeqCst) + 1;
    counters.most_at_once.fetch_max(now, Ordering::SeqCst);
    match script[n.min(script.len() - 1)] {
        CopyTurn::Answer => {
            let reply = StubReply { delay: COPY_WORK, ..StubReply::ok(r#"{"text": "hello"}"#) };
            read_request(&mut stream, &reply);
            counters.in_flight.fetch_sub(1, Ordering::SeqCst);
        }
        CopyTurn::Hang => {
            let mut buf = vec![0u8; 65536];
            let _ = stream.read(&mut buf);
            thread::sleep(Duration::from_secs(600));
        }
        CopyTurn::Reset => {
            counters.in_flight.fetch_sub(1, Ordering::SeqCst);
            reset(stream);
        }
    }
}

/// Serves each request on its own thread as `script` says; the last turn repeats.
/// Connections that send nothing (the pool's probes) are not counted.
pub(super) fn spawn_copy(listener: TcpListener, script: Vec<CopyTurn>) -> FakeCopy {
    let port = listener.local_addr().unwrap().port();
    let counters = CopyCounters::default();
    let copy = FakeCopy {
        port,
        requests: Arc::clone(&counters.requests),
        most_at_once: Arc::clone(&counters.most_at_once),
    };
    thread::spawn(move || {
        for stream in listener.incoming() {
            let (stream, script, counters) = (stream.unwrap(), script.clone(), counters.clone());
            thread::spawn(move || serve_copy_request(stream, &script, &counters));
        }
    });
    copy
}

#[test]
pub(super) fn should_send_requests_from_two_clients_at_once_to_two_copies_and_queue_the_rest() {
    // Given
    let (cfg, listeners) = free_copies(2);
    let copies: Vec<FakeCopy> = listeners
        .into_iter()
        .map(|listener| spawn_copy(listener, vec![CopyTurn::Answer]))
        .collect();
    // When
    let started = Instant::now();
    // every client starts before the first is joined, so they send at once
    let clients: Vec<_> = (0..4)
        .map(|_| {
            let cfg = cfg.clone();
            thread::spawn(move || transcribe(cfg))
        })
        .collect();
    let mut transcripts = Vec::with_capacity(clients.len());
    for client in clients {
        transcripts.push(client.join().unwrap());
    }
    // Then
    assert!(
        started.elapsed() >= COPY_WORK * 2,
        "four requests on two copies took {:?}: two of them did not wait their turn",
        started.elapsed()
    );
    for transcript in transcripts {
        assert_eq!(transcript.unwrap().as_deref(), Some("hello"));
    }
    for copy in &copies {
        assert!(copy.requests() >= 1, "copy {} got no request", copy.port);
        assert_eq!(
            copy.most_at_once(),
            1,
            "copy {} held two requests",
            copy.port
        );
    }
    assert_eq!(copies.iter().map(FakeCopy::requests).sum::<usize>(), 4);
}

#[test]
pub(super) fn should_skip_a_copy_closed_before_a_request() {
    // Given
    let (cfg, mut listeners) = free_copies(2);
    let other = spawn_copy(listeners.remove(1 - first_tried(2)), vec![CopyTurn::Answer]);
    capturing_logger();
    let closed = listeners.remove(0);
    let closed_port = closed.local_addr().unwrap().port();
    let closed = voicepuncher_test_steps::loopback::WhisperUntilClosed::on(closed);
    let mut transcriber = OpenAiTranscriber::connect(cfg).unwrap();
    closed.close();
    // When
    let transcript = transcriber.transcribe(&half_second()).unwrap();
    // Then
    assert_eq!(transcript.unwrap().as_str(), "hello");
    assert_eq!(other.requests(), 1);
    let skipped = format!("copy {closed_port} refused the connection: skipped");
    assert_eq!(logged_by_any_thread(&skipped).len(), 1, "{skipped}");
}

/// Two copies under the short time limit: the one tried first hangs on its request, the other
/// answers; the settings, the hung copy and the other one.
pub(super) fn one_hung_copy_of_two() -> (TranscriptionCfg, FakeCopy, FakeCopy) {
    let (cfg, mut listeners) = free_copies(2);
    let other = spawn_copy(listeners.remove(1 - first_tried(2)), vec![CopyTurn::Answer]);
    let hung = spawn_copy(listeners.remove(0), vec![CopyTurn::Hang]);
    (
        TranscriptionCfg { timeout_secs: SHORT_TIMEOUT_SECS, ..cfg },
        hung,
        other,
    )
}
