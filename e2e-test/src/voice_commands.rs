//! Voice control phrases spoken by macOS `say` and streamed to the command spotting service
//! as the app streams the microphone while listening.

use std::time::{Duration, Instant};

use anyhow::Result;

use voicepuncher::adapters::secondary::commandspotting::websocket::client::SpottingClient;
use voicepuncher::core::domain::{SpottingEvent, VoiceCommand, VoiceCommandRecognizer};

use crate::conversions::saturating_usize;
use crate::speech_cache::{synthesized, SayRequest};

const SAMPLE_RATE_HZ: f32 = 16_000.0;
/// The silence before and after the words: the pause around a command.
const PAUSE_SECS: f32 = 1.0;
/// Quieter samples are silence.
const SILENCE_LEVEL: f32 = 0.01;
/// 100 ms of audio per message, as the app sends it.
const FRAME_SAMPLES: usize = 1_600;
/// The loudest 10 ms of speech in a quiet voice: the user's quietest recorded commands reach
/// 0.0045, `say` about 0.24.
const QUIET_LEVEL: f32 = 0.005;
/// 10 ms at 16 kHz: the stretch whose loudness is measured.
const LOUDNESS_FRAME_SAMPLES: usize = 160;
/// 2 s of audio: sent before waiting for the service to answer it, so the audio in flight
/// stays far below what the socket holds and a send never blocks on a busy service.
const SAMPLES_BETWEEN_WAITS: usize = 20 * FRAME_SAMPLES;
/// How long the real command spotting service may take to answer a sync: more than 5 times
/// its worst normal answer (confirmations give up after 3 s, plus 2 s of audio).
pub const SYNC_ANSWER_LIMIT: Duration = Duration::from_secs(20);

/// English `words` spoken by `say`, 16 kHz mono, with a pause of silence before and after.
pub fn spoken_with_pauses(words: &str) -> Result<Vec<f32>> {
    spoken(None, words)
}

/// `words` spoken by the `say` voice `voice` ("Milena" for Russian), with pauses around them.
pub fn spoken_by(voice: &str, words: &str) -> Result<Vec<f32>> {
    spoken(Some(voice), words)
}

fn spoken(voice: Option<&str>, words: &str) -> Result<Vec<f32>> {
    let pause = vec![0.0f32; saturating_usize(PAUSE_SECS * SAMPLE_RATE_HZ)];
    let speech = synthesized(&SayRequest { voice, words_per_minute: None, text: words })?
        .into_iter()
        .map(|s| f32::from(s) / f32::from(i16::MAX));
    Ok(pause
        .iter()
        .copied()
        .chain(speech)
        .chain(pause.iter().copied())
        .collect())
}

/// `spoken_by` speech without the pause it puts before the words; the short silence `say`
/// starts with stays.
pub fn without_lead_pause(mut audio: Vec<f32>) -> Vec<f32> {
    audio.split_off(saturating_usize(PAUSE_SECS * SAMPLE_RATE_HZ).min(audio.len()))
}

/// `say` speech from its first sound: without the pause `spoken_by` puts before it and the
/// silence `say` starts with, so a pause built before it is the pause it names.
pub fn from_first_sound(mut audio: Vec<f32>) -> Vec<f32> {
    let first_sound = audio.iter().position(|s| s.abs() > SILENCE_LEVEL);
    audio.split_off(first_sound.unwrap_or(audio.len()))
}

/// `say` speech cut right after its last sound: `say` ends a phrase with about 0.4 s of
/// silence of its own, which would lengthen any pause built after it.
pub fn without_trailing_silence(mut audio: Vec<f32>) -> Vec<f32> {
    let last_sound = audio.iter().rposition(|s| s.abs() > SILENCE_LEVEL);
    audio.truncate(last_sound.map_or(0, |at| at + 1));
    audio
}

/// `audio` as quiet as the user's quietest recorded commands: scaled so its loudest 10 ms
/// has an RMS of 0.005.
pub fn at_quiet_level(audio: Vec<f32>) -> Vec<f32> {
    let loudest = audio
        .chunks(LOUDNESS_FRAME_SAMPLES)
        .map(|frame| (frame.iter().map(|s| s * s).sum::<f32>() / frame.len() as f32).sqrt())
        .fold(0.0_f32, f32::max);
    if loudest == 0.0 {
        return audio;
    }
    let gain = QUIET_LEVEL / loudest;
    audio.into_iter().map(|s| s * gain).collect()
}

/// `secs` of silence at 16 kHz.
pub fn silence(secs: f32) -> Vec<f32> {
    vec![0.0f32; saturating_usize(secs * SAMPLE_RATE_HZ)]
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
        events.extend(client.events_for_sent_audio(SYNC_ANSWER_LIMIT)?);
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
/// `commands`.
///
/// Like the app, it carries out a command only in a state that accepts it and then listens for the
/// phrases of the state the command leads to, from the next frame on.
pub fn carried_out_by_app(
    url: &str,
    audio: &[f32],
    commands: &'static [VoiceCommand],
) -> Result<Vec<VoiceCommand>> {
    let mut client = SpottingClient::connect(url, Instant::now())?;
    client.listen_for(commands)?;
    let mut app = AppStates {
        accepted: commands,
        recognizer: VoiceCommandRecognizer::new(),
        carried_out: Vec::new(),
    };
    for frame in audio.chunks(FRAME_SAMPLES) {
        client.send_audio(frame)?;
        let events = client.events_for_sent_audio(SYNC_ANSWER_LIMIT)?;
        app.hear(&mut client, events)?;
    }
    client.close();
    Ok(app.carried_out)
}

/// The commands the app accepts as it carries them out.
struct AppStates {
    accepted: &'static [VoiceCommand],
    recognizer: VoiceCommandRecognizer,
    carried_out: Vec<VoiceCommand>,
}

impl AppStates {
    /// Carries out the commands `events` make that the state accepts, each moving the client
    /// on to the phrases of the state it leads to.
    fn hear(&mut self, client: &mut SpottingClient, events: Vec<SpottingEvent>) -> Result<()> {
        for spoken in events.into_iter().flat_map(|e| self.recognizer.feed(e)) {
            if self.accepted.contains(&spoken.command) {
                self.carried_out.push(spoken.command);
                self.accepted = accepted_after(spoken.command);
                client.listen_for(self.accepted)?;
            }
        }
        Ok(())
    }
}
