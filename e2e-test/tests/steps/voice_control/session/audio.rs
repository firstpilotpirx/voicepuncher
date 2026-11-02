//! The audio of a hands-free session: recordings, room noise and command audio.

use super::{
    env, read_wav, recorded_form, spoken_by, CommandVoice, Expected, PathBuf, Result, Step,
    CANCELLED_EN, CLEARED_EN, COMMAND_PAUSE_SECS, ENGLISH_VOICE, RUSSIAN_VOICE, SAMPLE_RATE_HZ,
    SENT_AGAIN_RU, SENT_RU,
};
use voicepuncher_e2e_test::conversions::saturating_usize;

/// Send, cancel, clear, send again.
pub(super) fn session() -> Vec<Step> {
    use Expected::{Cancelled, Cleared, Pasted, RecordingStarted, ReturnPressed};
    vec![
        Step::Command("start", RecordingStarted),
        Step::Dictate(RUSSIAN_VOICE, SENT_RU),
        Step::Command("stop", Pasted(SENT_RU)),
        Step::Command("enter", ReturnPressed),
        Step::Command("start", RecordingStarted),
        Step::Dictate(ENGLISH_VOICE, CANCELLED_EN),
        Step::Command("cancel", Cancelled),
        Step::Command("start", RecordingStarted),
        Step::Dictate(ENGLISH_VOICE, CLEARED_EN),
        Step::Command("stop", Pasted(CLEARED_EN)),
        Step::Command("clear", Cleared),
        Step::Command("start", RecordingStarted),
        Step::Dictate(RUSSIAN_VOICE, SENT_AGAIN_RU),
        Step::Command("stop", Pasted(SENT_AGAIN_RU)),
        Step::Command("enter", ReturnPressed),
    ]
}

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
            audio.extend(recorded_form(said)?);
            audio.extend(room_noise(COMMAND_PAUSE_SECS)?);
            Ok(audio)
        }
    }
}
