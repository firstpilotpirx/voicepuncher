## 1. Reproduce the bug

- [x] 1.1 Red: add the scenario "Shell terms inside Russian dictation come out in Latin script" to `e2e-test/tests/features/speech_recognition.feature` with its steps in `e2e-test/tests/steps/speech_recognition/prompt.rs`; run it with `just e2e-smoke` and see it fail
- [x] 1.2 `just test` and commit the scenario alone as an intentionally red reproduction

## 2. Default prompt

- [x] 2.1 Red: unit tests in `app/src/config/transcription.rs`: a config without a prompt gets the default sentence, a configured prompt is kept as written, `prompt = ""` stays empty; run `just test`, see the first fail
- [x] 2.2 Green: `TranscriptionCfg::default_prompt()` with the sentence from design.md, used by serde and `Default`; doc comment of `prompt` says what the default does
- [x] 2.3 Keep `request.rs` tests that mean "no prompt" on an explicit empty prompt; `just test`
- [x] 2.4 Run the reproduction scenario with `just e2e-smoke` and see it pass
- [x] 2.5 README § Writing `[transcription] prompt`: the default sentence, that a configured prompt replaces it and `prompt = ""` turns it off
- [x] 2.6 `just test` and commit

## 3. Refinement evaluation

- [x] 3.1 Add the correctly recognized sentence with oh-my-zsh, zsh and Powerlevel10k as a `terms` case to `e2e-test/refinement/eval_set.jsonl`
- [x] 3.2 `just test` and commit

## 4. Version and manual checks

- [x] 4.1 Bump `version` in `app/Cargo.toml` to 0.15.4
- [ ] 4.2 `just check` once, then commit
- [ ] 4.3 Manual checks: none (no hotkey, recording, overlay, paste or permission change)
