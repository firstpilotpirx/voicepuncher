# Tasks

## 1. Rotating file writer

- [ ] 1.1 Red: unit tests in `app/src/adapters/secondary/logging/file/mod.rs` for `RotatingFileWriter` — writes below the limit stay in one file, a write that would pass the limit rotates to `.1` first with the line whole, five backups shift and the oldest is deleted, reopening counts the existing length, the folder is created when missing; run them and see them fail
- [ ] 1.2 Green: `RotatingFileWriter::open(path, max_bytes, keep)` implementing `Write` with size tracking, rename chain and reopen after rotation; `logs_dir(home)` in the same module; `pub mod logging` in `app/src/adapters/secondary/mod.rs`; tests pass
- [ ] 1.3 Red: `app/tests/logging_file_it.rs` — rotation through a real file in a temp dir across two writer instances (the second continues the count), and `logs_dir` resolves to `<home>/.voicepuncher/logs`; see it fail, then pass
- [ ] 1.4 `just test`, commit

## 2. Tee, wiring, header and panic hook

- [ ] 2.1 Red: unit tests for `TeeWriter` with a fake terminal sink — every write reaches the file and the terminal when a terminal is present, only the file without one, a file error after the first warning never fails a write; see them fail
- [ ] 2.2 Green: `TeeWriter` in `app/src/adapters/secondary/logging/file/mod.rs`; tests pass
- [ ] 2.3 Red: unit test for the panic hook — a child thread panics and a capturing logger receives `panic: <message> at <file>:<line> on thread <name>`; see it fail
- [ ] 2.4 Green: `install_panic_hook()` in the logging module chaining to the previous hook; test passes
- [ ] 2.5 Red: `app/tests/logging_file_it.rs` — the binary run in file mode with a temp HOME leaves no `.voicepuncher/logs`; the binary run in hotkey mode with a temp HOME, a written `--config` and a dead service port exits 2 and the log file's first line is the header (version, pid, config path, macOS version, arch) followed by the error; the same hotkey run under `/usr/bin/script -q` prints the header to the pty and under a closed stderr prints nothing; see them fail
- [ ] 2.6 Green: `main.rs` — hotkey mode builds `env_logger` with `Target::Pipe(TeeWriter)` over `RotatingFileWriter` at `logs_dir(home)/voicepuncher.log` (10 MB × 5), file mode keeps stderr; `install_panic_hook()` before the logger's first use; the header line through `log::info!` with the macOS version from `NSProcessInfo` (`objc2-foundation` feature `NSProcessInfo` in `app/Cargo.toml`); tests pass
- [ ] 2.7 `just test`, commit

## 3. Documentation

- [ ] 3.1 README § Logs after § Dictation archive: the path, `tail -n 200`, rotation, the terminal copy, file mode; AGENTS.md § How to verify a change: the user pastes the tail of `~/.voicepuncher/logs/voicepuncher.log` when the app was not started from a terminal; verify the documented commands run as written
- [ ] 3.2 Docs only, no behavior: `docs/ubiquitous-language.md`: "Log file" and "Log rotation"; `docs/architecture/overview.md` § Technology Choices logging row; `docs/project-map.md`: `adapters/secondary/logging/`; verify the architecture tests still pass
- [ ] 3.3 agent-kit `tech-matrix-rust.md` section "macOS Desktop": file logging row (`env_logger` + own rotating writer; no `tracing`, `log4rs`, `flexi_logger`), in a separate PR of the agent-kit repository; verify the PR exists
- [ ] 3.4 `just test`, commit

## 4. Version and checks

- [ ] 4.1 Bump `version` in `app/Cargo.toml` to 0.28.0 (`kit version-check file-logging` confirms), `just test`, commit
- [ ] 4.2 `just check` once after this group (fmt, clippy, tests, release build, E2E smoke, accuracy, refinement)
- [ ] 4.3 Manual checks by the user: `just start-model-servers`; `cargo run --release -p voicepuncher` from a terminal, one dictation, the healthy cycle on the terminal and in `~/.voicepuncher/logs/voicepuncher.log`; `nohup target/release/voicepuncher >/dev/null 2>&1 &`, one dictation, the header and the healthy cycle in the file; paste the file's tail

## Workflow follow-up

- Archive the change after verify; `just full-check` before the merge (build input changed); release 0.28.0.
