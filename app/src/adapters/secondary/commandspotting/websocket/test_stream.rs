//! An audio stream whose ends a test drives, for the tests of the command spotting client.

use super::{AudioStream, MicrophoneAudio};

/// The rate the stand-in microphones of the command spotting tests capture at.
pub const MICROPHONE_RATE_HZ: u32 = 48_000;

/// An audio stream to a command spotting service with the ends a test drives: the microphone's
/// buffers go into `audio`, what the stream reports comes out of `inputs`.
pub struct OpenStream {
    pub stream: AudioStream,
    pub audio: std::sync::mpsc::Sender<Vec<f32>>,
    pub inputs: std::sync::mpsc::Receiver<crate::core::application::PipelineInput>,
}

/// Opens an audio stream to the command spotting service at `url`, listening for `commands`,
/// its microphone capturing at [`MICROPHONE_RATE_HZ`] from now on.
pub fn open_stream(
    url: &str,
    commands: &[crate::core::domain::VoiceCommand],
) -> anyhow::Result<OpenStream> {
    let (audio, audio_rx) = std::sync::mpsc::channel();
    let (inputs_tx, inputs) = std::sync::mpsc::channel();
    let microphone = MicrophoneAudio {
        audio: audio_rx,
        native_rate: MICROPHONE_RATE_HZ,
        captured_from: std::time::Instant::now(),
    };
    let stream = AudioStream::start(url, commands, microphone, inputs_tx)?;
    Ok(OpenStream { stream, audio, inputs })
}

/// Takes the client's greeting as the command spotting service does: reads the hello and the
/// ping after it, and sends the pong at once; a stand-in service calls it before it speaks.
pub fn take_greeting<S: std::io::Read + std::io::Write>(socket: &mut tungstenite::WebSocket<S>) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    while std::time::Instant::now() < deadline {
        match socket.read() {
            Ok(tungstenite::Message::Ping(_)) => {
                let _ = socket.flush();
                return;
            }
            Ok(_) => {}
            Err(tungstenite::Error::Io(e)) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(_) => return,
        }
    }
}

/// Refuses the client's hello as a command spotting service of protocol version 2 does: closes
/// with the policy reason naming both versions, then reads until the client is gone.
pub fn refuse_as_version_2<S: std::io::Read + std::io::Write>(
    socket: &mut tungstenite::WebSocket<S>,
) {
    use tungstenite::protocol::{frame::coding::CloseCode, CloseFrame};
    let _ = socket.read();
    let reason = "the app speaks protocol version 3, the service version 2".into();
    let _ = socket.close(Some(CloseFrame { code: CloseCode::Policy, reason }));
    while socket.read().is_ok() {}
}
