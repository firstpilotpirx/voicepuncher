# Proposal

## Why

On 2026-10-15 the test binary `commandspotting_websocket_it` hung for 47 minutes on 21 s of CPU inside `just check` in a worktree. The check held the machine-wide test lock all that time, four other sessions queued behind it, and the session that had started the check never woke, because the command never exited. The cause was a bug on that branch (a sync held back until time passed while the client sent no audio), but nothing stopped the test that waited for it:

- No test has a time limit. `just test` runs every test binary to its end, the E2E recipes call plain `cargo test`, and the only limit is the one-hour backstop of `with-machine-lock` (`MACHINE_LOCK_TIMEOUT`).
- Waits inside the tests have no bound of their own. The command spotting client's wait for the answer to a sync (`events_for_sent_audio`), which the voice tests use, lasts as long as the service still answers pings; another test reads a WebSocket until the service closes it.

With several sessions running at once, one hung test stalls them all. The user wants it prevented mechanically: a hung test fails fast, by name, and frees the lock.

## What Changes

- **A time limit for every test.** The Rust test suites run through cargo nextest, configured in a checked-in `.config/nextest.toml`. By default a test is reported as slow after 30 s and stopped after four such periods (2 min). The suites that legitimately run long (`e2e-smoke`, `e2e-quick`, `e2e-bench`, `e2e-seams`, `e2e-resilience`, `refine-eval`) get longer limits of their own through overrides. A stopped test is reported as timed out under its name and fails the run, so the recipe exits non-zero and `with-machine-lock` frees the lock at once.
- **The Gherkin harnesses get a limit too.** `cucumber_integration` and `cucumber_e2e` are custom harnesses (`harness = false`): cucumber does not answer the libtest listing (`--list`) through which nextest finds tests, and the recipes pass the scenario selection (`--tags`) on the command line. design.md decides how they run under a limit; they do not stay unbounded.
- **Client tests fail on silence instead of hanging.** In the tests that talk to a service over WebSocket or HTTP (`commandspotting_websocket_it`, `commandspotting_client_it`, `commandspotting_reconnect_it`, `transcription_openai_it`, `textrefinement_openai_it`, `http_failure_it`, the command spotting service's `server_it`, the voice test helpers of `e2e-test`), every wait for a response gets a bound of its own, seconds rather than minutes, and fails with a message that names the service and what it waited for. These tests are synchronous (blocking `tungstenite` and `ureq` clients, threads and channels; nothing in the workspace uses tokio), so the bound is one a blocking wait allows (a deadline, a socket read timeout, `recv_timeout`) rather than `tokio::time::timeout`; design.md picks it.
- **Same entry points.** `just test`, `check`, `full-check`, the E2E recipes, `refine-eval`, `test-real-keyboard` and `release` keep their names and arguments and still run under `with-machine-lock`; only the recipe bodies that run tests change.
- **cargo-nextest becomes a required developer tool**, pinned to an exact version. A recipe that needs it and does not find it stops with the command that installs it, as the model guards do; README § Testing and AGENTS.md § How to verify a change say so.
- **The nextest config is a build input.** AGENTS.md's build inputs and `checks.build_inputs` in `.claude/kit.yaml` include `.config/nextest.toml`, so a branch that changes a limit runs `just check` and `just full-check`.

The tests and their bars stay as they are: no test is dropped, skipped, retried or loosened to fit a limit.

Decisions taken before design (user, 2026-10-15): cargo nextest with per-test limits in `.config/nextest.toml` and the default above; overrides for the slow suites; the `justfile` recipes and `with-machine-lock` stay the entry points; design only, no specs. The Rust tech matrix does not list cargo-nextest; the user asked for it, and approving this proposal confirms it.

## Capabilities

### New Capabilities

None. This is test tooling, not app behavior: `skip_specs: true`.

### Modified Capabilities

None.

## Scope

- **Current stage:** PoC. **Target stage:** PoC (test tooling only).
- **Acceptance bar:**
  - A deliberately hung test (one that waits forever; design.md says whether it stays as an opt-in probe or runs once and is removed) makes `just test` stop it at its limit plus nextest's grace period, name it as timed out and exit non-zero; the machine-wide test lock is free right after: a `just test` waiting in another checkout starts at once.
  - A deliberately hung scenario in an E2E recipe is stopped at its suite's longer limit; the run names its harness (and the scenario, if design.md limits scenarios one by one) and the recipe exits non-zero.
  - A client test whose service accepts the connection and then stays silent fails within its own bound, in seconds, with a message that names the service and what it waited for: shown for the command spotting WebSocket client and for an HTTP client.
  - `just test`, `just check` and `just full-check` pass on the final commit, running as many tests as before; the slow suites pass under their own limits.
  - Recipe names and arguments are unchanged, and every check still runs under `with-machine-lock`.
  - `just test` and `just check` keep their documented times (~20 s on a warm build, ~6 min), measured before and after on the same machine.
- **Manual run:** none; the app behaves as before.

### Out of scope

- Limits and stall detection for whole runs in `kit run`: the agent-kit change `stop-hung-runs`, which leaves limits per test to this change; neither waits for the other.
- `with-machine-lock` and its one-hour backstop.
- The branch bug behind the 2026-10-15 hang.
- Retrying failed or timed-out tests; splitting, merging or dropping tests; moving tests between `check` and `full-check`; loosening a bar.
- Limits for what is not a Rust test: builds, `cargo clean`, starting and stopping the services, `just bench` (Python), `just models`, `just corpus`.
- Teaching agent-kit's readers of check output the nextest format (see Impact), and adding cargo-nextest to the kit's Rust tech matrix: agent-kit changes of their own.
- Any change to the app, the services or their protocols.

## Release

No release: no app behavior changes; `version` in `app/Cargo.toml` stays as it is.

## Impact

- `justfile`: `_test` (its parallel binary loop becomes a nextest run), `_e2e-smoke`, `_e2e-quick`, `_e2e-seams`, `_e2e-bench`, `_e2e-resilience`, `_refine-eval`, `_test-real-keyboard`, and a guard that requires cargo-nextest (`_release` runs `_test`, so a release needs it too).
- New `.config/nextest.toml`.
- Tests: in `app/tests/` `commandspotting_websocket_it`, `commandspotting_client_it`, `commandspotting_reconnect_it`, `transcription_openai_it`, `textrefinement_openai_it`, `http_failure_it`; `command-spotting/tests/server_it.rs`; `e2e-test/src/voice_commands.rs` and the tests that wait through it (`voice_samples_it`, `voice_forms_it`); the harness entry points `app/tests/cucumber_integration.rs` and `e2e-test/tests/cucumber_e2e.rs`.
- `README.md` § Testing, `AGENTS.md` § How to verify a change, `.claude/kit.yaml`.
- A new tool on the developer machine: cargo-nextest (not installed today).
- Risks design.md settles:
  - One process per test: a costly setup that the tests of a binary share today (the command spotting engine that `engine_it` and `placed_commands_it` load once per binary) is paid per test, against the time bar.
  - Concurrency: nextest runs tests of different binaries at once, while the E2E recipes run their binaries one after another against the shared services today; the load on the services must not grow past what they serve reliably.
  - agent-kit reads check output: `kit run`'s failure summary and the test-results card understand libtest and cucumber output (`test … FAILED`, panics, `test result:` lines), not nextest's timed-out tests or its summary line. design.md keeps them working from this repo's side or names the agent-kit change that teaches them.
- App code: no path the app runs changes. `events_for_sent_audio` lives in the command spotting adapter, but only tests call it; if its wait is bounded there, the app's path stays as it is, though `.claude/kit.yaml` then flags a voice-control manual run by path alone.
