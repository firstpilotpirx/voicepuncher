## Why

`AGENTS.md` § Gherkin says one `.feature` per capability with one `Rule:` per `### Requirement:`, and the `spec-test-reviewer` relies on it; the audit of 2026-10-22 found the mirror incomplete: `visual-feedback` has 1 of 11 requirements in `visual_feedback.feature` (the rest in `visualfeedback_macos_it.rs`), `local-services` 14 of 29, `text-insertion` 4 of 6, `processing-stages` 3 of 4; the `push-to-talk` spec has 2 requirements, neither the core "hold to record, release to paste", and its only scenario is `@real_keyboard`, never run (`push_to_talk.feature:3`). Traceability spec → test then rests on plain-Rust integration tests, which the reviewer compares worse, and a spec requirement can lose its test without anyone noticing.

## What Changes

- Every requirement of every capability has a `Rule:` with at least one scenario in its `.feature` at the component or E2E level; where the behavior is only observable through AppKit, the scenario drives the real adapter behind the same steps the `_it` test uses today, and the `_it` test keeps only the adapter contract.
- The push-to-talk spec gains the core requirement (hold → recording, release → transcription and paste, short press dropped) with scenarios the component harness runs on the fake hotkey listener; `@real_keyboard` stays as the manual scenario.
- A check (the test `spec_mirror` in `app/tests/it/spec_mirror/`) maps `### Requirement:` headings to `Rule:` headings per capability and fails on a requirement without a rule, so the mirror cannot decay.

Current stage: PoC. Target stage: PoC.
Scope: the `.feature` files of all 21 capabilities (a new `change_workflow.feature` with its harness in `dev-tools`), the push-to-talk spec and feature, the mapping check, `AGENTS.md` § Gherkin.
Acceptance bar: the mapping check passes for all 21 capabilities; `just test` and the E2E smoke pass; the `spec-test-reviewer` reports no UNTRACED requirement on these capabilities.
Out of scope: changing any requirement's meaning except adding the missing push-to-talk one.

## Capabilities

### Modified Capabilities

- `push-to-talk`: the core requirement stated and tested.

## Impact

- the `.feature` files and steps of every capability under `app/tests/`, `e2e-test/tests/` and `dev-tools/tests/`, `openspec/specs/push-to-talk/spec.md`, `app/tests/it/spec_mirror/`, `AGENTS.md`.

## Open points

- Settled at preparation: the mapping check is a test of this project (`spec_mirror`), not a kit command (design § Decisions 1).

## Release

SemVer: no release — tests and spec text only.
Target version: unchanged.
