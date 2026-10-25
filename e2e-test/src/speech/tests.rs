use super::*;

#[test]
fn should_find_a_phrase_repeated_at_a_seam() {
    let spoken = "у которой миллион граней и мы на эти грани можем";
    let transcript = "у которой миллион граней. Миллион граней, и мы на эти грани можем";
    assert_eq!(repeated_phrases(spoken, transcript), vec!["миллион граней"]);
}

#[test]
fn should_ignore_a_phrase_the_speech_itself_repeats() {
    let spoken = "да да конечно да да";
    assert!(repeated_phrases(spoken, "Да, да, конечно. Да, да.").is_empty());
}

#[test]
fn should_not_count_a_pair_heard_twice_that_was_never_spoken() {
    let spoken = "we decided how to split the work for the team";
    let transcript = "we decided our test with the work with the team";
    assert!(repeated_phrases(spoken, transcript).is_empty());
}

#[test]
fn should_report_no_missing_words_for_the_same_text() {
    assert!(
        (missing_word_share("Hello, World!", "hello world") - 0.0).abs() < 1e-6,
        "{} is not {}",
        missing_word_share("Hello, World!", "hello world"),
        0.0
    );
}

#[test]
fn should_count_dropped_words() {
    let share = missing_word_share("one two three four", "one four");
    assert!((share - 0.5).abs() < 1e-9, "{share}");
}

#[test]
fn should_count_a_dropped_phrase() {
    assert_eq!(dropped_phrase_len("a b c d e f g h", "a b g h"), 4);
}

#[test]
fn should_not_count_a_misheard_phrase() {
    let spoken = "индекс по полю даты создания так и не построили";
    let transcript = "индекс попа людат и создание так и не построили";
    assert_eq!(dropped_phrase_len(spoken, transcript), 0);
}

#[test]
fn should_count_a_phrase_replaced_by_a_shorter_one() {
    let spoken = "at the end of a day I updated a report to the lead";
    let transcript = "at the Jordan reports to the lead";
    assert_eq!(dropped_phrase_len(spoken, transcript), 6);
}

#[test]
fn should_count_words_dropped_at_the_end() {
    assert_eq!(dropped_phrase_len("one two three four five", "one two"), 3);
}

#[test]
fn should_read_yo_as_ye() {
    assert_eq!(normalized_words("Ещё отчёт"), vec!["еще", "отчет"]);
}

#[test]
fn should_keep_the_clean_speech_aligned_with_the_degraded_one() {
    for condition in [Condition::NoiseHigh, Condition::Dropouts] {
        let speeches = synthesize_with_clean(Lang::En, condition, &[0.0, 0.7]).unwrap();
        let [(_, first), (degraded, clean)] = speeches.as_slice() else {
            panic!("{condition:?}: one speech per lead-in expected");
        };
        assert_eq!(clean.len(), degraded.len(), "{condition:?}");
        assert_eq!(clean.len(), first.len() + 11_200, "{condition:?}");
        assert_ne!(clean, degraded, "{condition:?}");
    }
}

/// Synthesizes `lang` under `condition` and checks it lasts longer than the chunk threshold.
fn assert_longer_than_the_threshold(lang: Lang, condition: Condition) {
    let samples = synthesize(lang, condition, 0.0).unwrap();
    let secs = samples.len() as f32 / SAMPLE_RATE_HZ as f32;
    assert!(secs > 30.0, "{lang:?} {condition:?}: only {secs:.1} s");
}

#[test]
fn should_synthesize_speech_longer_than_the_chunk_threshold_in_every_condition() {
    std::thread::scope(|scope| {
        for lang in Lang::ALL {
            for condition in Condition::ALL {
                scope.spawn(move || assert_longer_than_the_threshold(lang, condition));
            }
        }
    });
}
