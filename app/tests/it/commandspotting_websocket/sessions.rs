//! Microphone speech and the listening sessions the tests run against the service.

use super::*;
use voicepuncher::adapters::secondary::commandspotting::websocket::test_stream::{
    open_stream, OpenStream,
};
use voicepuncher::core::domain::conversions::saturating_usize;

/// A typical microphone: 48 kHz, delivered in CoreAudio-sized buffers.
pub(super) const MIC_RATE_HZ: u32 = 48_000;

pub(super) const MIC_BUFFER: usize = 512;

pub(super) const SPEECH_RATE_HZ: u32 = 16_000;

/// How long the service may stay quiet after the first command before every event is taken to
/// have arrived.
pub(super) const QUIET: Duration = Duration::from_secs(2);

/// How long connecting and the WebSocket handshake with the service may take.
pub(super) const HANDSHAKE_LIMIT: Duration = Duration::from_secs(5);

pub(super) const HANDSHAKE_FAILED: &str =
    "the command spotting service did not finish the handshake in 5 s";

/// How long the service may take to refuse an unknown phrase: it refuses at once.
pub(super) const REFUSAL_LIMIT: Duration = Duration::from_secs(5);

pub(super) const NOT_REFUSED: &str =
    "the command spotting service did not refuse the unknown phrase in 5 s";

/// How long the events of one utterance may keep coming after the speech was sent.
pub(super) const UTTERANCE_EVENTS_LIMIT: Duration = Duration::from_secs(30);

pub(super) const HELLO: &str = r#"{"type":"hello","version":4}"#;

/// A raw WebSocket to the service: connecting and the handshake are each given up after the
/// handshake limit, and every read after the refusal limit.
pub(super) fn raw_socket(url: &str) -> WebSocket<MaybeTlsStream<TcpStream>> {
    let addrs = url::Url::parse(url).unwrap().socket_addrs(|| None).unwrap();
    let stream = TcpStream::connect_timeout(&addrs[0], HANDSHAKE_LIMIT)
        .unwrap_or_else(|e| panic!("{HANDSHAKE_FAILED}: {e}"));
    stream.set_read_timeout(Some(HANDSHAKE_LIMIT)).unwrap();
    stream.set_write_timeout(Some(HANDSHAKE_LIMIT)).unwrap();
    let (socket, _) = tungstenite::client(url, MaybeTlsStream::Plain(stream))
        .unwrap_or_else(|e| panic!("{HANDSHAKE_FAILED}: {e}"));
    if let MaybeTlsStream::Plain(stream) = socket.get_ref() {
        stream.set_read_timeout(Some(REFUSAL_LIMIT)).unwrap();
    }
    socket
}

/// 16 kHz speech as a 48 kHz microphone would capture it.
pub(super) fn at_microphone_rate(speech: &[f32]) -> Vec<f32> {
    let factor = (MIC_RATE_HZ / SPEECH_RATE_HZ) as usize;
    speech
        .windows(2)
        .flat_map(|pair| {
            (0..factor).map(move |i| pair[0] + (pair[1] - pair[0]) * i as f32 / factor as f32)
        })
        .collect()
}

/// The word the user says for `command`: its one-word form.
pub(super) fn word_of(command: VoiceCommand) -> &'static str {
    voicepuncher_voice_commands::FORMS
        .iter()
        .find(|form| form.phrase == command.phrase())
        .and_then(|form| form.words.first())
        .unwrap_or_else(|| panic!("{command:?} has no one-word form"))
}

/// The phrases the app listens for in the state where `command` is accepted.
pub(super) fn listened_for(command: VoiceCommand) -> &'static [VoiceCommand] {
    if VoiceCommand::DICTATING.contains(&command) {
        &VoiceCommand::DICTATING
    } else {
        &VoiceCommand::STANDBY
    }
}

/// What the app recognizes from the events of `inputs` until the service goes quiet.
fn recognized_until_quiet(inputs: &mpsc::Receiver<PipelineInput>) -> Vec<VoiceCommand> {
    let mut recognized = Vec::new();
    let deadline = Instant::now() + UTTERANCE_EVENTS_LIMIT;
    let wait = |recognized: &[VoiceCommand]| {
        if recognized.is_empty() {
            COMMAND_TIMEOUT
        } else {
            QUIET
        }
    };
    while let Ok(input) = inputs.recv_timeout(wait(&recognized)) {
        assert!(
            Instant::now() < deadline,
            "the command spotting service still sent events 30 s after the speech"
        );
        let PipelineInput::Spotting(event) = input else {
            panic!("unexpected input {input:?}");
        };
        recognized.extend(
            event
                .into_commands()
                .into_iter()
                .map(|spoken| spoken.command),
        );
    }
    recognized
}

/// Sends `samples` to `audio` buffer by buffer, as fast as a microphone hears them.
pub(super) fn say_in_real_time(audio: &mpsc::Sender<Vec<f32>>, samples: &[f32]) {
    let buffer_time = Duration::from_secs_f32(MIC_BUFFER as f32 / MIC_RATE_HZ as f32);
    for buffer in samples.chunks(MIC_BUFFER) {
        audio.send(buffer.to_vec()).unwrap();
        std::thread::sleep(buffer_time);
    }
}

/// What the app recognizes when `words` reach it from a 48 kHz microphone.
pub(super) fn recognized_from_microphone(
    url: &str,
    words: &str,
    commands: &[VoiceCommand],
) -> Vec<VoiceCommand> {
    let OpenStream { stream, audio: audio_tx, inputs: inputs_rx } =
        open_stream(url, commands).unwrap();
    for buffer in at_microphone_rate(&spoken_with_pauses(words).unwrap()).chunks(MIC_BUFFER) {
        audio_tx.send(buffer.to_vec()).unwrap();
    }
    let recognized = recognized_until_quiet(&inputs_rx);
    stream.stop();
    recognized
}

/// As many apps as `should_serve_several_apps_listening_at_once` proves the service serves at
/// once.
pub(super) const APPS_AT_ONCE: usize = 3;

/// `hear` for each case, each on a connection of its own, at most [`APPS_AT_ONCE`] at the same
/// time, the results in the order of the cases: the cases of one test wait out their real-time
/// pauses together instead of one after another.
pub(super) fn each_at_once<C: Copy + Send, R: Send>(
    cases: &[C],
    hear: impl Fn(C) -> R + Sync,
) -> Vec<R> {
    cases
        .chunks(APPS_AT_ONCE)
        .flat_map(|chunk| {
            let hear = &hear;
            std::thread::scope(|scope| {
                let handles: Vec<_> = chunk
                    .iter()
                    .map(|&case| scope.spawn(move || hear(case)))
                    .collect();
                handles
                    .into_iter()
                    .map(|h| h.join().unwrap())
                    .collect::<Vec<_>>()
            })
        })
        .collect()
}

/// Longest the service may take to report a command after its audio reached it.
pub(super) const COMMAND_TIMEOUT: Duration = Duration::from_secs(10);

pub(super) fn microphone_silence(secs: f32) -> Vec<f32> {
    vec![0.0f32; saturating_usize(secs * MIC_RATE_HZ as f32)]
}

/// One listening session as the app runs it: a single connection, audio sent buffer by
/// buffer, the commands taken from every event.
pub(super) struct Listening {
    pub(crate) audio: mpsc::Sender<Vec<f32>>,
    inputs: mpsc::Receiver<PipelineInput>,
    pub(crate) stream: AudioStream,
}

impl Listening {
    pub(crate) fn start(url: &str, commands: &[VoiceCommand]) -> Self {
        let OpenStream { stream, audio, inputs } = open_stream(url, commands).unwrap();
        Self { audio, inputs, stream }
    }

    /// The next command the app recognizes, as the pipeline would get it.
    pub(crate) fn next_command(&mut self) -> Option<VoiceCommand> {
        let deadline = Instant::now() + COMMAND_TIMEOUT;
        let inputs = &self.inputs;
        std::iter::from_fn(|| {
            inputs
                .recv_timeout(deadline.checked_duration_since(Instant::now())?)
                .ok()
        })
        .find_map(|input| match input {
            PipelineInput::Spotting(event) => event.into_commands().first().map(|s| s.command),
            other => panic!("unexpected input {other:?}"),
        })
    }
}

/// The pause a quick user leaves between two commands.
pub(super) const SHORT_PAUSE_SECS: f32 = 0.8;

/// Longer than the silence the app allows the service before it gives up on it.
pub(super) const QUIET_SESSION: Duration = Duration::from_secs(15);
