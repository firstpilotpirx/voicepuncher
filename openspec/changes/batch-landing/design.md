## Context

agent-kit already implements batch landing behind `landing.batch` in `.claude/kit.yaml`; voicepuncher has it off, so every branch reruns `full-check` each time `main` moves.

## Decisions

- **Turn on agent-kit batch landing, `max_batch: 4`.** Configuration only: no project code changes. Four is agent-kit's default and keeps the bisect of a red group at two extra checks at most.
- **The own check before the queue is `just check`.** agent-kit queues a branch with build inputs on `after_group`; the group's `full-check` is the merge gate. AGENTS.md stages 8 and 11 say so, so sessions no longer run `full-check` on their own branch.
- **Sequential groups only.** Speculative parallel batches are out of scope: they would compete for the machine lock that the group check already holds.

## Dependencies

Waited for `eradicate-flaky-tests` (a 25 % pass rate makes a group of four red almost always) and `reuse-passed-check-stages` (names `kit land`); both are archived on `main`.

## Test levels

| Scenario | Level |
|---|---|
| A checked branch behind main joins the queue | manual run (agent-kit behavior; observed in the next orchestrated batch) |
| A group lands after one full check | manual run (`just stage-times 1d`: one passing full check per group) |
| A red group lands its green members | manual run (agent-kit's own tests cover the bisect) |
| A branch without build inputs needs no full check | manual run (agent-kit behavior) |
