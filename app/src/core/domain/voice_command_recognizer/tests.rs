use super::*;

fn at(base: Instant, secs: f32) -> Instant {
    base + Duration::from_secs_f32(secs)
}

fn phrase(base: Instant, text: &str, start: f32, end: f32) -> SpottingEvent {
    SpottingEvent::PhraseSpotted {
        phrase: text.into(),
        start: at(base, start),
        end: at(base, end),
        timing: None,
    }
}

fn feed_all(events: Vec<SpottingEvent>) -> Vec<SpokenCommand> {
    let mut recognizer = VoiceCommandRecognizer::new();
    events
        .into_iter()
        .flat_map(|e| recognizer.feed(e))
        .collect()
}

#[test]
fn should_recognize_a_phrase_that_fills_its_speech_segment() {
    // Given
    let base = Instant::now();
    let events = vec![
        SpottingEvent::SpeechStarted(at(base, 1.0)),
        phrase(base, "press enter", 1.1, 1.8),
        SpottingEvent::SpeechEnded(at(base, 1.9)),
    ];
    // When
    let commands = feed_all(events);
    // Then
    assert_eq!(
        commands,
        vec![SpokenCommand {
            command: VoiceCommand::PressEnter,
            started_at: at(base, 1.1),
            ends_at: at(base, 1.8),
            placement: CommandPlacement::Alone,
            timing: None,
        }]
    );
}

#[test]
fn should_recognize_a_phrase_reported_after_its_segment_ended() {
    // Given
    let base = Instant::now();
    let events = vec![
        SpottingEvent::SpeechStarted(at(base, 1.0)),
        SpottingEvent::SpeechEnded(at(base, 1.9)),
        phrase(base, "stop dictation", 1.1, 1.8),
    ];
    // When
    let commands = feed_all(events);
    // Then
    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].command, VoiceCommand::StopDictation);
}

#[test]
fn should_recognize_a_phrase_reported_after_its_segment_that_sticks_out_of_it() {
    // Given the spotter placed the phrase end 0.2 s after the speech ended
    let base = Instant::now();
    let events = vec![
        SpottingEvent::SpeechStarted(at(base, 1.0)),
        SpottingEvent::SpeechEnded(at(base, 1.9)),
        phrase(base, "press enter", 0.9, 2.1),
    ];
    // When
    let commands = feed_all(events);
    // Then
    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].command, VoiceCommand::PressEnter);
}

#[test]
fn should_not_take_a_phrase_that_began_after_the_last_segment_ended() {
    // Given
    let base = Instant::now();
    let events = vec![
        SpottingEvent::SpeechStarted(at(base, 1.0)),
        SpottingEvent::SpeechEnded(at(base, 1.9)),
        phrase(base, "press enter", 2.5, 3.2),
    ];
    // When
    let commands = feed_all(events);
    // Then
    assert!(commands.is_empty());
}

#[test]
fn should_ignore_a_phrase_with_speech_before_it() {
    // Given "I will press enter"
    let base = Instant::now();
    let events = vec![
        SpottingEvent::SpeechStarted(at(base, 1.0)),
        phrase(base, "press enter", 1.8, 2.5),
        SpottingEvent::SpeechEnded(at(base, 2.6)),
    ];
    // When
    let commands = feed_all(events);
    // Then
    assert!(commands.is_empty());
}

#[test]
fn should_ignore_a_phrase_with_speech_after_it() {
    // Given "press enter when I am done"
    let base = Instant::now();
    let events = vec![
        SpottingEvent::SpeechStarted(at(base, 1.0)),
        phrase(base, "press enter", 1.1, 1.8),
        SpottingEvent::SpeechEnded(at(base, 3.0)),
    ];
    // When
    let commands = feed_all(events);
    // Then
    assert!(commands.is_empty());
}

#[test]
fn should_take_two_spots_of_the_same_command_in_one_segment_once() {
    // Given the keyword spotter and the transcript both report "press enter"
    let base = Instant::now();
    let events = vec![
        SpottingEvent::SpeechStarted(at(base, 1.0)),
        phrase(base, "press enter", 1.3, 1.8),
        phrase(base, "press enter", 1.0, 1.9),
        SpottingEvent::SpeechEnded(at(base, 1.9)),
    ];
    // When
    let commands = feed_all(events);
    // Then
    assert_eq!(commands.len(), 1, "{commands:?}");
    assert_eq!(commands[0].command, VoiceCommand::PressEnter);
}

#[test]
fn should_take_a_late_whole_utterance_spot_after_an_early_one_failed_the_pause_rule() {
    // Given the keyword spotter placed the phrase badly, the transcript arrives later
    let base = Instant::now();
    let events = vec![
        SpottingEvent::SpeechStarted(at(base, 1.0)),
        phrase(base, "start dictation", 1.6, 2.0),
        SpottingEvent::SpeechEnded(at(base, 2.6)),
        phrase(base, "start dictation", 1.0, 2.6),
    ];
    // When
    let commands = feed_all(events);
    // Then
    assert_eq!(commands.len(), 1, "{commands:?}");
    assert_eq!(commands[0].command, VoiceCommand::StartDictation);
}

#[test]
fn should_take_a_command_reported_twice_after_its_segment_once() {
    // Given
    let base = Instant::now();
    let events = vec![
        SpottingEvent::SpeechStarted(at(base, 1.0)),
        SpottingEvent::SpeechEnded(at(base, 1.9)),
        phrase(base, "press enter", 1.1, 1.8),
        phrase(base, "press enter", 1.0, 1.9),
    ];
    // When
    let commands = feed_all(events);
    // Then
    assert_eq!(commands.len(), 1, "{commands:?}");
}

#[test]
fn should_carry_out_only_the_first_of_two_different_commands_in_one_segment() {
    // Given
    let base = Instant::now();
    let events = vec![
        SpottingEvent::SpeechStarted(at(base, 1.0)),
        SpottingEvent::SpeechEnded(at(base, 1.9)),
        phrase(base, "press enter", 1.0, 1.9),
        phrase(base, "clear all text", 1.0, 1.9),
    ];
    // When
    let commands = feed_all(events);
    // Then the first is carried out as it fills the closed segment; the second is not
    assert_eq!(commands.len(), 1, "{commands:?}");
    assert_eq!(commands[0].command, VoiceCommand::PressEnter);
}

#[test]
fn should_ignore_two_phrases_in_one_segment() {
    // Given
    let base = Instant::now();
    let events = vec![
        SpottingEvent::SpeechStarted(at(base, 1.0)),
        phrase(base, "press enter", 1.1, 1.5),
        phrase(base, "clear all text", 1.5, 1.8),
        SpottingEvent::SpeechEnded(at(base, 1.9)),
    ];
    // When
    let commands = feed_all(events);
    // Then
    assert!(commands.is_empty());
}

#[test]
fn should_not_recognize_a_phrase_before_its_segment_ends() {
    // Given
    let base = Instant::now();
    let events = vec![
        SpottingEvent::SpeechStarted(at(base, 1.0)),
        phrase(base, "press enter", 1.1, 1.8),
    ];
    // When
    let commands = feed_all(events);
    // Then
    assert!(commands.is_empty());
}

#[test]
fn should_ignore_an_unknown_phrase() {
    // Given
    let base = Instant::now();
    let events = vec![
        SpottingEvent::SpeechStarted(at(base, 1.0)),
        phrase(base, "open the door", 1.1, 1.8),
        SpottingEvent::SpeechEnded(at(base, 1.9)),
    ];
    // When
    let commands = feed_all(events);
    // Then
    assert!(commands.is_empty());
}

#[test]
fn should_recognize_the_next_phrase_after_an_ignored_segment() {
    // Given
    let base = Instant::now();
    let events = vec![
        SpottingEvent::SpeechStarted(at(base, 1.0)),
        phrase(base, "press enter", 1.8, 2.5),
        SpottingEvent::SpeechEnded(at(base, 2.6)),
        SpottingEvent::SpeechStarted(at(base, 4.0)),
        phrase(base, "clear all text", 4.1, 4.9),
        SpottingEvent::SpeechEnded(at(base, 5.0)),
    ];
    // When
    let commands = feed_all(events);
    // Then
    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].command, VoiceCommand::ClearAllText);
}

fn said(base: Instant, commands: &[VoiceCommand], start: f32, end: f32) -> SpottingEvent {
    SpottingEvent::CommandsSaid {
        commands: commands.to_vec(),
        start: at(base, start),
        end: at(base, end),
        placement: CommandPlacement::Anywhere,
        timing: None,
    }
}

#[test]
fn should_carry_out_commands_said_without_a_pause_at_once_in_order() {
    // Given
    let base = Instant::now();
    let events = vec![
        SpottingEvent::SpeechStarted(at(base, 1.0)),
        said(
            base,
            &[VoiceCommand::StopDictation, VoiceCommand::PressEnter],
            2.5,
            3.2,
        ),
    ];
    // When the utterance has not ended yet
    let commands = feed_all(events);
    // Then
    let carried: Vec<(VoiceCommand, CommandPlacement)> =
        commands.iter().map(|c| (c.command, c.placement)).collect();
    assert_eq!(
        carried,
        [
            (VoiceCommand::StopDictation, CommandPlacement::Anywhere),
            (VoiceCommand::PressEnter, CommandPlacement::Anywhere)
        ]
    );
    assert_eq!(commands[0].started_at, at(base, 2.5));
    assert_eq!(commands[0].ends_at, at(base, 3.2));
}

#[test]
fn should_not_take_again_a_phrase_of_words_already_carried_out() {
    // Given "enter enter" said as a whole utterance, reported both ways
    let base = Instant::now();
    let events = vec![
        SpottingEvent::SpeechStarted(at(base, 1.0)),
        said(base, &[VoiceCommand::PressEnter], 1.05, 1.9),
        phrase(base, "press enter", 1.05, 1.9),
        SpottingEvent::SpeechEnded(at(base, 2.0)),
        phrase(base, "press enter", 1.05, 1.95),
    ];
    // When
    let commands = feed_all(events);
    // Then
    assert_eq!(commands.len(), 1, "{commands:?}");
}
