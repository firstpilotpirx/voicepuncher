## Why

The terminal was the user interface of the proof of concept: `SetupWizard` (`app/src/adapters/primary/setup/`, built on `dialoguer` and `console`) asks the hotkey, the recording mode, the language and the refinement choice on the first run and on `--setup`, and writes `~/.voicepuncher/config.toml`; the README explains installation through Homebrew, `cargo`, `just models` and `just start-model-servers`; a missing permission or service ends the process with exit code 2 and a line on stderr. After `permissions-gate-window`, `settings-window` and `model-download` the application has windows for all of that: the gate checks and requests permissions, the Settings window edits every option, the Download window fetches the models, and the services start by themselves (`local-services-runtime`). The user's decision: the console is no longer the user interface; every feature stays, reachable from the app. This change removes the wizard, makes the first launch work without any question in a terminal, and rewrites the user documentation for the application.

## What Changes

- `SetupWizard`, the `--setup` flag and the dependencies `dialoguer` and `console` are removed. A launch with no `~/.voicepuncher/config.toml` creates one from an embedded template (`app/src/config/default.toml`, `include_str!`) that lists every key of every section with a comment explaining it and its default — the same text the README's settings table uses — and logs `config: created ~/.voicepuncher/config.toml with the defaults`. The defaults are the built-in defaults (`Config::default()`): hotkey right Option, language `auto`, refinement off; the long refinement instruction and examples stay built in (named in a comment, not copied), so they keep improving with the app. The user changes them in the Settings window.
- The first launch of the installed app is: Gatekeeper's Open Anyway (documented) → the permissions gate window → the models download window → services start → the menu bar icon says Ready → the optional login item offer (`launch-at-login`). Nothing asks anything in a terminal; nothing is printed to a terminal except the log tee when stderr is a TTY (`file-logging`).
- Exit codes are redefined for a GUI app: the process exits non-zero only for a wrong command line (clap, 2), for file mode failures (`--input-file`: 1 for a transcription error, 2 for a service that does not answer — unchanged), and for `--download-models` (0 or 1). In hotkey mode a missing permission, model or service never ends the process: the windows and the menu handle it. README § Developer lists the codes.
- The command line keeps only what developers and tests need, documented under README § Developer: `--config PATH`, `--input-file`, `--output`, `--no-llm`, `--hotkey`, `--voice-control`, `--transcription-url`, `--refinement-url`, `--archive-dir`, `--no-archive`, `--download-models`, `--models-dir`, `--version`, `--help`. Each flag's help text says it overrides the configuration for this run.
- `README.md` is rewritten for the application: Install (download the DMG from the GitHub release, drag to Applications, Open Anyway once, what the Gatekeeper dialog looks like), First launch (permissions — why each is needed and what the gate shows; models — size and folder; the menu bar icon states), Using (hold the hotkey, the overlay stages, voice control and its commands, the archive), Settings (a table: option, tab, control, config key, default), Logs (the folder, `Open Logs Folder`, what to paste in a bug report), Uninstall (quit, remove the app, remove the login item, delete `~/.voicepuncher` to remove models and settings), Privacy (offline; the only network use is the models download), Licenses, Developer (clone, `just`, Homebrew engines for development, `just start-model-servers` with `[services] manage = false`, `cargo run --release -p voicepuncher`, the flags, the checks, the release checklist with the signing identity). The current § Testing and § Release checklist content moves under Developer.
- Docs: `docs/architecture/context.md` (the user acts through the menu bar and the windows; the terminal is for developers and file mode), `docs/project-map.md` (the setup adapter removed; the windows listed), `docs/ubiquitous-language.md` (retire Setup wizard and Preflight; add Onboarding — the gate, the download, the services start — First launch, Default configuration), `docs/architecture/overview.md` (the CLI row becomes "developer flags and file mode"; the primary adapters list), AGENTS.md § What this project is (the app is a menu bar application; the CLI remains for file mode and development) and § How to verify a change (the manual run is done through the installed app or `cargo run`, the log in `~/.voicepuncher/logs/`).
- The `push-to-talk` spec's scenario about the first-run wizard (if any names it) is removed; `settings-window` gets the requirement that a missing configuration file is created with the defaults at first launch.

Current stage: Prototype. Target stage: MVP for installation and first use (a person installs and uses the app with the README and no terminal).
Scope: removal of the wizard, `--setup`, `dialoguer`, `console`; the default template; exit code semantics; README rewrite; docs; AGENTS.md lines; spec deltas.
Acceptance bar: `rg -n "dialoguer|console|SetupWizard|--setup"` finds nothing in `app/`; a launch with no config file creates the template, parses it back to the defaults (a test), and reaches Ready; `cargo run --release -p voicepuncher --help` lists exactly the flags above; the README's install and first-launch sections are followed by the user on a clean account (or with `~/.voicepuncher` moved aside) and the app works; `just full-check` green.
Out of scope: moving `~/.voicepuncher` to `~/Library/Application Support` (deferred); an in-app "what's new" or help window; localisation; removing file mode or any developer flag.

## Capabilities

### Modified Capabilities

- `settings-window`: a missing configuration file is created with the documented defaults at first launch; the Settings window is the only interactive way to configure the app.
- `push-to-talk`: no first-run questions in a terminal; the hotkey defaults come from the configuration file.
- `file-transcription`: exit codes stated for file mode (unchanged values, now the only non-zero exits beyond the command line).

## Requirements to specify

1. A first launch without a configuration file creates one with the defaults: the file lists every option with its comment, parses to the defaults, and the app reaches Ready without any question.
2. The terminal asks nothing: in hotkey mode no prompt is ever written to stdin/stdout; a missing permission, model or service is shown in the app, and the process keeps running.
3. Exit codes are stable: a wrong command line exits 2; file mode exits 1 on a transcription error and 2 when its service does not answer; `--download-models` exits 0 when every file is present, 1 otherwise; hotkey mode exits 0 at Quit.
4. The command line is for developers and file mode only: every flag documents that it overrides the configuration for this run, and `--setup` is gone.

## Implementation guide

1. **Remove** `app/src/adapters/primary/setup/` and its tests, the `--setup` flag in `adapters/primary/cli/mod.rs`, `SetupWizard::run_if_needed` in `main.rs`, `dialoguer` and `console` in `app/Cargo.toml` (`cargo tree` afterwards must not list them), their entries in `docs/project-map.md` and the architecture test's module list.
2. **Default template** `app/src/config/default.toml`: every section and key of `config.rs` with `# comment` lines (meaning, unit, allowed values, default), values equal to the defaults; `config::ensure_exists(path) -> Result<Created | Existed>` writes it atomically (reuse `ConfigDocument::save`); a unit test parses the template with `config::load` and asserts equality with `Config::default()`; a second test checks that every key of `config.rs` appears in the template (the same key list as `settings-window`'s field test).
3. **Main flow.** `main.rs`: `ensure_exists` before `config::load`; the log line; hotkey mode never returns an error for permissions, models or services (those paths moved to the runtime in the earlier changes — verify none remains with `exit(2)`); file mode and `--download-models` keep their codes; a table of codes in README § Developer and a test per code (`cli_it.rs`: wrong flag → 2; `--input-file` with a dead service port → 2; `--download-models` against a local server with a missing file and no network → 1).
4. **README** rewrite as listed in What Changes, in this order of sections; keep the screenshots out (the mockups' PNGs are not committed to the README; one picture of the menu may be added under `docs/images/` when the user supplies a screenshot).
5. **Docs and AGENTS.md** as listed; the `push-to-talk` and `file-transcription` deltas; the `settings-window` delta with requirement 1.
6. **Version** bump to the target.

Pitfalls: `include_str!` paths are relative to the source file; keep the template in `app/src/config/`; the template must stay valid TOML with every key commented *and* present (commented-out keys would not prove the defaults — write them out, with the comment above each); the architecture test lists primary adapters — update it, or it fails for the removed module; clap's `--help` output is part of the acceptance bar — a test snapshots the flag list; do not delete the `cli` adapter's tests for the remaining flags; `AGENTS.md` lines about the manual run must still ask for the pasted log — only the way to run changes.

## Impact

- removed `app/src/adapters/primary/setup/`, `dialoguer`, `console` (`app/Cargo.toml`, `Cargo.lock`)
- new `app/src/config/default.toml`, `app/src/config.rs` (`ensure_exists`), `app/src/main.rs`, `app/src/adapters/primary/cli/mod.rs` (`--setup` removed, help texts), `app/tests/architecture.rs` (module list)
- tests: unit in `config.rs`; `app/tests/cli_it.rs` (exit codes, help snapshot); component `app/tests/features/component/settings_window.feature` (new scenario: first launch creates the file) and steps
- `README.md` (rewrite), `AGENTS.md`, `docs/architecture/{context,overview}.md`, `docs/project-map.md`, `docs/ubiquitous-language.md`
- `openspec/specs/settings-window`, `push-to-talk`, `file-transcription` deltas
- Checks: `just test` after every edit; `just check` after the last group; `just full-check` before the merge; manual run by the user: `~/.voicepuncher/config.toml` moved aside, launch the installed app, follow the README's First launch, dictate; the log pasted.

## Open points

- Whether to keep `--setup` as an alias that opens the Settings window (proposed no: the flag's meaning was the terminal wizard; a developer opens the window from the menu).
- The template's wording (proposed: one comment per key, the README's table generated by hand from it; a test keeps keys in step, not the wording).
- Whether `README.md` should carry pictures of the windows (proposed later, when the user supplies screenshots of the real app).

## Release

SemVer: MINOR — a capability is removed (the terminal wizard) and the first launch changes; pre-1.0.
Target version: 0.35.0 (stacked on `model-download`, 0.34.0).
