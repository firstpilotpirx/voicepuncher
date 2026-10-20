# Design

## Context

`main.rs` initialises `env_logger` with a custom `format` (`log_line`) and the level from `[logging] level`, writing to stderr. `env_logger` 0.11 supports `Target::Pipe(Box<dyn Write + Send>)`. The services' logs already live in `~/.voicepuncher/logs/`. No logging framework beyond `log` + `env_logger` is in the tech matrix for this crate.

## Decisions

**D1 — Keep `env_logger`, add a writer.** The format, filter and every call site stay; only the target changes. A rotating writer is ~60 lines with tests, smaller than adopting `tracing-appender` or `flexi_logger` and their configuration. Alternative `log4rs` — rejected, a YAML-configured framework for one file.

**D2 — Tee decided once at start.** Whether stderr is a terminal is checked once (`IsTerminal`); a terminal gets a copy of every line. No environment variable to force either side; file mode is the only stderr-only path, by its nature as a command line tool.

**D3 — Size rotation, 10 MB × 5, done on write.** The writer tracks the size and rotates before the write that would cross the limit, so no line is split across files. Numbered suffixes, oldest deleted.

**D4 — Header and panic hook in `main`.** They belong to the process, not to an adapter; the panic hook chains to the default hook so the terminal still shows the panic in development.

**D5 — One folder function for everybody.** `logs_dir()` next to the config path resolution; the menu item and the service runtime use it, so there is one place to change if the folder ever moves to `~/Library/Logs`.

## Risks / Trade-offs

- [The writer's size counter drifts from the file (another process appends)] → nothing else writes this file; the counter is initialised from the file length at open.
- [A full disk makes writes fail] → write errors are swallowed after one stderr warning; the app keeps dictating.
- [Logging from the panic hook while the logger is poisoned] → `log` is lock-free at the call site; `env_logger`'s writer mutex is not held during a panic in app code.

## Test levels

| Scenario | Level |
|---|---|
| Rotation at the limit, shifting `.1`…`.5`, oldest deleted, continuing, reopen keeps size | unit tests in `logging/file/mod.rs` |
| The log file holds the session's lines in hotkey mode; file mode writes no file | adapter integration `logging_file_it.rs` (runs the binary with a temp HOME: `--input-file` leaves no log; a hotkey-mode start with a missing service exits 2 and leaves the header and the error in the file) |
| A terminal gets the lines too; no stderr without a terminal | adapter integration `logging_file_it.rs` (child process with and without a pty: with `script -q` for the pty case) |
| Header line content | adapter integration (same test) |
| A panic is logged | unit test with a child thread panic and a test logger capturing the hook's line |
| A real session's log from a Finder-like launch | manual check by the user |
