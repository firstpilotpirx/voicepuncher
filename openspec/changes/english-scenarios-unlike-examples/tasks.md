## 1. Scenarios unlike the examples

- [ ] 1.1 Red: rename the When and Then steps of "Two English words in one sentence both come out in Latin script" and "Product names spelled as syllables are restored" in `e2e-test/tests/features/text_refinement.feature` to the new dictations, and run `just test`: the scenarios have no matching steps
- [ ] 1.2 Green: in `e2e-test/tests/steps/text_refinement/normalization.rs` dictate «у этого флага всего два значения энейблд и дизейблд» (expect `enabled`, `disabled`) and «он переписал весь проект с джава скрипт на тайп скрипт» (expect `JavaScript`, `TypeScript`), with step texts matching the feature; `just test` passes
- [ ] 1.3 Run `VOICEPUNCHER_LLM_PORT=<free port> just refine-eval` on Qwen3-4B-Instruct-2507 Q4_K_M: every `@refinement` scenario passes
- [ ] 1.4 `just test` passes; commit `test(e2e): dictate refinement phrases unlike the prompt examples`

## 2. Version

- [ ] 2.1 No version bump: the proposal says no release (tests only); `app/Cargo.toml` stays as it is.
