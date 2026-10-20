## Why

The app logs to stderr through `env_logger` with the line format `[time LEVEL thread target] message`. That is fine for a terminal and useless for an application bundle: a `.app` launched from the Finder has no terminal, so every log line of a user's session is lost, and the "healthy log" the manual run of every change relies on (`pipeline recv Pressed` → `chunk 1 | … transcribed` → `pasting…` → `stop→paste: … total=…ms`, AGENTS.md § How to verify a change) can no longer be pasted. The services already log to files in `~/.voicepuncher/logs/` (`whisper-server-8081.log`, `llama-server-8080.log`, written by the `justfile`); the app's own log belongs there too. The user asked that logging to a folder stays a feature of the GUI version.

## What Changes

- In hotkey mode the app writes its log to `~/.voicepuncher/logs/voicepuncher.log`, always, with today's line format. When stderr is a terminal (`cargo run`, `cargo watch`), the same lines also go to stderr, as today; when it is not (launched from the Finder, by launchd, or with stderr redirected), the file is the only output. File mode (`--input-file`) keeps logging to stderr only: it is a command line tool whose stdout is the transcript.
- The file rotates by size: when it passes 10 MB the app renames it to `voicepuncher.log.1` (shifting `.1` to `.2` … up to `.5`, deleting the oldest) and starts a new file; rotation is done by the app on write, no external tool. Services' logs rotate the same way once `local-services-runtime` writes them; this change exposes the rotating writer for it.
- Every launch starts with a header line: `voicepuncher 0.28.0 starting (pid 1234, config /Users/me/.voicepuncher/config.toml, macOS 27.0.1, arm64)`, so a pasted log tells which version and config produced it.
- A panic is logged before the process dies (`panic: MESSAGE at FILE:LINE` with the thread name) through a panic hook installed before anything else; every `std::process::exit` path in `main.rs` and the runtime logs its reason first (they already log through `log::error!`, which now reaches the file).
- `[logging] level` keeps its meaning (the filter for both outputs). No new config key; the folder is fixed (`~/.voicepuncher/logs/`), created when missing. The log line format is unchanged, so the E2E harness and the user's reading habits hold.
- README § Logs: where the log is, how to read the last dictation (`tail -n 200`), rotation; AGENTS.md § How to verify a change: the user pastes the tail of `~/.voicepuncher/logs/voicepuncher.log` when the app was not started from a terminal. The glossary gets "Log file" and "Log rotation".
- Tech matrix (agent-kit PR, `tech-matrix-rust.md`, section "macOS Desktop"): file logging — `env_logger` with `Target::Pipe` into an own size-rotating writer (no `tracing`, `log4rs` or `flexi_logger`: one logger, ~60 lines of rotation); `docs/architecture/overview.md` § Technology Choices: the logging row.

Current stage: PoC. Target stage: Prototype (a bundled app keeps a readable log).
Scope: the rotating file writer, the tee to a terminal, the header line, the panic hook, docs.
Acceptance bar: after a launch from the Finder (or with stderr closed) and one dictation, `~/.voicepuncher/logs/voicepuncher.log` holds the header line and the healthy cycle lines; a launch from a terminal prints the same lines to the terminal and the file; writing 11 MB rotates into `.1` and a new file; `just test` green; the user confirms by pasting the file's tail.
Out of scope: a log viewer window; log upload or crash reporting; changing the line format; per-service rotation policy beyond reuse of the writer (`local-services-runtime`); the "Open Logs Folder" menu item (`menu-bar-presence` adds it; it opens the same folder).

## Capabilities

### New Capabilities

- `app-logging`: the application's log file: written in hotkey mode, rotated by size, mirrored to a terminal when there is one, started with a header, panics logged.

### Modified Capabilities

- none.

## Requirements to specify

1. The log is written to a file in hotkey mode: every log line of a session is in `~/.voicepuncher/logs/voicepuncher.log`, in the existing line format, with the folder created when missing.
2. A terminal gets the log too: when stderr is a terminal, every line also goes there; when it is not, nothing is written to stderr.
3. File mode logs to stderr only: with `--input-file` no log file is written or touched.
4. The log rotates by size: when the file passes 10 MB, it becomes `.1`, older ones shift up to `.5`, the oldest is deleted, and logging continues in a new file without a lost line.
5. Every launch starts with a header: the first line of a session names the version, pid, config path, macOS version and architecture.
6. A panic is logged: a panic on any thread writes its message, file, line and thread name to the log before the process ends.

## Implementation guide

1. **Rotating writer.** `app/src/adapters/secondary/logging/file/mod.rs`: `RotatingFileWriter::open(path, max_bytes, keep)` implementing `std::io::Write`; on every `write` it checks the current size (kept in memory, initialised from the file's length), rotates when the next write would pass the limit (rename `.4`→`.5` … `.log`→`.1`, create a new file), then writes; `flush` flushes the file. Unit tests with a temp dir: rotation at the limit, shifting, deletion of the oldest, continuing after rotation, reopening an existing file keeps its size. The writer is `Send` (one file handle; `env_logger` serialises writes).
2. **Tee.** A `TeeWriter(file, Option<Stderr>)` in the same module: writes to the file and, when stderr is a terminal (`std::io::IsTerminal` on `std::io::stderr()`), to stderr too; errors on stderr are ignored, errors on the file are ignored after one warning to stderr (logging must never crash the app).
3. **Wiring.** In `main.rs`, hotkey mode: `env_logger::Builder::new().filter_level(…).format(log_line).target(env_logger::Target::Pipe(Box::new(tee))).init()`; file mode: as today (stderr). The log path comes from the same home resolution as the config path (`dirs::home_dir()`), `~/.voicepuncher/logs/voicepuncher.log`; expose the folder as one function used by the menu's Open Logs Folder (`menu-bar-presence`) and by `local-services-runtime`.
4. **Header and panic hook.** Right after the logger is initialised: `log::info!("voicepuncher {} starting (pid {}, config {}, macOS {}, {})", …)` with the macOS version from `sw_vers -productVersion` (or `NSProcessInfo::operatingSystemVersionString` through `objc2-foundation`, already a dependency — prefer it, no process spawn) and `std::env::consts::ARCH`. `std::panic::set_hook` that logs `panic: {info} on thread {name}` through `log::error!` then calls the previous hook; installed before the runtime starts.
5. **Docs.** README § Logs; AGENTS.md (healthy log pasted from the file when not started from a terminal); `docs/ubiquitous-language.md`; `docs/architecture/overview.md`; `docs/project-map.md` (`adapters/secondary/logging/`); the agent-kit tech matrix PR.

Pitfalls: `env_logger` takes the pipe target only before `init`, so the file must open before the first log line (the config error path before `init` still prints to stderr — keep `eprintln!` there); never log from inside the writer; rotation renames must not race the services' writers (different files); keep the file open across rotation by reopening the handle after the rename; the E2E smoke scenarios run the binary in file mode and must see no new file in `~/.voicepuncher/logs/` (a test asserts it with a temporary HOME).

## Impact

- new `app/src/adapters/secondary/logging/file/mod.rs`, `app/src/adapters/secondary/mod.rs`
- `app/src/main.rs` (logger target, header, panic hook, log folder function)
- new `app/tests/logging_file_it.rs` (writer rotation with a temp dir; a binary run in file mode writes no log file), unit tests in the writer
- `README.md`, `AGENTS.md`, `docs/ubiquitous-language.md`, `docs/architecture/overview.md`, `docs/project-map.md`; agent-kit tech matrix (separate PR)
- Checks: `just test` after every edit; `just check` after the last group; `just full-check` before the merge (build input); manual check by the user: launch from a terminal and from the Finder (`open target/release/voicepuncher` is not a bundle — use `nohup target/release/voicepuncher >/dev/null 2>&1 &` to simulate no terminal), dictate once, paste the tail of the file.

## Open points

- Rotation numbers: proposed 10 MB and 5 kept files (about 50 MB at most, weeks of use); alternative daily rotation (rejected: size is what fills a disk).
- Whether debug level should write to the file at all times regardless of `[logging] level` (two filters): proposed no, one level for both outputs, as today.
- Where the macOS version comes from: proposed `NSProcessInfo` (no process spawn).

## Release

SemVer: MINOR — a new capability (the log file); pre-1.0.
Target version: 0.28.0 (corrected at merge with `kit version-check`).
