# Tasks

## 1. Configuration

- [x] 1.1 Replace `WhisperCfg` with `TranscriptionCfg` (`[transcription]`: `base_url`, `model`, `language`, `prompt`, `temperature`, `timeout_secs`, chunking keys) per design D5; verify with unit tests for defaults and TOML parsing in `app/src/config.rs`
- [x] 1.2 Reject non-loopback `base_url` hosts in `config::load` and in a `validate` function `main.rs` calls after the CLI overlay (D6); verify unit tests cover `localhost`, `127.0.0.1`, `127.1.2.3`, `[::1]` (accepted) and `192.168.1.10`, `example.com` (rejected)
- [x] 1.3 Report removed `[whisper]` keys with their replacement before strict parsing (D5); verify a unit test per removed key checks the message names the replacement
- [x] 1.4 Update `app/config.example.toml` and verify `config_loader_it.rs` loads it

## 2. Hallucination filter in the application layer

- [x] 2.1 Move `is_whisper_hallucination` and `has_repetition_loop` with their unit tests to `app/src/core/application/transcript_filter.rs` (D4); verify the moved tests pass and `app/tests/architecture.rs` passes

## 3. OpenAI transcription adapter

- [x] 3.1 Add `serde_json` to `app/Cargo.toml` and `app/src/adapters/secondary/transcription/openai/` with `OpenAiTranscriber`: WAV encoding, multipart request, `\n` normalization (D3), hallucination filter call, timeout; verify unit tests for WAV header, multipart body and normalization (`наход\nившаяся` → `находившаяся`)
- [x] 3.2 Add `OpenAiTranscriber::connect` TCP reachability check (D7); verify unit test against a bound and an unbound local port
- [x] 3.3 Add `app/tests/transcription_openai_it.rs` against an in-test stub server (`std::net::TcpListener`): request fields sent, `language` omitted for `auto`, HTTP 500 → `Err`, malformed JSON → `Err`, slow response → timeout `Err`; plus one test gated on `VOICEPUNCHER_TRANSCRIPTION_URL` that transcribes a corpus WAV through a real server; verify `cargo test -p voicepuncher --test transcription_openai_it`

## 4. Wiring and CLI

- [x] 4.1 Replace `--whisper-model` with `--transcription-url` in `app/src/adapters/primary/cli/` and `cli_it.rs`; verify `cargo test -p voicepuncher --test cli_it`
- [x] 4.2 Build `OpenAiTranscriber` in both modes of `main.rs` and exit with code 2 and the D7 message when unreachable; exit with code 2 on any config error instead of falling back to defaults (D9); verify by running `voicepuncher --input-file <wav>` with no server and checking exit code 2 and the message within 3 s

## 5. Remove the in-process engine

- [x] 5.1 Delete `transcription/whispercpp/` and `app/tests/transcription_whispercpp_it.rs`, drop `whisper-rs` from `app/Cargo.toml`, `whisper_rs::install_logging_hooks()` and the `whisper_rs`/`ggml` log filters; verify `cargo tree -p voicepuncher | grep whisper` prints nothing
- [x] 5.2 Update `docs/ubiquitous-language.md`: replace the `whispercpp` tech row with `openai` (OpenAI-compatible HTTP API via `ureq`), add the term "speech recognition service"; verify every term used in `specs/speech-recognition/spec.md` appears in the glossary
- [x] 5.3 Update `app/tests/architecture.rs` if it names `whispercpp`; verify `cargo test -p voicepuncher --test architecture`

## 6. Setup wizard

- [x] 6.1 Remove the Whisper model choice and download from `SetupWizard`; ask for the service address (default `http://127.0.0.1:8081/v1`) and write `[transcription]`; verify `setup_cli_it.rs` asserts the written section

## 7. Tooling, E2E and docs

- [x] 7.1 Add `just start-services` and `just stop-services` (D8): install check with the Homebrew offer, skip when the port already answers, PID and log files, readiness wait, stop only what was started; verify `start-services` twice starts one server, `stop-services` stops it, and a server started by hand survives `stop-services`
- [x] 7.2 Make `e2e-smoke`, `e2e-quick` and `e2e-bench` run `start-services` before the tests and `stop-services` on exit; verify `just e2e-smoke` passes from a state where no server runs and leaves none running
- [x] 7.3 Pass `--transcription-url` instead of `--whisper-model` and `--config /dev/null` in `e2e-test/tests/e2e/*.rs` (D9); verify `just e2e-smoke`
- [x] 7.4 Make `e2e-test/scripts/bench.py` start `whisper-server` per Whisper model (D10); verify `python3 -m py_compile` passes and the per-model server helper transcribes one corpus sample with the installed model (a full `just bench` downloads every model in the matrix, so it is not run here)
- [x] 7.5 Document `just start-services` / `just stop-services`, running the service by hand, the new `[transcription]` section and the migration from `[whisper]` in `README.md`; change CLAUDE.md "Zero network I/O" to "No internet; loopback connections only"; verify the README commands run as written

## 8. Verification

- [x] 8.1 Run `just check`; verify it passes
- [x] 8.2 Run `just e2e`; verify mean raw WER ≤ 6.5 %
- [ ] 8.3 User runs the manual checklist in `README.md` § Release checklist, plus: stop the speech recognition service and start the app → it exits naming the address; stop the service while the app runs and record → nothing is pasted and the next recording works after restarting the service
