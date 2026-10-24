## Context

See proposal.md § Why. `AGENTS.md` § Gherkin asks for one `.feature` per capability with one `Rule:` per `### Requirement:`. A count on 2026-10-24 found 130 requirements of the 21 capabilities without a rule of the same heading: some are rules reworded since (`The bundled engines use the GPU` for `The bundled speech recognition engine uses the GPU`), most have plain-Rust tests only, and `change-workflow` has no `.feature` at all. The preparation settled the open points (`orchestration.yaml` § prepared): the check is a test of this project, AppKit-only behavior is tested on the recording fake overlay, local services at component level with fake service binaries, and every capability is filled, with no exception list at the end.

## Goals / Non-Goals

**Goals:** every requirement of every capability under `openspec/specs/` has a `Rule:` of the same heading with at least one scenario that a harness runs; a test fails on a requirement without its rule and on a rule without its requirement.

**Non-Goals:** changing what a requirement means (except the added push-to-talk one); removing plain-Rust tests that stay the adapter contract; E2E scenarios where a component one proves the behavior.

## Decisions

1. **The mirror check.** A test `spec_mirror` in its own module directory `app/tests/it/spec_mirror/` (the listed `app/tests/it` directory does not grow) reads every `openspec/specs/<capability>/spec.md`, collects its `### Requirement:` headings and the `Rule:` headings of every `<capability, - as _>.feature` under `app/tests/features/`, `e2e-test/tests/features/` and `dev-tools/tests/features/`, and fails naming, per capability, each requirement without a rule and each rule without a requirement. Headings compare exactly after trimming. A requirement heading of an active change's delta spec (any of ADDED, MODIFIED, REMOVED, RENAMED) counts as known, so a rule written ahead of its archive, or still standing for a requirement a change removes, does not fail the test before that change is archived. Alternative: a `kit` command — rejected at preparation (the project owns its layout).
2. **A shrinking list while the change runs.** The test holds the requirements still without a rule in a list that only shrinks: a listed requirement that gains its rule fails the test until it leaves the list, so every task group commits green and proves what it added; the last group empties the list and removes it, so no exception list remains.
3. **Reworded rules take the requirement's heading.** A rule that tests a requirement under an older wording is renamed, its scenarios kept.
4. **Each new rule mirrors the spec's scenarios by meaning** (`~/.claude/rules-custom/gherkin.md` § Traceability), at component level on the existing stand-ins: the scripted pipeline, the recording fake overlay, paster and sound player, the fake hotkey listener, the fake service binaries of `localservices_process`, and the pure adapter mappings (`IconState`, overlay layout, permission rows) where the behavior is a drawing decision. A requirement whose behavior needs a real model or service gets its rule in its existing E2E `.feature`. Steps call the same public code the plain-Rust test of that requirement calls; that test stays as the adapter contract.
5. **Step code goes into each capability's step subdirectory** (`app/tests/steps/<capability>/`, `e2e-test/tests/steps/<capability>/`), so the listed step directories do not grow; a step text both harnesses use is a `SharedSteps` method.
6. **`change-workflow` gets a harness in `dev-tools`.** `dev-tools/tests/features/change_workflow.feature` runs under a cucumber harness `dev-tools/tests/cucumber_workflow.rs`, its steps calling the stage runner, journal, freezer and recipe checks of `dev-tools` the way its unit tests do, and reading `justfile`, `.config/` and `.github/workflows/ci.yml` for the requirements that are a rule of those files (the landing group size, the hosted check, the lint stage's analyzers).
7. **Push-to-talk's core requirement** is stated (spec delta) and tested on the fake hotkey listener: hold → recording, release → transcript pasted, short press → nothing; the `@real_keyboard` scenario stays as it is.

## Risks / Trade-offs

- [Many scenarios at once grow the suites and their time] → steps reuse the existing stand-ins and shared helpers; a suite past half its time limit gets a new limit in this change (`AGENTS.md` § Test limits).
- [Reduce Motion is read from macOS each draw] → the scenario asserts the branch the machine's setting selects (pulse when off, still when on); proving both branches on one machine would need a test override in the overlay, whose directory is at its size budget, so the other branch runs wherever that setting differs.
- [A process requirement (`change-workflow`) tested as a rule of a file] → the scenario names the file that holds the rule and checks it, which is what the requirement asks of the project.

## Test mapping

| Requirement | Test |
|---|---|
| Every requirement has its rule, every rule its requirement | unit-level check `app/tests/it/spec_mirror/` `should_mirror_every_requirement_in_a_rule` |
| push-to-talk: A held hotkey records and a release pastes | component: `push_to_talk.feature` |
| push-to-talk: A short press records nothing | component: `push_to_talk.feature` |
| Every other requirement the check lists | component or E2E: the scenarios of its `Rule:` in the capability's `.feature` |
