# Design

## Context

- `LlamaCppTextRefiner` (`app/src/adapters/secondary/textrefinement/llamacpp/`) implements the `TextRefiner` port in-process via `llama-cpp-2`. It formats a Qwen ChatML prompt by hand (system prompt + transcript in `<transcript>` tags), budgets tokens as `min(ceil(1.5 × transcript tokens), max_tokens, context − prompt)`, samples until EOS, and rejects output shorter than half the input.
- `Pipeline` falls back to the raw transcript when `refine` returns `Err` (landed in #2). `PassthroughTextRefiner` serves `--no-llm`.
- `main.rs` builds the refiner in file mode (E2E) and push-to-talk mode; a load failure exits with code 2.
- Verified on this machine: `llama-server` (Homebrew llama.cpp) started with `-m Qwen2.5-1.5B-Instruct-Q4_K_M.gguf` answers `POST /v1/chat/completions` and lists the model at `GET /v1/models`.

## Goals / Non-Goals

**Goals:**
- `llama-server`, LM Studio, Ollama and MLX servers work by changing only `base_url` and `model`.
- Refinement quality on the corpus stays within 1 pp of today.

**Non-Goals:**
- Server supervision; streaming; tool calls; prompt tuning.

## Decisions

### D1. API: OpenAI Chat Completions
`POST {base_url}/chat/completions` with `model`, `messages` (`system` = configured prompt, `user` = `<transcript>\n…\n</transcript>`), `temperature`, `max_tokens`, `stream: false`. The server applies the model's own chat template, so the hard-coded ChatML disappears and any instruction model works.
*Alternatives:* Anthropic Messages API (fewer local servers implement it, and the app never needs Anthropic-specific features); `llama-server`'s native `/completion` (one server only, and it would keep the template in the app).

### D2. Adapter `textrefinement/openai/` with `ureq`
`OpenAiTextRefiner` sends one blocking JSON request per transcript with a per-request timeout and an optional `Authorization: Bearer` header. Response: `choices[0].message.content`. The tech segment `openai` names the wire protocol, shared with the transcription adapter if that change lands.
*Alternative:* the `async-openai` crate (async runtime and a large API surface for one call).

### D3. Output limit and truncation
`max_tokens` = `min(config max_tokens, max(64, ceil(transcript chars / 2)))`. Two characters per token is a conservative bound for Russian and English with Qwen/Llama tokenizers, and it reproduces today's "about 1.5 × transcript" budget without a tokenizer. `finish_reason == "length"` → `Err` (cut off). The existing "output shorter than half the input" check stays. Both make the pipeline paste the raw transcript.
*Alternative:* call the server's `/tokenize` (not part of the OpenAI API).

### D4. Strip reasoning
Remove a leading `<think>…</think>` block and trim before building `CleanedText`, so reasoning models (Qwen3, DeepSeek-R1 distills) do not leak their reasoning.

### D5. Config `[refinement]`
| Key | Default | Note |
|---|---|---|
| `base_url` | `http://127.0.0.1:8080/v1` | `llama-server` default port; must be loopback |
| `model` | `local` | `llama-server` ignores it; LM Studio and Ollama need the real name |
| `api_key` | `""` | sent only when not empty (`llama-server --api-key`) |
| `system_prompt` | today's prompt | unchanged |
| `temperature` | `0.0` | |
| `max_tokens` | `512` | upper bound for D3 |
| `timeout_secs` | `30` | per request |

Removed: `model_path`, `context_size` → service launch flags (`-m`, `-c`). `config::load` reports them with their replacement before strict parsing. The loopback rule is the same as in the transcription change (`localhost`, `127.0.0.0/8`, `::1`), checked at load; if that change lands first, both sections share one validator.

### D6. Startup check
Unless `--no-llm`, `OpenAiTextRefiner::connect(cfg)` calls `GET {base_url}/models` with a 2 s timeout. Unreachable → exit 2: `text refinement service is not reachable at <url> — start it with 'just llm-server'`. If the list holds more than one model and none matches `model` → exit 2 listing the offered ids. A single-model server (`llama-server`) is accepted whatever `model` says.

### D7. The user runs the service
README documents `llama-server`, LM Studio and Ollama setups; `just llm-server` runs `llama-server -m <model> --host 127.0.0.1 --port 8080 -c 4096`. `just e2e` starts it in the background, waits for `/v1/models`, runs the tests and stops it.

## Risks / Trade-offs

- [Server-side chat template or sampling differs from today's hand-rolled loop] → temperature 0 and the same system prompt; the acceptance bar requires WER with LLM ≤ 7.2 %.
- [Per-request latency over HTTP] → loopback and one short JSON body; the `refine=…ms` log line keeps reporting it.
- [A wrong model name on multi-model servers] → D6 fails at startup with the offered list.
- [Users forget to start the service] → D6 fails fast with the exact command; `--no-llm` remains an escape hatch.
- [Breaking config for existing users] → D5 migration error; README shows the before/after.

## Migration Plan

1. Land the change with the migration error and README instructions.
2. Users start `llama-server` (or `just llm-server`, LM Studio, Ollama) with the model they already have and rename `[llm]` to `[refinement]`.
3. Rollback: revert the PR.
