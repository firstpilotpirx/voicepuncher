# Tasks

## 1. Russian by default

- [x] 1.1 Red: rule "Spoken language follows the configuration", scenario "Russian is the language when none is configured" in `app/tests/features/component/speech_recognition.feature` with its steps
- [x] 1.2 Red: E2E scenario "English speech stays English with the default language" in `e2e-test/tests/features/speech_recognition.feature` with its steps
- [x] 1.3 Red: unit test `should_default_language_to_russian` in `app/src/config/transcription/tests.rs`
- [x] 1.4 Green: default language `ru` in `app/src/config/transcription/mod.rs` and `app/src/config/default.toml`
- [x] 1.5 Set `auto` in the configuration of every test that relied on the old default (`app/tests/it/config/config_loader.rs`, E2E steps)
- [x] 1.6 `just test`; commit

## 2. 10-second chunks

- [x] 2.1 Red: rule "Stopping a dictation waits only for its last chunk" with scenarios "A long dictation is transcribed while it goes on" and "A short dictation is one chunk" in `app/tests/features/component/speech_recognition.feature` with their steps
- [x] 2.2 Green: default chunk threshold 10 s in `app/src/config/transcription/mod.rs` and `app/src/config/default.toml`
- [x] 2.3 Fix every test that relied on the 30 s default
- [x] 2.4 `just test`; commit

## 3. Language choice of three

- [x] 3.1 Red: rule "The language choice offers Russian, English and automatic detection" with its three scenarios in `app/tests/features/component/settings_window.feature` with their steps
- [x] 3.2 Green: the Languages choices offer Russian, English, Detect automatically in `app/src/adapters/primary/settings/macos/values/mod.rs`; update `values/tests.rs`
- [x] 3.3 `just test`; commit

## 4. Version and checks

- [x] 4.1 Bump `version` in `app/Cargo.toml` to 0.43.0
- [x] 4.2 `just test`; commit
- [ ] 4.3 Manual checks: one dictation of 20–30 s after "stop" logs `stop→paste: total` at most ~0.8 s (recording and paste timing changed)
