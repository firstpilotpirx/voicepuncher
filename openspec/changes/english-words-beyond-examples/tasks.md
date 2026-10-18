## 1. English words beyond the examples

- [ ] 1.1 Red: add the scenario "English words the instruction never shows come out in Latin script" to `e2e-test/tests/features/text_refinement.feature` with its steps in `e2e-test/tests/steps/text_refinement/normalization.rs`; run `just refine-eval`, see it fail on «шаффл»/«репит»; commit as intentionally red
- [ ] 1.2 Green: reword the English-words paragraph of the default instruction and append the new example in `app/src/config/refinement.rs`; update the unit tests that count the examples
- [ ] 1.3 Add the evaluation case `english-11` (ordinary Russian word kept) to `e2e-test/refinement/eval_set.jsonl`
- [ ] 1.4 Run `just test` and `just refine-eval`; every `@refinement` scenario and the evaluation pass; commit

## 2. Version and checks

- [ ] 2.1 Bump `version` in `app/Cargo.toml` to 0.18.1; run `just check`; commit
- [ ] 2.2 Manual checks by the user: none
