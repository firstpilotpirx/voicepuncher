## Context

Two `@refinement` E2E scenarios of `text-refinement` dictate phrases that are near copies of `DEFAULT_EXAMPLES` in `app/src/config/refinement.rs`, so they show that Qwen3-4B repeats its few-shot examples, not that the rules carry over to new speech. The dictations live in `e2e-test/tests/steps/text_refinement/normalization.rs`; the step texts in `e2e-test/tests/features/text_refinement.feature` name the words.

## Goals / Non-Goals

**Goals:** both scenarios dictate sentences whose structure, English words and IT terms differ from every example, and pass on Qwen3-4B-Instruct-2507 Q4_K_M.

**Non-Goals:** any change to the prompt, the examples, the default model, other scenarios or the evaluation set.

## Decisions

| Question | Chosen | Why | Rejected |
|---|---|---|---|
| Two English words | «у этого флага всего два значения энейблд и дизейблд» → `enabled`, `disabled` | no example has a flag, its values or a list after a colon-like pause; passed on Qwen3-4B on an own port | «в плеере включи шаффл и выключи репит» (shuffle, repeat): the model kept «шаффл» in Cyrillic, which would turn this into a prompt change |
| Product names as syllables | «он переписал весь проект с джава скрипт на тайп скрипт» → `JavaScript`, `TypeScript` | "from X to Y" structure and languages instead of tools; canonical spellings, so the case-sensitive check has no ambiguity; passed | phrases from the evaluation's `english`/`terms` cases: the test would duplicate the scored evaluation |
| Step texts | rename the When and Then steps to the new words | the Gherkin text must say what is dictated and checked | keep the old step texts: they would describe a dictation no longer sent |
| Checks | unchanged helpers: case-insensitive contains for the words, case-sensitive contains for the product names | same meaning as before | |

## Test levels

| Scenario | Level |
|---|---|
| English word spelled in Cyrillic comes out in Latin script | E2E `.feature`, `@refinement`, live service, unchanged |
| Two English words in one sentence both come out in Latin script | E2E `.feature`, `@refinement`, live service, new dictation |
| Misheard English word is restored from context | E2E `.feature`, `@refinement`, live service, unchanged |
| Russian loanwords stay in Cyrillic | E2E `.feature`, `@refinement`, live service, unchanged |
| Russified English verbs stay in Cyrillic | E2E `.feature`, `@refinement`, live service, unchanged |
| Shell framework and theme spelled as syllables are restored | E2E `.feature`, `@refinement`, live service, unchanged |
| Product names spelled as syllables are restored | E2E `.feature`, `@refinement`, live service, new dictation |

## Risks / Trade-offs

- A phrase passed in one run; a model's answer can vary between runs → `just check` runs the scenarios again, and a failure there is reported as a flake per AGENTS.md, not loosened.
- The rejected «шаффл» shows the rule does not carry over to every word → reported to the user as possible follow-up work, out of this change.
