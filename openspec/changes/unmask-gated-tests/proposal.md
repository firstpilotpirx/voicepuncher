## Why

The audit of 2026-10-22 found tests that look green and never run: five integration tests return `Ok` after `eprintln!("GATED…")` when an environment variable is unset, and no `justfile` recipe sets it — `VOICEPUNCHER_WINDOWS_IT` (`app/tests/menubar_macos_it.rs:147`, `settings_macos_it.rs:320`), `VOICEPUNCHER_REAL_AX_IT` (`focus_macos_it.rs:27`), `VOICEPUNCHER_REAL_MIC` (`recording_cpal_it.rs:31`), `VOICEPUNCHER_TEST_WAV` (`recording_filesystem_it.rs:106, 123`). Nineteen more sites pass "GATED" without a service or a model (`engine_it.rs:60`, `placed_commands_it.rs:102, 123`, `commandspotting_websocket_it.rs:42`, `chunked_transcription_it.rs:34`, `transcription_openai_it.rs:636–645`, …), so the whole `command-spotting` suite is green under a bare `cargo nextest run`; nextest has no "skipped" verdict, so a pass and a skip are indistinguishable. Ceremony tests cannot fail: `focus_macos_it.rs:9–22` accepts `Ok(Some)`, `Ok(None)` and `Err`; `visualfeedback_macos_it.rs:199–219` has four functions without an assertion; `pipeline.rs:1241` "full pipeline" never checks the pasted text; `pipeline.rs:1175` asserts `Idle` after nothing. `textpasting_macos_it.rs:205–225` writes the user's real clipboard. Twenty step texts are implemented twice, once per harness (`"the overlay is hidden"` in `app/tests/steps/voice_control.rs` and `e2e-test/.../outages.rs`), and the two drift.

## What Changes

- No test passes by returning early: a gated test is either run by a recipe (a `just it-real` recipe for the window, microphone, accessibility and WAV tests, in the stage table and `full-check` under its own lock class, with the manual-run note in `AGENTS.md` where TCC blocks a sandbox) or deleted with its reason in the change; "no service" and "no model" gates become nextest filters (`--skip`/profile) with the skipped count printed by the recipe, never a green verdict.
- A rule of `app/tests/architecture.rs` fails on `GATED` returns and on a test function without an assertion (or with an assertion that cannot fail).
- The ceremony tests get a real assertion or go; the clipboard test runs on a pasteboard of its own (`NSPasteboard::pasteboardWithUniqueName`).
- Step texts implemented in both harnesses live once (a shared steps crate or a generated module) or get distinct texts.

Current stage: PoC. Target stage: PoC.
Scope: the tests named, the architecture rule, the recipes, `.config/nextest.toml`, `AGENTS.md` § How to verify a change.
Acceptance bar: `cargo nextest list` shows no test that returns before its first assertion when its precondition is missing; every former gate is either a recipe stage with a journaled run or gone; `just full-check` prints how many tests were skipped and why.
Out of scope: accuracy thresholds (`tighten-accuracy-gates`); spec mirroring (`mirror-specs-in-features`).

## Capabilities

### Modified Capabilities

- `change-workflow`: skipped tests are counted, never green.

## Impact

- `app/tests/*_it.rs`, `command-spotting/tests/*`, `app/tests/architecture.rs`, `app/tests/steps/*`, `e2e-test/tests/steps/*`, `.config/nextest.toml`, `justfile`, `AGENTS.md`.

## Open points

- Whether the real-device tests (microphone, accessibility, windows) can run under the machine lock on the owner's machine without TCC prompts; proposed: they run in `full-check` with a `_require-tcc` guard that names the permission missing, and in `AGENTS.md` as part of the manual run when the guard fails.

## Release

SemVer: no release — tests and recipes only.
Target version: unchanged.
