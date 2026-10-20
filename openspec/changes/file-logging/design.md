# Design

## Context

`main.rs` initialises `env_logger` with a custom `format` (`log_line`) and the level from `[logging] level`, writing to stderr. `env_logger` 0.11 supports `Target::Pipe(Box<dyn Write + Send>)`. The services' logs already live in `~/.voicepuncher/logs/`. No logging framework beyond `log` + `env_logger` is in the tech matrix for this crate.

## Decisions

**D1 — Keep `env_logger`, add a writer.** The format, filter and every call site stay; only the target changes. A rotating writer is ~60 lines with tests, smaller than adopting `tracing-appender` or `flexi_logger` and their configuration. Alternative `log4rs` — rejected, a YAML-configured framework for one file.

**D2 — Tee decided once at start.** Whether stderr is a terminal is checked once (`IsTerminal`); a terminal gets a copy of every line. No environment variable to force either side; file mode is the only stderr-only path, by its nature as a command line tool.

**D3 — Size rotation, 10 MB × 5, done on write.** The writer tracks the size and rotates before the write that would cross the limit, so no line is split across files. Numbered suffixes, oldest deleted.

**D4 — Header and panic hook in `main`.** They belong to the process, not to an adapter; the panic hook chains to the default hook so the terminal still shows the panic in development.

**D5 — One folder function for everybody.** `logs_dir()` lives in the library crate beside the writer (`adapters/secondary/logging/file/`), not in `main.rs`, so `menu-bar-presence` and `local-services-runtime` import it; one place to change if the folder ever moves to `~/Library/Logs`.

**D6 — macOS version from `NSProcessInfo`.** `objc2-foundation` is already a dependency; its `NSProcessInfo` feature is turned on (a `Cargo.toml` line). No process spawn at start.

## Contract for other changes

The rotating writer implements `std::io::Write`, is `Send`, and survives rotation while open: `local-services-runtime` may hand one to a thread that pumps a child's stdout and stderr into it (a `Write` is not a `Stdio`; that pump is its own work). `logs_dir()` creates nothing; the caller creates the folder.

## Risks / Trade-offs

- [The writer's size counter drifts from the file (another process appends)] → nothing else writes this file; the counter is initialised from the file length at open.
- [A full disk makes writes fail] → write errors are swallowed after one stderr warning; the app keeps dictating.
- [Logging from the panic hook while the logger is poisoned] → `log` is lock-free at the call site; `env_logger`'s writer mutex is not held during a panic in app code.

## Test levels

| Scenario | Level |
|---|---|
| A session's lines are in the log file | adapter integration `logging_file_it.rs` (the binary in hotkey mode with a temp HOME and a dead service port exits 2; the file holds the header and the error) |
| A later session appends to the same file | adapter integration `logging_file_it.rs` (two writer instances on one file; the second continues the count) |
| Started from a terminal | adapter integration `logging_file_it.rs` (the binary under `/usr/bin/script -q`: the pty shows the header) |
| Started without a terminal | adapter integration `logging_file_it.rs` (the binary with stderr piped: nothing arrives on it) |
| Transcribing a file leaves no log file | adapter integration `logging_file_it.rs` (the binary in file mode with a temp HOME) |
| The limit is reached | unit tests in `logging/file/mod.rs` |
| Backups shift and the oldest goes | unit tests in `logging/file/mod.rs` |
| A session continues on an existing file | unit tests in `logging/file/mod.rs` (reopen counts the existing length) |
| The header line | adapter integration `logging_file_it.rs` (same hotkey run; the first line's fields) |
| A worker thread panics | unit test in `logging/file/mod.rs` (a child thread panic and a capturing logger) |
| A real session's log from a Finder-like launch | manual check by the user |
