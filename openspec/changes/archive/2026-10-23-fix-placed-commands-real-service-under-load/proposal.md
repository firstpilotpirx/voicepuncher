## Why

Two real-service integration tests of the command spotting service, `placed_commands::should_hear_a_chain_of_commands` (heard `[]` instead of `["stop dictation", "press enter"]`, `command-spotting/tests/it/placed_commands.rs:265`) and `placed_commands::should_hear_a_stop_right_after_the_last_dictated_word` (`:206`), failed in the `e2e-smoke-voice` stage of `just full-check` on branch `fix/fix-load-sensitive-confirm-tests` (commit 6ace76a0) on 2026-10-22 at 19:38, while the `full-check` of `chore/slim-pipeline-module` (pid 12575, worktree practical-hofstadter-aa5b44) held the machine lock beside it. The same real-service tests failed before: on 2026-10-21 at 14:11 and 14:46 (`fix/ordered-wait-for-busy-copies`), at 19:35 (`fix/eradicate-flaky-tests`) and on 2026-10-22 at 06:57 (`feat/command-spotting-stall-handling`). `eradicate-flaky-tests` (archived 2026-10-21) put the scripted-confirmation tests of that file into speech time and noted that its real-service tests "feed the speech as fast as possible, so where in the speech an answer lands depends on the shared service's speed" — those are the tests still failing. A flake is a failure (AGENTS.md § How to verify a change): `fix-load-sensitive-confirm-tests` (PR #297) waits for this fix.

## What Changes

- The real-service tests of `placed_commands.rs` give the same result on an idle and on a loaded machine: the harness feeds the speech in speech time, or waits for each confirmation before feeding on, or takes whatever seam it needs — without widening any budget of the engine.

Current stage: PoC. Target stage: PoC.
Scope: `command-spotting/tests/it/placed_commands.rs` and its harness helpers (`engine()`, `heard()`, `speech()`).
Acceptance bar: the two tests pass 20 times in a row while another `just full-check` holds the machine lock beside them; `just test` and `e2e-smoke-voice` green.
Out of scope: the engine's confirmation logic and its budgets; the scripted-confirmation tests of the file, already in speech time.

## Capabilities

### Modified Capabilities

- none (test-only fix; the voice control spec stays).

## Impact

- `command-spotting/tests/it/placed_commands.rs`; `just test`, the `e2e-smoke-voice` stage of `check` and `full-check`.

## Open points

- Whether speech time (as the scripted tests) or a wait for each confirmation is the smaller seam for a real service: proposed — feed in speech time, the mechanism `eradicate-flaky-tests` already built for the file.
- Whether the other real-service tests of the file (`should_hear_a_doubled_stop_in_the_middle_of_speech`, `should_hear_enter_right_after_other_speech`, `should_hear_start_opening_an_utterance_and_place_it_before_the_speech`, which failed the same way on 2026-10-21) take the same fix: proposed — yes, every real-service test of the file.

## Release

SemVer: no release — test-only change, no behavior change.
Target version: unchanged.
