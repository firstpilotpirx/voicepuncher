## 1. New dictations for the three scenarios

- [x] 1.1 Red: in `e2e-test/tests/features/text_refinement.feature` reword the three scenarios' `When`/`Then` lines to the retry, debug and Bash-it/Powerline dictations, and run `just test` — the E2E build fails because no step matches the new texts
- [x] 1.2 Green: in `e2e-test/tests/steps/text_refinement/normalization.rs` replace the feedback, dynamic and oh-my-zsh constants and step texts with the new dictations and checks; `just test` passes
- [x] 1.3 Run every `@refinement` scenario on the default model on test ports (`just check`) and confirm all pass

## 2. Version

- [x] 2.1 Keep `version` in `app/Cargo.toml` unchanged: the change touches tests only, no release (proposal § Release)
