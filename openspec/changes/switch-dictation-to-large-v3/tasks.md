## 1. The model and its decoding (D1, D2)

- [x] 1.1 Red: rename the rule and the default scenario in `e2e-test/tests/features/speech_recognition.feature` to the large-v3 model, the step in `e2e-test/tests/steps/speech_recognition/service_models.rs` and `DICTATION_MODEL` in `e2e-test/tests/steps/local_services/model_home.rs` to `ggml-large-v3.bin`; `kit feature-lint`
- [x] 1.2 Red: the catalog unit tests in `app/src/core/domain/model_catalog.rs` expect `ggml-large-v3.bin`, its URL, sha256 prefix and size; the `whisper_args` tests expect `-bs 5` (`just test-one` fails)
- [x] 1.3 Green: the catalog entry, `DICTATION_MODEL` in `app/src/adapters/secondary/localservices/catalog/mod.rs`, `-bs 5` in `app/src/adapters/secondary/localservices/catalog/args.rs` and `dev-tools/src/services/command_lines.rs` with its doc comment, `dictation_model_file` and the `models` comment in `justfile`; `just test-one` green, `just static`
- [x] 1.4 Every other test and step naming the turbo file (`app/src/runtime/`, `app/src/core/application/local_services/`, `app/src/adapters/`, `app/tests/`, `dev-tools/src/services/tests.rs`) names the new one; `just static`
- [x] 1.5 `just test`; `kit ship -m "feat(speech-recognition): dictate with whisper large-v3 and beam search 5"`

## 2. Documents (D3)

- [ ] 2.1 `README.md` (services table, command line, model table, prompt notes, memory and disk), `AGENTS.md`, `docs/architecture/` and `docs/ubiquitous-language.md` name large-v3; the turbo benchmark rows stay as history
- [ ] 2.2 `just test`; `kit ship -m "docs: dictation runs on whisper large-v3"`

## 3. Measured budgets and version (D4)

- [ ] 3.1 Put `ggml-large-v3.bin` into `~/.voicepuncher/models/`; `just check`; set every model stage budget and nextest limit it passes from that run; the accuracy baseline only if better
- [ ] 3.2 `kit version-check switch-dictation-to-large-v3`; bump `version` in `app/Cargo.toml` to 0.47.0
- [ ] 3.3 `just test`; `kit ship -m "chore(release): bump version to 0.47.0"`
- [ ] 3.4 Manual check by the user: dictate in the installed app; the log shows `transcribed` with the new model
