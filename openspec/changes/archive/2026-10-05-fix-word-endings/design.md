# Design

## Context

See proposal.md — Why. Three places decide whether a corrected ending reaches the paste:

- the default instruction in `app/src/config/refinement.rs` lists punctuation, capitals, misrecognized words, technical terms and filler sounds — nothing about agreement — and its three examples show no ending fix;
- `check_words_kept` in `app/src/core/domain/normalization_check.rs` counts a transcript word as kept only when the answer has the identical lowercase form, so "у меня нет время на это задачу" → "У меня нет времени на эту задачу." keeps 5 of 7 words, but a short phrase with three of four endings fixed keeps 1 of 4 and is rejected;
- the refinement evaluation (`e2e-test/refinement/eval_set.jsonl`) has no item with a wrong ending, so nothing measures the duty.

A probe of the default model (Qwen2.5-1.5B-Instruct) with the current prompt already fixes many endings ("она пошёл в магазин и купил хлеба" → "Она пошла в магазин и купила хлеба."), so the prompt makes the duty explicit and the check stops discarding it.

## Goals / Non-Goals

Goals: an explicit ending duty in the default instruction; a words-kept check tolerant of ending changes and still rejecting translations, answers and rewrites; a measured `endings` category.

Non-goals: a morphology library or dictionary; changing the Cyrillic-share check; English inflection rules.

## Decisions

### D1. Same word, other ending: shared stem with short tails

Two lowercase words (ё read as е) are the same word when they are equal, or when their common prefix is at least 2 letters and at least half of the longer word, and the remaining tail of each word is at most 3 letters. Examples: время/времени (prefix 4 of 7, tails 1/3), пошел/пошла, это/эту, жена/жены, сказал/сказали — same; моей/своей, имею/нет, dogs/foxes — different. One-letter words must be equal.

Alternatives: a Russian stemmer crate (Snowball) — a new dependency for a heuristic that only has to separate "same dictation" from "different text"; edit distance ≤ 2 — matches unrelated short words (кот/кит) and misses время/времени. The prefix rule is a few lines in the domain, no framework import.

It applies to both alphabets: English "fix"/"fixed" counts as kept, which loosens nothing that matters — a translation still fails the writing-system check and an unrelated answer still shares few stems.

### D2. Prompt and example

The instruction gains one duty after the filler sounds: fix wrong word endings — case, gender, number, declension, conjugation — so the words agree, changing only the ending and keeping the word, the word order and the meaning; with two short arrows as in the term list (`она пошёл -> она пошла`, `нет время -> нет времени`). A fourth default example shows a sentence with four wrong endings and its normalization. The unit test that pins the duties gains "ending"; the example-count test becomes four with one ending fix.

The prompt's "never add or drop meaning, never shorten" stays, so the model is not invited to rewrite.

### D3. Evaluation category `endings`

About eight Russian items with wrong case, gender, number or verb endings, the expected text fixing only endings plus punctuation. The category list in the evaluation steps gains `endings`. The new scenario compares the `endings` category score with its raw score from the same report.

### D4. Stub answer that corrects endings

The stub scenario must not depend on how the speech recognition service writes the spoken sentence. A new stub behavior answers the received transcript with the last letter of each Cyrillic word of three or more letters replaced, plus a capital and a period — most endings changed, every stem kept. The app must paste that answer.

## Test levels

| Scenario | Level |
|---|---|
| Wrong word endings are corrected | E2E `.feature`, `@refinement` (live default model) |
| Service answers with most word endings corrected | E2E `.feature`, `@smoke` (stub); unit tests in `normalization_check.rs` for D1 |
| Word ending fixes improve on the raw transcript | E2E `.feature`, `@refinement` |
| Evaluation reports a score per category | E2E `.feature`, existing; category list gains `endings` |
| Unpunctuated dictation gets punctuation and capitals | E2E `.feature`, `@refinement`, existing |
| Filler sounds are removed | E2E `.feature`, `@refinement`, existing |
| Technical term is written in its usual spelling | E2E `.feature`, `@refinement`, existing |
| Mixed Russian and English dictation keeps both languages | E2E `.feature`, `@refinement`, existing |
| Service answers with a translation | E2E `.feature`, `@smoke`, existing; unit tests in `normalization_check.rs` |
| Service answers instead of correcting | E2E `.feature`, `@smoke`, existing; unit tests in `normalization_check.rs` |
| Service answers with a normalization | E2E `.feature`, `@smoke`, existing |
| Evaluation fails when a dictated command is carried out | E2E `.feature`, `@smoke`, existing |

The bug "a short phrase with most endings fixed is rejected" gets its reproduction first: the unit test and the stub scenario, committed red on their own.

## Risks / Trade-offs

- [The 1.5B model over-corrects or rewrites when told to fix grammar] → the evaluation runs every category; any language change or carried-out command fails it; the `endings` score must beat raw.
- [Looser matching lets a rewrite through] → the rule needs a long shared prefix and short tails; the existing "answers instead of correcting" and translation scenarios stay as guards.
- [A user's own `system_prompt` in config does not get the new duty] → expected; defaults only change when the key is absent.

## Migration Plan

None: defaults change, no config keys change. Rollback is a revert.
