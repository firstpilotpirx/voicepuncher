//! A closed utterance goes into the command history, also when the connection ends while
//! its recognizers still decode the silence after it.

use super::*;

fn history_in(dir: &std::path::Path) -> CommandHistory {
    let cfg = HistoryCfg { dir: dir.to_owned(), max_size_bytes: 1 << 30 };
    CommandHistory::start(cfg).expect("a history")
}

fn entries_in(dir: &std::path::Path) -> usize {
    std::fs::read_dir(dir).expect("the history folder").count()
}

#[test]
fn should_keep_an_utterance_with_a_reported_command_in_the_history() {
    // Given
    let dir = tempfile::tempdir().expect("a temporary folder");
    let history = history_in(dir.path());
    let en = Scripted::new("en", &[("press", 0.6), ("enter", 0.9)]);
    let mut session = session(&[&en], Some(&history));
    // When
    session.accept(&[silence(1.0), speech(0.8), silence(1.0)].concat());
    session.sync(0);
    history.flush();
    // Then
    assert_eq!(entries_in(dir.path()), 1);
}

#[test]
fn should_keep_an_utterance_still_decoding_when_the_session_ends() {
    // Given a history and an English recognizer that takes 0.1 s per decode
    let dir = tempfile::tempdir().expect("a temporary folder");
    let history = history_in(dir.path());
    let en = Scripted::new("en", &[("stop", 0.6)]).slow(Duration::from_millis(100));
    let mut session = session(&[&en], Some(&history));
    // When the connection ends right after the utterance closed, its tail still decoding
    session.accept(&[silence(1.0), speech(0.3), silence(1.0)].concat());
    drop(session);
    history.flush();
    // Then
    assert_eq!(entries_in(dir.path()), 1);
}
