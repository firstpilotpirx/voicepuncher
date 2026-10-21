# Design

## Context

`SetupWizard` runs on the first launch and on `--setup` (dialoguer prompts), writes `config.toml`; `main.rs` exits 2 for missing permissions and silent services; the README is written for a developer with Homebrew and `just`. After `permissions-gate-window`, `settings-window`, `model-download` and `local-services-runtime`, every one of those paths has a window or an automatic step.

## Decisions

**D1 — An embedded, commented default template instead of questions.** The first launch writes `default.toml` (every key, commented, with the defaults) and goes on; the Settings window is where choices are made. The template doubles as documentation and is tested to equal `Config::default()` and to list every key.

**D2 — Hotkey mode never exits for a recoverable condition.** Permissions, models and services are handled by windows and the runtime; the process exits only at Quit, for a wrong command line, or in the developer modes (file mode, headless download), with the codes written down and tested.

**D3 — The command line stays, for developers and tests, documented as overrides.** File mode and the E2E harness need the flags; nothing of the GUI depends on them.

**D4 — The README is rewritten for the person who installs the DMG; developer matter moves under one section.** Install, first launch, use, settings, logs, uninstall, privacy, licenses first; developer last.

**D5 — The long refinement texts stay built in.** `refinement.system_prompt` and `refinement.examples` are named in a comment of the default file, not copied: a copy would freeze them at the version that wrote the file. The retired `audio.debug_save_wav` is left out too. The key test lists these three exemptions by name.

**D6 — File mode reads the recording before it asks a service.** An unreadable recording ends with 1 without any service, so the exit code test needs no service; a service that does not answer still ends with 2.

**D7 — The default file is written in every mode but the headless download**, before the configuration is read; `--config /dev/null` (the E2E harness) exists and is never written.

## Dependencies

Stacked on `model-download` (`kit stack`): its Download window and models gate complete the first launch this change documents; it merges first.

## Risks / Trade-offs

- [A developer used `--setup` to reset the config] → delete the file; the next launch recreates it (README § Developer says so).
- [The template drifts from `config.rs`] → two tests: parses to the defaults; every key present.
- [An exit path for permissions or services survives in `main.rs`] → the acceptance bar greps for `exit(2)` outside file mode and the remaining `exit(2)` paths in hotkey mode are start failures (services start, signal setup, runtime start) and `[services] manage = false`, not a missing dependency; `cli_it.rs` proves a first launch asks nothing.

## Test levels

| Scenario | Level |
|---|---|
| A first launch writes the default file | unit test of `config::ensure_exists`; CLI integration `cli_it.rs` (binary with a missing `--config`) |
| The default file holds the defaults | unit test in `config.rs` |
| The default file names every option | unit test in `config.rs` (keys of `Config::default()` serialized, minus the three exemptions of D5) |
| An existing file is kept | unit test of `config::ensure_exists` |
| A first launch asks nothing | CLI integration `cli_it.rs` (stdin closed, missing `--config`, unreadable recording: file written, exit 1) |
| The setup option is gone | CLI integration `cli_it.rs` |
| The help lists the developer options | CLI integration `cli_it.rs` (help text snapshot of the flag list) |
| An unknown option exits with 2 | CLI integration `cli_it.rs` |
| File mode without its service exits with 2 | CLI integration `cli_it.rs` (refused port) |
| File mode with an unreadable recording exits with 1 | CLI integration `cli_it.rs` |
| No wizard code or dependency remains | CLI integration `cli_it.rs` reads `Cargo.lock` |
| Install and first launch by the README on a clean account | manual run by the user |
