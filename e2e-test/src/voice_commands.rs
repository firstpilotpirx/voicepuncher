//! Voice control phrases spoken by macOS `say` and streamed to the command spotting service
//! as the app streams the microphone while listening.

use std::time::Instant;

use anyhow::Result;

use voicepuncher::adapters::secondary::commandspotting::websocket::client::SpottingClient;
use voicepuncher::core::domain::{SpottingEvent, VoiceCommand, VoiceCommandRecognizer};

use crate::speech_cache::{synthesized, SayRequest};

const SAMPLE_RATE_HZ: f32 = 16_000.0;
/// The silence before and after the words: the pause around a command.
const PAUSE_SECS: f32 = 1.0;
/// 100 ms of audio per message, as the app sends it.
const FRAME_SAMPLES: usize = 1_600;
/// 2 s of audio: sent before waiting for the service to answer it, so the audio in flight
/// stays far below what the socket holds and a send never blocks on a busy service.
const SAMPLES_BETWEEN_WAITS: usize = 20 * FRAME_SAMPLES;

/// English `words` spoken by `say`, 16 kHz mono, with a pause of silence before and after.
pub fn spoken_with_pauses(words: &str) -> Result<Vec<f32>> {
    spoken(None, words)
}

/// `words` spoken by the `say` voice `voice` ("Milena" for Russian), with pauses around them.
pub fn spoken_by(voice: &str, words: &str) -> Result<Vec<f32>> {
    spoken(Some(voice), words)
}

fn spoken(voice: Option<&str>, words: &str) -> Result<Vec<f32>> {
    let pause = vec![0.0f32; (PAUSE_SECS * SAMPLE_RATE_HZ) as usize];
    let speech = synthesized(&SayRequest {
        voice,
        words_per_minute: None,
        text: words,
    })?
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
    for stretch in audio.chunks(SAMPLES_BETWEEN_WAITS) {
        for frame in stretch.chunks(FRAME_SAMPLES) {
            client.send_audio(frame)?;
        }
        events.extend(client.events_for_sent_audio()?);
    }
    client.close();
    Ok(events)
}

/// The voice commands the app recognizes in `events`.
pub fn recognized_commands(events: Vec<SpottingEvent>) -> Vec<VoiceCommand> {
    let mut recognizer = VoiceCommandRecognizer::new();
    events
        .into_iter()
        .flat_map(|e| recognizer.feed(e))
        .map(|spoken| spoken.command)
        .collect()
}

/// The phrases accepted after `command` was carried out, as the app switches them.
fn accepted_after(command: VoiceCommand) -> &'static [VoiceCommand] {
    match command {
        VoiceCommand::StartDictation | VoiceCommand::RestartDictation => &VoiceCommand::DICTATING,
        _ => &VoiceCommand::STANDBY,
    }
}

/// The commands the app carries out for `audio`, starting in the state that accepts
/// `commands`: like the app, it carries out a command only in a state that accepts it and
/// then listens for the phrases of the state the command leads to, from the next frame on.
pub fn carried_out_by_app(
    url: &str,
    audio: &[f32],
    commands: &'static [VoiceCommand],
) -> Result<Vec<VoiceCommand>> {
    let mut client = SpottingClient::connect(url, Instant::now())?;
    let mut accepted = commands;
    client.listen_for(accepted)?;
    let mut recognizer = VoiceCommandRecognizer::new();
    let mut carried_out = Vec::new();
    let mut hear = |client: &mut SpottingClient, events: Vec<SpottingEvent>| -> Result<()> {
        for spoken in events.into_iter().flat_map(|e| recognizer.feed(e)) {
            if accepted.contains(&spoken.command) {
                carried_out.push(spoken.command);
                accepted = accepted_after(spoken.command);
                client.listen_for(accepted)?;
            }
        }
        Ok(())
    };
    for frame in audio.chunks(FRAME_SAMPLES) {
        client.send_audio(frame)?;
        let events = client.events_for_sent_audio()?;
        hear(&mut client, events)?;
    }
    client.close();
    Ok(carried_out)
}
