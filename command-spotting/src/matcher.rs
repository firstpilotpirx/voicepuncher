//! Decides which command phrase, if any, a transcript of a whole utterance is.

mod placed;

pub use placed::{Found, Placement};

/// Where a transcript's sentences end.
const SENTENCE_ENDS: &[char] = &['.', ',', '!', '?', '\n'];
/// Said after a command out of politeness; it does not change the command.
const POLITE_WORD: &str = "please";
/// Words a transcript may hold inside a phrase without changing it ("cancel the quotation").
const FILLER_WORDS: &[&str] = &["the", "a", "an"];
/// Other ways to say a phrase, and slips the speech recognition service makes.
const ALIASES: &[(&str, &str)] = &[
    ("clear text", "clear all text"),
    ("presenter", "press enter"),
    ("star dictation", "start dictation"),
    ("stuck dictation", "start dictation"),
    // The short forms: one word for each command.
    ("enter", "press enter"),
    ("clear", "clear all text"),
    ("start", "start dictation"),
    ("stop", "stop dictation"),
    ("restart", "restart dictation"),
    ("cancel", "cancel dictation"),
];
/// Largest share of characters that may differ between a transcript and a phrase.
const MAX_DIFFERENCE: f32 = 0.25;
/// Largest share of characters that may differ within one word ("activation" for
/// "dictation"), so that "stop" never passes for "start".
const MAX_WORD_DIFFERENCE: f32 = 0.4;

/// The selected command phrases and the ways people say them.
pub struct PhraseMatcher {
    phrases: Vec<String>,
}

impl PhraseMatcher {
    /// The selected command phrases.
    fn selected(&self) -> &[String] {
        &self.phrases
    }
}

impl PhraseMatcher {
    pub fn new(phrases: &[String]) -> Self {
        Self {
            phrases: phrases.to_vec(),
        }
    }

    /// The command phrase the whole `transcript` says, if it says one of the selected. A
    /// transcript of several sentences counts when every sentence says the same phrase.
    pub fn phrase_in(&self, transcript: &str) -> Option<String> {
        self.phrase_in_sentence(transcript)
            .or_else(|| self.phrase_in_every_sentence(transcript))
    }

    fn phrase_in_every_sentence(&self, transcript: &str) -> Option<String> {
        let sentences: Vec<&str> = transcript
            .split(SENTENCE_ENDS)
            .filter(|s| !words(s).is_empty())
            .collect();
        let mut phrases = sentences.iter().map(|s| self.phrase_in_sentence(s));
        let first = phrases.next()??;
        phrases.all(|p| p.as_ref() == Some(&first)).then_some(first)
    }

    fn phrase_in_sentence(&self, sentence: &str) -> Option<String> {
        let said = once(&without_please(&words(sentence)));
        if said.is_empty() {
            return None;
        }
        let joined = said.join(" ");
        self.surface_forms()
            .filter(|(_, form)| close_word_by_word(&said, form))
            .filter(|(_, form)| difference(&joined, form) <= MAX_DIFFERENCE)
            .min_by(|(_, a), (_, b)| difference(&joined, a).total_cmp(&difference(&joined, b)))
            .map(|(phrase, _)| phrase)
    }

    /// Every way to say each selected phrase, with the phrase it stands for.
    fn surface_forms(&self) -> impl Iterator<Item = (String, String)> + '_ {
        self.phrases.iter().flat_map(|phrase| {
            let aliases = ALIASES
                .iter()
                .filter(move |(_, target)| target == phrase)
                .map(|(alias, _)| (*alias).to_owned());
            std::iter::once(phrase.clone())
                .chain(aliases)
                .map(move |form| (phrase.clone(), form))
        })
    }
}

/// Lowercase words of letters only, without filler words.
fn words(transcript: &str) -> Vec<String> {
    transcript
        .to_lowercase()
        .split(|c: char| !c.is_ascii_lowercase())
        .filter(|w| !w.is_empty() && !FILLER_WORDS.contains(w))
        .map(str::to_owned)
        .collect()
}

fn without_please(words: &[String]) -> Vec<String> {
    match words.split_last() {
        Some((last, rest)) if last == POLITE_WORD => rest.to_vec(),
        _ => words.to_vec(),
    }
}

/// The words said once when the utterance repeats them ("press enter, press enter").
fn once(words: &[String]) -> Vec<String> {
    let half = words.len() / 2;
    if words.len() % 2 == 0 && half > 0 && words[..half] == words[half..] {
        words[..half].to_vec()
    } else {
        words.to_vec()
    }
}

/// As many words as the phrase, each close to the phrase's word in its place.
fn close_word_by_word(said: &[String], form: &str) -> bool {
    let form: Vec<&str> = form.split(' ').collect();
    said.len() == form.len()
        && said
            .iter()
            .zip(form)
            .all(|(word, expected)| difference(word, expected) <= MAX_WORD_DIFFERENCE)
}

/// Character edit distance between `a` and `b`, as a share of the longer one.
fn difference(a: &str, b: &str) -> f32 {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    let mut previous: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut current = vec![i + 1];
        for (j, cb) in b.iter().enumerate() {
            let substitution = previous[j] + usize::from(ca != cb);
            current.push(substitution.min(previous[j + 1] + 1).min(current[j] + 1));
        }
        previous = current;
    }
    previous[b.len()] as f32 / a.len().max(b.len()).max(1) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn standby() -> PhraseMatcher {
        PhraseMatcher::new(&[
            "start dictation".into(),
            "press enter".into(),
            "clear all text".into(),
        ])
    }

    fn dictating() -> PhraseMatcher {
        PhraseMatcher::new(&[
            "stop dictation".into(),
            "cancel dictation".into(),
            "restart dictation".into(),
            "press enter".into(),
        ])
    }

    #[test]
    fn should_take_an_exact_phrase_whatever_its_case_and_punctuation() {
        assert_eq!(
            standby().phrase_in("Press Enter."),
            Some("press enter".into())
        );
        assert_eq!(
            dictating().phrase_in(" stop dictation "),
            Some("stop dictation".into())
        );
    }

    #[test]
    fn should_take_a_phrase_followed_by_please() {
        assert_eq!(
            standby().phrase_in("Press enter, please."),
            Some("press enter".into())
        );
        assert_eq!(
            dictating().phrase_in("Cancel dictation please"),
            Some("cancel dictation".into())
        );
    }

    #[test]
    fn should_take_clear_text_as_clear_all_text() {
        assert_eq!(
            standby().phrase_in("Clear text."),
            Some("clear all text".into())
        );
        assert_eq!(
            standby().phrase_in("Clear text, please."),
            Some("clear all text".into())
        );
    }

    #[test]
    fn should_take_the_slips_heard_in_the_users_voice() {
        assert_eq!(
            standby().phrase_in("Star dictation."),
            Some("start dictation".into())
        );
        assert_eq!(
            standby().phrase_in("Start.\n Activation."),
            Some("start dictation".into())
        );
        assert_eq!(
            standby().phrase_in("Presenter."),
            Some("press enter".into())
        );
        assert_eq!(
            standby().phrase_in("Clear all texts."),
            Some("clear all text".into())
        );
        assert_eq!(
            dictating().phrase_in("Cancel the quotation please."),
            Some("cancel dictation".into())
        );
    }

    #[test]
    fn should_take_a_phrase_said_twice_once() {
        assert_eq!(
            standby().phrase_in("Star dictation, star dictation."),
            Some("start dictation".into())
        );
        assert_eq!(
            standby().phrase_in("Press enter. Press enter."),
            Some("press enter".into())
        );
    }

    #[test]
    fn should_take_a_repeat_whose_sentences_say_the_same_command() {
        assert_eq!(
            dictating().phrase_in(" Cancel dictation please. Cancel dictation.\n"),
            Some("cancel dictation".into())
        );
    }

    #[test]
    fn should_not_take_sentences_that_say_different_things() {
        assert_eq!(standby().phrase_in("Press enter. Hello."), None);
        assert_eq!(standby().phrase_in("Press enter. Clear text."), None);
    }

    #[test]
    fn should_not_take_a_phrase_with_other_words_around_it() {
        for transcript in [
            "Please press enter.",
            "Press enter when you are ready.",
            "I think we should press enter.",
            "Let's start dictation tomorrow.",
        ] {
            assert_eq!(standby().phrase_in(transcript), None, "{transcript}");
        }
    }

    #[test]
    fn should_take_the_short_forms() {
        for (transcript, phrase) in [
            ("Enter.", "press enter"),
            ("Enter, please.", "press enter"),
            ("Clear.", "clear all text"),
            ("Clear please.", "clear all text"),
            ("Start.", "start dictation"),
            ("Start please.", "start dictation"),
        ] {
            assert_eq!(
                standby().phrase_in(transcript),
                Some(phrase.into()),
                "{transcript}"
            );
        }
        assert_eq!(
            dictating().phrase_in("Stop."),
            Some("stop dictation".into())
        );
        assert_eq!(
            dictating().phrase_in("Stop, please."),
            Some("stop dictation".into())
        );
        assert_eq!(
            dictating().phrase_in("Cancel."),
            Some("cancel dictation".into())
        );
        assert_eq!(
            dictating().phrase_in("Restart, please."),
            Some("restart dictation".into())
        );
    }

    #[test]
    fn should_not_take_other_short_words() {
        for transcript in ["Hello.", "Thank you.", "Okay.", "Yes.", "Restart.", ""] {
            assert_eq!(standby().phrase_in(transcript), None, "{transcript}");
        }
        for transcript in ["Start.", "Cancelled.", "Stopped.", "Okay."] {
            assert_eq!(dictating().phrase_in(transcript), None, "{transcript}");
        }
    }

    #[test]
    fn should_not_take_a_phrase_outside_the_selected_set() {
        assert_eq!(standby().phrase_in("Stop dictation."), None);
        assert_eq!(dictating().phrase_in("Clear all text."), None);
    }
}
