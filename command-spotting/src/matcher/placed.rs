//! Commands said without a pause around them: at the end of an utterance, "start" at its
//! start, and doubled or chained commands anywhere. Works on the words of a transcript,
//! so the caller can place what it finds in the audio by the words' times.

use std::ops::Range;

use super::PhraseMatcher;

/// Where in the utterance the command words sat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Placement {
    /// The utterance ends with the command.
    Tail,
    /// "start" opens the utterance and speech follows.
    Head,
    /// A doubled short form or a chain of commands, wherever it sits.
    Anywhere,
}

/// Commands found in a transcript: carried out in order; `first..=last` are the indices
/// of their words in the words given, and `spans` those of each command's words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    pub commands: Vec<String>,
    pub spans: Vec<(usize, usize)>,
    pub first: usize,
    pub last: usize,
    pub placement: Placement,
}

/// How a command may be said: its phrase and the words of one way to say it.
struct Form {
    phrase: &'static str,
    words: &'static [&'static str],
}

const fn form(phrase: &'static str, words: &'static [&'static str]) -> Form {
    Form { phrase, words }
}

const START: &str = "start dictation";
const STOP: &str = "stop dictation";
const CANCEL: &str = "cancel dictation";
const RESTART: &str = "restart dictation";
const ENTER: &str = "press enter";
const CLEAR: &str = "clear all text";
const PASTE: &str = "paste text";

/// Longer forms first, so "stop dictation" wins over "stop".
const FORMS: &[Form] = &[
    form(CLEAR, &["clear", "all", "text"]),
    form(START, &["start", "dictation"]),
    form(START, &["star", "dictation"]),
    form(START, &["stuck", "dictation"]),
    form(STOP, &["stop", "dictation"]),
    form(CANCEL, &["cancel", "dictation"]),
    form(RESTART, &["restart", "dictation"]),
    form(ENTER, &["press", "enter"]),
    form(CLEAR, &["clear", "text"]),
    form(PASTE, &["paste", "text"]),
    form(ENTER, &["presenter"]),
    form(START, &["start"]),
    form(STOP, &["stop"]),
    form(RESTART, &["restart"]),
    form(ENTER, &["enter"]),
    form(CLEAR, &["clear"]),
    form(PASTE, &["paste"]),
    form(CANCEL, &["cancel"]),
];

/// The phrases each state accepts; a dictation runs or it does not.
const STANDBY: &[&str] = &[START, ENTER, CLEAR, PASTE];
/// "enter" during a dictation ends it and presses Return.
const DICTATING: &[&str] = &[STOP, CANCEL, RESTART, ENTER];

/// The phrases accepted after `phrase` was carried out.
fn accepted_after(phrase: &str) -> &'static [&'static str] {
    if phrase == START || phrase == RESTART {
        DICTATING
    } else {
        STANDBY
    }
}

/// A word of the transcript as the matcher reads it, with its index in the words given.
struct Token {
    at: usize,
    text: String,
}

fn tokens(words: &[&str]) -> Vec<Token> {
    words
        .iter()
        .enumerate()
        .flat_map(|(at, word)| {
            super::words(word)
                .into_iter()
                .map(move |text| Token { at, text })
        })
        .collect()
}

/// How far the words at the start of `tokens` are from `form`: none when they do not say
/// it. A one-word form must be said exactly ("clean" is no "clear"); a longer one may
/// differ a little in each word, as the speech recognition service mishears it.
fn difference_from(tokens: &[Token], form: &Form) -> Option<f32> {
    if tokens.len() < form.words.len() {
        return None;
    }
    let mut total = 0.0;
    for (token, word) in tokens.iter().zip(form.words) {
        let difference = super::difference(&token.text, word);
        let close = if form.words.len() == 1 {
            difference == 0.0
        } else {
            difference <= super::MAX_WORD_DIFFERENCE
        };
        if !close {
            return None;
        }
        total += difference;
    }
    Some(total)
}

/// The command said at `from`, and how many tokens it takes with a trailing "please".
fn command_at(tokens: &[Token], from: usize) -> Option<(&'static str, usize)> {
    let rest = tokens.get(from..)?;
    // The longest form said, and of those the closest: "restart dictation" is close to
    // "start dictation" too.
    let (form, _) = FORMS
        .iter()
        .filter_map(|form| difference_from(rest, form).map(|d| (form, d)))
        .min_by(|(a, da), (b, db)| b.words.len().cmp(&a.words.len()).then(da.total_cmp(db)))?;
    let mut used = form.words.len();
    if rest.get(used).is_some_and(|t| t.text == super::POLITE_WORD) {
        used += 1;
    }
    Some((form.phrase, used))
}

impl PhraseMatcher {
    /// The phrases the current state accepts: the selected ones the matcher knows.
    fn accepted(&self) -> Vec<&'static str> {
        STANDBY
            .iter()
            .chain(DICTATING)
            .copied()
            .filter(|phrase| self.selected().iter().any(|s| s == phrase))
            .collect()
    }

    /// The command of the selected set that ends `words`, speech before it or not. Its last
    /// word must be the last word said: "clear the" does not end with "clear".
    pub fn at_end(&self, words: &[&str]) -> Option<Found> {
        let tokens = tokens(words);
        let accepted = self.accepted();
        let last_said = words
            .iter()
            .rposition(|w| w.chars().any(char::is_alphabetic))?;
        if tokens.last()?.at != last_said {
            return None;
        }
        (0..tokens.len()).find_map(|from| {
            let (phrase, used) = command_at(&tokens, from)?;
            (from + used == tokens.len() && accepted.contains(&phrase)).then(|| Found {
                commands: vec![phrase.to_owned()],
                spans: vec![(tokens[from].at, tokens[tokens.len() - 1].at)],
                first: tokens[from].at,
                last: tokens[tokens.len() - 1].at,
                placement: Placement::Tail,
            })
        })
    }

    /// "start dictation" or "start" as the only words so far of an utterance still being
    /// spoken: the speech after it has not been heard yet.
    pub fn start_so_far(&self, words: &[&str]) -> Option<Found> {
        let tokens = tokens(words);
        let (phrase, used) = command_at(&tokens, 0)?;
        (phrase == START && self.accepted().contains(&START) && used == tokens.len()).then(|| {
            Found {
                commands: vec![START.to_owned()],
                spans: vec![(tokens[0].at, tokens[used - 1].at)],
                first: tokens[0].at,
                last: tokens[used - 1].at,
                placement: Placement::Head,
            }
        })
    }

    /// "start dictation" or "start" opening `words`, with more speech after it or closed by
    /// "please".
    pub fn start_at_head(&self, words: &[&str]) -> Option<Found> {
        let tokens = tokens(words);
        let (phrase, used) = command_at(&tokens, 0)?;
        // "start stop" ends the dictation it would start: no dictation at all.
        let command_follows = command_at(&tokens, used).is_some();
        // "please" closes the command: nothing has to follow it.
        let closed = used < tokens.len() || tokens[used - 1].text == super::POLITE_WORD;
        (phrase == START && self.accepted().contains(&START) && closed && !command_follows).then(
            || Found {
                commands: vec![START.to_owned()],
                spans: vec![(tokens[0].at, tokens[used - 1].at)],
                first: tokens[0].at,
                last: tokens[used - 1].at,
                placement: Placement::Head,
            },
        )
    }

    /// Whether `found` is a start closed by "please" with nothing after it in `words`.
    pub fn start_closed_by_please(&self, words: &[&str], found: &Found) -> bool {
        let tokens = tokens(words);
        found.placement == Placement::Head
            && found.commands == [START]
            && tokens
                .last()
                .is_some_and(|t| t.at == found.last && t.text == super::POLITE_WORD)
    }

    /// A doubled short form or a chain of two or three commands anywhere in `words`.
    #[expect(clippy::excessive_nesting, reason = "size baseline")]
    pub fn anywhere(&self, words: &[&str]) -> Option<Found> {
        let tokens = tokens(words);
        let accepted = self.accepted();
        (0..tokens.len()).find_map(|from| {
            let (first, used) = command_at(&tokens, from)?;
            if !accepted.contains(&first) {
                return None;
            }
            let chain = chain_from(&tokens, first, from, from + used);
            let chain = if chain.len() >= 2 {
                chain
            } else {
                let (again, used_again) = command_at(&tokens, from + used)?;
                if again != first {
                    return None;
                }
                vec![(first, from..from + used + used_again)]
            };
            let end = chain[chain.len() - 1].1.end;
            Some(Found {
                commands: chain.iter().map(|(c, _)| (*c).to_owned()).collect(),
                spans: chain
                    .iter()
                    .map(|(_, said)| (tokens[said.start].at, tokens[said.end - 1].at))
                    .collect(),
                first: tokens[from].at,
                last: tokens[end - 1].at,
                placement: Placement::Anywhere,
            })
        })
    }
}

/// The commands said one after another from `first`, said in tokens `from..next`, each
/// accepted after the one before and each once, "start" only last; with the tokens of each.
fn chain_from(
    tokens: &[Token],
    first: &'static str,
    from: usize,
    mut next: usize,
) -> Vec<(&'static str, Range<usize>)> {
    const LONGEST_CHAIN: usize = 3;
    let mut chain = vec![(first, from..next)];
    while chain.len() < LONGEST_CHAIN {
        let last = chain[chain.len() - 1].0;
        if last == START || last == RESTART {
            break;
        }
        if chain.len() + 2 <= LONGEST_CHAIN
            && ends_with_and_restart(tokens, next)
            && accepted_after(last).contains(&ENTER)
        {
            // "enter start" said quickly, heard as "and restart", ends the chain.
            chain.extend([(ENTER, next..next + 1), (START, next + 1..next + 2)]);
            break;
        }
        let Some((command, used)) = command_at(tokens, next) else {
            break;
        };
        if chain.iter().any(|(c, _)| *c == command) || !accepted_after(last).contains(&command) {
            break;
        }
        chain.push((command, next..next + used));
        next += used;
    }
    chain
}

/// "and restart" closes the transcript at `next`: how the speech recognition service hears
/// a quick "enter start".
fn ends_with_and_restart(tokens: &[Token], next: usize) -> bool {
    let words: Vec<&str> = tokens[next.min(tokens.len())..]
        .iter()
        .map(|t| t.text.as_str())
        .collect();
    words == ["and", "restart"]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn standby() -> PhraseMatcher {
        PhraseMatcher::new(&[
            "start dictation".into(),
            "press enter".into(),
            "clear all text".into(),
            "paste text".into(),
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

    fn words(text: &str) -> Vec<&str> {
        text.split_whitespace().collect()
    }

    /// Each command with the indices of its first and last word.
    fn found(commands: &[(&str, usize, usize)], placement: Placement) -> Option<Found> {
        Some(Found {
            commands: commands.iter().map(|(c, _, _)| (*c).to_owned()).collect(),
            spans: commands.iter().map(|&(_, f, l)| (f, l)).collect(),
            first: commands[0].1,
            last: commands[commands.len() - 1].2,
            placement,
        })
    }

    #[test]
    fn should_find_a_command_that_ends_the_utterance() {
        assert_eq!(
            dictating().at_end(&words("We meet on Friday. Stop.")),
            found(&[("stop dictation", 4, 4)], Placement::Tail)
        );
        assert_eq!(
            dictating().at_end(&words("that is all stop dictation please")),
            found(&[("stop dictation", 3, 5)], Placement::Tail)
        );
        assert_eq!(
            dictating().at_end(&words("never mind, cancel dictation.")),
            found(&[("cancel dictation", 2, 3)], Placement::Tail)
        );
        assert_eq!(
            standby().at_end(&words("Okay, press enter.")),
            found(&[("press enter", 1, 2)], Placement::Tail)
        );
        assert_eq!(
            standby().at_end(&words("okay enter")),
            found(&[("press enter", 1, 1)], Placement::Tail)
        );
    }

    #[test]
    fn should_not_find_a_command_that_does_not_end_the_utterance() {
        for text in [
            "we should stop the server tonight",
            "stop and think",
            "",
            "the shop",
            "keep it clean",
        ] {
            assert_eq!(dictating().at_end(&words(text)), None, "{text}");
            assert_eq!(standby().at_end(&words(text)), None, "{text}");
        }
        assert_eq!(standby().at_end(&words("we should stop")), None);
        assert_eq!(
            dictating().at_end(&words("please cancel")),
            found(&[("cancel dictation", 1, 1)], Placement::Tail)
        );
    }

    #[test]
    fn should_find_start_opening_the_utterance() {
        assert_eq!(
            standby().start_at_head(&words("Start, meeting moved to Friday.")),
            found(&[("start dictation", 0, 0)], Placement::Head)
        );
        assert_eq!(
            standby().start_at_head(&words("start dictation we meet at ten")),
            found(&[("start dictation", 0, 1)], Placement::Head)
        );
    }

    #[test]
    fn should_find_start_closed_by_please_with_nothing_after_it() {
        assert_eq!(
            standby().start_at_head(&words("Start please.")),
            found(&[("start dictation", 0, 1)], Placement::Head)
        );
        assert_eq!(
            standby().start_at_head(&words("Start dictation, please.")),
            found(&[("start dictation", 0, 2)], Placement::Head)
        );
    }

    #[test]
    fn should_not_find_start_without_speech_after_it_or_other_commands_at_the_head() {
        for text in [
            "Start.",
            "enter the building on the left",
            "clear the table",
            "starting tomorrow we meet",
        ] {
            assert_eq!(standby().start_at_head(&words(text)), None, "{text}");
        }
        assert_eq!(dictating().start_at_head(&words("start the report")), None);
    }

    #[test]
    fn should_find_a_doubled_short_form_anywhere() {
        assert_eq!(
            dictating().anywhere(&words("we move to Friday stop stop and more")),
            found(&[("stop dictation", 4, 5)], Placement::Anywhere)
        );
        assert_eq!(
            standby().anywhere(&words("fine, enter, enter, thanks")),
            found(&[("press enter", 1, 2)], Placement::Anywhere)
        );
        assert_eq!(
            dictating().anywhere(&words("forget it cancel cancel")),
            found(&[("cancel dictation", 2, 3)], Placement::Anywhere)
        );
        assert_eq!(
            standby().anywhere(&words("Clear. Clear.")),
            found(&[("clear all text", 0, 1)], Placement::Anywhere)
        );
        assert_eq!(
            standby().anywhere(&words("start start")),
            found(&[("start dictation", 0, 1)], Placement::Anywhere)
        );
    }

    #[test]
    #[expect(clippy::too_many_lines, reason = "size baseline")]
    fn should_find_a_chain_of_commands_anywhere() {
        assert_eq!(
            dictating().anywhere(&words("that is it, stop, enter.")),
            found(
                &[("stop dictation", 3, 3), ("press enter", 4, 4)],
                Placement::Anywhere
            )
        );
        assert_eq!(
            dictating().anywhere(&words("stop enter start")),
            found(
                &[
                    ("stop dictation", 0, 0),
                    ("press enter", 1, 1),
                    ("start dictation", 2, 2)
                ],
                Placement::Anywhere
            )
        );
        assert_eq!(
            dictating().anywhere(&words("stop dictation press enter please")),
            found(
                &[("stop dictation", 0, 1), ("press enter", 2, 4)],
                Placement::Anywhere
            )
        );
        assert_eq!(
            dictating().anywhere(&words("cancel clear")),
            found(
                &[("cancel dictation", 0, 0), ("clear all text", 1, 1)],
                Placement::Anywhere
            )
        );
        assert_eq!(
            dictating().anywhere(&words("stop clear enter")),
            found(
                &[
                    ("stop dictation", 0, 0),
                    ("clear all text", 1, 1),
                    ("press enter", 2, 2)
                ],
                Placement::Anywhere
            )
        );
        assert_eq!(
            standby().anywhere(&words("ok enter start")),
            found(
                &[("press enter", 1, 1), ("start dictation", 2, 2)],
                Placement::Anywhere
            )
        );
    }

    #[test]
    fn should_not_find_chains_the_states_do_not_allow_or_single_commands() {
        for text in [
            "start stop",
            "start cancel",
            "enter stop",
            "stop start stop",
        ] {
            let found = standby().anywhere(&words(text));
            assert!(
                found
                    .as_ref()
                    .is_none_or(|f| !f.commands.contains(&"stop dictation".to_owned())),
                "{text}: {found:?}"
            );
        }
        assert_eq!(standby().anywhere(&words("enter stop")), None);
        assert_eq!(dictating().anywhere(&words("clear enter")), None);
        assert_eq!(
            dictating().anywhere(&words("enter clear")),
            found(
                &[("press enter", 0, 0), ("clear all text", 1, 1)],
                Placement::Anywhere
            )
        );
        for text in [
            "we should stop the server",
            "stop",
            "the shop shop",
            "clean clean",
            "",
        ] {
            assert_eq!(dictating().anywhere(&words(text)), None, "{text}");
            assert_eq!(standby().anywhere(&words(text)), None, "{text}");
        }
    }

    #[test]
    fn should_find_paste_like_the_other_standby_commands() {
        assert_eq!(
            standby().at_end(&words("okay, paste.")),
            found(&[("paste text", 1, 1)], Placement::Tail)
        );
        assert_eq!(
            standby().anywhere(&words("fine paste paste")),
            found(&[("paste text", 1, 2)], Placement::Anywhere)
        );
        assert_eq!(
            standby().anywhere(&words("paste enter")),
            found(
                &[("paste text", 0, 0), ("press enter", 1, 1)],
                Placement::Anywhere
            )
        );
        assert_eq!(
            dictating().anywhere(&words("stop paste text enter")),
            found(
                &[
                    ("stop dictation", 0, 0),
                    ("paste text", 1, 2),
                    ("press enter", 3, 3)
                ],
                Placement::Anywhere
            )
        );
        assert_eq!(dictating().at_end(&words("we should paste")), None);
        assert_eq!(
            standby().at_end(&words("I will paste the link later")),
            None
        );
    }

    #[test]
    fn should_find_restart_like_the_other_commands() {
        assert_eq!(
            dictating().at_end(&words("let me say it again restart")),
            found(&[("restart dictation", 5, 5)], Placement::Tail)
        );
        assert_eq!(
            dictating().anywhere(&words("wait restart restart")),
            found(&[("restart dictation", 1, 2)], Placement::Anywhere)
        );
        // After "cancel" no dictation runs, so a "restart" after it is no chain.
        assert_eq!(dictating().anywhere(&words("cancel restart")), None);
        assert_eq!(dictating().anywhere(&words("restart stop")), None);
        assert_eq!(dictating().at_end(&words("we start")), None);
    }

    #[test]
    fn should_not_find_a_command_followed_by_a_filler_word() {
        assert_eq!(standby().at_end(&words("Please clear the...")), None);
        assert_eq!(dictating().at_end(&words("we had to stop the")), None);
    }

    #[test]
    fn should_find_start_as_all_said_so_far_in_a_running_utterance() {
        assert_eq!(
            standby().start_so_far(&words("Start dictation.")),
            found(&[("start dictation", 0, 1)], Placement::Head)
        );
        assert_eq!(standby().start_so_far(&words("start the meeting")), None);
        assert_eq!(dictating().start_so_far(&words("start")), None);
    }

    #[test]
    fn should_tell_restart_dictation_from_start_dictation() {
        assert_eq!(
            dictating().at_end(&words("Let me try once more, restart dictation.")),
            found(&[("restart dictation", 5, 6)], Placement::Tail)
        );
        assert_eq!(
            standby().at_end(&words("Okay, start dictation.")),
            found(&[("start dictation", 1, 2)], Placement::Tail)
        );
    }

    #[test]
    fn should_find_no_command_in_start_stop() {
        let said = words("Okay, start stop");
        assert_eq!(standby().anywhere(&said), None);
        assert_eq!(standby().at_end(&said), None);
        assert_eq!(standby().start_at_head(&words("start stop")), None);
        assert_eq!(standby().start_so_far(&words("start stop")), None);
    }

    #[test]
    fn should_take_stuck_dictation_as_start_dictation() {
        assert_eq!(
            standby().start_at_head(&words("Stuck dictation, the build is green again.")),
            found(&[("start dictation", 0, 1)], Placement::Head)
        );
    }

    #[test]
    fn should_take_and_restart_after_a_command_as_enter_start() {
        // "enter start" said quickly is heard as "and restart"
        assert_eq!(
            dictating().anywhere(&words("That is all. Stop and restart.")),
            found(
                &[
                    ("stop dictation", 3, 3),
                    ("press enter", 4, 4),
                    ("start dictation", 5, 5)
                ],
                Placement::Anywhere
            )
        );
        assert_eq!(
            dictating().anywhere(&words("we stop and restart the server")),
            None
        );
    }
}
