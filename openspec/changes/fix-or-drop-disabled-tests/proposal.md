## Why

Several tests do not run today, so the behavior they cover is unchecked:

- frozen in `.config/frozen-tests.toml` (flaked under load, run by no recipe until fixed):
  - `scenario: The user's recorded restart restart followed by the new dictation at once` (e2e-smoke-voice, since 2026-10-23);
  - `scenario: Russian speech that makes speech recognition repeat itself` (e2e-resilience, since 2026-10-23; also tagged `@wip`);
  - `scenario: The first request of a chunk hangs` (test, since 2026-10-24);
  - `scenario: Restart restart followed by the new dictation at once gives its cue at once` (e2e-cues, since 2026-10-24: 735 ms against a 700 ms bound);
- tagged `@wip` as frozen or not met in the E2E features: two `voice_control.feature` scenarios frozen on 2026-10-23 (e2e-smoke-voice, e2e-regression), one `voice_control.feature` scenario not met yet (the early checks hear the synthesized "Enter" as "and a"), `app_distribution.feature` "The pinned engines are staged" (hung past the bundle stage limit, 2026-10-23).

Seven more `@wip` scenarios in `app_distribution.feature` are manual by design (they need a person, a fresh install or Gatekeeper) and the `#[ignore]` test in `e2e-test/tests/it/confirmation_time.rs` is a measurement; they stay as they are.

The user wants every disabled test either enabled and fixed, or deleted when it cannot be made reliable.

## What Changes

- For each disabled test above: find why it fails or hangs, fix the code or the test, and enable it (remove its `.config/frozen-tests.toml` entry or its `@wip` tag) — or, when it cannot be made reliable on this machine, delete it, keeping its requirement covered by another test (a lower-level one where possible) and saying which in the design.
- `.config/frozen-tests.toml` ends with no entry; no `@wip` tag is left except the manual scenarios of `app_distribution.feature`.

Current stage: PoC. Target stage: PoC.
Scope: the listed disabled tests and the code they exercise.
Acceptance bar: `just frozen-tests` lists none; every enabled test passes in two consecutive `full-check` runs on the landing queue; every deleted test names the test that now covers its requirement.
Out of scope: the manual `@wip` scenarios and the measurement test.

## Capabilities

### Modified Capabilities

- `voice-control`, `local-services`, `app-distribution`, `speech-recognition`: their disabled scenarios are fixed or replaced; behavior changes only where a fix in the code is needed.

## Impact

- `.config/frozen-tests.toml`, `e2e-test/tests/features/{voice_control,local_services,app_distribution}.feature` and their steps, `app/tests/features/**` (the chunk hang scenario), the code each fix needs

## Open points

- A test that passes again without a fix (load gone): proposed enable it only after it passed two `full-check` runs in a row.
- Release: PATCH when a fix changes the app's code, else no release; proposed decided in design.

## Release

SemVer: PATCH — fixes behind flaky voice-control and service scenarios may change the app (no release if none does).
Target version: 0.38.6.
