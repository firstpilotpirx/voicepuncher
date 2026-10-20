## Why

The capability `app-distribution` (`openspec/specs/app-distribution/spec.md`) has no `app_distribution.feature`, so `kit feature-lint` fails on `main` (`~/.claude/rules-custom/gherkin.md` § Traceability: one `.feature` per capability). Every change that runs `kit feature-lint` at verify meets this failure, though it is not its own; seen on `feat/file-logging` on 2026-10-21.

## What Changes

- `app-distribution` gets its Gherkin feature, one `Rule:` per requirement, with steps, so its scenarios are traced and `kit feature-lint` passes on `main`.

Current stage: PoC. Target stage: PoC.
Scope: the `.feature` and steps for `app-distribution`, at the level each scenario can run.
Acceptance bar: `kit feature-lint` passes on `main`; each `app-distribution` scenario matches a `.feature` scenario by meaning.
Out of scope: changing how the app is distributed or released.

## Capabilities

### Modified Capabilities

- none: tests only.

## Impact

- a new `app_distribution.feature` (component or E2E level) with steps, harness registration.

## Open points

- The level: scenarios about the release archive may only run as part of `just release`; whether they go to a component feature over a test-built archive (proposed) or stay `@wip` with the reason.

## Release

SemVer: no release — tests only, no behavior change.
Target version: unchanged.
