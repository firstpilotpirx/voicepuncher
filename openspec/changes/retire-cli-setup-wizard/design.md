# Design

## Context

`SetupWizard` runs on the first launch and on `--setup` (dialoguer prompts), writes `config.toml`; `main.rs` exits 2 for missing permissions and silent services; the README is written for a developer with Homebrew and `just`. After `permissions-gate-window`, `settings-window`, `model-download` and `local-services-runtime`, every one of those paths has a window or an automatic step.

## Decisions

**D1 — An embedded, commented default template instead of questions.** The first launch writes `default.toml` (every key, commented, with the defaults) and goes on; the Settings window is where choices are made. The template doubles as documentation and is tested to equal `Config::default()` and to list every key.

**D2 — Hotkey mode never exits for a recoverable condition.** Permissions, models and services are handled by windows and the runtime; the process exits only at Quit, for a wrong command line, or in the developer modes (file mode, headless download), with the codes written down and tested.

**D3 — The command line stays, for developers and tests, documented as overrides.** File mode and the E2E harness need the flags; nothing of the GUI depends on them.

**D4 — The README is rewritten for the person who installs the DMG; developer matter moves under one section.** Install, first launch, use, settings, logs, uninstall, privacy, licenses first; developer last.

## Risks / Trade-offs

- [A developer used `--setup` to reset the config] → delete the file; the next launch recreates it (README § Developer says so).
- [The template drifts from `config.rs`] → two tests: parses to the defaults; every key present.
- [An exit path for permissions or services survives in `main.rs`] → the acceptance bar greps for `exit(2)` outside file mode and the component scenario "the terminal asks nothing" runs the runtime with every dependency missing.

## Test levels

| Scenario | Level |
|---|---|
| The template parses to the defaults and lists every key | unit tests in `config.rs` |
| A first launch without a configuration file creates it and reaches Ready; the terminal asks nothing with permissions, models or services missing | component `settings_window.feature` and `app_runtime.feature` (new scenarios, runtime with mocks) |
| Exit codes: wrong flag, file mode errors, headless download; `--help` lists the flags | CLI integration `cli_it.rs` |
| No wizard code or dependency remains | `just check` (clippy on unused deps is not enough: a test greps `Cargo.lock` for `dialoguer` and `console`) |
| Install and first launch by the README on a clean account | manual run by the user |
