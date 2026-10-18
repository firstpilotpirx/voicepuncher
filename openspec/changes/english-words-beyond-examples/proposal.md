## Why

With the default model Qwen3-4B-Instruct-2507 Q4_K_M and the default instruction, the dictation «в плеере включи шаффл и выключи репит» comes out as «В плеере включи шаффл и выключи репит.»: «шаффл» and «репит» stay in Cyrillic instead of «shuffle» and «repeat». The requirement "English words spelled in Cyrillic are written in Latin script" of `text-refinement` says they must come out in Latin script. This was found in `english-scenarios-unlike-examples` (PR #142) while picking E2E phrases unlike the few-shot examples (`DEFAULT_EXAMPLES` in `app/src/config/refinement.rs`): the rule carries over to some new words («энейблд», «дизейблд» passed) but not to every English word.

## What Changes

- English words that speech recognition spelled in Cyrillic, such as «шаффл» and «репит», come out in Latin script on dictation unlike the prompt's examples.
- Common Russian loanwords and words with Russian endings still stay in Cyrillic.
- An E2E scenario or an evaluation case covers the «шаффл»/«репит» kind of dictation.

Current stage: PoC. Target stage: PoC.
Scope: the text refinement instruction, its examples or evaluation cases, and the tests that cover English words in Cyrillic.
Acceptance bar: «в плеере включи шаффл и выключи репит» comes out with «shuffle» and «repeat» on Qwen3-4B-Instruct-2507 Q4_K_M; the loanword scenarios and the evaluation's loanword check still pass; `just check` passes.
Out of scope: changing the default model; IT terms split into syllables.

## Capabilities

### Modified Capabilities

- `text-refinement`: English words spelled in Cyrillic come out in Latin script also for words the instruction's examples do not show.

## Impact

- `app/src/config/refinement.rs` (instruction, examples), the evaluation set, `e2e-test/tests/features/text_refinement.feature` and `e2e-test/tests/steps/text_refinement/normalization.rs`.
- `just refine-eval`, the `@refinement` scenarios of `just check`.

## Open points

- How to fix it: a rule wording plus one new example, chosen by measurement (design.md); the new phrases stay unlike every example.
- Whether «репит» counts as a loanword some speakers keep in Cyrillic; proposed: no, it is expected in Latin script.
- Target version: 0.18.1 (PATCH on 0.18.0).

## Release

SemVer: PATCH — a fix of refinement behavior the spec already requires.
Target version: 0.18.1.
