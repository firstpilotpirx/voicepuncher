# Design

## Context

A freshly started `whisper-server` copy opens its port right after loading its model, but macOS builds its Metal pipeline states on the first decode: the `MTLCompilerService` processes at 264–310 % CPU beside the 2026-10-22 failure were that build. The service start of the scripts (`start_service` in the `justfile`) counts a copy ready when its port opens; the app's supervision (`watch_start` in `app/src/core/application/local_services/supervision.rs`) when `GET /health` answers. Both hand a cold copy to its first client, and in `e2e-cues` that client is the first timed confirmation with a 1 s budget.

## Measurement (2026-10-23, M1 Max, ggml-small, a 0.86 s utterance, the machine busy with other sessions' tests)

Three fresh copies started one after another, each sent the same utterance three times:

| copy | port open after | 1st transcription | 2nd | 3rd |
|---|---|---|---|---|
| A | 1.06 s | 0.44 s | 0.60 s | 0.43 s |
| B | 1.62 s | 3.98 s | 0.47 s | 0.36 s |
| C | 1.49 s | 0.64 s | 4.40 s* | 2.86 s* |

\* beside another checkout's test binary at 105 % CPU. The second start (B) paid the shader build again although A had just run the same binary and model: macOS's shader cache is not reliable between copy starts. After one transcription a copy is at its steady state (0.36–0.6 s), so the warm-up is exactly one transcription per copy; a second would buy nothing.

## Decisions

### 1. One warm-up transcription per copy, in the service start, scripts and app alike

A copy counts ready only after it transcribed `app/assets/warm-up.wav` ("hello there", 16 kHz mono, 0.86 s, 31 KB, synthesized once with `say` and committed): the same utterance for the scripts (posted with `curl` from the checkout) and the app (`include_bytes!`, so the bundle carries it). A real utterance, not silence: `--vad` would skip silence and the decoder would stay cold. Rejected: a warm-up in the `@cues` `Background` only (the app's first dictation would stay cold, and `check`'s other stages too); a readiness bound on the confirmation time (would hide the slowness the bound guards).

### 2. Scripts: the readiness check of a whisper copy is the warm-up

`start_whisper` passes `whisper_ready <port>` as the readiness check of each copy: the port answers and a `POST /v1/audio/transcriptions` of the warm-up returns 200 within 30 s; the ready line adds `first transcription N ms`. The 60 s start limit of `start_service` stays; a copy that never transcribes fails its start as before. The supervisor's restart after an exit keeps its `GET /health` watch: a copy the scripts restart is warmed by its first real request (the scripts restart no copy under `e2e-cues`: the stage starts fresh services).

### 3. App: a `warm_up` on the probe port, once per start

`ServiceProbe` gets `warm_up(kind, port) -> bool`: for a speech recognition kind `HttpProbe` posts the embedded utterance with a 30 s answer limit; for the other kinds it is true at once. `watch_start` goes `Ready` only when the probe answers ready and the warm-up returned true; a false keeps `Starting`, so the next tick asks again until `READINESS_LIMIT` fails the start and restarts the copy, as a copy that never answered. Every spawn, the restarts included, passes `watch_start`, so a restarted copy is warmed too; `watch_owned` and the adopted checks keep calling `answer` only. The ready log line adds the first transcription's time. Rejected: a warm transcription inside `answer` (it runs every 5 s on every copy and would send speech beside the user's dictation).

### 4. A missing cue names the confirmations cut short

When a timed start gives no cue, `check_within` reads the command spotting service's log of the stage (`~/.voicepuncher/logs/voicepuncher-command-spotting-<port>.log`, the port from the service's address) and appends the `confirmation cut short after Nms on copy <url>` lines written since the start's words began, as "copy <port>: cut short after N ms", then the machine load report of the late-cue case. The parser lives in `e2e-test/src/stalled_copies/` (`e2e-test/src/` is at its directory budget, as `machine_load` found), unit-tested on sample log lines.

### 5. Acceptance: the 10-run series

`VOICEPUNCHER_REUSE_STAGES=0 just e2e-cues` ten times in a row from freshly started services, in the background; the runs' cue times and the first-transcription times of the copies go into § Series below. No bound or budget moves.

## Dependencies

None: no running change reworks the service start, the supervision or the cue steps.

## Test levels

| Spec scenario | Level |
|---|---|
| The start cue is timed against warm copies | E2E (`e2e-cues`: the stage's services are started by the scripts, decision 2; the `@cues` scenarios run against them) |
| A missing start cue names the confirmations cut short | unit (`stalled_copies` parser) + the `check_within` report |
| The service scripts report a copy ready after its first transcription | E2E `local_services.feature` |
| The app counts a started copy ready after its first transcription | unit (`supervision_tests.rs`) + adapter integration (`HttpProbe::warm_up` against a stub server) |
| A copy that cannot transcribe in time is not ready | unit (`supervision_tests.rs`) |
| The check of a ready copy sends it no speech | unit (`supervision_tests.rs`) |
| Start start followed by the dictation at once gives its cue at once | E2E `voice_control.feature`, unchanged |
| Start followed by the dictation at once gives its cue at once | E2E `voice_control.feature`, unchanged |
| Start said on its own gives its cue at once | E2E `voice_control.feature`, unchanged |
| Restart restart followed by the new dictation at once gives its cue at once | E2E `voice_control.feature`, unchanged |
| Restart said on its own gives its cue at once | E2E `voice_control.feature`, unchanged |
| The user's recorded starts give their cue at once | E2E `voice_control.feature`, unchanged |
| A faster start keeps the dictation where the command's words end | E2E `voice_control.feature`, unchanged |
| A faster start adds no false start | E2E `voice_control.feature`, unchanged |
| Start please followed by the dictation at once gives its cue at once | E2E `voice_control.feature`, unchanged |
| Start please said on its own gives its cue at once | E2E `voice_control.feature`, unchanged |
| A brief dip inside the command words starts nothing early | E2E `voice_control.feature`, unchanged |
| The start cue is timed once the other checks' builds have ended | E2E `voice_control.feature`, unchanged |
| A late start cue names what ran beside it | E2E `voice_control.feature`, unchanged |

## Series

Filled by task 4.1.
