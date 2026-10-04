# Tasks

## 1. Configuration

- [x] 1.1 Red: unit tests in `app/src/config/refinement.rs` for `RefinementCfg` defaults (D5) and for parsing a `[refinement]` section in `app/src/config.rs`; run them and see them fail
- [x] 1.2 Green: add `RefinementCfg` and the `[refinement]` section to `Config` (the `[llm]` section stays until group 3)
- [x] 1.3 Red: unit tests that `ensure_loopback` names the rejected setting, and that `load` and `validate` reject a remote `[refinement].base_url` (`192.168.1.10`, `api.openai.com`) and accept `localhost`, `127.0.0.1`, `[::1]`; see them fail
- [x] 1.4 Green: move `ensure_loopback(setting, url)` to `app/src/config/loopback.rs` and check both service addresses in `validate`
- [x] 1.5 `just check` and commit

## 2. OpenAI refinement adapter

- [x] 2.1 Red: unit tests in `textrefinement/openai/request.rs` for the request body (D1: model, system and delimited user message, temperature, `max_tokens`, `stream: false`), the budget (D3), the answer: `finish_reason == "length"` → `Err`, shorter than half the input → `Err`, `<think>` block stripped (D4); see them fail
- [x] 2.2 Green: add `textrefinement/openai/` with `OpenAiTextRefiner` (D2): request body, optional bearer header, timeout, answer parsing
- [x] 2.3 Red: unit tests for `OpenAiTextRefiner::connect` (D6) against an in-test stub: reachable single-model server with any name, closed port → message with the address and `just start-services`, multi-model server without the configured name → message listing the offered ids, multi-model server with it → `Ok`; see them fail
- [x] 2.4 Green: implement `connect`
- [x] 2.5 Red: `app/tests/textrefinement_openai_it.rs` against an in-test stub server: request fields and bearer header sent, no header without a key, HTTP 500 → `Err`, malformed JSON → `Err`, slow response → timeout `Err`; plus one test gated on `VOICEPUNCHER_REFINEMENT_URL` that refines an English sentence through a real service; see the stub tests fail where the adapter falls short, then make them pass
- [x] 2.6 `just check` and commit

## 3. Switch the app to the text refinement service

- [x] 3.1 Red: `e2e-test/tests/features/text_refinement.feature` with one scenario per spec scenario (D10), steps in `e2e-test/tests/steps/text_refinement.rs` (speech WAV via `say`, stub text refinement service, real service through `VOICEPUNCHER_REFINEMENT_URL`); `just start-services` / `stop-services` and the E2E helpers also run `llama-server` (D7) and export `VOICEPUNCHER_REFINEMENT_URL`; run `just e2e-smoke` and see the new scenarios fail
- [x] 3.2 Green: replace `--llm-model` with `--refinement-url` in `app/src/adapters/primary/cli/` and `cli_it.rs`, keep `--no-llm`
- [x] 3.3 Green: build `OpenAiTextRefiner` in both modes of `main.rs`, skip the startup check with `--no-llm`, exit 2 with the D6 message otherwise
- [x] 3.4 Red: unit test in `app/src/core/application/pipeline.rs` that `run_once` (file mode) pastes the raw transcript when refinement fails; see it fail (today only a push-to-talk cycle falls back)
- [x] 3.5 Green: `run_once` falls back to the raw transcript through the same code as a push-to-talk cycle
- [x] 3.6 Green: replace the `[llm]` section with the migration error (D5, unit test per removed key checks the replacement it names) and update `app/config.example.toml` (`config_loader_it.rs` loads it)
- [x] 3.7 Green: pass `--refinement-url` instead of `--llm-model` in `e2e-test/tests/steps/file_transcription.rs`; `just e2e-smoke` passes from a state where no service runs and leaves none running
- [x] 3.8 `just check` and commit

## 4. Remove the in-process engine

- [x] 4.1 Delete `textrefinement/llamacpp/` and `app/tests/textrefinement_llamacpp_it.rs`, drop `llama-cpp-2` from `app/Cargo.toml` and the `ggml` log filter; `cargo tree -p voicepuncher | grep llama` prints nothing
- [x] 4.2 Remove the Homebrew OpenSSL `CPATH`/`LIBRARY_PATH` exports from `justfile`; `cargo build --release` succeeds in a shell without those variables
- [x] 4.3 Remove `llama_cpp_2` from the forbidden lists in `app/tests/architecture.rs`; `cargo test -p voicepuncher --test architecture` passes
- [x] 4.4 `just check` and commit

## 5. Setup wizard

- [x] 5.1 Red: `setup_cli_it.rs` asserts the written `[refinement]` section (address and model) loads back and no `[llm]` section is written; see it fail
- [x] 5.2 Green: ask for the text refinement service address and model name in `SetupWizard`, write `[refinement]`; remove the models-directory question, the LLM choice and download, `ModelDownloader`, `model_downloader_it.rs` and `indicatif` (D8)
- [x] 5.3 `just check` and commit

## 6. Tooling and docs

- [x] 6.1 `e2e-test/scripts/bench.py` starts `llama-server` per LLM model (D9) and passes `--refinement-url`; `python3 -m py_compile` passes and the per-model server helper refines one sentence with the installed model
- [x] 6.2 `README.md`: running the text refinement service (`just start-services` and by hand), the `[refinement]` section, `--refinement-url`, the migration from `[llm]`, the release checklist items below; `AGENTS.md`: text cleanup through a local `llama-server`; the README commands run as written
- [x] 6.3 `docs/ubiquitous-language.md`: drop the `llamacpp` row, extend the `openai` row to `OpenAiTextRefiner`, add "Text refinement service", update `SetupWizard`; `docs/project-map.md`: replace the `llamacpp` adapter row; every term in `specs/text-refinement/spec.md` appears in the glossary
- [x] 6.4 `just check` and commit

## 7. Release

- [x] 7.1 Bump `version` in `app/Cargo.toml` to `0.3.0`; `just check` and commit
- [x] 7.2 `just e2e`: mean WER with LLM on the corpus ≤ 7.2 % (6.9 % over 300 recordings in `just full-check`)
- [ ] 7.3 Manual run by the user: `README.md` § Release checklist, plus: stop the text refinement service and start the app → it exits naming the address; start with `--no-llm` and no text refinement service → the raw transcript is pasted; stop the text refinement service while the app runs and record → the raw transcript is pasted
