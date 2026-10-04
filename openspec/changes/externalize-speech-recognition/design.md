# Design

## Context

- `WhisperCppTranscriber` (`app/src/adapters/secondary/transcription/whispercpp/`) implements the `Transcriber` port in-process via `whisper-rs`. It also owns the hallucination filter (`is_whisper_hallucination`, `has_repetition_loop`).
- `Pipeline` already treats the port as fallible: an `Err` drops the cycle (`transcription failed, dropping cycle`), and `Ok(None)` means "nothing to paste". Live chunking moves the transcriber into a worker thread, so it must stay `Send`.
- `main.rs` builds the transcriber twice: once for file mode (`--input-file`, used by E2E tests) and once for push-to-talk mode. Both exit with code 2 when the model fails to load.
- `ureq` 2 (blocking HTTP) and `hound` (WAV) are already dependencies of `app`; `serde_json` is not (only `e2e-test` has it).
- Verified on this machine: `whisper-server` 1.9.4 from Homebrew started with `--inference-path /v1/audio/transcriptions` accepts an OpenAI multipart request (`file`, `model`, `language`, `response_format=json`) and answers `{"text": "..."}`. It joins segments with `\n`, sometimes inside a word (`наход\nившаяся`).

## Goals / Non-Goals

**Goals:**
- Any OpenAI-compatible transcription server on loopback works without app changes.
- Recognition quality on the corpus stays within 1 pp of today.

**Non-Goals:**
- Server supervision (spawn, restart, health dashboard).
- Async I/O or streaming; one blocking request per chunk is enough at current chunk sizes.

## Decisions

### D1. API: OpenAI Audio Transcriptions
`POST {base_url}/audio/transcriptions`, multipart: `file` (WAV), `model`, `language` (omitted for `auto`), `prompt`, `temperature`, `response_format=json`. It is supported by whisper.cpp `whisper-server` (via `--inference-path`), Speaches (faster-whisper) and LocalAI.
*Alternatives:* whisper.cpp's native `/inference` (one server only); the Wyoming protocol (Home Assistant ecosystem, needs a TCP event protocol client).

### D2. Adapter `transcription/openai/` with `ureq`
`OpenAiTranscriber` encodes `AudioBuffer16k` as an in-memory 16 kHz mono 16-bit WAV with `hound`, builds the multipart body by hand (ureq 2 has no multipart builder; the body is three text fields plus one file), and sets a per-request timeout. The tech segment `openai` names the wire protocol, not the vendor.
*Alternatives:* `reqwest` (pulls in tokio for a blocking pipeline); upgrading to ureq 3 for multipart (unrelated churn).

### D3. Transcript normalization
Remove every `\n` without inserting a space, then trim. whisper-server starts a segment with a space at word boundaries, so this restores both `обозначения периодов` and `находившаяся`. The E2E corpus run guards against regressions.
*Alternative:* replace `\n` with a space (turns `наход\nившаяся` into two non-words).

### D4. Hallucination filter moves to `core/application/`
The filter is application policy about Whisper-family output, not transport. `transcript_filter.rs` in `core/application/` holds the pure functions and their unit tests; the adapter calls them and returns `Ok(None)` for a hallucination, as today.
*Alternative:* keep it inside the adapter (couples the policy to one transport).

### D5. Config `[transcription]`
| Key | Default | Note |
|---|---|---|
| `base_url` | `http://127.0.0.1:8081/v1` | must be loopback |
| `model` | `whisper-1` | sent as-is; whisper-server ignores it |
| `language` | `auto` | |
| `prompt` | `""` | was `initial_prompt` |
| `temperature` | `0.0` | |
| `timeout_secs` | `60` | per request |
| `chunk_*`, `vad_*` | unchanged | live chunking stays in the app |

The removed keys are listed in one table; `config::load` checks for them before strict parsing and returns an error naming the replacement ("set it when starting the service" or the new key). Port 8081 avoids the 8080 default that `llama-server` uses.

### D6. Loopback check at config load
`base_url` host must be `localhost`, an IPv4 address in `127.0.0.0/8` or `::1`; anything else is a config error. Done at load time so both modes and the setup wizard share it.

### D7. Reachability check at startup
`OpenAiTranscriber::connect(cfg)` opens a TCP connection to the host and port with a 2 s timeout (whisper-server has no `/v1/models`, so a TCP probe is the portable check). On failure `main.rs` exits with code 2: `speech recognition service is not reachable at <url> — start it with 'just stt-server'`.

### D8. The user runs the service
README documents the launch command; `just stt-server` runs:
`whisper-server -m <model> --host 127.0.0.1 --port 8081 --inference-path /v1/audio/transcriptions -l auto -nt`.
`just e2e` starts it in the background, waits for the port, runs the tests and stops it.
*Alternative:* the app spawns the server as a child process (lifecycle, ports and logs become the app's job again).

## Risks / Trade-offs

- [Accuracy shifts because decoder settings now come from server defaults] → `just stt-server` passes today's values (greedy decoding, no-speech threshold 0.6); the acceptance bar requires WER ≤ 6.5 %.
- [Added latency from HTTP and WAV encoding] → both are loopback and in memory (a few ms per chunk); the `stop→paste` log line keeps reporting totals so regressions show up.
- [whisper-server handles one request at a time] → live chunks are already sent sequentially from a single worker.
- [Users forget to start the service] → D7 fails fast with the exact command.
- [Breaking config for existing users] → D5 migration error names every replacement; the README changelog note shows the before/after.

## Migration Plan

1. Land the change with the migration error and README instructions.
2. Users start `whisper-server` (or `just stt-server`) with the model they already downloaded and rename `[whisper]` to `[transcription]`.
3. Rollback: revert the PR; old configs work again because the removed keys are the old ones.
