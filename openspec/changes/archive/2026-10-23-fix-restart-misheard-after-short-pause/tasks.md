## 1. Regression test under load

- [x] 1.1 Red: in `command-spotting/tests/it/placed_commands/under_load/mod.rs`, write `should_restart_before_a_short_pause_and_more_speech_under_load` (dictation phrases, the E2E scenario's speech with a 0.4 s pause after "Restart", confirmed by `loaded_copy`); run it with `Pace::budget` temporarily fixed at the 1 s floor and record that it fails (no "restart dictation"); revert the temporary edit
- [x] 1.2 Green: run it on `main`'s code and see it pass; `just test`
- [x] 1.3 Commit: `test(command-spotting): restart before a short pause under load`

## 2. Version

- [x] 2.1 Keep the version: test and spec only, no behavior change, no release (AGENTS.md § Release)
- [x] 2.2 Commit with the archive of the change

Red (1.1), 2026-10-23: with `Pace::budget` returning the fixed 1 s floor, the new test reported only speech segments (`[SpeechStart 1.03, SpeechEnd 3.72, SpeechStart 4.84, SpeechEnd 5.45, SpeechStart 5.96, SpeechEnd 9.85]`, no "restart dictation"), as did #288's two under-load tests; with `main`'s budget it passes (13 s).
