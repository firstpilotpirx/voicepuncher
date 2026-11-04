# Tasks

## 1. Glued and mangled forms of "enter" read as the command

- [x] 1.1 Red: add `should_read_each_glued_or_mangled_enter_from_the_history` and `should_leave_ordinary_words_with_the_letters_of_enter` to `command-spotting/src/streaming/respell/tests.rs`; `should_hear_one_word_for_one_command_once_or_twice_in_an_alias_row` to `voice-commands/src/tests.rs`; `should_report_a_glued_enter_enter_the_russian_recognizer_writes_once` and `should_report_a_mangled_enter_the_russian_recognizer_writes_alone` to `command-spotting/src/streaming/session/tests/russian_forms.rs`; `just test-one` on each, see them fail
- [x] 1.2 Green: `MishearingKind::Alias` and the alias rows in `voice-commands/src/lib.rs`; the alias lookup before the command sounds and the hyphen dropping in `command-spotting/src/streaming/respell/mod.rs`; `just test-one` on the tests of 1.1 green
- [x] 1.3 Bump `trufflehog_version` in the `justfile` to the installed 3.99.8 so `_require-tools` passes; `just test` and commit

## 2. Version

- [x] 2.1 Bump `version` in `app/Cargo.toml` to 0.46.1; `just test`, `just check` and commit
- [x] 2.2 Manual checks: none (the change touches the command spotting service only)
