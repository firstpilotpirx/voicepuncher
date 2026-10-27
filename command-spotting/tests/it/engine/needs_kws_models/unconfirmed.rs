//! Commands reported without confirmation: the keyword spotter decides at once, and what the
//! speech recognition service heard follows as a verdict.

use super::*;
use voicepuncher_command_spotting::protocol::{PhraseSelection, Verdict};

/// A session of `engine` listening for `phrases` without confirmation.
fn unconfirmed<'e>(engine: &'e Engine, phrases: &[&str]) -> Session<'e> {
    let mut session = engine.session().unwrap();
    let phrases = phrases.iter().map(|p| (*p).to_owned()).collect();
    let selection = PhraseSelection { phrases, confirm: Some(false), ..PhraseSelection::default() };
    session.select(&selection).unwrap();
    session
}

/// Hears the same text in every clip.
struct HearsAlways(&'static str);

impl Hear for HearsAlways {
    fn hear(&self, _: &[f32], _: Duration) -> anyhow::Result<Heard> {
        Ok(Heard { text: self.0.to_owned(), words: Vec::new() })
    }
}

/// Every verdict in `events`, with the commands it is about.
fn verdicts(events: &[ServerEvent]) -> Vec<(Vec<String>, Verdict)> {
    events
        .iter()
        .filter_map(|e| match e {
            ServerEvent::Heard { commands, verdict, .. } => Some((commands.clone(), *verdict)),
            _ => None,
        })
        .collect()
}

/// Where the first unconfirmed report of `command` and the first verdict lie in `events`.
fn report_and_verdict(events: &[ServerEvent], command: &str) -> (Option<usize>, Option<usize>) {
    let reported = events.iter().position(|e| match e {
        ServerEvent::Phrase { phrase, timing: None, .. } => phrase == command,
        ServerEvent::Command { commands, timing: None, .. } => commands == &[command],
        _ => false,
    });
    let verdict = events
        .iter()
        .position(|e| matches!(e, ServerEvent::Heard { .. }));
    (reported, verdict)
}

#[test]
fn should_report_a_spotted_command_before_its_hearing_without_confirmation() {
    let _shared = engine();
    // Given
    let engine = own_engine(None).hearing(Arc::new(HearsAlways(" press enter")));
    let mut session = unconfirmed(&engine, &ALL_PHRASES);
    // When
    let events = fed(&mut session, &spoken("press enter"));
    // Then
    let (reported, verdict) = report_and_verdict(&events, "press enter");
    assert!(
        reported.is_some() && verdict.is_some() && reported < verdict,
        "{events:?}"
    );
    assert_eq!(
        verdicts(&events),
        [(vec!["press enter".to_owned()], Verdict::Agreed)]
    );
}

#[test]
fn should_keep_a_spotted_command_the_hearing_does_not_confirm_and_say_so() {
    let _shared = engine();
    // Given
    let engine = own_engine(None).hearing(Arc::new(HearsAlways(" hello there")));
    let mut session = unconfirmed(&engine, &ALL_PHRASES);
    // When
    let events = fed(&mut session, &spoken("press enter"));
    // Then
    assert_eq!(reported_phrases(&events), ["press enter"], "{events:?}");
    assert_eq!(
        verdicts(&events),
        [(vec!["press enter".to_owned()], Verdict::None)]
    );
}

/// How long after the end of "start start" said alone it may be reported without
/// confirmation: well before the 0.3 s of silence that would end its utterance, and within
/// the 0.1 s frame the stream is fed in.
const UNCONFIRMED_START_WITHIN_SECS: f32 = 0.3;

#[test]
fn should_report_start_start_said_alone_once_its_utterance_ends_without_confirmation() {
    let _shared = engine();
    // Given "start start" said alone
    let hear = HearsTheWordsInTheClip::said(&["start", "start"], 0.12);
    let engine = own_engine(None).hearing(hear.clone());
    let mut session = unconfirmed(&engine, &STANDBY_PHRASES);
    // When
    let (events, reported_at) = fed_until_reported(&mut session, &hear.stream, "start dictation");
    // Then
    let late = reported_at.map(|at| (at - hear.words[1].1.end) as f32 / SAMPLE_RATE as f32);
    assert!(
        late.is_some_and(|late| late <= UNCONFIRMED_START_WITHIN_SECS),
        "start start reported {late:?} s after its end: {events:?}"
    );
    assert_eq!(
        verdicts(&events),
        [(vec!["start dictation".to_owned()], Verdict::Agreed)],
        "{events:?}"
    );
}

/// Every event of `audio` fed frame by frame, each confirmation answered before the next
/// frame, and the stream position at which `command` was first reported unconfirmed.
fn fed_until_reported(
    session: &mut Session<'_>,
    audio: &[f32],
    command: &str,
) -> (Vec<ServerEvent>, Option<usize>) {
    let (mut events, mut reported_at, mut fed_so_far) = (Vec::new(), None, 0);
    for frame in audio.chunks(FRAME_SAMPLES) {
        let mut got = session.accept(frame);
        got.extend(session.settle());
        fed_so_far += frame.len();
        if reported_at.is_none() && report_and_verdict(&got, command).0.is_some() {
            reported_at = Some(fed_so_far);
        }
        events.extend(got);
    }
    events.extend(session.settle());
    (events, reported_at)
}
