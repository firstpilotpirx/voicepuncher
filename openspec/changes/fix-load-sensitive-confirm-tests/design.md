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
