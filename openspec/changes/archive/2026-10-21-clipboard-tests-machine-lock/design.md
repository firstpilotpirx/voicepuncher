## Context

See proposal.md — Why. `app/tests/textpasting_macos_it.rs` writes the real general pasteboard, which macOS keeps once per machine. Its nextest test group `pasteboard` (`.config/nextest.toml`) runs its tests one at a time inside one nextest run only; a `just test` or `just full-check` of another checkout runs the same binary at the same moment and writes the pasteboard between a test's write and its read. The only other code that writes the real pasteboard in a test is the `@real_keyboard` step "the app pasted text into the focused app" (`app/tests/steps/push_to_talk.rs`), run by hand through the `real-keyboard` profile. Unit tests in `app/src/adapters/secondary/textpasting/macos/` never reach the pasteboard (the untrusted test fails before the snapshot), and the E2E steps use a test paster.

## Goals / Non-Goals

**Goals:** every test that writes the real pasteboard holds one machine-wide lock while it does, so runs from parallel checkouts take turns; a second run waits instead of failing; `just test` never waits for the model machine lock.

**Non-Goals:** no change to the app's paste code; no retry, no loosened assertion, no raised time limit; other flaky tests.

## Decisions

**D1 — A dedicated pasteboard file lock, not the machine-wide test lock.** A test helper takes an exclusive `flock` on `~/.cache/voicepuncher/pasteboard.lock` (directory created when missing) and holds it until it is dropped. The pasteboard tests last under a second each, so a waiter waits seconds, while the model machine lock is held for minutes by `check` and `full-check`: taking that one would make `just test` wait behind model runs, which the proposal rules out. Rejected: the machine-wide test lock (the reason above); a lock directory with `mkdir` (a killed test leaves it behind, while the kernel drops an `flock` when its process dies, including a nextest TIMEOUT kill).

**D2 — The lock is taken first and released last.** Each pasteboard test takes the lock before it saves the user's pasteboard, and the guard is declared first so Rust drops it after the saved pasteboard is written back; the paster's delayed restore runs inside the test's wait, so it too stays inside the lock. The nextest test group stays: it keeps one run's pasteboard tests from occupying test slots while they wait for each other.

**D3 — A waiting run says so.** When the lock is busy, the helper prints one line to stderr (`waiting for the pasteboard lock <path>`) and then blocks. A hung holder is bounded by the holder's own nextest limit (2 min); when it is killed, the kernel frees the lock.

**D4 — One helper for both test binaries.** The helper lives in `app/tests/support/pasteboard_lock.rs`, a module `textpasting_macos_it` and the `cucumber_integration` harness (for the `@real_keyboard` paste step) both declare; it takes the lock path as a parameter for its own test, with the default path for every other caller.

**D5 — The lock's own test is the Red test.** The cross-checkout race needs two runs at once and fails only sometimes, so the Red test proves the cause instead: while one handle holds the lock at a temporary path, a second handle does not get it within 200 ms and gets it within 2 s after the first is dropped. It fails against a helper that opens the file without locking it. The race itself is checked once by hand (task 2.4): two processes running the pasteboard tests in a loop at the same time.

**D6 — Wait for the restored items, not for the pasted text to go.** Verifying the lock found a second cause of the empty restored clipboard: the restore clears the pasteboard and then writes the saved items, and the two restore tests stopped waiting as soon as the pasted text was gone, so under load (load average 63–155, single process) they read the empty pasteboard in between — 3 of 40 single-process rounds and 5 of 40 two-process rounds failed with the lock alone. The two tests now poll until every copied item is back within the same `RESTORE_DEADLINE`, then assert as before; a restore that never comes still fails. Rejected: changing the restore to write without clearing first (app code, out of scope, and `writeObjects` needs a cleared pasteboard). Its Red step is a check by reading the code: the gap is too short to force without changing the app.

## Risks / Trade-offs

- [A run waits for another checkout's pasteboard tests] → a few seconds at most; the tests are short and serialized only among themselves.
- [A stale lock file] → the file stays but holds no lock once no process has it open; nothing to clean.
