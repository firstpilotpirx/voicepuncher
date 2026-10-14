## Context

`just start-model-servers` launches `whisper-server`, `llama-server` and `voicepuncher-command-spotting` with `nohup … &`, writes each PID to `~/.voicepuncher/run/<binary>-<port>.pid` and waits until the service answers. After that nothing watches the process. `just run` stops and starts the services once, then runs the app in the foreground.

On 2026-10-14 `whisper-server` (whisper.cpp 1.9.5) died with a segmentation fault on the first command confirmation of an utterance without speech, seconds after every start. The command spotting service sends its confirmations with `response_format=verbose_json`; in that format the server computes language probabilities with `whisper_lang_auto_detect`, which returns `-2` when voice activity detection left no audio; `whisper_lang_str_full(-2)` returns a null pointer and building the JSON answer from it crashes. The server flag `-nlp` (`--no-language-probabilities`) skips that branch; nothing in voicepuncher reads the probabilities.

## Goals / Non-Goals

**Goals:**
- Audio without speech never crashes the speech recognition service.
- A service started by the service scripts is started again when its process exits, until the user stops it.
- A service that cannot start is reported once, with its log, and leaves no process.
- `start-model-servers` starts the named services only, as `stop-model-servers` stops the named ones.

**Non-Goals:**
- Telling the app that the speech recognition service is unreachable while confirming commands (separate change).
- launchd agents, restarts across logins or reboots.
- Patching whisper.cpp.

## Decisions

1. **`-nlp` on `whisper-server`.** The fix at the source of the crash. Alternative: ask the command spotting service for `json` instead of `verbose_json`; rejected, it needs the word timings that only `verbose_json` carries.
2. **A supervisor shell loop per service, inside `start-model-servers`.** `start_service` launches `nohup bash -c "<supervise function>" …` instead of the service itself; the PID file holds the supervisor's PID. The loop runs the service in the background, `wait`s for it, appends `… <binary> exited with code N — restarting in S s` to the service's log, sleeps and starts it again. The back-off starts at 1 s and doubles up to 30 s while the service keeps exiting within 30 s of its start; a run longer than that resets it.
   - Alternatives: a watchdog in `just run` while the app runs (rejected: services started by `start-model-servers` alone, as the AGENTS.md manual run and the E2E recipes do, would stay unwatched); launchd agents (rejected: out of scope, a per-user system install); a separate script file (rejected: the justfile is the only place for task logic, and the loop is a dozen lines).
3. **Stopping.** The supervisor traps `TERM`/`INT`: it kills its current service process, waits for it and exits without a restart. `stop-model-servers` sends `TERM` to the PID in the PID file as before; it recognizes the supervisor (and a service started by an older justfile without one) by the binary name in the process's command line (`ps -o command=`) instead of its executable name. Its `kill -9` fallback first kills the supervisor's children.
4. **Start failure.** `start_service` watches the log for the supervisor's exit note while it waits for readiness. A service that exits before it was ever ready is a start failure: the supervisor is stopped (so it does not retry), the last log lines are printed with the service and log path, and the PID file is removed.
5. **Logs.** The first start truncates the log as today; the supervisor appends every later run, so the crash output stays above the restart note.
6. **Service selection.** `start-model-servers *services` takes the same names as `stop-model-servers` (`whisper-server`, `llama-server`, `voicepuncher-command-spotting`), all three when none is named; an unknown name fails with the list of names.

## Test levels

| Spec scenario | Level | Where |
|---|---|---|
| speech-recognition / Command confirmation of an utterance without speech | E2E `.feature` | `e2e-test/tests/features/speech_recognition.feature`, `@smoke`; it starts its own service with the service scripts on a free port and sends the command spotting service's own confirmation request (`voicepuncher_command_spotting::confirm::UtteranceTranscriber`) |
| local-services / The speech recognition service dies while voice control runs | E2E `.feature` | `e2e-test/tests/features/local_services.feature`, `@smoke @serial`; kills the service process with `SIGKILL` |
| local-services / The user stops the speech recognition service | E2E `.feature` | same file |
| local-services / The speech recognition model is not a model | E2E `.feature` | same file |
| local-services / Only the speech recognition service is started | E2E `.feature` | same file |

Every scenario runs the real `just` recipes on free ports of its own (`VOICEPUNCHER_STT_PORT`, `VOICEPUNCHER_LLM_PORT`, `VOICEPUNCHER_KWS_PORT`), so it never touches the shared services on 8080–8082, and stops what it started when it ends, also on failure. The `@serial` rule keeps the extra speech recognition service from loading beside the other scenarios.

## Risks / Trade-offs

- [A service that crashes on every request restarts for ever] → the back-off caps the restarts at one per 30 s and each is in the log; better than a dead service.
- [The supervisor outlives a `kill -9` of itself and leaves the service orphaned] → the service is then a plain unsupervised process on its port; `start-model-servers` skips a port that answers, and `stop-model-servers` cannot find it. Same as before this change.
- [Services started by an older justfile have no supervisor] → `stop-model-servers` still stops them; `just run` restarts them under a supervisor.
