//! The user's own recorded voice commands, heard as one continuous stream through the real
//! command spotting service and the app's recognizer, the phrase set switched before each
//! utterance as the app switches it with its state. The recordings stay on the machine
//! (`~/.voicepuncher/voice-samples/<session>.wav` and `<session>/manifest.jsonl`); the justfile
//! sets `VOICEPUNCHER_VOICE_SAMPLES` when they exist and `VOICEPUNCHER_COMMAND_SPOTTING_URL`
//! when the service runs. Without either the test is skipped.

use std::path::{Path, PathBuf};
use std::time::Instant;

use serde::Deserialize;

use voicepuncher::adapters::secondary::commandspotting::websocket::client::SpottingClient;
use voicepuncher::core::domain::{SpokenCommand, VoiceCommand, VoiceCommandRecognizer};
use voicepuncher_e2e_test::concurrency::map_concurrently;
use voicepuncher_e2e_test::speech::read_wav;
use voicepuncher_e2e_test::voice_commands::SYNC_ANSWER_LIMIT;

const SAMPLE_RATE_HZ: f32 = 16_000.0;
const FRAME_SAMPLES: usize = 1_600;
/// 2 s of audio: sent before waiting for the service to answer it, so the audio in flight
/// stays far below what the socket holds.
const SAMPLES_BETWEEN_WAITS: usize = 20 * FRAME_SAMPLES;
/// Sessions heard at once, each on its own connection to the service.
const CONCURRENT_SESSIONS: usize = 4;

#[derive(Debug, Deserialize)]
struct Utterance {
    file: String,
    start: f32,
    end: f32,
    said: String,
    command: String,
    note: String,
    #[serde(default)]
    unclear: bool,
}

/// A recorded session: the whole recording and its labelled utterances.
struct Session {
    name: String,
    audio: Vec<f32>,
    utterances: Vec<Utterance>,
}

fn sessions() -> Option<(String, Vec<Session>)> {
    let (Ok(url), Ok(root)) = (
        std::env::var("VOICEPUNCHER_COMMAND_SPOTTING_URL"),
        std::env::var("VOICEPUNCHER_VOICE_SAMPLES"),
    ) else {
        eprintln!(
            "GATED: VOICEPUNCHER_COMMAND_SPOTTING_URL or VOICEPUNCHER_VOICE_SAMPLES not set, skipping"
        );
        return None;
    };
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(&root)
        .ok()?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.join("manifest.jsonl").is_file() && p.with_extension("wav").is_file())
        .collect();
    dirs.sort();
    Some((url, dirs.iter().map(|dir| load(dir)).collect()))
}

fn load(dir: &Path) -> Session {
    let utterances = std::fs::read_to_string(dir.join("manifest.jsonl"))
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let audio = read_wav(&dir.with_extension("wav"))
        .unwrap()
        .into_iter()
        .map(|s| f32::from(s) / f32::from(i16::MAX))
        .collect();
    Session { name: dir.file_name().unwrap().to_string_lossy().into_owned(), audio, utterances }
}

/// The phrases the app listens for in the state where `command` is accepted.
fn listened_for(command: VoiceCommand) -> &'static [VoiceCommand] {
    if VoiceCommand::STANDBY.contains(&command) {
        &VoiceCommand::STANDBY
    } else {
        &VoiceCommand::DICTATING
    }
}

fn sample_at(secs: f32) -> usize {
    (secs * SAMPLE_RATE_HZ) as usize
}

/// Every command the app recognizes in the session, with its time in the recording.
fn recognized_in(url: &str, session: &Session) -> Vec<(f32, VoiceCommand)> {
    let audio_start = Instant::now();
    let mut client = SpottingClient::connect(url, audio_start).unwrap();
    let mut recognizer = VoiceCommandRecognizer::new();
    let mut recognized = Vec::new();
    let mut sent = 0;
    for (i, utterance) in session.utterances.iter().enumerate() {
        let command = VoiceCommand::from_phrase(&utterance.command).expect("known command");
        client.listen_for(listened_for(command)).unwrap();
        let until = session
            .utterances
            .get(i + 1)
            .map_or(session.audio.len(), |next| sample_at(next.start));
        for stretch in session.audio[sent..until.max(sent)].chunks(SAMPLES_BETWEEN_WAITS) {
            for frame in stretch.chunks(FRAME_SAMPLES) {
                client.send_audio(frame).unwrap();
            }
            for event in client.events_for_sent_audio(SYNC_ANSWER_LIMIT).unwrap() {
                recognized.extend(recognizer.feed(event));
            }
        }
        sent = until.max(sent);
    }
    client.close();
    recognized
        .into_iter()
        .map(|SpokenCommand { command, started_at, .. }| {
            (
                started_at.duration_since(audio_start).as_secs_f32(),
                command,
            )
        })
        .collect()
}

/// Repeats of one command closer than this are one utterance: the service ends an utterance
/// after 0.3 s of silence and its detector holds speech a little longer, so a shorter
/// pause does not separate them and they count as one command.
const REPEAT_PAUSE_SECS: f32 = 0.3;

/// Consecutive utterances of the same command said without a pause between them.
fn groups(utterances: &[Utterance]) -> Vec<Vec<&Utterance>> {
    let mut groups: Vec<Vec<&Utterance>> = Vec::new();
    for utterance in utterances {
        match groups.last_mut() {
            Some(group)
                if group.last().is_some_and(|last| {
                    last.command == utterance.command
                        && utterance.start - last.end < REPEAT_PAUSE_SECS
                }) =>
            {
                group.push(utterance)
            }
            _ => groups.push(vec![utterance]),
        }
    }
    groups
}

/// How far from its utterance a recognized command may be placed, in seconds.
const PLACEMENT_TOLERANCE_SECS: f32 = 1.0;

/// How far `at` is from the time `group` was said.
fn distance(group: &[&Utterance], at: f32) -> f32 {
    let (start, end) = (group[0].start, group[group.len() - 1].end);
    (start - at).max(at - end).max(0.0)
}

#[test]
#[expect(clippy::too_many_lines, reason = "size baseline")]
fn should_recognize_every_recorded_command_of_the_user() {
    let Some((url, sessions)) = sessions() else {
        return;
    };
    let mut misses = Vec::new();
    let mut extras = Vec::new();
    let mut counted = 0;
    let recognized = map_concurrently(&sessions, CONCURRENT_SESSIONS, |session| {
        recognized_in(&url, session)
    });
    for (session, recognized) in sessions.iter().zip(recognized) {
        let groups = groups(&session.utterances);
        let mut heard = vec![0usize; groups.len()];
        for (at, command) in recognized {
            let nearest = groups
                .iter()
                .enumerate()
                .filter(|(index, g)| {
                    VoiceCommand::from_phrase(&g[0].command) == Some(command)
                        && heard[*index] < g.len()
                        && distance(g, at) <= PLACEMENT_TOLERANCE_SECS
                })
                .min_by(|(_, a), (_, b)| distance(a, at).total_cmp(&distance(b, at)));
            match nearest {
                Some((index, _)) => heard[index] += 1,
                None => extras.push(format!("{}: {command:?} at {at:.2}s", session.name)),
            }
        }
        for (group, heard) in groups.iter().zip(heard) {
            if group.iter().all(|u| u.unclear) {
                continue;
            }
            counted += 1;
            if heard != 1 {
                misses.push(format!(
                    "{}/{} at {:.2}s said {:?} ({}) ×{}: recognized {heard}",
                    session.name,
                    group[0].file,
                    group[0].start,
                    group[0].said,
                    group[0].note,
                    group.len(),
                ));
            }
        }
    }
    assert!(
        counted > 0,
        "no recorded commands found in {sessions:?}",
        sessions = sessions.len()
    );
    assert!(
        misses.is_empty() && extras.is_empty(),
        "{} of {counted} recorded commands missed:\n{}\nunexpected commands:\n{}",
        misses.len(),
        misses.join("\n"),
        extras.join("\n")
    );
}
