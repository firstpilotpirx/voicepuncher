## Why

Measured on the changes done one after another in one session (fix-permissions-and-login-item, mirror-specs-in-features, tame-confirming-special-cases, improve-recording-pill-look), with nothing running beside them: `just test` took about 30 % of the time, `full-check` 21 %, `just test-one` 3 %, the agent's own work about 47 %, lock waits 0. The user finds this far too slow.

1. **`just test` is the agent's inner loop and fails late.** It takes about 2.3 min, and about half of its runs failed — on the size budgets, the analyzer ratchets (metrics counts) or compile errors — each failure costing another full run. Those checks need seconds, yet they run only inside the whole suite. `just lint` alone takes 35–76 s, because clippy compiles; the size and metric checks are not clippy and should not wait for it.
2. **One slow test sets the length of `just test`.** `recording_cpal::…long_dictation…busy_machine` runs about 85 s; without it the suite would take about a minute.
3. **`full-check` takes 17–22 min per change**, run on the machine lock one change at a time. The user wants it down to a few minutes.

## What Changes

- A fast static check the agent runs first while coding, in seconds: formatting, size budgets, analyzer ratchets and metrics (and clippy only if it can be made fast), stopping at the first problem before any test runs. The agent's workflow (AGENTS.md, the TDD loop) runs it before `just test`, and `just test` itself fails fast on it before the test suite starts.
- The ~85 s `recording_cpal` busy-machine test is made fast, or moved out of `just test` into `full-check`.
- `full-check` is cut to a few minutes: fewer E2E scenarios and example rows (keeping at least one scenario per spec requirement), stages that can run in parallel run in parallel, and what is redundant with `check` or with unit tests is dropped.

Current stage: PoC. Target stage: PoC.
Scope: the `justfile` recipes and stages (`test`, `check`, `full-check`, a new static recipe), the slow test, the E2E scenario sets, AGENTS.md § How to verify a change and § TDD.
Acceptance bar: the static check runs in seconds and catches size-budget, ratchet and formatting failures before any test; `just test` runs in about a minute; `full-check` runs in a few minutes on a typical change; every spec requirement keeps a covering test.
Out of scope: the app's behavior; the hosted CI's own runtime.

## Capabilities

### Modified Capabilities

- none: tooling and test layout only (the `change-workflow` capability may change if the check levels are specified there).

## Impact

- `justfile`, `dev-tools/` (stage runner, `.config/stages.toml`), `.config/nextest.toml`
- `app/tests/it/recording_cpal.rs`
- `e2e-test/tests/features/*.feature` and the E2E subsets (`@smoke`, `@regression`, …)
- `AGENTS.md`

## Open points

- A separate recipe (for example `just static`) or a fail-fast first stage inside `just test`, or both — needs the user's decision after research.
- Whether clippy belongs in the fast check (35–76 s now) or stays in `lint`.
- Which E2E scenarios and example rows to drop, and the target time of `full-check` (proposed: under 5 min) — to be chosen with the user from measured stage times (`just stage-times`).
- Speed up the slow test or move it to `full-check` — proposed: move it, unless a shorter version still proves the busy-machine case.
- Order: run last, after the rest of the backlog; it needs research and the user's participation (autonomy review).

## Release

SemVer: no release — no behavior change in the app.
Target version: 0.38.4.
