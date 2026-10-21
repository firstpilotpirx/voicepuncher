## Why

Every option of voicepuncher lives in `~/.voicepuncher/config.toml` (sections `[hotkey]`, `[audio]`, `[transcription]`, `[refinement]`, `[paste]`, `[sounds]`, `[voice_control]`, `[archive]`, `[logging]`) or in a command line flag (`--no-llm`, `--hotkey`, `--voice-control`, `--transcription-url`, `--refinement-url`, `--archive-dir`, `--no-archive`), and the first-run `SetupWizard` asks five of them in the terminal. A user of the application has no terminal and should not edit TOML. The user asked for a settings window with tabs that exposes every option — the hotkey, the recording mode, text refinement on or off, the models and folders, the sounds, the language — and applies it in place. `restartable-app-runtime` makes applying possible without a relaunch; this change is the window.

## What Changes

- `Settings…` (⌘,) in the menu (`menu-bar-presence`) opens one Settings window (the look is fixed by `mockups/design-spec.md`; no pictures were made, design D13): a toolbar-style tab bar as in System Settings on macOS 26 (`NSTabViewController` with the toolbar tab style, one SF Symbol per tab), 560 pt wide, content height per tab, `Save` and `Revert` buttons at the bottom, and a status line (`Saved`, `Applying after the current dictation…`, `Applied`, or the validation error).
- Tabs and their controls, each bound to one config key (right-aligned labels, `NSGridView` rows, system controls):
  1. **General** (`gearshape`): Hotkey — a pop-up with every `HotKey` in human words (`Right Option (⌥)`, `Right Control (⌃)`, `Right Shift (⇧)`, `Right Command (⌘)`, `Left Option`, `Left Control`, `Fn (🌐)`, `Caps Lock`, `Space`, `Option+E`, `Cmd+E`, `F13` … `F19`); Recording mode — radio buttons `Hold the key to record` / `Press once to start, again to stop` (`hold`/`toggle`); Minimum press — a text field with a stepper, milliseconds, 50–1000 (`min_press_ms`); Microphone — a pop-up of the input devices on the Mac plus `System default` (`[audio] input_device`); Longest recording — seconds, 10–3600 (`max_recording_seconds`);  Voice control — a switch (`[voice_control] enabled`) and, under a `Show advanced` disclosure, its service address (`url`). (`launch-at-login` adds its switch here.)
  2. **Dictation** (`waveform`): Language — a pop-up `Detect automatically` or a language (`[transcription] language`: `auto` or a code; the list of codes and names is the one whisper supports, kept in a mapping table); Languages to expect — a text field of comma-separated codes (`languages`); Prompt — a multi-line text view (`prompt`); Temperature — a slider 0–1 with the value (`temperature`); Timeout — seconds (`timeout_secs`); under `Show advanced`: Chunk threshold, Chunk overlap, VAD lookback (seconds), Model name sent to the service (`model`), Service address (`base_url`, loopback only).
  3. **Refinement** (`sparkles`): Refine text with the language model — a switch (`[refinement] enabled`, the key `refinement-off-by-default` adds, default off; `--llm` / `--no-llm` override it for the run and the switch then shows the note of overridden fields); Model — a text field (`model`); System prompt — a text view (`system_prompt`); Examples — a two-column table (`transcript`, `normalized`) with `+` and `−` buttons (`examples`); Temperature, Max tokens, Timeout; under `Show advanced`: Service address (`base_url`), API key (`api_key`).
  4. **Sounds** (`speaker.wave.2`): one row per event — Start, Stop, Cancel, Listening on, Listening off, Enter, Clear, Paste, Restart — each a pop-up of the 14 system sounds (Basso, Blow, Bottle, Frog, Funk, Glass, Hero, Morse, Ping, Pop, Purr, Sosumi, Submarine, Tink) plus `None`, and a ▶ button that plays it (`NSSound soundNamed:`).
  5. **Paste** (`doc.on.clipboard`): Clipboard restore delay — ms (`clipboard_restore_delay_ms`); Wait for a text field — seconds (`focus_wait_seconds`).
  6. **Archive** (`archivebox`): Keep every dictation — a switch (`[archive] enabled`); Folder — an `NSPathControl` with `Choose…` (`NSOpenPanel`, folders only) (`dir`); Size limit — MB (`max_size_mb`); `Show in Finder`.
  7. **Logging** (`doc.text`): Level — a pop-up `error / warn / info / debug / trace` (`[logging] level`); `Open Logs Folder`.
  (`model-download` adds a **Services** tab: models folder, copies, service status, downloads.)
- Save validates every field (ranges above; loopback addresses as `config::validate` does; a folder that exists or can be created), writes `config.toml` keeping the user's comments, key order and the format of untouched keys (edited with `toml_edit`, not rewritten from a struct; unknown keys cannot occur: the loader rejects them), then asks the runtime to reload; while a dictation is in progress the status line says `Applying after the current dictation…` (`restartable-app-runtime` waits for Idle), then `Applied`. A field that fails validation gets a red outline and the message in the status line; nothing is written. `Revert` reloads the fields from the file. Closing the window with unsaved edits asks `Save / Don't Save / Cancel` (`NSAlert`). The window is one instance: opening it again brings it to front.
- Found while planning, fixed here: `[audio] input_device` was never used (recording and command spotting always took the default device) — both now record from the named device, falling back to the default; the refinement switch is read on every reload, not only at start; a reload rejected because a service does not answer is reported to the window, not shown as applied. `[audio] debug_save_wav` is retired and gets no control.
- Values set by command line flags for this run (`--hotkey`, `--transcription-url`, `--refinement-url`, `--archive-dir`, `--no-archive`, `--llm`, `--no-llm`, `--voice-control`) show the file's value with a note `Set to X by the command line for this run` under the field; Save still writes the file.
- A missing `config.toml` is created with defaults on first Save (and, in `retire-cli-setup-wizard`, at first launch). The `SetupWizard` stays until that change; nothing in this change asks questions in a terminal.
- Docs: `README.md` § Settings — a table of every option with its tab, control and config key; `docs/ubiquitous-language.md` (Settings window, Config document, Field validation); `docs/project-map.md`; `docs/architecture/overview.md` § Technology Choices (config editing: `toml_edit`; GUI forms: AppKit `NSGridView`, `NSTabViewController`). Tech matrix (agent-kit PR, section "macOS Desktop"): settings UI — AppKit tabbed window via `objc2-app-kit` (`NSTabViewController`, `NSGridView`, `NSPopUpButton`, `NSSwitch`, `NSPathControl`, `NSTableView`); config editing — `toml_edit` for comment-preserving writes.

Current stage: PoC. Target stage: Prototype (every option reachable without a terminal or a text editor).
Scope: the window, seven tabs, every config key (with `[refinement] enabled` from `refinement-off-by-default`), validation, comment-preserving writes, apply through the runtime, unsaved-changes handling, the device list, docs, the design spec of the window.
Acceptance bar: every key of `config.rs` has a control (a test lists the keys and the bound controls); changing the hotkey and a sound, Save, and the next dictation uses them without a relaunch; a wrong value (`min_press_ms = 5`) is refused with a message and the file is untouched; the file's comments survive a Save (an integration test); the window matches `mockups/design-spec.md` in light and dark; the user pastes a healthy log after a manual run that changed the hotkey, the language and a sound through the window.
Out of scope: the Services tab and model downloads (`model-download`); launch at login (`launch-at-login`); removing the wizard and `--setup` (`retire-cli-setup-wizard`); a hotkey recorder that captures any key combination (the pop-up of supported keys is what the code supports); localisation (English UI).

## Capabilities

### New Capabilities

- `settings-window`: the window with a tab per configuration section, every option as a system control, validated, written to the configuration file with comments kept, applied without a relaunch.

### Modified Capabilities

- `menu-bar-presence`: `Settings…` opens the window instead of the file.

`text-refinement` and `app-runtime` keep their requirements: the configuration switch for refinement comes from `refinement-off-by-default`, and the runtime already reports Restarting while a reload waits for a dictation (`restartable-app-runtime`); this change only adds `Reloader::reload_reporting`, so the window hears the outcome of a reload (design D10).

## Requirements to specify

1. Every option has a control: each configuration key appears on its tab as a system control showing the file's value; a key the file does not set shows the default.
2. Saving applies without a relaunch: Save writes the file and the next dictation uses the new values; the process is the same.
3. An invalid value is refused: Save with a value outside its range or a non-loopback address writes nothing, marks the field and says why.
4. The user's file is preserved: a Save keeps comments, key order and the format of untouched keys of the file and changes only the edited keys.
5. Revert restores the file's values; closing with unsaved edits asks.
6. A reload waits for the dictation in progress and says so in the window.
7. Command line overrides are shown: a field overridden for this run says so and still edits the file.
8. (moved to `refinement-off-by-default`: the configuration switch for refinement.)
9. Sounds can be previewed: the ▶ button plays the chosen system sound.

## Implementation guide

1. **Mockups first.** `mockups/design-spec.md`: every tab, the error state, the alert, light and dark; shown with the proposal at stage 3 (no PNGs: design D13).
2. **Config document.** `app/src/config/document.rs`: `ConfigDocument::load(path)` (`toml_edit = "=0.25.11"`, the version Cargo.lock already holds), `get(section, key) -> Option<Item>`, `set(section, key, value)` (creates the table when missing, keeps everything else), `save(path)` (atomic: write `.tmp`, rename), `to_config() -> Result<Config>` (parse through the existing `config::load` logic on the document's string, so validation is the same). Unit tests: comments and order survive; a new key is added at the end of its section; the examples array of tables round-trips.
3. **Field model.** `app/src/adapters/primary/settings/macos/fields.rs` (pure, unit-tested): an enum of every field with its section, key, control kind (`Switch`, `PopUp(options)`, `Text`, `TextView`, `Number { min, max, step, unit }`, `Slider { min, max }`, `Path`, `Table(columns)`), label, help, tab; `validate(field, value) -> Result<(), String>`; conversions between control values and TOML values; the hotkey names table shared with the menu's status line (`menu-bar-presence` mapping); the whisper language table; the device list through a new `InputDevices` port (`core/application/input_devices.rs`, implemented by the cpal recording adapter: names of input devices).
4. **Window.** `adapters/primary/settings/macos/mod.rs`: `SettingsWindowController` (`define_class!` for the target object with `save:`, `revert:`, `choosePath:`, `playSound:`, `addExample:`, `removeExample:`, `toggleAdvanced:` and `windowShouldClose:`), the window with an `NSTabViewController` (toolbar style; `objc2-app-kit` features: `NSTabView`, `NSTabViewController`, `NSTabViewItem`, `NSViewController`, `NSGridView`, `NSGridRow`, `NSGridCell`, `NSPopUpButton`, `NSSwitch`, `NSSlider`, `NSStepper`, `NSTextView`, `NSScrollView`, `NSTableView`, `NSTableColumn`, `NSPathControl`, `NSOpenPanel`, `NSSavePanel`, `NSAlert`, `NSSound`, `NSToolbar`, `NSBox`, `NSStackView`); one builder function per tab that returns the grid and a list of (field, control) bindings; a `dirty` flag from control actions; the `Save` button enabled only when dirty.
5. **Apply.** On Save: collect values → validate every field (first error wins, field outlined with `NSColor::systemRed` layer border) → `document.set` for changed fields → `save` → `Runtime::reload(document.to_config() with CLI overlay)`; `Reloader::reload_reporting` reports `WaitsForDictation` (status line `Applying after the current dictation…`) then `Applied` or `Rejected(reason)` (design D10). Errors from reload (a service address that does not answer) show in the status line and the file stays saved (the user asked for it).
6. **Refinement switch.** Bound to `[refinement] enabled` from `refinement-off-by-default`; nothing to wire here.
7. **Docs** as listed; the agent-kit tech matrix PR.

Pitfalls: `NSTabViewController` with the toolbar style needs the window's `toolbar` to be managed by the controller — set the controller as the window's `contentViewController`, do not add a toolbar by hand; `NSGridView` rows need the label cells right-aligned (`xPlacement = trailing`) and a fixed first column width (160 pt) for all tabs so labels line up; `NSTextView` lives in an `NSScrollView` with a fixed height (90 pt); values must be read from controls on the main thread, then handed to the runtime on its thread — never block the main thread waiting for a reload; the examples table edits in place (`NSTableView` with editable text cells; the data source is the Rust side through a `define_class!` data source object or an `NSArrayController`-free manual reload); the sound preview must not go through the pipeline's `SoundPlayer` (which maps events), call `NSSound` directly; `toml_edit` writes `examples` as an array of tables — keep the existing format `[[refinement.examples]]`; the device names from cpal can repeat (two "External Microphone") — show them with an index.

## Impact

- new `app/src/config/document.rs`, `app/src/config.rs` (`[refinement] enabled`, module), `app/src/core/application/input_devices.rs`, `app/src/adapters/secondary/recording/cpal/` (device list)
- new `app/src/adapters/primary/settings/macos/{mod.rs,fields.rs,tabs/*.rs}`, `app/src/adapters/primary/mod.rs`, `app/src/adapters/primary/menubar/macos/` (Settings opens the window)
- `app/src/runtime/` (reload from the window; `enabled` → passthrough refiner), `app/Cargo.toml` (`toml_edit`, features), `Cargo.lock`
- tests: unit in `document.rs`, `fields.rs`; new `app/tests/config_document_it.rs` (comments survive, examples round-trip, atomic write), `app/tests/features/component/settings_window.feature` + `app/tests/steps/settings_window.rs` (fake controls through the field model, real document, runtime with mocks), a test that every config key has a field
- `README.md`, `docs/ubiquitous-language.md`, `docs/project-map.md`, `docs/architecture/overview.md`; agent-kit tech matrix (separate PR); `mockups/`
- `openspec/specs/menu-bar-presence` delta
- Checks: `just test` after every edit; `just check` after the last group; `just full-check` before the merge; manual run by the user (hotkey, recording, paste areas): change the hotkey, the language, a sound, switch refinement off and on, Save each time, dictate; the log pasted.

## Open points

- Save-and-apply (proposed) versus apply-on-change as System Settings does: proposed an explicit Save because a reload restarts the pipeline and reopens service connections.
- The hotkey control: a pop-up of supported keys (proposed) versus a key recorder (would need rdev to report arbitrary keys and a config format for them: a later change).
- Whether `api_key` belongs in the window at all (the local service ignores it): proposed under advanced, kept for compatibility.
- `toml_edit` versus `toml` serialisation (comments lost): proposed `toml_edit`.
- The width 560 pt and the tab icons: the mockups decide.

## Release

SemVer: MINOR — a new capability and a new config key; pre-1.0.
Target version: 0.31.0 (corrected at merge with `kit version-check`).
