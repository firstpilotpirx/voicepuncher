//! The audio stream under test, its microphone and the free ports of the stand-in copies.

use super::*;
use voicepuncher::core::domain::conversions::saturating_usize;

pub(super) fn start_stream(url: &str) -> (AudioStream, Sender<Vec<f32>>, Receiver<PipelineInput>) {
    start_stream_from(url, Instant::now())
}

/// The stream of a client whose reconnect pauses and replay run `TIMINGS_SPEEDUP` times faster.
pub(super) fn start_fast_stream(
    url: &str,
) -> (AudioStream, Sender<Vec<f32>>, Receiver<PipelineInput>) {
    let (audio_tx, audio_rx) = mpsc::channel();
    let (inputs_tx, inputs_rx) = mpsc::channel();
    let stream = AudioStream::start_with_timings(
        url,
        &VoiceCommand::STANDBY,
        MicrophoneAudio {
            audio: audio_rx,
            native_rate: MIC_RATE_HZ,
            captured_from: Instant::now(),
        },
        inputs_tx,
        LinkTimings::default().faster(TIMINGS_SPEEDUP),
    )
    .unwrap();
    (stream, audio_tx, inputs_rx)
}

/// The stream of a microphone whose first sample was captured at `audio_start`.
pub(super) fn start_stream_from(
    url: &str,
    audio_start: Instant,
) -> (AudioStream, Sender<Vec<f32>>, Receiver<PipelineInput>) {
    let (audio_tx, audio_rx) = mpsc::channel();
    let (inputs_tx, inputs_rx) = mpsc::channel();
    let stream = AudioStream::start(
        url,
        &VoiceCommand::STANDBY,
        MicrophoneAudio { audio: audio_rx, native_rate: MIC_RATE_HZ, captured_from: audio_start },
        inputs_tx,
    )
    .unwrap();
    (stream, audio_tx, inputs_rx)
}

pub(super) fn microphone(level: f32) -> Vec<f32> {
    vec![level; saturating_usize(MIC_BUFFER_SECS * MIC_RATE_HZ as f32)]
}

/// Feeds the microphone `level` until `input` arrives, at most for `within`.
pub(super) fn feed_until(
    audio: &Sender<Vec<f32>>,
    inputs: &Receiver<PipelineInput>,
    level: f32,
    input: &PipelineInput,
    within: Duration,
) -> bool {
    let deadline = Instant::now() + within;
    while Instant::now() < deadline {
        audio.send(microphone(level)).unwrap();
        if inputs
            .recv_timeout(Duration::from_millis(100))
            .is_ok_and(|got| got == *input)
        {
            return true;
        }
    }
    false
}

/// The service ports the fake copies' addresses take: their copies land 41 000 below, on
/// 2001–8004, clear of the system's ephemeral ports (49152+) and of the user's services.
pub(super) const FAKE_SERVICE_PORTS: std::ops::Range<u16> = 43_000..49_000;

/// Listeners on the first two copy ports of a command spotting service address nothing
/// listens on, and that address.
pub(super) fn two_free_copies() -> (TcpListener, TcpListener, String) {
    loop {
        let base = reserve_port();
        let url = format!("ws://127.0.0.1:{base}");
        let ports = voicepuncher_copy_pool::ServiceAddress::parse(&url)
            .unwrap()
            .with_copy_offset(voicepuncher_copy_pool::COMMAND_SPOTTING_COPY_OFFSET)
            .copy_ports();
        if let (Ok(first), Ok(second)) = (
            TcpListener::bind(("127.0.0.1", ports[0])),
            TcpListener::bind(("127.0.0.1", ports[1])),
        ) {
            return (first, second, url);
        }
    }
}

/// A port of `FAKE_SERVICE_PORTS` bound without listening for the life of the test process.
pub(super) fn reserve_port() -> u16 {
    crate::reserved_ports::reserve_port(FAKE_SERVICE_PORTS)
}

/// A stub service on `listener` that keeps every connection, recording what it receives.
pub(super) fn stub_service_that_stays(listener: TcpListener) -> Arc<Mutex<Vec<Received>>> {
    let connections = Arc::new(Mutex::new(Vec::new()));
    let recorded = Arc::clone(&connections);
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let connections = Arc::clone(&recorded);
            std::thread::spawn(move || serve(stream, 1, &connections));
        }
    });
    connections
}
