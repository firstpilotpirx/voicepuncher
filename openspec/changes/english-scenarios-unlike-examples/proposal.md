## Why

Two E2E scenarios of the `text-refinement` capability dictate phrases that are almost the same as few-shot examples the refinement prompt sends to the model (`DEFAULT_EXAMPLES` in `app/src/config/refinement.rs`):

- "Two English words in one sentence both come out in Latin script" dictates «поставь режим мануал а не автоматик»; the example is «короче поставь режим мануал а не автоматик».
- "Product names spelled as syllables are restored" dictates «нужно обновить гит хаб экшнс и вс код»; the example is «надо обновить гит хаб экшнс и пи эн пи эм».

So these scenarios show that the model repeats its examples, not that the rules work on sentences it has never seen. Only the evaluation's `english` and `terms` categories cover new sentences, and they are scored, not pass/fail per item. The spec-test reviewer found this in `english-words-in-latin` (v0.16.0). The second scenario also showed the risk: right after the GitHub Actions example, Qwen3-4B first wrote «вс код» as «VSCod», which led to a prompt fix in that change.

## What Changes

- Replace the dictations of the two scenarios in `e2e-test/tests/features/text_refinement.feature` (step code in `e2e-test/tests/steps/text_refinement/normalization.rs`) with phrases unlike every example in `DEFAULT_EXAMPLES`: other sentence structure, other English words or IT terms, so a pass means the rule carries over to new speech.
- The scenarios keep their meaning and checks: two English words in one sentence come out in Latin script; product names split into Cyrillic syllables come out in their usual spelling.
- No prompt, example or code change; no version bump (tests only).
- Before landing, the new phrases pass on Qwen3-4B-Instruct-2507 Q4_K_M, the default refinement model, run on an own port.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None: the spec scenarios of `text-refinement` stay as they are; only the test dictations change.

## Scope

- Current stage: PoC. Target stage: PoC.
- Acceptance bar: neither scenario's dictation shares its sentence structure or its English words and IT terms with any example in `DEFAULT_EXAMPLES`; both scenarios pass on Qwen3-4B-Instruct-2507 Q4_K_M; `just check` passes.

## Out of scope

- Changes to the refinement prompt, its examples or the default model.
- Other scenarios and the evaluation set.

## Release

No release: tests only, the version stays.

## Impact

- `e2e-test/tests/steps/text_refinement/normalization.rs` (dictated phrases and their checks), maybe the scenario wording in `e2e-test/tests/features/text_refinement.feature`.
- `just refine-eval` and the `@refinement` scenarios of `just check`.

## Open points

- Which phrases: whether to take them from the evaluation set's `english` and `terms` cases or write new ones; a phrase the model fails on today would turn this into a prompt change, which is out of this change's scope and would need its own decision.
