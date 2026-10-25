//! How the chunk transcripts merge at their seams.

use super::*;

fn merge_two(tail: &str, head: &str) -> String {
    merge_chunk_texts(&[tail.to_owned(), head.to_owned()])
}

#[test]
fn should_merge_every_recorded_seam_as_reviewed() {
    let failures: Vec<String> = super::seam_cases::all()
        .iter()
        .filter_map(|case| {
            let merged = merge_two(case.tail, case.head);
            (merged != case.merged).then(|| {
                format!(
                    "{}\n    tail:     {}\n    head:     {}\n    expected: {}\n    merged:   {}",
                    case.recording, case.tail, case.head, case.merged, merged
                )
            })
        })
        .collect();
    assert!(
        failures.is_empty(),
        "{} of {} recorded seams merged wrongly:\n{}",
        failures.len(),
        super::seam_cases::all().len(),
        failures.join("\n")
    );
}

#[test]
fn should_keep_overlap_once_when_punctuation_and_case_differ() {
    let merged = merge_two(
        "сущность, у которой миллион уровней, миллион граней.",
        "Миллион граней, и мы на эти грани можем смотреть",
    );
    assert_eq!(
        merged,
        "сущность, у которой миллион уровней, миллион граней, и мы на эти грани можем смотреть"
    );
}

#[test]
fn should_keep_overlap_once_when_yo_is_written_as_ye() {
    let merged = merge_two(
        "иметь понятный интерфейс. Ещё нужно написать",
        "Еще нужно написать тесты на возврат денег",
    );
    assert_eq!(
        merged,
        "иметь понятный интерфейс. Ещё нужно написать тесты на возврат денег"
    );
}

#[test]
fn should_keep_overlap_once_when_a_word_is_hyphenated_in_one_chunk() {
    let merged = merge_two(
        "просто к обычным раунд-робинам тебе один запрос кидает,",
        "обычным раундробином тебе один запрос кидает на одну ноду",
    );
    assert_eq!(
        merged,
        "просто к обычным раунд-робинам тебе один запрос кидает на одну ноду"
    );
}

#[test]
fn should_keep_the_whole_word_when_one_chunk_cut_it_short() {
    let merged = merge_two(
        "чтобы на нём такие. Но своего рода чека,",
        "Но своего рода чекап, это как нагрузочное тестирование",
    );
    assert_eq!(
        merged,
        "чтобы на нём такие. Но своего рода чекап, это как нагрузочное тестирование"
    );
}

#[test]
fn should_keep_overlap_once_when_chunks_read_a_word_differently() {
    let merged = merge_two(
        "прохожу 300 тысяч шагов по 30-30 дней.",
        "шагов по 30 километров в день пробегаю, и вот",
    );
    assert_eq!(
        merged,
        "прохожу 300 тысяч шагов по 30 километров в день пробегаю, и вот"
    );
}

#[test]
fn should_keep_overlap_once_when_the_first_chunk_misheard_its_last_number() {
    let merged = merge_two(
        "and the response time dropped from 2 seconds to 4.",
        "dropped from 2 seconds to 40 milliseconds. In the evening",
    );
    assert_eq!(
        merged,
        "and the response time dropped from 2 seconds to 40 milliseconds. In the evening"
    );
}

#[test]
fn should_keep_a_lone_word_once_when_it_ends_one_chunk_and_starts_the_next() {
    let merged = merge_two(
        "have a clear interface. face. We",
        "We also need tests for refunds",
    );
    assert_eq!(
        merged,
        "have a clear interface. face. We also need tests for refunds"
    );
}

#[test]
fn should_not_trust_a_lone_similar_word() {
    let merged = merge_two(
        "на несколько маленьких частей.",
        "Каждую часть должна отвечать только за одну вещь",
    );
    assert_eq!(
        merged,
        "на несколько маленьких частей. Каждую часть должна отвечать только за одну вещь"
    );
}

#[test]
fn should_keep_a_phrase_repeated_outside_the_overlap() {
    let merged = merge_two(
        "I said yes and then we left the building quietly",
        "Next morning I said yes again to the offer",
    );
    assert_eq!(
        merged,
        "I said yes and then we left the building quietly Next morning I said yes again to the offer"
    );
}

#[test]
fn should_keep_every_word_when_chunks_share_none() {
    let merged = merge_two(
        "it was a long day at the office.",
        "Tomorrow we start early and review the plan.",
    );
    assert_eq!(
        merged,
        "it was a long day at the office. Tomorrow we start early and review the plan."
    );
}

#[test]
fn should_keep_overlap_once_across_three_differing_chunks() {
    let merged = merge_chunk_texts(&[
        "Сегодня утром я проверил очередь задач и понял, что половина".to_owned(),
        "что половина из них устарела. Мы договорились с командой".to_owned(),
        "Мы договорились с командой, что будем пересматривать список".to_owned(),
    ]);
    assert_eq!(
        merged,
        "Сегодня утром я проверил очередь задач и понял, что половина из них устарела. Мы договорились с командой, что будем пересматривать список"
    );
}

#[test]
fn should_return_single_chunk_unchanged() {
    let texts = vec!["hello world foo".to_string()];
    let merged = merge_chunk_texts(&texts);
    assert_eq!(merged, "hello world foo");
}

#[test]
fn should_drop_only_actual_overlap_words() {
    let texts = vec![
        "alpha beta gamma".to_string(),
        "gamma delta epsilon zeta".to_string(),
    ];
    let merged = merge_chunk_texts(&texts);
    assert_eq!(merged, "alpha beta gamma delta epsilon zeta");
}

#[test]
fn should_produce_empty_string_from_all_empty_chunks() {
    let texts = vec![String::new(), String::new(), String::new()];
    let merged = merge_chunk_texts(&texts);
    assert_eq!(merged, "");
}

#[test]
fn should_keep_all_words_from_first_chunk() {
    let texts = vec![
        "one two three four five".to_string(),
        "five six seven".to_string(),
    ];
    let merged = merge_chunk_texts(&texts);
    let first_five: Vec<&str> = merged.split_whitespace().take(5).collect();
    assert_eq!(first_five, vec!["one", "two", "three", "four", "five"]);
}

fn word_count(s: &str, word: &str) -> usize {
    s.split_whitespace().filter(|w| *w == word).count()
}

#[test]
fn should_drop_actual_overlap_even_if_exceeds_fraction() {
    let texts = vec![
        "the quick brown fox".to_string(),
        "brown fox jumps over lazy dog".to_string(),
    ];
    let merged = merge_chunk_texts(&texts);
    assert_eq!(
        word_count(&merged, "brown"),
        1,
        "word 'brown' duplicated at chunk boundary: {merged:?}"
    );
    assert_eq!(
        word_count(&merged, "fox"),
        1,
        "word 'fox' duplicated at chunk boundary: {merged:?}"
    );
}

#[test]
fn should_drop_multi_word_phrase_overlap() {
    let texts = vec![
        "I think it is great".to_string(),
        "it is great that we are here".to_string(),
    ];
    let merged = merge_chunk_texts(&texts);
    assert_eq!(
        word_count(&merged, "is"),
        1,
        "word 'is' duplicated at chunk boundary: {merged:?}"
    );
    assert_eq!(
        word_count(&merged, "great"),
        1,
        "word 'great' duplicated at chunk boundary: {merged:?}"
    );
}

#[test]
fn should_keep_all_words_when_no_actual_overlap() {
    let texts = vec![
        "first sentence ends with a long pause".to_string(),
        "second sentence starts after the pause continues here for more".to_string(),
    ];
    let merged = merge_chunk_texts(&texts);
    assert!(
        merged.contains("second sentence"),
        "first word of chunk2 wrongly dropped: {merged:?}"
    );
}

#[test]
fn should_drop_overlap_across_three_chunks() {
    let texts = vec![
        "good morning everyone today we will discuss".to_string(),
        "we will discuss the architecture of our system".to_string(),
        "of our system and how it scales".to_string(),
    ];
    let merged = merge_chunk_texts(&texts);
    assert_eq!(
        word_count(&merged, "will"),
        1,
        "word 'will' duplicated across seam 1: {merged:?}"
    );
    assert_eq!(
        word_count(&merged, "discuss"),
        1,
        "word 'discuss' duplicated across seam 1: {merged:?}"
    );
    assert_eq!(
        word_count(&merged, "our"),
        1,
        "word 'our' duplicated across seam 2: {merged:?}"
    );
    assert_eq!(
        word_count(&merged, "system"),
        1,
        "word 'system' duplicated across seam 2: {merged:?}"
    );
}

#[test]
fn should_handle_completely_disjoint_chunks() {
    let texts = vec!["hello world".to_string(), "foo bar baz".to_string()];
    let merged = merge_chunk_texts(&texts);
    assert_eq!(merged, "hello world foo bar baz");
}

#[test]
fn should_handle_identical_chunks() {
    let texts = vec![
        "alpha beta gamma".to_string(),
        "alpha beta gamma".to_string(),
    ];
    let merged = merge_chunk_texts(&texts);
    assert_eq!(merged, "alpha beta gamma");
}

#[test]
fn should_handle_inner_empty_chunks() {
    let texts = vec![
        "first chunk text".to_string(),
        String::new(),
        "third chunk here".to_string(),
    ];
    let merged = merge_chunk_texts(&texts);
    assert!(merged.starts_with("first chunk text"));
    assert!(merged.ends_with("third chunk here"));
}
