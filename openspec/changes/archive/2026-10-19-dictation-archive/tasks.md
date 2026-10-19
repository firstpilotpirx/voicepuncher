## 1. Glossary and port

- [x] 1.1 Add `Dictation archive` and `Archive entry` to `docs/ubiquitous-language.md`
- [x] 1.2 Add the `DictationArchive` port and `ArchiveEntry` with its enums in `app/src/core/application/dictation_archive.rs`; export it from `core/application/mod.rs`
- [x] 1.3 Run `just test` and commit

## 2. Filesystem archive adapter

- [x] 2.1 Red: unit tests for the UTC folder name and the `entry.json` shape in `app/src/adapters/secondary/archiving/filesystem/mod.rs`
- [x] 2.2 Green: write `audio.wav` and `entry.json` into a `.partial` folder and rename it; `-2` suffix for a taken name
- [x] 2.3 Red: "The oldest entries go first" in `app/tests/archiving_filesystem_it.rs`
- [x] 2.4 Green: prune the oldest entries until the archive fits its size limit, keeping the new one
- [x] 2.5 Run `just test` and commit

## 3. Whole-dictation audio from the recorder

- [x] 3.1 Red: unit test that the cpal recorder's `recording()` returns the last recording resampled to 16 kHz, lead-in included
- [x] 3.2 Green: add `AudioRecorder::recording` (default `None`) and implement it in the cpal recorder; the filesystem recorder returns the file's audio
- [x] 3.3 Run `just test` and commit

## 4. Pipeline archives every dictation

- [x] 4.1 Red: add `app/tests/features/component/dictation_archive.feature` with the component scenarios of design § Test levels and steps in `app/tests/steps/dictation_archive.rs`
- [x] 4.2 Green: trace a dictation from start to end in the pipeline (`with_archive`), close it with its outcome after the paste, drop it on cancel, restart and too-short presses
- [x] 4.3 Green: archive file mode in `run_once`
- [x] 4.4 Green: log a warning when the archive fails, nothing else changes
- [x] 4.5 Run `just test` and commit

## 5. Configuration, CLI and wiring

- [x] 5.1 Red: unit tests for `[archive]` defaults, `--archive-dir`, `--no-archive` and the retired `debug_save_wav` key in `config.rs` and `cli/mod.rs`
- [x] 5.2 Green: add `[archive]` config, the CLI flags, remove `--debug-save-wav` and the cpal WAV writer; update `cli_it.rs`, `config_loader_it.rs`, `config.example.toml` and the setup wizard
- [x] 5.3 Green: wire the archive in `main.rs` for live mode (on by default) and file mode (only with `--archive-dir`)
- [x] 5.4 Run `just test` and commit

## 6. End to end

- [x] 6.1 Red: add `e2e-test/tests/features/dictation_archive.feature` with the three recording-file scenarios and steps in `e2e-test/tests/steps/dictation_archive.rs`
- [x] 6.2 Green: make them pass
- [x] 6.3 Update README (archive folder, format, settings, privacy)
- [x] 6.4 Run `just test` and commit

## 7. Release

- [x] 7.1 Bump `version` in `app/Cargo.toml` to 0.22.0
- [x] 7.2 Run `just check` and commit
- [x] 7.3 Manual run by the user (at the manual run stage of the pull request): dictate with the hotkey and by voice, then check a new folder under `~/.voicepuncher/history/` holds `audio.wav` that plays the dictation and `entry.json` with the raw and pasted texts
