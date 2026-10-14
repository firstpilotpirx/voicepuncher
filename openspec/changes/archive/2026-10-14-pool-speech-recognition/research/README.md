# Several whisper-server copies on one Mac

## whisper-server serves one request at a time

whisper.cpp's `examples/server/server.cpp` creates one `whisper_context` and holds
`std::lock_guard<std::mutex> lock(whisper_mutex)` for the whole inference handler. It has no
option for parallel requests: `-t` is the CPU threads of one request, `-p` splits one long
audio into parts (quality drops at the splits). Measured: 16 short requests take 3.7 s one
after another and 3.6 s all at once on one server.

## Copies against speed (2026-10-14, M1 Max, ggml-small, whisper.cpp 1.9.5)

10 requests sent at once, spread round robin over N copies; the 10 CPU threads split between
the copies (`-t 10/N`, at least 1). Same flags as `start-model-servers`.

| Copies | Threads each | 10 × 3 s clip | 10 × 35 s clip |
|---|---|---|---|
| 1 | 10 | 2.26 s | 21.01 s |
| 2 | 5 | 1.91 s | 9.54 s |
| 3 | 3 | 1.87 s | 9.71 s |
| 4 | 2 | 1.86 s | 10.04 s |
| 5 | 2 | 2.05 s | 12.30 s |
| 6 | 1 | 2.00 s | 19.48 s |
| 7 | 1 | 2.59 s | 29.16 s |
| 8 | 1 | 4.11 s | 48.38 s |
| 9 | 1 | 4.36 s | 63.03 s |
| 10 | 1 | 7.68 s | 89.66 s |

Two to four copies are twice as fast on long speech: one request leaves the GPU idle while it
runs voice activity detection, the mel spectrogram and the token-by-token decoder; a second
copy fills those gaps. From five copies on they compete for the one GPU and the CPU threads.

## Tools that run copies behind one address (web research, 2026-10-14)

| Tool | Engine on macOS | N copies on one Mac |
|---|---|---|
| GPUStack | faster-whisper (vox-box) | no — no macOS workers |
| Xinference | transformers / MLX | `--replica N`, not whisper.cpp |
| LocalAI | whisper.cpp | only in distributed mode (PostgreSQL + NATS) |
| llama-swap | any command | one entry per copy, spillover fills in order, over the limit 429 instead of a queue |
| process-compose | any command | `replicas: N`, but not in Homebrew core (third-party tap) |
| HAProxy | — (proxy) | `leastconn`, `maxconn 1` per server queues requests, `set-priority-class` orders the queue; Homebrew core |

Chosen: the supervisor `start-model-servers` already runs (keep-services-running) starts the
copies, HAProxy is the one address with the queue.

## Checks on a fresh pool of three copies (2026-10-14)

Services on spare ports (9080–9082), started fresh by the checks; one copy is the timing of
`speed-up-checks` on the same machine.

| Stage | One whisper-server | Three copies behind HAProxy |
|---|---|---|
| `just check`: e2e-smoke | 202–225 s | 202–219 s |
| `just check`: e2e-quick | 49 s | 39 s |
| `just full-check`: `chunked_transcription_it` (fails on the known seam bug either way) | 77–78 s | 71 s |
| `just e2e-bench` | 140–145 s | 119–120 s (three runs in a row, all passing) |
| `just check` total | ≈ 6 min | 6.1 min (368 s, cold release build included) |

The E2E scenarios that wait for whisper in real time (`@serial`) and run one at a time gain
little; the stages that send many requests at once (WER sample, recorded forms, ordinary
speech) gain the most.
