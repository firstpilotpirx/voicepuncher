# Tasks

## 1. A quit from the system stops the owned services (design decisions 1–3)

- [x] 1.1 Red: E2E scenario "A quit from the system stops the owned services" in `e2e-test/tests/features/local_services.feature` (rule "The app starts the services it needs") with its steps in `e2e-test/tests/steps/local_services/app.rs` (quit sent with `NSRunningApplication terminate` by pid); show it red by hand with the debug binary (no log line, copy left); commit it alone (`test: reproduce …`, intentionally red).
- [x] 1.2 Green: `system_quit.rs` with `VPAppDelegate` and `SystemQuit::install`; log in `MacosMenuActions::quit`; `or_exit` in `main.rs` and the delegate installed there, `main.rs` not grown; show it green by hand.
- [x] 1.3 `cargo fmt --all`, `just lint`, `just test`; commit.

## 2. Version

- [x] 2.1 Bump `version` in `app/Cargo.toml` to 0.36.2. Manual check (quit from Activity Monitor, README § Release checklist) deferred to the batch's manual run.
