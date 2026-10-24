## 1. One declaration of the commands

- [x] 1.1 Red: `command-spotting/src/keywords/tests.rs` `should_have_a_keyword_line_for_every_declared_command_and_none_else` against a `voice_commands` crate that does not exist yet (fails to build)
- [x] 1.2 Green: add the `voice-commands` crate (phrases, forms, states, `accepted_after`) as a workspace member with `rust-version`, in `stage_inputs`, `recognition_paths`, the `test-steps` members list and `.config/analyzers.toml`; `command-spotting` takes `FORMS`, `STANDBY`, `DICTATING`, `accepted_after` from it
- [x] 1.3 Red: `app/tests/it/voice_commands/mod.rs` `should_list_the_declared_commands_in_each_state` and `should_strip_every_form_of_a_command_that_reaches_a_dictation` (fail: `COMMAND_FORMS` is private)
- [x] 1.4 Green: `voice-commands` as an app dev-dependency, `command_words::COMMAND_FORMS` public; prove each test fails on a planted drift, then remove the drift
- [x] 1.5 `just test`; commit

## 2. Mishearings and trailing words as data

- [x] 2.1 Red: `matcher/tests.rs` `should_read_every_alias_row_as_its_command` and `should_close_a_chain_with_an_after_command_row` over a `MISHEARINGS` table not there yet
- [x] 2.2 Green: `MISHEARINGS` (alias, near-miss and after-command rows, with recordings where they exist) and `TRAILING_WORDS` in `voice-commands`; `ALIASES`, `NEAR_MISSES`, `ends_with_and_restart`, `CUT_WORD_SECS` and the literal "please" go; `place` takes the cut from the trailing-word row
- [x] 2.3 Red/Green: `command-spotting/tests/it/mishearings/mod.rs` `should_read_each_recorded_mishearing_as_its_command` (`needs_voice_samples::needs_command_stt`), listed in `tests/it/main.rs`
- [x] 2.4 `just test`; commit

## 3. Matcher cleanups

- [x] 3.1 Red: `matcher/tests.rs` `should_read_the_words_of_a_cyrillic_transcript`
- [x] 3.2 Green: `words()` on Unicode letters; join the two `impl PhraseMatcher` blocks; `LONGEST_CHAIN` to the top of `placed.rs`
- [x] 3.3 `just test`; commit

## 4. Confirming split

- [x] 4.1 Move `confirming.rs` into `engine/confirming/` (`mod.rs`, `ask.rs`, `answer.rs`, `segment.rs`, `half_pause.rs`, `rehear.rs`, `place.rs`, `unheld.rs`) without changing a line of logic; `just test-one` on the engine and placed-command suites
- [x] 4.2 `Standing` in place of `Result<AskedJob, Vec<ServerEvent>>`; the matcher passed as `reading` instead of `mem::replace`; `clip_end: Option<usize>` in `place`; `JobContext::is_check` through `JobContext::kind`
- [x] 4.3 Split `apply_answer`, `close_segment` and `place` under the function budget; drop their `#[expect]` and entries from `size_baseline.toml`
- [x] 4.4 `just test`; commit

## 5. Session sub-states and the early start

- [x] 5.1 Group `Session` into `Jobs`, `Hearings`, `Utterances` and `Detector`; split `watch_running` under the function budget; drop their entries from `size_baseline.toml`
- [x] 5.2 Red: `protocol/tests.rs` `should_read_a_selection_with_and_without_early_start` and `command-spotting/tests/it/early_start/mod.rs` `should_not_search_the_start_early_when_the_app_says_no_early_start`
- [x] 5.3 Green: `early_start: Option<bool>` in `PhraseSelection`, used by the engine with the fallback to "start dictation is selected"; the app's client sends it; `STARTED_EARLY` leaves `confirming`
- [x] 5.4 One `ask_spotted_check` per frame (kept with a `WHY:` when an engine test changes), the stale comment above `raised`, `TRIGGERS_KEPT_SECS` to the top of `engine.rs`
- [x] 5.5 `just test`; `just lint`; commit
- [x] 5.6 Keep the version: a refactor without behavior change gets no release (proposal § Release)
