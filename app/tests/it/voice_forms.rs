//! Every way of saying the commands, in the user's own recorded voice: short forms, "please",
//! commands at the end or start of speech, doubled and chained commands, and speech that
//! is no command. Each labelled stretch of a recording goes on its own through the real
//! command spotting service and the app's recognizer, in the state it was said in, and
//! must give exactly the commands labelled. The recordings stay on the machine
//! (`~/.voicepuncher/voice-samples/<session>.wav` and `<session>/forms.jsonl`); without them
//! or the service the test is skipped.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use voicepuncher::core::domain::VoiceCommand;
use voicepuncher_e2e_test::concurrency::map_concurrently;
use voicepuncher_e2e_test::speech::read_wav;
use voicepuncher_e2e_test::voice_commands::carried_out_by_app;

const SAMPLE_RATE_HZ: f32 = 16_000.0;
/// Recording kept around a labelled stretch, so its first and last sounds are whole.
const MARGIN_SECS: f32 = 0.3;
/// Silence before a stretch, and after it so its utterance ends.
const LEAD_SILENCE_SECS: f32 = 1.0;
const TRAIL_SILENCE_SECS: f32 = 1.5;
/// Forms checked at once, each on its own connection to the service.
const CONCURRENT_FORMS: usize = 4;
/// Forms known to go wrong, by session and start, until the change named beside each fixes
/// them: still heard and reported, not counted. One that goes right fails the test, so the
/// entry is removed with the fix.
const KNOWN_WRONG: &[(&str, f32, &str)] = &[];
/// Forms that flake under machine load, by session and start: not checked until fixed separately.
const FROZEN: &[(&str, f32, &str)] = &[
    ("session-4", 32.41, "flaky, 2026-10-23"),
    (
        "session-5",
        8.74,
        "flaky beside another full-check, 2026-10-23",
    ),
];

fn listed<'a>(list: &[(&str, f32, &'a str)], session: &Session, form: &Form) -> Option<&'a str> {
    let at = |e: &&(&str, f32, &str)| e.0 == session.name && (e.1 - form.start).abs() < 0.01;
    list.iter().find(at).map(|e| e.2)
}

#[derive(Debug, Deserialize)]
struct Form {
    start: f32,
    end: f32,
    state: String,
    said: String,
    commands: Vec<String>,
    note: String,
    /// No listener could tell what was said: the stretch is kept but not checked.
    #[serde(default)]
    unclear: bool,
}

struct Session {
    name: String,
    audio: Vec<f32>,
    forms: Vec<Form>,
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
        .filter(|p| p.join("forms.jsonl").is_file() && p.with_extension("wav").is_file())
        .collect();
    dirs.sort();
    Some((url, dirs.iter().map(|dir| load(dir)).collect()))
}

fn load(dir: &Path) -> Session {
    let forms = std::fs::read_to_string(dir.join("forms.jsonl"))
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let audio = read_wav(&dir.with_extension("wav"))
        .unwrap()
        .into_iter()
        .map(|s| f32::from(s) / f32::from(i16::MAX))
        .collect();
    Session { name: dir.file_name().unwrap().to_string_lossy().into_owned(), audio, forms }
}

fn sample_at(secs: f32) -> usize {
    (secs.max(0.0) * SAMPLE_RATE_HZ) as usize
}

fn silence(secs: f32) -> Vec<f32> {
    vec![0.0; sample_at(secs)]
}

fn listened_for(state: &str) -> &'static [VoiceCommand] {
    match state {
        "dictating" => &VoiceCommand::DICTATING,
        "standby" => &VoiceCommand::STANDBY,
        other => panic!("unknown state {other:?}"),
    }
}

/// The commands the app carries out for `form`, said on its own.
fn heard(url: &str, session: &Session, form: &Form) -> Vec<String> {
    let from = sample_at(form.start - MARGIN_SECS);
    let to = sample_at(form.end + MARGIN_SECS).min(session.audio.len());
    let stream: Vec<f32> = silence(LEAD_SILENCE_SECS)
        .into_iter()
        .chain(session.audio[from..to].iter().copied())
        .chain(silence(TRAIL_SILENCE_SECS))
        .collect();
    carried_out_by_app(url, &stream, listened_for(&form.state))
        .unwrap()
        .into_iter()
        .map(|command| command.phrase().to_owned())
        .collect()
}

#[test]
fn should_carry_out_every_recorded_way_of_saying_the_commands() {
    let Some((url, all)) = sessions() else { return };
    let checked: Vec<(&Session, &Form)> = all
        .iter()
        .flat_map(|session| {
            session
                .forms
                .iter()
                .filter(|f| !f.unclear && listed(FROZEN, session, f).is_none())
                .map(move |form| (session, form))
        })
        .collect();
    let carried_out = map_concurrently(&checked, CONCURRENT_FORMS, |(session, form)| {
        heard(&url, session, form)
    });
    let counted = checked.len();
    let mut wrong = Vec::new();
    for ((session, form), commands) in checked.iter().zip(carried_out) {
        let known = listed(KNOWN_WRONG, session, form);
        let report = format!(
            "{} at {:.2}s, {}, said {:?} ({}): carried out {commands:?}, expected {:?}",
            session.name, form.start, form.state, form.said, form.note, form.commands
        );
        match (commands == form.commands, known) {
            (true, None) => {}
            (false, None) => wrong.push(report),
            (false, Some(why)) => eprintln!("known wrong ({why}): {report}"),
            (true, Some(why)) => wrong.push(format!(
                "{report} — goes right now: remove it from KNOWN_WRONG ({why})"
            )),
        }
    }
    assert!(counted > 0, "no labelled forms found");
    assert!(
        wrong.is_empty(),
        "{} of {counted} recorded forms went wrong:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}
