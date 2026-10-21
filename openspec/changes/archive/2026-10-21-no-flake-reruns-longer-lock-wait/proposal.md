## Why

Two check-policy rules cost whole lock cycles on 2026-10-20/21. First, AGENTS.md § Flakes allows one unchanged rerun of a failed test and then the failed stage's recipe whole; in practice a flake under the lock meant a second `full-check` with its ~45 min queue, and the flaky test stayed flaky (the same three scenarios failed 38 times across every branch). The user's decision: a flake is a failure — never rerun it; fix it or file its fix, nothing lands on a lucky second roll. Second, the machine lock gives up after `MACHINE_LOCK_WAIT` = 3600 s (exit 75): nine `full-check` runs waited exactly 60 min, got no lock, ran nothing and were requeued — 9 h of pure waiting producing nothing in one night, because the queue of 4–8 sessions × ~20 min is longer than an hour.

## What Changes

- `AGENTS.md` § Flakes and § How to verify a change: a failed test is a failure of the run, whatever it failed on; no unchanged rerun, not once. The change fixes the test when its cause is in scope, otherwise files the fix with `/create-change` naming the test, the recipe and what ran beside it, and the branch waits for that fix (`depends_on`). `just rerun-failed` stays for reruns **after a fix**, never for an unchanged retry. The `[machine-lock]` and parallel-run notes in the stage report stay as evidence, not as an excuse.
- `justfile` (`locked :=`): `MACHINE_LOCK_WAIT` defaults to 21600 s (6 h) when the caller does not set it, so a queued run waits out the queue instead of giving up; the stage report still shows the wait.
- `~/.claude/rules-custom/machine-lock.md` and `openspec.md` (agent-kit) say the same about flakes, raised in agent-kit `parallel-check-churn`.

Current stage: PoC. Target stage: PoC.
Scope: `AGENTS.md`, `justfile`.
Acceptance bar: `grep -c rerun AGENTS.md` names only the after-a-fix use; a `full-check` started behind a 90 min queue waits and runs (journal: `lock_wait` > 3600 with stages run); no `rerun-failed` record in the journal without a commit between it and the failed run.
Out of scope: fixing the flaky tests (`eradicate-flaky-tests`); the kit default of `MACHINE_LOCK_WAIT` (agent-kit).

## Capabilities

### Modified Capabilities

- none: check policy and test tooling only; no product capability changes (`skip_specs: true`).

## Impact

- `AGENTS.md`, `justfile`.

## Open points

- Whether the kit default (`lock.go`, 3600) changes too, for every project — proposed: yes, in agent-kit `parallel-check-churn`; this change sets it here so voicepuncher does not wait for a kit release.
- Target version: no release — proposed.

## Release

SemVer: no release — tooling and rules only.
Target version: unchanged.
