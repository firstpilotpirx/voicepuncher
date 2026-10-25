use super::*;

#[test]
fn should_restart_once_in_a_dictation_while_confirmations_come_back_slowly() {
    let events = heard_in_time(
            &DICTATING,
            "We moved the weekly meeting to Friday restart restart and the notes begin here with a few more words for everyone",
            SLOW_HEARING,
        );
    assert_eq!(
        times_carried_out(&events, "restart dictation"),
        1,
        "{events:?}"
    );
}

#[test]
fn should_carry_out_the_chain_when_confirmations_come_back_at_once() {
    assert_chain_carried_out(Duration::ZERO);
}

#[test]
fn should_carry_out_the_same_chain_when_confirmations_come_back_slowly() {
    assert_chain_carried_out(SLOW_HEARING);
}

#[test]
fn should_carry_out_the_same_chain_on_a_busy_machine() {
    // Given a dictation, each confirmation 2 s late, on a machine so busy that the threads
    // that hear wait for the processor while the speech goes on
    let listening: Vec<&str> = DICTATING.into_iter().chain(["start dictation"]).collect();
    // When "stop enter start" is said
    let events = heard_in_time_by(&listening, CHAIN_SENTENCE, SLOW_HEARING, |hearing| {
        Arc::new(Starved(hearing))
    });
    // Then the chain is carried out as on an idle machine
    assert_eq!(carried_out(&events), CHAIN, "{events:?}");
}

#[test]
fn should_keep_a_start_confirmed_after_the_dictation_began_a_start() {
    let events = heard_with_dictation_started_at_first_pause(
        &[
            "start dictation",
            "and the notes begin here with a few more words",
        ],
        SLOW_HEARING,
    );
    let said = said(&events);
    assert!(said.iter().any(|c| c == "start dictation"), "{events:?}");
    assert!(!said.iter().any(|c| c == "restart dictation"), "{events:?}");
}

#[test]
fn should_keep_a_start_said_right_after_a_slowly_confirmed_stop_a_start() {
    // Given a dictation running, every confirmation 2.5 s late
    // When "stop dictation" is said and then, before its confirmation, "start"
    let events = heard_with_standby_once_stopped(&["stop dictation", "start"], SLOWER_HEARING);
    // Then the dictation ends and a new one starts: the spotter's report of the stop and
    // its confirmation are one command
    let mut said = said(&events);
    said.dedup();
    assert_eq!(said, ["stop dictation", "start dictation"], "{events:?}");
}

#[test]
fn should_report_a_restart_misheard_at_first_one_hearing_after_its_pause() {
    // Given "restart" said alone in a dictation, its segment heard first as another word and
    // its steady clip as "restart", each answer 0.6 s after it is asked
    let delay = Duration::from_millis(600);
    let hearing = Arc::new(HearsFirstThenSteady {
        first: "Restocked.",
        steady: "Restart!",
        delay,
        steady_clips: Mutex::new(0),
    });
    // When
    let (events, after) = restart_alone_heard_by(hearing);
    // Then the restart comes out sooner than two hearings in a row after the pause that
    // closes it: the keyword spotter reports the trigger up to 0.2 s after that pause
    assert_eq!(phrases(&events), ["restart dictation"], "{events:?}");
    let two_hearings = MIN_SILENCE_SECS + 2.0 * delay.as_secs_f32();
    assert!(
        after < two_hearings,
        "restart out {after:.2}s after the word, two hearings take {two_hearings:.2}s: {events:?}"
    );
}

#[test]
fn should_report_a_restart_heard_at_first_once_and_drop_its_early_second_hearing() {
    // Given "restart" said alone in a dictation, heard as "restart" by both hearings
    let hearing = Arc::new(HearsFirstThenSteady {
        first: "Restart.",
        steady: "Restart!",
        delay: Duration::from_millis(600),
        steady_clips: Mutex::new(0),
    });
    // When
    let (events, _) = restart_alone_heard_by(Arc::clone(&hearing));
    // Then the second hearing was asked at once, and the restart is reported once
    assert_eq!(*hearing.steady_clips.lock().unwrap(), 1, "{events:?}");
    assert_eq!(phrases(&events), ["restart dictation"], "{events:?}");
}

#[test]
fn should_report_start_said_alone_in_standby_sooner_than_one_hearing_after_its_pause() {
    // Given each confirmation 0.6 s late
    let delay = Duration::from_millis(600);
    // When "start" is said alone in standby
    let heard = heard_finely(&STANDBY, &[(0.0, "start")], delay);
    // Then the start comes out well before one hearing after the pause that closes it
    let events = &heard.events;
    assert_eq!(
        said(events).first().map(String::as_str),
        Some("start dictation"),
        "{events:?}"
    );
    let one_hearing_after_pause = MIN_SILENCE_SECS + delay.as_secs_f32();
    let after = heard.said_after.unwrap_or(f32::MAX);
    assert!(
        after < one_hearing_after_pause - MIN_SILENCE_SECS / 4.0,
        "start out {after:.2}s after the word, one hearing after the pause is \
             {one_hearing_after_pause:.2}s: {events:?}"
    );
}

#[test]
fn should_start_nothing_for_start_and_stop_with_a_dip_shorter_than_a_pause_in_standby() {
    // Given each confirmation 0.6 s late
    // When "start", a dip shorter than a pause and "stop" are said in standby
    let heard = heard_finely(
        &STANDBY,
        // With the quiet ends of the words: past half a pause, short of a whole one.
        &[(0.0, "start"), (0.15, "stop")],
        Duration::from_millis(600),
    );
    // Then no dictation starts: the hearing asked at the dip is dropped
    let events = &heard.events;
    assert!(
        heard
            .asked_after
            .iter()
            .any(|&after| after < -heard.last_word_secs),
        "no clip asked during the dip: {:?}, {events:?}",
        heard.asked_after
    );
    assert!(
        !said(events).iter().any(|c| c == "start dictation"),
        "{events:?}"
    );
}

#[test]
fn should_end_a_start_closed_by_please_where_please_ends() {
    // Given each confirmation 0.3 s late
    // When "start dictation please" is said in standby
    let heard = heard_finely(
        &STANDBY,
        &[(0.0, "start"), (0.0, "dictation"), (0.0, "please")],
        Duration::from_millis(300),
    );
    // Then a start a check placed ends no sooner than "please" does: the dictation after it
    // holds none of it
    let events = &heard.events;
    assert!(!said(events).is_empty(), "no start: {events:?}");
    for event in events {
        if let ServerEvent::Command { end, .. } = event {
            assert!(
                *end >= heard.said_end_secs - 0.05,
                "start ends at {end:.2}s, \"please\" at {:.2}s: {events:?}",
                heard.said_end_secs
            );
        }
    }
}

#[test]
fn should_hear_no_utterance_in_a_dictation_before_its_pause_ends() {
    // Given each confirmation 0.6 s late
    // When a short word is said alone in a dictation
    let heard = heard_finely(&DICTATING, &[(0.0, "Friday")], Duration::from_millis(600));
    // Then nothing is asked before most of the pause has passed
    assert!(!heard.asked_after.is_empty(), "{:?}", heard.events);
    let earliest = heard.asked_after.iter().copied().fold(f32::MAX, f32::min);
    assert!(
        earliest > MIN_SILENCE_SECS * 0.75,
        "a clip asked {earliest:.2}s after the word: {:?}",
        heard.events
    );
}

mod needs_command_stt {
    use super::*;

    #[test]
    fn should_hear_a_stop_right_after_the_last_dictated_word() {
        let engine = engine();
        let events = heard(
            &engine,
            &DICTATING,
            &speech("We meet on Friday at ten stop"),
        );
        assert_eq!(
            commands(&events),
            [command(&["stop dictation"], Placement::Tail)]
        );
    }

    #[test]
    fn should_hear_enter_right_after_other_speech() {
        let engine = engine();
        let events = heard(&engine, &STANDBY, &speech("Okay, press enter"));
        // A phrase for the whole utterance would fail the app's pause rule here: only a
        // command placed at the end counts.
        assert_eq!(
            commands(&events),
            [command(&["press enter"], Placement::Tail)],
            "{events:?}"
        );
    }

    #[test]
    fn should_hear_start_opening_an_utterance_and_place_it_before_the_speech() {
        let engine = engine();
        let audio = speech("Start, the meeting moved to Friday at ten");
        let events = heard(&engine, &STANDBY, &audio);
        assert_eq!(
            commands(&events),
            [command(&["start dictation"], Placement::Head)]
        );
        let end = events
            .iter()
            .find_map(|e| match e {
                ServerEvent::Command { end, .. } => Some(*end),
                _ => None,
            })
            .unwrap();
        assert!(end < PAUSE_SECS + 1.2, "start ends at {end}");
    }

    #[test]
    fn should_hear_a_doubled_stop_in_the_middle_of_speech() {
        let engine = engine();
        let events = heard(
            &engine,
            &DICTATING,
            &speech("We move the meeting to Friday stop stop and some more words here"),
        );
        assert_eq!(
            commands(&events),
            [command(&["stop dictation"], Placement::Anywhere)]
        );
    }

    #[test]
    fn should_hear_a_chain_of_commands() {
        let engine = engine();
        let events = heard(&engine, &DICTATING, &speech("That is all, stop enter"));
        assert_eq!(
            commands(&events),
            [command(
                &["stop dictation", "press enter"],
                Placement::Anywhere
            )]
        );
    }

    #[test]
    fn should_hear_a_stop_after_a_short_breath_as_an_utterance_of_its_own() {
        let engine = engine();
        // `say` ends a phrase with about 0.4 s of its own silence: the breath is cut from the
        // last sound to the next.
        let audio: Vec<f32> = sound_only(&speech("We meet on Friday"))
            .into_iter()
            .chain(silence(BREATH_SECS))
            .chain(sound_only(&speech("stop dictation")))
            .collect();
        let events = heard(&engine, &DICTATING, &audio);
        let starts = events
            .iter()
            .filter(|e| matches!(e, ServerEvent::SpeechStart { .. }))
            .count();
        assert_eq!(starts, 2, "two utterances expected: {events:?}");
        assert!(
            phrases(&events).contains(&"stop dictation".to_owned()),
            "{events:?}"
        );
    }
}
