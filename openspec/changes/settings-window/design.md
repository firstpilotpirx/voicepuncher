# Design

## Context

`config.rs` deserialises `config.toml` into `Config` (`Deserialize` only), `config::load` reads the file, `validate` checks loopback addresses; `Cli::overlay` applies flags. `restartable-app-runtime` gives `Runtime::reload` and observable states; `menu-bar-presence` the menu item and the AppKit technique (`define_class!`, main-thread dispatch). `SetupWizard` (dialoguer) still exists. `refinement-off-by-default` adds `[refinement] enabled` (default false) with `--llm` / `--no-llm` overriding it. Every config struct is `deny_unknown_fields`, so a file that loads has no unknown keys.

## Decisions

**D1 — Edit the file with `toml_edit`, never rewrite it from the struct.** The user edits `config.toml` by hand too (comments, grouping); a serialised struct would erase that. `ConfigDocument` changes only the edited keys and parses the result through the existing loader, so validation stays in one place. Alternative: `Config: Serialize` + `toml::to_string_pretty` — rejected for the loss of comments and order.

**D2 — A pure field model drives the window.** Every field (section, key, control kind, range, label, tab) is data in `fields.rs`; the window is built from it, validation runs on it, and a test checks that the set of fields equals the set of keys in `config.rs` (parsed from the `Deserialize` structs by a small reflection test over the file text). The GUI code becomes a generic builder plus a few special tabs (sounds, examples table).

**D3 — Explicit Save, applied through the runtime.** Save writes, then `Runtime::reload`; the runtime's Restarting/Ready states feed the status line. No live apply on each keystroke: a reload restarts the pipeline. The window never touches the pipeline directly.

**D4 — The refinement switch edits `[refinement] enabled` from `refinement-off-by-default`.** This change adds no key and no wiring for it; `--llm` / `--no-llm` are shown as command line overrides like the other flags. Alternative: own key with default true (the original plan) — rejected, the user turned refinement off by default in that change.

**D5 — Input devices through a port.** `InputDevices` in the application layer, implemented by the cpal adapter, keeps `cpal` out of the GUI adapter (architecture rule: adapters do not import other adapters' technology).

**D6 — AppKit forms in code: `NSTabViewController` + `NSGridView`.** The toolbar tab style is the macOS 26 settings look; grids give aligned labels for free; no nib files, no SwiftUI. Controls are standard so light/dark, accessibility and keyboard navigation come from the system.

**D8 — Overrides are computed, not remembered.** The window gets the file's `Config` and the effective one (after `Cli::overlay`); a field whose two values differ shows the note with the effective value. Save writes file values only and reloads through the same overlay, so the override stays for the run.

**D9 — Every key, including the rarely used ones.** `[audio] debug_save_wav` (General, advanced) and `[transcription] model` (Dictation, advanced) get controls too, so the key-coverage test needs no exception list.

**D7 — One window instance owned by the runtime wiring.** Created lazily on first open on the main thread, kept for the process life, shown with `makeKeyAndOrderFront` and `NSApp.activate`.

## Risks / Trade-offs

- [`toml_edit` formatting differs slightly from the user's for edited lines] → only edited keys change; a round-trip test pins the format of the examples array.
- [A reload fails after the file was written (service address wrong)] → the status line shows the error; the file keeps the value (the user asked for it); Revert does not undo it — the user corrects the field and saves again.
- [Big tabs on small screens] → the window is 560 pt wide and at most 620 pt high; advanced fields sit under a disclosure.
- [The device list is read on the main thread (cpal enumerates quickly)] → read once per window open.

## Test levels

| Scenario | Level |
|---|---|
| Settings opens the settings window (menu-bar-presence) | manual run; menu action wiring in `menu_bar_presence.feature` |
| Settings opens the window | manual run |
| Settings chosen again reuses the window | manual run; `settings_macos_it.rs` builds the window twice and gets one instance |
| A value from the file is shown | component `settings_window.feature` |
| An option the file does not set shows its default | component `settings_window.feature` |
| No option is left without a control | unit test in `fields.rs` (keys parsed from `config.rs` and `config/*.rs`) |
| The microphone choice lists the Mac's input devices | unit test in `fields.rs` with a fake `InputDevices`; cpal list in `recording_cpal_it.rs` |
| A new hotkey and sound apply after Save | component `settings_window.feature` (real document, runtime with mock assembler) |
| A missing configuration file is created on Save | component `settings_window.feature`; adapter integration `config_document_it.rs` |
| Save during a recording | component `settings_window.feature` |
| A minimum press too short is refused | component `settings_window.feature`; range unit tests in `fields.rs` |
| A service address on another machine is refused | component `settings_window.feature`; unit tests in `fields.rs` |
| Comments and order survive a Save | unit tests in `document.rs`; adapter integration `config_document_it.rs` |
| The refinement examples survive a Save | unit tests in `document.rs` |
| Revert drops an unsaved edit | component `settings_window.feature` |
| Closing with an unsaved edit asks first | component `settings_window.feature` (session reports the question); the alert itself manual |
| Closing without edits asks nothing | component `settings_window.feature` |
| A hotkey overridden on the command line | component `settings_window.feature` |
| Refinement switched off on the command line | component `settings_window.feature` |
| Previewing a sound | adapter integration `settings_macos_it.rs` (`NSSound` named lookup for all 14); playing heard in the manual run |
| The window's look, every tab, light and dark | manual run by the user against the mockups |
