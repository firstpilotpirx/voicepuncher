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
