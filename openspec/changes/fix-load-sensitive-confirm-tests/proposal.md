## Why

Two unit tests of the command spotting service's confirmation, `confirm::tests::should_send_a_confirmation_refused_by_its_copy_to_the_other_copy` (asserted `requests == 1`, got 0) and `confirm::tests::should_send_a_confirmation_its_copy_does_not_answer_within_the_budget_to_the_other_copy` (asserted `answered == 1`, got 2) in `command-spotting/src/confirm/tests.rs`, failed in `just test` on branch `chore/slim-pipeline-module` on 2026-10-22 at 17:08 (commit 465a7679 plus uncommitted edits outside `command-spotting/`), while the `full-check` of `chore/fast-test-build` ran its `e2e-resilience` stage beside it: the machine was under load. The stage journal shows the same tests failing twice before, on 2026-10-22 at 03:07 (together with `should_fail_a_confirmation_when_every_copy_stays_silent_past_its_deadline`) and at 03:46, on `feat/command-spotting-stall-handling`; 3 runs of 919. The tests drive real copies on local ports with wall-clock budgets — a slow copy answering after 5 s, a transcribe budget of 2–3 s, `took < 1_800 ms` — so a loaded machine changes which copy answers and how often. A flake is a failure (AGENTS.md § How to verify a change): the change that hit it waits for this fix.

## What Changes

- The two tests that serve two copies on the copy ports of an ephemeral base port each reserve a copy-port window of their own, so a test's pool finds only that test's copies (reproduced in this change: macOS hands out ephemeral ports in sequence, two of these tests running at the same time get bases a few ports apart, and one test's pool finds the other's fake copies — the ` press enter` heard with 0 requests on the test's own copy came from another test; load only makes the two overlap in time more often; design.md).
- The budgets the tests assert stay as they are; none is widened.

Current stage: PoC. Target stage: PoC.
Scope: `command-spotting/src/confirm/tests.rs` and whatever seam in `command-spotting/src/confirm/` the tests need to control time or the copies' answers.
Acceptance bar: the three tests pass 20 times in a row while `just e2e-resilience` runs beside them; `just test` green.
Out of scope: the confirmation logic itself; the budgets of the production code; other load-sensitive tests.

## Capabilities

### Modified Capabilities

- none (test-only fix; the voice control spec stays).

## Impact

- `command-spotting/src/confirm/tests.rs`, possibly `command-spotting/src/confirm/mod.rs` (an injectable clock or copy order); `just test`.

## Open points

- Whether a test clock or a scripted copy order is the smaller seam: neither — the stray copy belongs to another test process, which no clock or order in this process controls; the fix is a copy-port window of the test's own, in the test file alone (design.md).
- Whether `should_fail_a_confirmation_when_every_copy_stays_silent_past_its_deadline` has the same cause: no — it serves its copy on the service port, which the pool takes alone; measured under load in this change (design.md § Results), unchanged unless the measurement fails.

## Release

SemVer: no release — test-only change, no behavior change.
Target version: unchanged.
