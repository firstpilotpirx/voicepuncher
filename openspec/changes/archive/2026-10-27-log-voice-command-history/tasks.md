## 1. Command history entries in the command spotting service

- [x] 1.1 Red: unit tests of the outcome (agreed, corrected, rejected, missed, none) in `command-spotting/src/history/`
- [x] 1.2 Green: the outcome and the entry type in `command-spotting/src/history/`
- [x] 1.3 Red: unit tests of writing an entry (audio.wav, entry.json) and of pruning the oldest entries in `command-spotting/src/history/`
- [x] 1.4 Green: the writer thread, entry writing and pruning in `command-spotting/src/history/`
- [x] 1.5 Red: E2E Rules "Every utterance with a command heard is kept in the command history", "The command history stays within its size limit", "Keeping the command history never costs a command" and the scenario "A service given no folder writes no entry" in `e2e-test/tests/features/voice_control.feature`, steps in `e2e-test/tests/steps/voice_control/spotting_service/`
- [x] 1.6 Green: the session recorder (spotted phrases, answered hearings, outgoing events, segment grouping, audio clip) in `command-spotting/src/engine/` and `command-spotting/src/history/`
- [x] 1.7 Green: `--command-history-dir` and `--command-history-max-mb` in `command-spotting/src/main.rs` and `EngineCfg`
- [x] 1.8 `just test` and commit

## 2. The app turns the command history on

- [x] 2.1 Red: component scenarios "The app's own services keep the command history by default" and "Command history turned off" in `app/tests/features/component/voice_control.feature`, steps in `app/tests/steps/`
- [x] 2.2 Green: `[archive] keep_commands` in `app/src/config/` and `default.toml`, its Settings row in `app/src/adapters/primary/settings/macos/table/mod.rs`
- [x] 2.3 Green: the catalog passes the history folder and limit to the command spotting copies in `app/src/adapters/secondary/localservices/catalog/mod.rs`
- [x] 2.4 Add command history, command history entry and the four outcomes to `docs/ubiquitous-language.md`
- [x] 2.5 `just test` and commit

## 3. Release

- [x] 3.1 Bump `version` in `app/Cargo.toml` to 0.39.0
- [x] 3.2 `just check` once (run as `just full-check`, the level `kit checks-needed` named)
- [x] 3.3 Manual checks: no manual-run area changed (`kit checks-needed`: none); the Settings row "Keep every voice command" is checked in code, not on screen, so a second app copy never runs beside the user's; after the release the user says a few commands and finds their entries in `~/.voicepuncher/command-history/`
