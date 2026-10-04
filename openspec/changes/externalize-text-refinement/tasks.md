# Tasks

## 1. Configuration

- [ ] 1.1 Replace `LlmCfg` with `RefinementCfg` (`[refinement]`: `base_url`, `model`, `api_key`, `system_prompt`, `temperature`, `max_tokens`, `timeout_secs`) per design D5; verify with unit tests for defaults and TOML parsing in `app/src/config.rs`
- [ ] 1.2 Reject non-loopback `base_url` hosts at load, reusing the transcription validator if it exists; verify unit tests cover accepted (`localhost`, `127.0.0.1`, `[::1]`) and rejected (`192.168.1.10`, `api.openai.com`) hosts
- [ ] 1.3 Report removed `[llm]` keys (`model_path`, `context_size`) with their replacement before strict parsing; verify a unit test per key checks the message
- [ ] 1.4 Update `app/config.example.toml` and verify `config_loader_it.rs` loads it

## 2. OpenAI refinement adapter

- [ ] 2.1 Add `serde_json` to `app/Cargo.toml` if missing, and `app/src/adapters/secondary/textrefinement/openai/` with `OpenAiTextRefiner`: request body (D1), optional bearer header, timeout, `max_tokens` budget (D3), `finish_reason == "length"` → `Err`, half-length check, `<think>` stripping (D4); verify unit tests for the request body, budget, truncation and stripping
- [ ] 2.2 Add `OpenAiTextRefiner::connect` startup check (D6); verify unit tests for reachable/unreachable, single-model server with any name, multi-model server with a missing name → error listing offered ids
- [ ] 2.3 Add `app/tests/textrefinement_openai_it.rs` against an in-test stub server (`std::net::TcpListener`): request fields and bearer header sent, HTTP 500 → `Err`, malformed JSON → `Err`, slow response → timeout `Err`; plus one test gated on `VOICEPUNCHER_REFINEMENT_URL` that refines a Russian sentence through a real server and keeps it Russian; verify `cargo test -p voicepuncher --test textrefinement_openai_it`

## 3. Wiring and CLI

- [ ] 3.1 Replace `--llm-model` with `--refinement-url` in `app/src/adapters/primary/cli/` and `cli_it.rs`, keep `--no-llm`; verify `cargo test -p voicepuncher --test cli_it`
- [ ] 3.2 Build `OpenAiTextRefiner` in both modes of `main.rs`, skip the startup check with `--no-llm`, exit with code 2 and the D6 message otherwise; verify by running `voicepuncher --input-file <wav>` with no LLM server (exit 2 within 3 s, address in the message) and with `--no-llm` (raw transcript printed)
- [ ] 3.3 Add a pipeline unit test in `app/src/core/application/pipeline.rs` asserting the raw transcript is pasted when refinement returns a truncation error; verify `cargo test -p voicepuncher --lib pipeline`

## 4. Remove the in-process engine

- [ ] 4.1 Delete `textrefinement/llamacpp/` and `app/tests/textrefinement_llamacpp_it.rs`, drop `llama-cpp-2` from `app/Cargo.toml`; verify `cargo tree -p voicepuncher | grep llama` prints nothing
- [ ] 4.2 Remove the Homebrew OpenSSL `CPATH`/`LIBRARY_PATH` exports from `justfile` and the OpenSSL note from `CLAUDE.md` once no C++ engine remains; verify `cargo build --release` succeeds in a shell without those variables
- [ ] 4.3 Update `docs/ubiquitous-language.md`: replace the `llamacpp` tech row with `openai` (or extend it if it exists), add the term "text refinement service"; verify every term used in `specs/text-refinement/spec.md` appears in the glossary
- [ ] 4.4 Update `app/tests/architecture.rs` if it names `llamacpp`; verify `cargo test -p voicepuncher --test architecture`

## 5. Setup wizard

- [ ] 5.1 Remove the LLM model choice and download from `SetupWizard`; ask for the service address (default `http://127.0.0.1:8080/v1`) and model name, write `[refinement]`; verify `setup_cli_it.rs` asserts the written section

## 6. Tooling, E2E and docs

- [ ] 6.1 Add `just llm-server` (D7) and make `e2e-smoke`/`e2e-bench` start the server, wait for `/v1/models`, run, and stop it; verify `just e2e-smoke` passes from a state where no server runs
- [ ] 6.2 Pass `--refinement-url` instead of `--llm-model` in `e2e-test/tests/steps/*.rs`; verify `just e2e-smoke`
- [ ] 6.3 Document running `llama-server`, LM Studio and Ollama, the `[refinement]` section and the migration from `[llm]` in `README.md`; change CLAUDE.md "Zero network I/O" to "No internet; loopback connections only" if not already changed; verify the README commands run as written

## 7. Verification

- [ ] 7.1 Run `just check`; verify it passes
- [ ] 7.2 Run `just e2e`; verify mean WER with LLM ≤ 7.2 %
- [ ] 7.3 User runs the manual checklist in `README.md` § Release checklist, plus: stop the text refinement service and start the app → it exits naming the address; start with `--no-llm` and no service → raw text is pasted; stop the service while the app runs and record → the raw transcript is pasted
