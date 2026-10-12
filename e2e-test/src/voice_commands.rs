//! Voice control phrases spoken by macOS `say` and streamed to the command spotting service
//! as the app streams the microphone while listening.

use std::process::Command;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};

use voicepuncher::adapters::secondary::commandspotting::websocket::client::SpottingClient;
use voicepuncher::core::domain::{SpottingEvent, VoiceCommand, VoiceCommandRecognizer};

use crate::speech::read_wav;

const SAMPLE_RATE_HZ: f32 = 16_000.0;
/// The silence before and after the words: the pause around a command.
const PAUSE_SECS: f32 = 1.0;
/// 100 ms of audio per message, as the app sends it.
const FRAME_SAMPLES: usize = 1_600;
/// How long the service may stay quiet before every event is taken to have arrived.
const QUIET_SECS: Duration = Duration::from_secs(2);

/// English `words` spoken by `say`, 16 kHz mono, with a pause of silence before and after.
pub fn spoken_with_pauses(words: &str) -> Result<Vec<f32>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("words.wav");
    let status = Command::new("say")
        .args(["--file-format=WAVE", "--data-format=LEI16@16000", "-o"])
        .arg(&path)
        .arg(words)
        .status()
        .context("failed to run macOS 'say'")?;
    if !status.success() {
        bail!("'say' failed to synthesize speech ({status})");
    }
    let pause = vec![0.0f32; (PAUSE_SECS * SAMPLE_RATE_HZ) as usize];
    let speech = read_wav(&path)?
        .into_iter()
        .map(|s| f32::from(s) / f32::from(i16::MAX));
    Ok(pause
        .iter()
        .copied()
        .chain(speech)
        .chain(pause.iter().copied())
        .collect())
}

/// `secs` of silence at 16 kHz.
pub fn silence(secs: f32) -> Vec<f32> {
    vec![0.0f32; (secs * SAMPLE_RATE_HZ) as usize]
}

/// Every spotting event the service reports for `audio`, listening for `commands`.
pub fn spotting_events(
    url: &str,
    audio: &[f32],
    commands: &[VoiceCommand],
) -> Result<Vec<SpottingEvent>> {
    let mut client = SpottingClient::connect(url, Instant::now())?;
    client.listen_for(commands)?;
    let mut events = Vec::new();
    for frame in audio.chunks(FRAME_SAMPLES) {
        client.send_audio(frame)?;
        events.extend(client.read_events()?);
    }
    let mut last_event = Instant::now();
    while last_event.elapsed() < QUIET_SECS {
        let arrived = client.read_events()?;
        if !arrived.is_empty() {
            last_event = Instant::now();
        }
        events.extend(arrived);
    }
    client.close();
    Ok(events)
}

/// The voice commands the app recognizes in `events`.
pub fn recognized_commands(events: Vec<SpottingEvent>) -> Vec<VoiceCommand> {
    let mut recognizer = VoiceCommandRecognizer::new();
    events
        .into_iter()
        .filter_map(|e| recognizer.feed(e))
        .map(|spoken| spoken.command)
        .collect()
}
