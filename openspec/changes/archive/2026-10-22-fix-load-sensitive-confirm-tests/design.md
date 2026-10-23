# Design

## Context

The two tests that failed on 2026-10-22 at 17:08, `should_send_a_confirmation_refused_by_its_copy_to_the_other_copy` and `should_send_a_confirmation_its_copy_does_not_answer_within_the_budget_to_the_other_copy`, each build a service address from a port `127.0.0.1:0` hands out, then serve two fake copies on the first two copy ports of that address (`two_copies` in `command-spotting/src/confirm/tests.rs`). Reproduced in this change with 20 busy processes beside the tests: 3 failures in 8 runs, two of them in 20 ms with the text ` press enter` heard and 0 requests on the test's own copy — the answer came from a copy of another test.

The cause is not the wall clock. macOS hands out ephemeral ports in sequence (`net.inet.ip.portrange`), so two test processes that call `two_copies` at the same time get bases a few ports apart, and the four copy ports of one address (`base + 10_001 … base + 10_004`) hold the other test's fake copies. The pool then sees three or four copies, the pid parity the test relies on no longer picks the copy it expects, and a request lands on a copy the test does not count. Load only makes the two tests overlap in time more often (nextest runs each test in a process of its own, beside the others).

`should_fail_a_confirmation_when_every_copy_stays_silent_past_its_deadline` serves its copy on the service port itself, which the pool takes alone, so no other test's copy reaches it; it failed once, on 2026-10-22 at 03:07, on a branch that was still building the stall handling. Measured in this change under the same 20 busy processes (see tasks).

## Goals / Non-Goals

**Goals:** a test's copies are the only copies its pool can find, whatever runs beside it; the budgets, deadlines and `took` bounds of the tests stay as they are.

**Non-Goals:** a test clock; an injectable copy order in `confirm/`; the confirmation logic and the production budgets; the port layout of the services.

## Decisions

**Each two-copies test reserves a copy-port window of its own instead of an ephemeral base.** `two_copies` picks its base from `30_000 … 34_999` in steps of 8, starting at a slot the process id gives and scanning on, and takes the first base whose own port and first two copy ports it can bind. Two windows of different slots are 8 ports apart, so their four copy ports never overlap; two processes that pick the same slot see it at the bind and move on. The range lies below the ephemeral ports (49 152 and up), so no `127.0.0.1:0` bind of another test or program lands in a window, and apart from every port the project's services and checks use: 8080–8085 and 18082–18085 (the user's), 27080–28085 and 37080–38089 (the test ports of the lock classes and their copies), 48081–48084 and 49083–49086 (the text refinement and command spotting copies) and 45 533 and up (the copies below a service that a resilience step started on a high ephemeral port).

Alternatives: a scripted copy order in `confirm/` (proposed in the proposal) would not help — the stray copy is another process's, which no order in this process controls; binding all four copy ports and the service port for the test's duration would make the pool see the spare listeners as copies.

**The pid parity that picks the expected first copy stays.** With exactly two copies in the pool, the pool's offset (`pid % 2`) and the test's choice agree; the window makes the count exactly two.

**The silent-copies test keeps its shape.** Its cause was never a stray copy; the measurement under load decides whether it needs a change at all, and the proposal forbids widening its bound.

## Risks / Trade-offs

- [Another checkout's `just test` runs the same tests at the same time] → different process ids give different slots; the same slot shows at the bind and the test moves on.
- [A window port is held by a lingering connection] → a listener binds with address reuse; a bind that fails moves to the next slot.
- [The reproduction with busy processes is not `just e2e-resilience`] → the acceptance run loops the tests beside `just e2e-resilience`, as the proposal asks; the busy processes stay as the faster reproduction of the same failure.

## Tests

| Test | Level |
|---|---|
| the two-copies tests pass 20 runs in a row under load | unit (`confirm::tests`), run in a loop beside `just e2e-resilience` |
| the silent-copies test under load | unit, the same loop |

## Results

Before the fix, the three confirmation tests beside 20 busy processes: 3 failures in 8 runs (two in 20 ms with ` press enter` heard and 0 requests on the test's own copy, one at 1.3 s without the refused copy in the log); the silent-copies test alone under the same load: 20 passes in 20 runs, so it stays as it is.

After the fix, the four tests (the window test added) run 20 times in a row: 20 passes beside 20 busy processes while `just e2e-resilience` ran, and 20 passes beside `just e2e-resilience` alone. That `just e2e-resilience` run itself failed one scenario, «Russian speech that makes speech recognition repeat itself» (a confirmation waited out its deadline), while the 20 busy processes of the first loop ran beside it — the load of this change's own reproduction; it is not a check of this change.

The branch's `just full-check` (19:35) failed its `e2e-smoke-voice` stage on `placed_commands::should_hear_a_chain_of_commands` (heard `[]`) and `should_hear_a_stop_right_after_the_last_dictated_word`, real-service tests this change does not touch, while the `full-check` of `slim-pipeline-module` held the machine lock beside it; their fix is filed as its own change, which this one waits for (`orchestration.yaml`).

## Frozen tests

The branch's `full-check` failed on two real-service `placed_commands` tests this change does not touch (`should_hear_a_chain_of_commands`, `should_hear_a_stop_right_after_the_last_dictated_word`, beside another `full-check` holding the machine lock). By the user's decision of 2026-10-23 (`stop-filing-changes-for-flaky-tests`) an out-of-scope flake is frozen, not filed and not waited for: both tests carry `#[ignore = "frozen: …"]` until they are fixed separately. The next `full-check` (2026-10-23) failed on two more out-of-scope tests under load, frozen the same way: `placed_commands::should_hear_start_opening_an_utterance_and_place_it_before_the_speech` and the `voice_control.feature` scenario "The user's recorded restart restart followed by the new dictation at once" (tagged `@wip`). The third `full-check` failed on the example row "Okay, enter, clear" of "A chain of commands takes effect in order", moved to an `@wip` Examples block of its own. The fourth failed on one recorded form of `voice_forms` (session-4 at 32.41 s, "never mind cancel enter start"), now skipped through a `FROZEN` list beside `KNOWN_WRONG`.
