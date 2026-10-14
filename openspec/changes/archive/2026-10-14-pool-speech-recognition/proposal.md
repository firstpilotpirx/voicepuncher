# Proposal

## Why

The speech recognition service (`whisper-server`) transcribes one request at a time: every request holds one global lock on its only model context. Everything waits in that one queue — dictation chunks, the command spotting service's confirmations of voice commands, and every E2E check. A short command confirmation that lands behind a long dictation chunk waits for it, and under load the confirmations reached the command spotting service's 10 s timeout (failed bench runs, missed commands in the hands-free sessions).

Measured on 2026-10-14 (M1 Max, `ggml-small`, 10 requests sent at once, `research/README.md`): one copy needs 21.0 s for 35 s clips and 2.3 s for 3 s clips; two to four copies on the same GPU need 9.5–10.0 s and 1.9 s; five and more get slower again (10 copies: 89.7 s and 7.7 s). No popular tool runs several whisper.cpp copies behind one address on a Mac (GPUStack has no macOS workers, Xinference uses another engine, LocalAI needs a distributed cluster; process-compose is not in Homebrew core).

## What Changes

- **Several copies behind one address.** `start-model-servers` starts the speech recognition service as several `whisper-server` copies on internal ports, each under the existing supervisor (restart after a crash, stop with the service), and HAProxy on the service's own port in front of them. The app, the command spotting service and the tests keep using one address.
- **Three copies by default**, set by one number (`VOICEPUNCHER_STT_INSTANCES`); the CPU threads are split between the copies. One copy runs `whisper-server` on the service's port as today, without HAProxy.
- **A request goes to a free copy, or waits.** Each copy takes one request at a time; HAProxy queues the others instead of sending them to a busy copy.
- **Short speech first.** A request with little audio — a command confirmation — goes ahead of the long ones waiting in the queue.
- **One command, as today.** `just run` and `just start-model-servers` start the copies and HAProxy with the other services; `just stop-model-servers` stops all of them.
- HAProxy (Homebrew `haproxy`) becomes a requirement, like `whisper-cpp` and `llama.cpp`.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `local-services`: the speech recognition service runs as several copies behind one address, keeps answering when a copy stops, transcribes short speech ahead of long speech, stops with all its copies, and leaves nothing behind when a copy cannot start; a stopped single service stays stopped even when a list of copies was left from an earlier start.

## Impact

- `justfile`: `start-model-servers` (copies, HAProxy configuration, service selection), `stop-model-servers` (copies and HAProxy), recipe comments.
- `e2e-test/`: new scenarios in `local_services.feature` with their steps; the existing scenarios start one copy.
- `README.md`, `AGENTS.md`: HAProxy requirement, the number of copies.
- No change to the app or the command spotting service code.

## Scope

- **Lifecycle stage:** PoC, stays PoC.
- **Acceptance bar:** the new `local_services` scenarios pass; `just check` and `just full-check` pass (bar the known seam failures of `fix-dropped-phrase-at-word-pauses`) on a fresh pool of three copies; `e2e-bench` passes three runs in a row; the E2E times are measured against `research/README.md` of `speed-up-checks`.
- **Out of scope:** a hung copy that keeps its port open but never answers (HAProxy only checks that the port answers); several copies of the text refinement service; launchd agents.

## Release

MINOR — the local services change how they run; target version **0.12.0**.
