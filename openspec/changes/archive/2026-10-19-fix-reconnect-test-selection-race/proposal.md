## Why

The integration test `commandspotting_reconnect_it::should_reach_for_the_service_again_after_it_closes_the_connection` is flaky. Right after the app reports `SpottingBack`, the test reads which commands the stub service recorded as selected. Sometimes that happens before the stub has read the selections from the new connection, so the test gets an empty list (`[]`) and fails.

Evidence: it failed twice on 2026-10-19, and an unchanged rerun passed both times:

- in `just check` of `fix-spotting-lost-at-check-start`;
- in the first `just release` of v0.21.1.

AGENTS.md asks for a change on the cause when the same test flakes again.

## What Changes

- The test waits, up to a bound, until the stub has received the selections, instead of reading them right after `SpottingBack`.
- The app's reconnection behavior stays the same; only the test changes.

Current stage: PoC. Target stage: PoC.
Scope: the reconnect integration test and its stub service in `app/tests/`.
Acceptance bar: the test passes 20 runs in a row under CPU load; `just test` is green.
Out of scope:
- the app's reconnection code;
- the command-STT warm-up flake, which `fix-flaky-tests` covers.

## Capabilities

### Modified Capabilities

- None: test-only change; `voice-control` behavior is unchanged.

## Impact

- `app/tests/commandspotting_reconnect_it.rs`, and the stub service it uses if the wait needs a hook there.
- Checks: `just test`.

## Open points

- Bound on the wait: proposed 2 s, well under the test's time limit.
- Whether other tests that read the stub's selections right after an event have the same race: proposed to check them during the fix and give them the same wait.

## Release

SemVer: no release — a test-only change, no behavior change.
Target version: 0.21.1.
