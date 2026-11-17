//! The second look at the last second of an utterance's speech: a fast "enter" glued to the
//! last word that the running decode missed is reported after the close.

use super::*;

/// The end of the command and when it went out, in seconds of the stream.
fn command_end(events: &[ServerEvent]) -> Option<f32> {
    events.iter().find_map(|e| match e {
        ServerEvent::Command { end, .. } => Some(*end),
        _ => None,
    })
}

#[test]
fn should_report_an_enter_only_the_second_look_hears() {
    // Given speech from 1.0 s to 2.5 s whose running decode hears no command, and whose last
    // second, decoded alone, ends in "enter" 0.7 s into the window
    let ru = Scripted::new("ru", &[("там", 0.6), ("отмечать", 1.2)])
        .looking_again(&[("отмечать", 0.2), ("энтер", 0.7)]);
    // When
    let events = utterance_events(&[&ru], 1.5);
    // Then
    assert_eq!(commands(&events), [["press enter"]], "{events:?}");
    let end = command_end(&events).expect("a command");
    assert!((2.0..=2.6).contains(&end), "{end}");
}

#[test]
fn should_report_the_second_look_within_its_bound_after_the_speech() {
    // Given a recognizer that takes 5 ms per decode, as fast as the real ones keep up
    let en = Scripted::new("en", &[("okay", 0.6)])
        .looking_again(&[("okay", 0.3), ("enter", 0.7)])
        .slow(Duration::from_millis(5));
    let mut session = session(&[&en], None);
    let began = Instant::now();
    // When the speech ends and the pause after it streams in real time
    let mut events = session.accept(&[silence(1.0), speech(1.0)].concat());
    let spoken = Instant::now();
    for frame in silence(2.0).chunks(320) {
        events.extend(session.accept(frame));
        events.extend(session.poll());
        if !commands(&events).is_empty() {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    // Then
    assert_eq!(commands(&events), [["press enter"]], "{events:?}");
    assert!(
        spoken.elapsed() < Duration::from_millis(1500),
        "{:?} after {:?}",
        spoken.elapsed(),
        began.elapsed()
    );
}

#[test]
fn should_not_look_again_after_a_reported_command() {
    // Given "okay, enter" the running decode hears, and a second look that would hear it too
    let en = Scripted::new("en", &[("okay", 0.6), ("enter", 0.9)]).looking_again(&[("enter", 0.5)]);
    // When
    let events = utterance_events(&[&en], 0.8);
    // Then
    assert_eq!(commands(&events), [["press enter"]], "{events:?}");
    assert_eq!(en.streams.load(Ordering::SeqCst), 1);
}

#[test]
fn should_report_nothing_when_the_second_look_hears_no_command() {
    // Given
    let ru = Scripted::new("ru", &[("это", 0.6), ("интересно", 1.0)])
        .looking_again(&[("интересно", 0.4)]);
    // When
    let events = utterance_events(&[&ru], 1.2);
    // Then
    assert_eq!(commands(&events), Vec::<Vec<String>>::new(), "{events:?}");
}

#[test]
fn should_report_only_enter_from_the_second_look() {
    // Given a second look that hears "stop" ending the speech
    let en = Scripted::new("en", &[("we", 0.6), ("go", 0.9)]).looking_again(&[("stop", 0.7)]);
    // When
    let events = utterance_events(&[&en], 1.2);
    // Then
    assert_eq!(commands(&events), Vec::<Vec<String>>::new(), "{events:?}");
}

#[test]
fn should_keep_the_second_look_in_the_history_entry() {
    // Given
    let dir = tempfile::tempdir().expect("a temporary folder");
    let cfg = HistoryCfg { dir: dir.path().to_owned(), max_size_bytes: 1 << 30 };
    let history = CommandHistory::start(cfg).expect("a history");
    let ru = Scripted::new("ru", &[("там", 0.6)]).looking_again(&[("энтер", 0.5)]);
    let mut session = session(&[&ru], Some(&history));
    // When
    session.accept(&[silence(1.0), speech(1.0), silence(2.0)].concat());
    session.sync(0);
    drop(session);
    history.flush();
    // Then
    let entry = std::fs::read_dir(dir.path())
        .expect("the history folder")
        .next()
        .expect("an entry")
        .expect("an entry")
        .path()
        .join(crate::history::ENTRY_FILE);
    let text = std::fs::read_to_string(entry).expect("the entry");
    assert!(text.contains("ru second look"), "{text}");
    let entry: serde_json::Value = serde_json::from_str(&text).expect("JSON");
    assert_eq!(entry["outcome"], "agreed", "{text}");
}
