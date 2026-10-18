## Why

Three `@refinement` E2E scenarios of `text-refinement` still dictate words the model is shown in the prompt, so their pass may show that the model repeats its examples, not that the rule works on new speech: «фидбек» and «даймэк» appear word for word in the system prompt, and «поставь о май зш и тему пауэрлевел десять к» has the structure of the oh-my-fish example in `DEFAULT_EXAMPLES` (`app/src/config/refinement.rs`). The spec-test reviewer found this in `english-scenarios-unlike-examples` (PR #142), which fixed the same problem for two other scenarios.

## What Changes

- The scenarios "English word spelled in Cyrillic comes out in Latin script", "Misheard English word is restored from context" and "Shell framework and theme spelled as syllables are restored" dictate phrases unlike every example and every word the system prompt shows.
- The scenarios keep their meaning and checks.
- No prompt, example or app code change.

Current stage: PoC. Target stage: PoC.
Scope: the dictations and step texts of those three scenarios and their spec scenarios.
Acceptance bar: none of the three dictations shares its sentence structure, English words or IT terms with the system prompt or `DEFAULT_EXAMPLES`; all `@refinement` scenarios pass on Qwen3-4B-Instruct-2507 Q4_K_M run on an own port; `just check` passes.
Out of scope: the prompt, its examples, the default model, other scenarios and the evaluation set.

## Capabilities

### Modified Capabilities

- `text-refinement`: the three scenarios name the new dictations; app behavior is unchanged.

## Impact

- `e2e-test/tests/features/text_refinement.feature`, `e2e-test/tests/steps/text_refinement/normalization.rs`, `openspec/specs/text-refinement/spec.md`.
- `just refine-eval`, the `@refinement` scenarios of `just check`.

## Open points

- A new phrase the model fails on today: proposed, pick another phrase and report the failure as its own change, as `english-scenarios-unlike-examples` did, since a prompt change is out of scope.
- The misheard-word scenario needs a context that makes the word clear without the prompt's «даймэк»; proposed: a new misheard English word in a new context.

## Release

SemVer: no release — tests only.
Target version: unchanged (0.18.0 on `main`).
