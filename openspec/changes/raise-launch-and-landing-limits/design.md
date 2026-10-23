## Context

`.claude/kit.yaml` sets `landing.max_batch: 4` and no `launch.max_running`. kit reads both in `internal/config/config.go`: each has a minimum and no maximum, and `max_running` falls back to kit's default when unset. Overnight 2026-10-22/23 three slots were held by changes waiting for fixes, so the fixes never started.

## Goals / Non-Goals

**Goals:** 7 changes running at once; landing groups of up to 7.

**Non-Goals:** kit's own slot accounting and stall handling (agent-kit).

## Decisions

| Question | Chosen | Why | Rejected |
|---|---|---|---|
| Whether kit accepts `max_batch` 7 | set 7 | kit checks only a minimum | lowering to 4 |
| Write `max_running: 7` though kit's default is now 7 | write it | a later kit default must not change this project's limit silently | relying on the default |
| How to prove the limits | `kit launchable` and kit's config reading on this file | the behavior lives in kit; this project only sets numbers | a project test of kit's queue |
| PR #302 (`max_running: 5`) | close it, pointing here | superseded | merging it first |

## Test levels

| Scenario | Level |
|---|---|
| A checked branch behind main joins the queue | kit's own landing-queue tests (agent-kit); unchanged here |
| A group lands after one full check | kit's own landing-queue tests (agent-kit); unchanged here |
| Seven queued branches land as one group | kit's batch-size tests (agent-kit) with this project's `landing.max_batch: 7` read by kit's config loader (`kit land` refuses a config it cannot read) |
| A red group lands its green members | kit's own landing-queue tests (agent-kit); unchanged here |
| A branch without build inputs needs no full check | kit's own landing-queue tests (agent-kit); unchanged here |
| Seven ready changes start together | `kit launchable` on this checkout, by hand: no config error with `launch.max_running: 7` |
| An eighth change waits for a free slot | kit's launch-cap tests (agent-kit) with this project's value |

## Risks / Trade-offs

- [7 changes queue more model stages on the machine lock] → measure lock wait with `just stage-times` after the first night; lower `max_running` if the wait grows past the 2026-10-21 baseline.
- [A larger group's red full-check takes more splits] → kit splits by halves, so 7 costs at most one more split than 4.
