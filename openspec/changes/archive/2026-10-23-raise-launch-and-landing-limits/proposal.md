## Why

Overnight 2026-10-22/23 the orchestrator finished one change in about 9 hours: the default limit of 3 changes running at once was filled by changes waiting for fixes, so the fixes never started. The user wants more changes to run and land together: 7 at once, landed in groups of up to 7.

## What Changes

- `.claude/kit.yaml`: `launch.max_running: 7` (unset now, so kit's default applies: 3 overnight, 7 in current kit; written out so a later default cannot change it).
- `.claude/kit.yaml`: `landing.max_batch: 7` (4 now), so up to 7 branches merge through one landing group and one `full-check`.
- Supersedes the open PR #302, which sets `max_running: 5`: close it.

Current stage: PoC. Target stage: PoC.
Scope: the two limits in `.claude/kit.yaml`.
Acceptance bar: `kit launchable` lets 7 changes run at once and `kit land` forms groups of up to 7.
Out of scope: kit's own behavior (blocked changes holding slots, stall handling) — filed in agent-kit.

## Capabilities

### Modified Capabilities

- `change-workflow`: up to 7 changes run at once and land in groups of up to 7.

## Impact

- `.claude/kit.yaml`; `AGENTS.md` § Workflow (stage 11 says "at most 4 per group").

## Open points

- Whether kit accepts `max_batch` above 4: resolved in design — kit checks only a minimum, so 7 stands.
- Whether 7 parallel changes overload the machine lock queue: proposed to measure with `just stage-times` after the first night.

## Release

SemVer: no release — tooling configuration only.
Target version: 0.37.4.
