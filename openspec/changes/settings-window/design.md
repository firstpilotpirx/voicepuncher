# Design

## Context

`config.rs` deserialises `config.toml` into `Config` (`Deserialize` only), `config::load` reads the file, `validate` checks loopback addresses; `Cli::overlay` applies flags. `restartable-app-runtime` gives `Runtime::reload` and observable states; `menu-bar-presence` the menu item and the AppKit technique (`define_class!`, main-thread dispatch). `SetupWizard` (dialoguer) still exists. The refinement on/off lives only in `--no-llm`.

## Decisions

**D1 — Edit the file with `toml_edit`, never rewrite it from the struct.** The user edits `config.toml` by hand too (comments, grouping); a serialised struct would erase that. `ConfigDocument` changes only the edited keys and parses the result through the existing loader, so validation stays in one place. Alternative: `Config: Serialize` + `toml::to_string_pretty` — rejected for the loss of comments and order.

**D2 — A pure field model drives the window.** Every field (section, key, control kind, range, label, tab) is data in `fields.rs`; the window is built from it, validation runs on it, and a test checks that the set of fields equals the set of keys in `config.rs` (parsed from the `Deserialize` structs by a small reflection test over the file text). The GUI code becomes a generic builder plus a few special tabs (sounds, examples table).

**D3 — Explicit Save, applied through the runtime.** Save writes, then `Runtime::reload`; the runtime's Restarting/Ready states feed the status line. No live apply on each keystroke: a reload restarts the pipeline. The window never touches the pipeline directly.

**D4 — `[refinement] enabled` as the configuration twin of `--no-llm`.** The flag stays as the per-run override (E2E and debugging); the key is what the switch edits; the wiring takes the passthrough refiner when either says off.

**D5 — Input devices through a port.** `InputDevices` in the application layer, implemented by the cpal adapter, keeps `cpal` out of the GUI adapter (architecture rule: adapters do not import other adapters' technology).

**D6 — AppKit forms in code: `NSTabViewController` + `NSGridView`.** The toolbar tab style is the macOS 26 settings look; grids give aligned labels for free; no nib files, no SwiftUI. Controls are standard so light/dark, accessibility and keyboard navigation come from the system.

**D7 — One window instance owned by the runtime wiring.** Created lazily on first open on the main thread, kept for the process life, shown with `makeKeyAndOrderFront` and `NSApp.activate`.

## Risks / Trade-offs

- [`toml_edit` formatting differs slightly from the user's for edited lines] → only edited keys change; a round-trip test pins the format of the examples array.
- [A reload fails after the file was written (service address wrong)] → the status line shows the error; the file keeps the value (the user asked for it); Revert does not undo it — the user corrects the field and saves again.
- [Big tabs on small screens] → the window is 560 pt wide and at most 620 pt high; advanced fields sit under a disclosure.
- [The device list is read on the main thread (cpal enumerates quickly)] → read once per window open.

## Test levels

| Scenario | Level |
|---|---|
| Comments, order and unknown keys survive; examples round-trip; atomic save | unit tests in `document.rs`, adapter integration `config_document_it.rs` |
| Every configuration key has a field with the right control and range | unit test in `fields.rs` (keys parsed from `config.rs`) |
| Validation per field (ranges, loopback, folder) | unit tests in `fields.rs` |
| Saving applies without a relaunch; waits for a dictation and says so | component `settings_window.feature` (field model + real document + runtime with mocks) |
| An invalid value is refused and nothing is written | component `settings_window.feature` |
| Revert; closing with unsaved edits asks; overrides shown | component `settings_window.feature` |
| Refinement off by configuration pastes raw text | component `text_refinement.feature` (new scenario) |
| Sound preview plays the chosen sound | adapter integration `settings_macos_it.rs` (`NSSound` named lookup for all 14), manual |
| The window's look, every tab, light and dark | manual run by the user against the mockups |
