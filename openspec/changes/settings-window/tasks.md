# Tasks

## 1. Config document

- [ ] 1.1 Add `toml_edit` (latest 0.23.x, pinned exactly) to `app/Cargo.toml`; `cargo build -p voicepuncher` passes
- [ ] 1.2 Red: unit tests in `app/src/config/document.rs` — load, `set` changes one value and keeps comments and order, `set` in a missing section adds it, untouched `[[refinement.examples]]` round-trip, `to_config` rejects an invalid value; see them fail
- [ ] 1.3 Green: `ConfigDocument` (`load`, `load_or_default` for a missing file, `get`, `set`, `to_config` through the loader's checks, `save` atomic via temp file and rename); refactor `config::load` to share the string parsing
- [ ] 1.4 Red → Green: `app/tests/config_document_it.rs` — a commented real-world file changes only the edited line; a missing file is created with the edited key; the target is never half written
- [ ] 1.5 `docs/ubiquitous-language.md`: Config document; `just test` and commit

## 2. Field model

- [ ] 2.1 Red: unit test in `app/src/adapters/primary/settings/macos/fields.rs` that the keys of every `Deserialize` struct in `app/src/config.rs` and `app/src/config/*.rs` equal the fields' keys, each once; see it fail
- [ ] 2.2 Green: the field table (section, key, tab, label, control kind with range or options, advanced flag) for every key, including `debug_save_wav`, `transcription.model` and `refinement.enabled`
- [ ] 2.3 Red → Green: `validate` per field — ranges, loopback addresses through `config::ensure_loopback*`, folder exists or can be created — unit tests for each kind
- [ ] 2.4 Red → Green: value conversions control ↔ TOML (hotkey human names shared with the menu bar mapping, the whisper language table, sound names plus None, logging levels, comma-separated languages, examples table) — unit tests
- [ ] 2.5 Red → Green: `InputDevices` port in `app/src/core/application/input_devices.rs`, cpal implementation listing input device names (repeated names numbered), `recording_cpal_it.rs` case; microphone options = System default + devices, unit test with a fake port; `architecture.rs` stays green
- [ ] 2.6 Red → Green: override notes — given the file config and the effective config, the fields that differ and the effective value to show; unit tests for hotkey and refinement
- [ ] 2.7 `just test` and commit

## 3. Settings session (save, revert, apply)

- [ ] 3.1 Red: `app/tests/features/component/settings_window.feature` (`@settings_window @component`) with every non-visual scenario of the spec, steps in `app/tests/steps/settings_window.rs` driving the field model over a real document in a temp folder and the runtime with mock assembler; see them fail
- [ ] 3.2 Green: a GUI-free `SettingsSession` (field values, dirty flag, `save` → validate all, write changed keys, reload the runtime with the overlay; `revert`; `close` → needs question when dirty) and status messages from runtime states (Restarting → applying after the current dictation, Ready → applied, Failed → reason)
- [ ] 3.3 `docs/ubiquitous-language.md`: Settings window, Field validation; `just test` and commit

## 4. Window

- [ ] 4.1 Add the `objc2-app-kit` features the window needs (`NSTabViewController`, `NSGridView`, `NSPopUpButton`, `NSSwitch`, `NSSlider`, `NSStepper`, `NSTextView`, `NSScrollView`, `NSTableView`, `NSPathControl`, `NSOpenPanel`, `NSAlert`, `NSSound` …); build passes
- [ ] 4.2 `app/src/adapters/primary/settings/macos/mod.rs`: `SettingsWindowController` — toolbar tabs, one grid per tab from the field table, advanced disclosure, Save/Revert, status line, red outline on the failing field, unsaved-changes alert, one instance brought to front, layout per `mockups/design-spec.md`
- [ ] 4.3 Special controls: examples table with + and −, folder chooser and Show in Finder, Open Logs Folder, ▶ sound preview through `NSSound` directly
- [ ] 4.4 Red → Green: `app/tests/settings_macos_it.rs` — every one of the 14 system sound names resolves with `NSSound`; the window builds and closes headless without a crash (skipped without a window server)
- [ ] 4.5 Menu bar `Settings…` opens the window instead of the file (`menubar/macos`); its component scenario in `menu_bar_presence.feature` updated to the new meaning
- [ ] 4.6 `just test` and commit

## 5. Docs

- [ ] 5.1 `README.md` § Settings: every option with its tab, control and key; `docs/project-map.md`: `adapters/primary/settings/`, `config/document.rs`; `docs/architecture/overview.md` § Technology Choices: `toml_edit`, AppKit forms
- [ ] 5.2 agent-kit tech matrix PR (`tech-matrix-rust.md`, macOS Desktop: settings UI, `toml_edit`)
- [ ] 5.3 `just test` and commit

## 6. Version and manual checks

- [ ] 6.1 Bump `version` in `app/Cargo.toml` to 0.32.0 (corrected by `kit version-check`)
- [ ] 6.2 `just check` once, then commit
- [ ] 6.3 Manual run by the user: open Settings from the menu; every tab in light and dark matches the mockups; change the hotkey, the language and a sound, Save, dictate with the new hotkey; enter a minimum press of 5 → refused, file unchanged; switch refinement on and off, Save, dictate; paste the log

## Workflow follow-up

- Archive after verify; `menu-bar-presence` and `restartable-app-runtime` must be archived on `main` first (the menu-bar-presence delta modifies their spec).
