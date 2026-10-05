# Proposal

## Why

Russian dictation often reaches the paste with wrong word endings — wrong case, gender, number or verb form ("она пошёл в магазин", "у меня нет время на это задачу") — because the speaker slurred an ending or misspoke, or the speech recognizer misheard it. Text refinement does not fix them on purpose: the default instruction never asks for grammatical agreement, the spec tells it to keep every word as dictated, and the normalization check compares exact word forms, so an answer that corrects most endings of a short phrase is rejected and the raw transcript is pasted instead.

## What Changes

- The default refinement instruction asks the model to fix grammatical agreement: case, gender, number, declension and conjugation endings, changing only the ending and keeping the word, its order and the meaning. One default example shows an ending fix.
- The normalization check counts a transcript word as kept when the answer has the same word with a different ending, so correcting endings no longer makes the answer look like a rewrite; a translation, an answer or a rewrite is still rejected.
- The refinement evaluation gets an `endings` category of dictation with wrong endings and reports its score next to the others.

Current stage: PoC. Target stage: PoC. Scope: Russian word endings in text refinement with the default instruction and model. Acceptance bar: the word endings category scores above the raw transcript in the refinement evaluation, every existing category keeps passing (no dictated command carried out, no language change), and the E2E scenarios below pass on the default model.

Out of scope:
- Fixing word choice, word order, style or meaning ("я не имею времени" for "у меня нет времени" stays a rewrite, not a fix).
- Grammar of English dictation beyond what the model already does.
- Changing the default language model or its size.
- A rule-based (non-LLM) morphology fixer.

SemVer: MINOR (new normalization duty of an existing capability), target version 0.6.0.

## Capabilities

### New Capabilities

### Modified Capabilities
- `text-refinement`: "Refinement normalizes dictation" also fixes wrong word endings; "Answer that is not a normalization falls back to the raw transcript" counts a word with a corrected ending as kept; "Refinement quality can be measured" adds a word endings category.

## Impact

- `app/src/config/refinement.rs`: default system prompt and default examples.
- `app/src/core/domain/normalization_check.rs`: word matching in the words-kept check.
- `e2e-test/refinement/eval_set.jsonl`, `e2e-test/src/refinement_eval.rs`: word endings (`endings`) category.
- `e2e-test/tests/features/text_refinement.feature` and its steps: word endings scenarios.
- `docs/ubiquitous-language.md`: the term for a word ending fix, if one is introduced.
- No new dependencies, no config keys added or removed; a user config with its own `system_prompt` or `examples` keeps them.
