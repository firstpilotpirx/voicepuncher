## Context

See proposal.md — Why. The adapter writes the phrase selections to the new connection and then reports `SpottingBack`; the stub service reads that connection on its own thread, polling with a 10 ms read timeout. The test reads the stub's record right after `SpottingBack`, so on a busy machine it can read before the stub thread has recorded the selections.

`should_send_the_service_that_is_back_the_speech_heard_while_it_was_away` has the same shape: the stub answers a speech start on the first audio of the new connection, while the rest of the replayed audio may still be in transit when the test counts the marked samples.

## Goals / Non-Goals

**Goals:** the two tests that read the stub's record after an app event wait, within a bound, for what they expect; an early read fails every run before the fix.

**Non-Goals:** no change to the app's code, to the stub's protocol or to any other test file; no retry and no raised time limit.

## Decisions

**D1 — Wait for the record, not for the event.** A helper polls the stub's record of a connection until a condition holds or `RECORD_WITHIN` (2 s) passes, then the test asserts on what it last saw, so a miss still prints the record. Rejected: a fixed sleep after `SpottingBack` (slower on every run and still a race); making the stub acknowledge the selections (changes the stub's protocol for a test-only problem).

**D2 — The bound is 2 s.** The stub records within milliseconds on an idle machine; 2 s covers a loaded one and keeps the test far under its 2 min limit. Rejected: reusing `BACK_WITHIN` (5 s), which would hide a real slowdown longer than the reconnection itself.

**D3 — A lag in the stub proves the wait.** A `RECORD_LAG` of 50 ms before the stub records a selection stays in the stub (doc comment: it makes an early read fail every run). The Red commit adds it alone and the reconnect test fails every run; the Green commit adds the wait. Rejected: relying on CPU load to reproduce, which fails only sometimes.

**D4 — The replay test gets the same wait.** It waits until the new connection holds more marked samples than the outage lasted, within `RECORD_WITHIN`, then asserts as before. The same lag does not touch audio, so its Red step is a check by reading the code, not a forced failure.

## Risks / Trade-offs

- [The lag slows the suite] → 50 ms once per selection message, a handful per test.
- [A real regression in what the app sends is hidden by the wait] → the wait ends at 2 s and the assertions are unchanged, so a missing selection still fails.
