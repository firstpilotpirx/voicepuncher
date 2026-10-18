# Tasks

## 1. Evaluation: English words category and loanword controls

- [x] 1.1 Red: unit tests in `e2e-test/src/refinement_eval.rs` — an item's `keep_cyrillic` word missing from the pasted text yields `LoanwordLeftCyrillic`; a word kept with another ending passes; the committed set has `english` items
- [x] 1.2 Green: optional `keep_cyrillic` field, the `LoanwordLeftCyrillic` verdict and its line in the report table in `e2e-test/src/refinement_eval.rs`
- [x] 1.3 Add the 10 research cases as category `english` to `e2e-test/refinement/eval_set.jsonl`, the 3 controls with `keep_cyrillic`, стендап expected in Cyrillic
- [x] 1.4 Red: scenario "Evaluation fails when a loanword is respelled in Latin script" (`@smoke`, stub answer) in `e2e-test/tests/features/text_refinement.feature` with steps in `e2e-test/tests/steps/text_refinement/evaluation.rs`; add `english` to the reported categories
- [x] 1.5 Green: run the scenario with `just e2e-smoke`; `just test`; commit

## 2. Default instruction: English words in Latin script

- [x] 2.1 Red: unit test in `app/src/config/refinement.rs` that the default instruction carries the misheard IT terms rule and the English-words rule, limits "exactly as dictated" to English words in Latin script, and the examples include the manual/automatic and the two IT-term examples
- [x] 2.2 Green: both rules, reworded clause and three examples in `app/src/config/refinement.rs`; the misheard-term inputs of `research/misheard-terms.md` as `terms` cases in `e2e-test/refinement/eval_set.jsonl`
- [x] 2.3 Red: `@refinement` scenarios for the new requirement (feedback, manual and automatic, даймэк → dynamic, loanwords, Russified verbs, oh-my-zsh and Powerlevel10k, GitHub Actions and VS Code) and "English word fixes improve on the raw transcript" in `e2e-test/tests/features/text_refinement.feature`, steps in `e2e-test/tests/steps/text_refinement/normalization.rs` and `evaluation.rs`
- [x] 2.4 `just test`; commit

## 3. Respellings pass the not-a-normalization check

- [x] 3.1 Red: unit tests in `app/src/core/domain/normalization_check.rs` — feedback, oh-my-zsh/Powerlevel10k and GitHub Actions/VS Code respellings are accepted; a translated Russian part of a mixed dictation, an English translation and a Russian rewrite are still rejected
- [x] 3.2 Red: scenarios "Service respells most of a short dictation in Latin script" and "Service translates the Russian part of a mixed dictation" (`@smoke`, stub) in `e2e-test/tests/features/text_refinement.feature` with steps in `e2e-test/tests/steps/text_refinement/answer_check.rs`
- [x] 3.3 Green: sound skeletons and respelling matching in `normalization_check.rs`; matched words count as kept and leave the writing-system shift
- [x] 3.4 `just test`; commit

## 4. Default model: Qwen3-4B-Instruct-2507

- [x] 4.1 `justfile`: find the refinement model by the file name `Qwen_Qwen3-4B-Instruct-2507-Q4_K_M.gguf`; `just models` downloads it pinned to one revision with its sha256 instead of Qwen2.5-1.5B; update the `models` and settings comments
- [x] 4.2 README and `AGENTS.md`: the new default refinement model and its size
- [x] 4.3 Run `just refine-eval` and the `@refinement` scenarios pass with the new model on an own port (design § Verification); record per-category scores against 1.5B and the latency on the 180-character phrase for the PR
- [x] 4.4 `just test`; commit

## 5. Version and checks

- [x] 5.1 Bump `version` in `app/Cargo.toml` to 0.16.0
- [x] 5.2 `just check` with the new model on an own port; commit
- [x] 5.3 Manual checks by the user: none (no hotkey, recording, overlay, paste or permissions change); after the merge, restart the shared text refinement service to load the new model
