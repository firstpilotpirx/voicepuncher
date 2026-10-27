use super::*;
use crate::history::Outcome;
use crate::protocol::Timing;

fn segment(start: f32, end: f32) -> Vec<ServerEvent> {
    vec![
        ServerEvent::SpeechStart { t: start },
        ServerEvent::SpeechEnd { t: end },
    ]
}

fn heard(phrase: &str, start: f32, end: f32) -> ServerEvent {
    let timing = Some(Timing { asked_at: 2.0, took_ms: 150 });
    ServerEvent::Phrase { phrase: phrase.to_owned(), start, end, timing }
}

fn audio() -> RecentAudio {
    let mut audio = RecentAudio::new(sample_at(30.0));
    audio.push(&vec![0.1; sample_at(10.0)]);
    audio
}

#[test]
fn should_keep_a_spotted_and_reported_phrase_as_agreed_once_its_segment_is_due() {
    let mut recorder = Recorder::new(1);
    recorder.spotted("press enter", "▁PRESS ▁EN TER", 1.2, 1.9);
    recorder.asked(0, "segment", 1.0, 2.0);
    recorder.answered(0, Some(" Press enter."));
    recorder.went_out(&[segment(1.0, 2.0), vec![heard("press enter", 1.2, 1.9)]].concat());

    assert!(recorder.due(2.5, false, &audio()).is_empty());
    let entries = recorder.due(3.0, false, &audio());

    let [entry] = entries.as_slice() else {
        panic!("one entry: {entries:?}")
    };
    assert_eq!(entry.outcome, Outcome::Agreed);
    assert_eq!(entry.transcript, "Press enter.");
    assert_eq!(entry.audio.len(), sample_at(1.6));
    assert!((entry.spotted[0].start - 0.5).abs() < 1e-4);
}

#[test]
fn should_keep_no_entry_of_a_segment_in_which_nothing_was_spotted_or_reported() {
    let mut recorder = Recorder::new(1);
    recorder.asked(0, "segment", 1.0, 2.0);
    recorder.answered(0, Some(" Good morning."));
    recorder.went_out(&segment(1.0, 2.0));

    assert!(recorder.due(5.0, false, &audio()).is_empty());
}

#[test]
fn should_wait_for_a_hearing_of_the_segment_still_asked() {
    let mut recorder = Recorder::new(1);
    recorder.spotted("press enter", "▁PRESS ▁EN TER", 1.2, 1.9);
    recorder.asked(0, "segment", 1.0, 2.0);
    recorder.went_out(&segment(1.0, 2.0));

    assert!(recorder.due(5.0, false, &audio()).is_empty());
    let entries = recorder.due(5.0, true, &audio());

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].outcome, Outcome::Rejected);
    assert_eq!(entries[0].hearings[0].text, None);
}

#[test]
fn should_keep_a_spotted_phrase_reported_without_a_hearing_of_it_as_rejected() {
    let mut recorder = Recorder::new(1);
    recorder.spotted("press enter", "▁PRESS ▁EN TER", 1.2, 1.9);
    let unheard = ServerEvent::Phrase {
        phrase: "press enter".to_owned(),
        start: 1.2,
        end: 1.9,
        timing: None,
    };
    recorder.went_out(&[segment(1.0, 2.0), vec![unheard]].concat());

    let entries = recorder.due(3.0, false, &audio());

    assert_eq!(entries[0].outcome, Outcome::Rejected);
    assert!(!entries[0].reported[0].heard);
}
