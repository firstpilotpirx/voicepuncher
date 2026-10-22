# Design

## Context

The cause, found by reading the service and the E2E client: keyword spotting spots "clear text please" every time (0 misses in 40 replays, idle and under CPU load), and the raw spot fits the recognizer's margins even when the confirmation is empty. What loses the command is a race between the sync answer and a confirmation asked a second time.

The E2E client asks for a sync after each stretch of audio, waits for its answer and then closes. The service answers a sync as soon as no confirmation asked *before* it is still pending. A segment's events are held under the number of its confirmation job and leave with its answer. When the first hearing is useless — under load the pace budget cuts it short at the 1 s floor and it comes back empty, or the half-pause stand-in mishears a word — the service asks the whole clip again and *moves the held events to that later job*. That job was asked after the sync, so the sync no longer waits for it: the client closes and the phrase is reported to nobody. Hence `recognized []` in one run under parallel builds: a late first hearing is the trigger; the keyword spotting and the sample are fine.

## Goals / Non-Goals

**Goals:**
- A sync is answered only after every segment closed in the audio before it has been reported, whichever job its events were moved to.
- The fix is a change of the service's outbox alone; the pace budget, the deadlines and the E2E step stay as they are.

**Non-Goals:**
- Making the first hearing right under load (the 1 s floor of the pace budget stays).
- The other flaky scenarios of the audit of 2026-10-21; the lock classes.

## Decisions

**A held segment remembers the job it was first held under, and a sync waits for it.** Moving held events to a later job keeps that first number; a sync is answered only when no pending job asked before it remains *and* no held segment first held under such a job remains. Rejected: waiting for every held segment whatever its job (speech in the next stretch would delay an answer about earlier audio); closing the E2E client later (the real app has the same window: it syncs when the key is released and would equally miss a command moved to a later hearing).

## Risks / Trade-offs

- A sync now waits for the second hearing of a late segment, up to that job's deadline (3 s): the E2E step waits longer in that run instead of failing; the sync answer limit of the client (20 s) covers it.
- No change to the protocol or to any public type: the app needs no change.

## Test levels

| Spec scenario | Level |
|---|---|
| A command whose first hearing is empty under load is reported before the sync is answered | unit (`command-spotting/src/outbox/tests.rs`: a held segment moved to a job asked after the sync still blocks the sync) and integration (`command-spotting/tests/it/placed_commands.rs`: a scripted hearing whose first answer is empty, a sync sent right after the speech, the command reported before the sync answer) |
| A command whose half-pause hearing is misheard is reported before the sync is answered | integration (`command-spotting/tests/it/placed_commands.rs`: the existing mishearing of "okay, clear" plus a sync right after the speech) |
| "clear text please" alone takes effect (existing row, the failing E2E example) | E2E `.feature` (`e2e-test/tests/features/voice_control.feature`, row `clear text please`, unchanged; it passes in the next three `full-check` runs, per the acceptance bar) |
