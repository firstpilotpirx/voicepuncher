# Tasks

## 1. Normalization check keeps words with corrected endings (bug)

- [x] 1.1 Red: unit tests in `app/src/core/domain/normalization_check.rs` — a short Russian answer with most endings corrected passes; same-word pairs from design D1 (время/времени, пошёл/пошла, это/эту) count as kept, different words (моей/своей, кот/кит, one-letter words) do not; run `just test`, see them fail
- [x] 1.2 Red: stub behavior in `e2e-test/tests/steps/refinement_service_stub.rs` that answers the transcript with the last letter of every Cyrillic word of three or more letters replaced, plus a capital and a period (design D4); scenario "Service answers with most word endings corrected" (`@smoke`) in `e2e-test/tests/features/text_refinement.feature` with its steps in `answer_check.rs`; run it, see it fail; commit the red tests on their own (`test: reproduce answer with corrected endings rejected`, intentionally red)
- [x] 1.3 Green: same-word matching by shared stem and short tails in `check_words_kept` (design D1); `just test` and the new smoke scenario pass
- [x] 1.4 Update the "Normalization check" entry in `docs/ubiquitous-language.md`: a word with a different ending counts as kept
- [x] 1.5 `just check`, commit

## 2. Default instruction fixes word endings

- [x] 2.1 Red: unit tests in `app/src/config/refinement.rs` — the default prompt names the ending duty ("ending"), and the defaults have four examples, one of them a Russian sentence whose normalization changes only word endings and punctuation; run `just test`, see them fail
- [x] 2.2 Red: scenario "Wrong word endings are corrected" (`@refinement`) in `text_refinement.feature`, steps in `e2e-test/tests/steps/text_refinement/normalization.rs` — a Russian sentence whose verbs disagree with the subject in gender; the pasted text has the agreeing verb forms and the same words in the same order; run `just refine-eval`, see it fail or record that the current model already passes and tighten the sentence until it fails
- [x] 2.3 Green: add the ending duty to the default prompt and the fourth default example (design D2); `just test` and the new scenario pass
- [x] 2.4 Update the "Text refinement" and "CleanedText" entries in `docs/ubiquitous-language.md` to list word endings among the fixes
- [x] 2.5 `just check`, commit

## 3. Evaluation measures word endings

- [x] 3.1 Red: scenario "Word ending fixes improve on the raw transcript" (`@refinement`) in `text_refinement.feature`, step in `evaluation.rs` comparing the `endings` category score with its raw score; `endings` added to the category list of "Evaluation reports a score per category"; the unit test loading the committed set requires an `endings` category; run them, see them fail
- [x] 3.2 Green: about eight `endings` items in `e2e-test/refinement/eval_set.jsonl` (case, gender, number, verb endings; expected text fixes only endings, punctuation and capitals); `just test` and `just refine-eval` pass
- [x] 3.3 `just check`, commit

## 4. Version and manual checks

- [x] 4.1 Bump `version` in `app/Cargo.toml` to 0.6.0
- [x] 4.2 `just check`, commit
- [x] 4.3 Manual checks: none (no hotkey, recording, overlay, paste or permissions change)
