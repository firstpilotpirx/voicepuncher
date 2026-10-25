//! The audio of a hands-free session: recordings, room noise and command audio.

use super::{
    env, read_wav, spoken_by, CommandVoice, Expected, PathBuf, Result, Step, CANCELLED_EN,
    CLEARED_EN, COMMAND_PAUSE_SECS, ENGLISH_VOICE, RUSSIAN_VOICE, SAMPLE_RATE_HZ, SENT_AGAIN_RU,
    SENT_RU,
};
use voicepuncher_e2e_test::conversions::saturating_usize;

/// Send, cancel, clear, send again — each with and without "please".
pub(super) fn session() -> Vec<Step> {
    use Expected::{Cancelled, Cleared, Pasted, RecordingStarted, ReturnPressed};
    vec![
        Step::Command("start dictation", RecordingStarted),
        Step::Dictate(RUSSIAN_VOICE, SENT_RU),
        Step::Command("stop dictation", Pasted(SENT_RU)),
        Step::Command("press enter", ReturnPressed),
        Step::Command("start dictation", RecordingStarted),
        Step::Dictate(ENGLISH_VOICE, CANCELLED_EN),
        Step::Command("cancel dictation", Cancelled),
        Step::Command("start dictation please", RecordingStarted),
        Step::Dictate(ENGLISH_VOICE, CLEARED_EN),
        Step::Command("stop dictation please", Pasted(CLEARED_EN)),
        Step::Command("clear text", Cleared),
        Step::Command("start dictation", RecordingStarted),
        Step::Dictate(RUSSIAN_VOICE, SENT_AGAIN_RU),
        Step::Command("stop dictation", Pasted(SENT_AGAIN_RU)),
        Step::Command("press enter please", ReturnPressed),
    ]
}

/// The user's recording of `said`, among the labelled recordings of the first session.
pub(super) fn recorded(said: &str) -> Result<Vec<f32>> {
    let (_, file) = RECORDINGS
        .iter()
        .find(|(phrase, _)| *phrase == said)
        .ok_or_else(|| anyhow::anyhow!("no recording of {said:?}"))?;
    let dir = PathBuf::from(env("VOICEPUNCHER_VOICE_SAMPLES")?).join("session-1");
    Ok(to_f32(read_wav(&dir.join(file))?))
}

/// The recording of the first session that holds each phrase.
const RECORDINGS: [(&str, &str); 8] = [
    ("start dictation", "02-start-dictation.wav"),
    ("start dictation please", "05-start-dictation.wav"),
    ("stop dictation", "10-stop-dictation.wav"),
    ("stop dictation please", "13-stop-dictation.wav"),
    ("cancel dictation", "18-cancel-dictation.wav"),
    ("press enter", "26-press-enter.wav"),
    ("press enter please", "25-press-enter.wav"),
    ("clear text", "38-clear-text.wav"),
];

pub(super) fn room_noise(secs: f32) -> Result<Vec<f32>> {
    let path = PathBuf::from(env("VOICEPUNCHER_VOICE_SAMPLES")?).join("session-1.wav");
    let room = to_f32(read_wav(&path)?);
    let quiet =
        &room[saturating_usize(0.5 * SAMPLE_RATE_HZ)..saturating_usize(4.5 * SAMPLE_RATE_HZ)];
    Ok(quiet
        .iter()
        .copied()
        .cycle()
        .take(saturating_usize(secs * SAMPLE_RATE_HZ))
        .collect())
}

pub(super) fn to_f32(samples: Vec<i16>) -> Vec<f32> {
    samples
        .into_iter()
        .map(|s| f32::from(s) / f32::from(i16::MAX))
        .collect()
}

pub(super) fn command_audio(voice: CommandVoice, said: &str) -> Result<Vec<f32>> {
    match voice {
        CommandVoice::Synthesized => spoken_by(ENGLISH_VOICE, said),
        CommandVoice::Recorded { .. } => {
            let mut audio = room_noise(COMMAND_PAUSE_SECS)?;
            audio.extend(recorded(said)?);
            audio.extend(room_noise(COMMAND_PAUSE_SECS)?);
            Ok(audio)
        }
    }
}
