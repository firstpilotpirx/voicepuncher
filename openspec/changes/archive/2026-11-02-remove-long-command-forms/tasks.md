## 1. Forms and recognizers

- [x] 1.1 Red: `voice-commands` test that every command has exactly one one-word form; matcher tests that long forms are no commands
- [x] 1.2 Green: one-word `FORMS`, no long-form mishearings, no `Alias` kind in `voice-commands/src/lib.rs`
- [x] 1.3 Green: matcher without aliases and filler words; `command_at` takes the closest form; the streaming decide step without the two-word path
- [x] 1.4 Green: `keywords.txt` without long phrases; `keywords::select` counts trigger-served phrases; no empty phrase stream in the engine
- [x] 1.5 Green: `COMMAND_FORMS`, start, restart and paste forms without long forms in `app/src/core/domain/command_words/`
- [x] 1.6 `just test`, commit

## 2. Tests and docs

- [x] 2.1 Command spotting integration tests say one-word forms; long-form tests deleted
- [x] 2.2 App component features and steps say one-word forms
- [x] 2.3 E2E features and steps say one-word forms; the "clear text" rule removed
- [x] 2.4 `docs/ubiquitous-language.md`: the command word
- [x] 2.5 `just test`, commit

## 3. Release

- [x] 3.1 Bump `version` in `app/Cargo.toml` to 0.45.0
- [x] 3.2 `just check`, commit
- [x] 3.3 Manual checks: none
