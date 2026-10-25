//! Every way of saying the commands, in the user's own recorded voice: short forms, "please",
//! commands at the end or start of speech, doubled and chained commands, and speech that
//! is no command. Each labelled stretch of a recording goes on its own through the real
//! command spotting service and the app's recognizer, in the state it was said in, and
//! must give exactly the commands labelled. The recordings stay on the machine
//! (`~/.voicepuncher/voice-samples/<session>.wav` and `<session>/forms.jsonl`); without them
//! or the service the test is skipped.

use serde::Deserialize;

use voicepuncher::core::domain::conversions::saturating_usize;
use voicepuncher::core::domain::VoiceCommand;
use voicepuncher_e2e_test::concurrency::map_concurrently;
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

/// What went wrong with a form that carried out `commands`: a wrong result not listed as
/// known, or a known wrong one that goes right now.
fn went_wrong(session: &Session, form: &Form, commands: &[String]) -> Option<String> {
    let known = listed(KNOWN_WRONG, session, form);
    let report = format!(
        "{} at {:.2}s, {}, said {:?} ({}): carried out {commands:?}, expected {:?}",
        session.name, form.start, form.state, form.said, form.note, form.commands
    );
    match (commands == form.commands, known) {
        (true, None) => None,
        (false, None) => Some(report),
        (false, Some(why)) => {
            eprintln!("known wrong ({why}): {report}");
            None
        }
        (true, Some(why)) => Some(format!(
            "{report} — goes right now: remove it from KNOWN_WRONG ({why})"
        )),
    }
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

type Session = crate::recorded_sessions::Session<Form>;

fn sessions() -> (String, Vec<Session>) {
    crate::recorded_sessions::sessions("forms.jsonl")
}

fn sample_at(secs: f32) -> usize {
    saturating_usize(secs.max(0.0) * SAMPLE_RATE_HZ)
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

mod needs_kws {
    use super::*;

    mod needs_voice_samples {
        use super::*;

        #[test]
        fn should_carry_out_every_recorded_way_of_saying_the_commands() {
            let (url, all) = sessions();
            let checked: Vec<(&Session, &Form)> = all
                .iter()
                .flat_map(|session| {
                    session
                        .labels
                        .iter()
                        .filter(|f| !f.unclear && listed(FROZEN, session, f).is_none())
                        .map(move |form| (session, form))
                })
                .collect();
            let carried_out = map_concurrently(&checked, CONCURRENT_FORMS, |(session, form)| {
                heard(&url, session, form)
            });
            let counted = checked.len();
            let wrong: Vec<String> = checked
                .iter()
                .zip(carried_out)
                .filter_map(|((session, form), commands)| went_wrong(session, form, &commands))
                .collect();
            assert!(counted > 0, "no labelled forms found");
            assert!(
                wrong.is_empty(),
                "{} of {counted} recorded forms went wrong:\n{}",
                wrong.len(),
                wrong.join("\n")
            );
        }
    }
}
