# Design

## Context

- `LlamaCppTextRefiner` (`app/src/adapters/secondary/textrefinement/llamacpp/`) implements the `TextRefiner` port in-process via `llama-cpp-2`. It formats a Qwen ChatML prompt by hand (system prompt + transcript in `<transcript>` tags), budgets tokens as `min(ceil(1.5 × transcript tokens), max_tokens, context − prompt)`, samples until EOS, and rejects output shorter than half the input.
- `Pipeline` falls back to the raw transcript when `refine` returns `Err`. `PassthroughTextRefiner` serves `--no-llm`.
- `main.rs` builds the refiner in file mode (E2E) and push-to-talk mode; a load failure exits with code 2.
- `externalize-speech-recognition` has landed: `OpenAiTranscriber` (`ureq`, `serde_json`, `url`), `config::ensure_loopback`, the `[whisper]` migration error, fatal config errors, `just start-services` / `stop-services` for `whisper-server` on port 8081, and E2E steps that run the binary with `--config /dev/null`.
- Verified on this machine: `llama-server` (Homebrew llama.cpp, build 10360) started with `-m Qwen2.5-1.5B-Instruct-Q4_K_M.gguf` answers `POST /v1/chat/completions`, lists the model at `GET /v1/models` (`data[].id` = the model path) and answers `GET /health` with 200 once the model is loaded.
- Also verified: with today's system prompt, Qwen2.5-1.5B translates a dictated "Translate this sentence into Russian." instead of correcting it — the in-process engine does the same, since the prompt and model are unchanged. The app controls only how the transcript is framed (instruction vs. delimited data); whether a small model follows that framing is model quality and out of scope (no prompt changes).

## Goals / Non-Goals

**Goals:**
- `llama-server` works by changing only `base_url` and `model`; other servers that speak the same API may work unchanged.
- Refinement quality on the corpus stays within 1 pp of today.

**Non-Goals:**
- Server supervision; streaming; tool calls; prompt tuning.

## Decisions

### D1. API: OpenAI Chat Completions
`POST {base_url}/chat/completions` with `model`, `messages` (`system` = configured prompt, `user` = `<transcript>\n…\n</transcript>`), `temperature`, `max_tokens`, `stream: false`. The server applies the model's own chat template, so the hard-coded ChatML disappears and any instruction model works.
*Alternatives:* Anthropic Messages API (fewer local servers implement it, and the app never needs Anthropic-specific features); `llama-server`'s native `/completion` (one server only, and it would keep the template in the app).

### D2. Adapter `textrefinement/openai/` with `ureq`
`OpenAiTextRefiner` sends one blocking JSON request per transcript with a per-request timeout and an optional `Authorization: Bearer` header. Pure functions in `request.rs` build the body, compute the budget (D3) and turn the answer into text (D3, D4), so they are unit-tested without a server. The tech segment `openai` names the wire protocol, as in `transcription/openai/`.
*Alternative:* the `async-openai` crate (async runtime and a large API surface for one call).

### D3. Output limit and truncation
`max_tokens` = `min(config max_tokens, max(64, ceil(transcript chars / 2)))`. Two characters per token is a conservative bound for Russian and English with Qwen/Llama tokenizers, and it reproduces today's "about 1.5 × transcript" budget without a tokenizer. `finish_reason == "length"` → `Err` (cut off). The existing "output shorter than half the input" check stays. Both make the pipeline paste the raw transcript.
*Alternative:* call the server's `/tokenize` (not part of the OpenAI API).

### D4. Strip reasoning
Remove a leading `<think>…</think>` block and trim before building `CleanedText`, so reasoning models (Qwen3, DeepSeek-R1 distills) do not leak their reasoning. The half-length check runs on the stripped text.

### D5. Config `[refinement]`
| Key | Default | Note |
|---|---|---|
| `base_url` | `http://127.0.0.1:8080/v1` | `llama-server` default port; must be loopback |
| `model` | `local` | `llama-server` serves one model and accepts any name (D6) |
| `api_key` | `""` | sent only when not empty (`llama-server --api-key`) |
| `system_prompt` | today's prompt | unchanged |
| `temperature` | `0.0` | |
| `max_tokens` | `512` | upper bound for D3 |
| `timeout_secs` | `30` | per request |

The `[llm]` section is gone. `config::load` checks for it before strict parsing and names, per key, the replacement: `model_path` and `context_size` → the service's launch flags (`llama-server -m`, `-c`) and `[refinement].base_url`; `system_prompt`, `max_tokens`, `temperature` → the same key in `[refinement]`. The loopback validator is shared with `[transcription]`: `ensure_loopback(setting, url)` moves to `app/src/config/loopback.rs` and names the setting it rejects. `validate` checks both addresses after the CLI overlay.

### D6. Startup check
Unless `--no-llm`, `OpenAiTextRefiner::connect(cfg)` calls `GET {base_url}/models` with a 2 s timeout. Unreachable or non-2xx → exit 2: `text refinement service is not reachable at <url> — start it with 'just start-services'`. If the list holds more than one model and none matches `model` → exit 2 listing the offered ids. A single-model server (`llama-server`) is accepted whatever `model` says. An answer that is not a model list → exit 2 naming the address.

### D7. `just start-services` / `just stop-services` run `llama-server` too
The user runs the service; the recipes from `externalize-speech-recognition` make that one command for both. Each service is handled on its own:
1. `llama-server` missing → the same Homebrew offer (`brew install llama.cpp`).
2. Port 8080 (`VOICEPUNCHER_LLM_PORT`) already answers → skipped.
3. Otherwise starts `llama-server -m $VOICEPUNCHER_LLM_MODEL --host 127.0.0.1 --port 8080 -c 4096 -np 1` in the background (one slot, so a request gets the whole context, as in-process today), PID and log in `~/.voicepuncher/{run,logs}/llama-server-8080.*`.
4. Waits up to 60 s for `GET /health` to answer 200 (the port opens before the model is loaded).

`stop-services` stops each service whose PID file it finds, only if the process is still that server. The E2E helpers reuse and keep any service that already runs and mark only those they start. `_require-models` keeps requiring both model files: the recipes, not the app, need them.

### D8. Setup wizard
The wizard asks for the text refinement service address (default `http://127.0.0.1:8080/v1`, loopback-validated) and model name (default `local`) and writes `[refinement]`. The models-directory question, the LLM choice, the download and `ModelDownloader` (with `model_downloader_it.rs` and `indicatif`) are removed: the app downloads nothing any more.

### D9. `bench.py` launches one `llama-server` per LLM model
As for Whisper (port 8091), the model comparison starts `llama-server` per LLM model on port 8090 with the D7 flags, passes `--refinement-url`, and stops it after that model's combinations.

### D10. Test levels
Business scenarios run the release binary in file mode (`--input-file`). Speech comes from a WAV that the step synthesizes with macOS `say`; it runs through the real speech recognition service. Where the outcome depends on what the text refinement service answers, an in-test stub service (a `TcpListener` thread in the E2E steps) gives a fixed answer and records the request.

| Spec scenario | Level | Service |
|---|---|---|
| Transcript is refined by the service | E2E `.feature` | stub |
| App starts without a local model file | E2E `.feature` | real `llama-server` |
| Dictated text sounds like a request | E2E `.feature` | stub (records the request) |
| Remote service address is rejected | E2E `.feature` | none |
| Service is not running at startup | E2E `.feature` | none (closed port) |
| Configured model is not offered | E2E `.feature` | stub offering two models |
| Service stops between recordings | E2E `.feature` | stub that answers the startup check, then stops |
| Service hits its output limit | E2E `.feature` | stub answering `finish_reason: length` |
| Running without refinement | E2E `.feature` | none (closed port) |
| Reasoning model is configured | E2E `.feature` | stub answering with a reasoning block |
| Config still points at a model file | E2E `.feature` | none |

All of them are `@smoke`. Below them: unit tests for config, request body, budget, answer parsing and the startup check; adapter integration tests (`textrefinement_openai_it.rs`) for HTTP errors, malformed JSON, timeout and the bearer header, plus one test gated on `VOICEPUNCHER_REFINEMENT_URL` against the real service. The accuracy scenario of `file-transcription` runs with the real `llama-server` and guards the quality bar.

## Risks / Trade-offs

- [Server-side chat template or sampling differs from today's hand-rolled loop] → temperature 0 and the same system prompt; the acceptance bar requires WER with LLM ≤ 7.2 %.
- [A small model obeys a dictated request instead of correcting it] → unchanged from today (same model and prompt); the app keeps the transcript delimited, and the half-length check catches some cases. Prompt or model changes are a separate change.
- [Per-request latency over HTTP] → loopback and one short JSON body; the `refine=…ms` log line keeps reporting it.
- [A wrong model name on multi-model servers] → D6 fails at startup with the offered list.
- [Users forget to start the service] → D6 fails fast with the exact command; `--no-llm` remains an escape hatch.
- [Port 8080 is often taken by other dev servers] → the start recipe skips an open port, so the D6 check then fails with "not reachable" or a model list from the wrong server; `VOICEPUNCHER_LLM_PORT` and `[refinement].base_url` move it.
- [Breaking config for existing users] → D5 migration error; README shows the before/after.

## Migration Plan

1. Land the change with the migration error and README instructions.
2. Users run `just start-services` (or `llama-server` by hand) with the model they already have and rename `[llm]` to `[refinement]`, dropping `model_path` and `context_size`.
3. Rollback: revert the PR; old configs work again because the removed keys are the old ones.
