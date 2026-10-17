# Design

## Context

See proposal.md (Why, the chosen lever). Today:

- One speech recognition service serves everything: three `whisper-server` copies (`-t 3` each on the 10-core M1 Max) behind HAProxy on 8081. The app sends dictation chunks and recording files there (`[transcription].base_url`); the command spotting service on 8082 sends every confirmation there (`--transcription-url {{stt_url}}`); HAProxy puts audio under 160 kB (a confirmation) ahead of long chunks.
- The justfile takes the largest `ggml-*.bin` (Silero excluded) in `~/.voicepuncher/` or `~/.voicepuncher/models/` as the Whisper model; `just models` fetches `ggml-small.bin` from `resolve/main`, without a checksum.
- whisper-server loads one model per process and serves one request at a time; every copy shares the one GPU.
- The services are shared by every checkout on the machine: an E2E recipe reuses whatever answers on a port and starts (and later stops) only what is missing. The `@local_services` and `@resilience` steps start services through the scripts on free ports of their own (`VOICEPUNCHER_STT_PORT`, `_LLM_PORT`, `_KWS_PORT`) and stop them with `just stop-model-servers` on those ports.

Measured on 2026-10-16 (M1 Max, whisper.cpp 1.9.5, three copies, app 0.14.2):

| | small (today) | large-v3-turbo q5_0 |
|---|---|---|
| `@seams` scenarios | 22 of 29 (7 of the 22 difficult-conditions cases over the bar; worst Ru short dropouts 19.0 %) | 29 of 29 (worst 8.6 %), seam sweep passes |
| corpus WER, fixed 100 | 0.116 (ru 0.145, en 0.088) | 0.077 (ru 0.092, en 0.063); 38 recordings better, 8 worse |
| one request, idle | 2.5 s clip 234–245 ms; 30 s chunk median 908 ms | 1015–1022 ms; median 1379 ms |
| memory per copy | 647–691 MB (3 copies ≈ 2.0 GB) | 739–748 MB (3 copies ≈ 2.2 GB) |
| voice control with its confirmations | `commandspotting_websocket_it` passes; confirmations median 181 ms, p90 309 ms | 11 of 13, twice; confirmations median 1.9–2.2 s, 9 given up at the 3 s deadline in one run |

Turbo for dictation with small kept for confirmations (the user's pick) passed `e2e-seams` 29/29, `commandspotting_websocket_it` 13/13 in 3 runs out of 3, `placed_commands_it` 7/7, `@smoke` 137/137 and `refine-eval` 9/9; the shared 3-copy small pool stood in for a service of its own. Beam 5 / best of 5 on small left 6 of 22 cases over the bar.

## Goals / Non-Goals

**Goals:**
- One start, copy, supervision, stop and failure path in the service scripts for both speech recognition services, so the existing local-services promises hold for each.
- Each service names its model; the dictation model is downloaded pinned and checked.
- Every test that starts services on ports of its own keeps both speech recognition services off the shared ports.

**Non-Goals:**
- No change to the app's or the command spotting service's code: the app keeps sending to `[transcription].base_url`, the command spotting service to its `--transcription-url`; only the scripts change where each points.
- No decoding settings, recognition prompt (`restore-recognition-prompt`), hallucination filter or seam merge change.
- No check that a reused running service runs the model the scripts would start; no health check for whisper-server.
- No pinning of the other four downloads (the two Silero VAD files, Qwen, the sherpa-onnx keyword model); their lines stay as they are.
- No permanent test of confirmation latency under dictation load: apply measures it once (Test plan).

## Decisions

### D1. Two speech recognition services, one code path

The scripts start the speech recognition service for dictation (8081, what the app uses) and the speech recognition service for commands (D4), from one parametrized function. The command spotting service is the only client of the second; the app never talks to it.

Alternatives: (a) turbo for confirmations too — measured above: confirmations ≈ 2 s, commands lost, `commandspotting_websocket_it` 11/13; (b) one HAProxy address routing by audio length to a small and a turbo backend — every short dictation would land on small and lose the accuracy gain, and a confirmation cannot be told from a short dictation by size; (c) swapping the model per request (`/load`) — process-wide and a reload per request.

### D2. Dictation model: large-v3-turbo q5_0, downloaded pinned

- `just models` fetches `ggml-large-v3-turbo-q5_0.bin` into `~/.voicepuncher/models/` from `https://huggingface.co/ggerganov/whisper.cpp/resolve/5359861c739e955e79d9a303bcbc70fb988958b1/ggml-large-v3-turbo-q5_0.bin` (574,041,195 bytes), and moves the download into place only after `shasum -a 256` gives `394221709cd5ad1f40c46e6031ca61bce88931e6e088c188294c6d5a55ffa7e2`; a mismatch removes the download and fails naming both hashes. A file already present is skipped without hashing, as today (`just run` runs `models` every time).
- `ggml-small.bin` moves to the same revision with its sha256 `1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b` (487,601,967 bytes), since the commands service now names it; `fetch` takes the hash as a third argument, optional for the four lines this change leaves alone.
- Checked on 2026-10-16: a HEAD request on the pinned URL answers `x-repo-commit: 5359861c…` and `x-linked-etag` equal to the hash above (the revision dates from 2024-10-29); the spike's copy in `~/.voicepuncher/models/research/` and the local `ggml-small.bin` have exactly these hashes.
- Why q5_0: it is the measured one (bar met, WER 0.077, ≈ 0.75 GB per copy). The f16 `ggml-large-v3-turbo.bin` that the README's model table names today (≈ 1.6 GB) and q8_0 were not measured and take two to three times the memory per copy.

### D3. Each service names its model

| Variable | Default | Used by |
|---|---|---|
| `VOICEPUNCHER_WHISPER_MODEL` | `ggml-large-v3-turbo-q5_0.bin` in `~/.voicepuncher/` or `~/.voicepuncher/models/` | dictation |
| `VOICEPUNCHER_COMMAND_WHISPER_MODEL` | `ggml-small.bin` there | commands |
| `VOICEPUNCHER_VAD_MODEL` | newest `ggml-silero-*.bin` (unchanged) | both |

- The defaults look up these exact file names (`find ~/.voicepuncher -maxdepth 2 -name <file>`). "Largest file" goes: with both files in `models/` it picks turbo for both services, and any larger ggml file the user keeps there would become the dictation model, against the spec's default.
- The user names another model with the variable (environment or `.env`); naming one for dictation leaves the commands model alone. The existing name keeps its meaning for the app's speech, so a `.env` that sets it still works.
- A default that is not on disk fails the start (and `_require-models`) with "Run 'just models' or set it in .env" — no fallback to small, which would hide the accuracy loss. `_require-models` requires and prints all three model files (dictation, commands, text refinement).

### D4. The commands service: small, three copies behind HAProxy on 8085

- **Port** `VOICEPUNCHER_COMMAND_STT_PORT`, default 8085. Copies listen 10000 above their service's port (8081 → 18082–18084), so 8083's copies (18084–18086) would collide with the third dictation copy; 8085's (18086–18088) stay clear of up to four dictation copies.
- **Copies** `VOICEPUNCHER_COMMAND_STT_INSTANCES`, default 3, behind HAProxy with the dictation service's configuration (`leastconn`, `maxconn 1` per copy, `abortonclose`, retries, health checks); its short-audio priority is moot, as every confirmation is short. Three is the measured setup. `commandspotting_websocket_it` runs 13 tests at once on up to 15 connections, and one connection's confirmations run one after another, so bursts overlap (about 7 at once in the turbo run's log, by its 1-second stamps): one copy at ≈ 0.24 s each would answer the last of 7 after ≈ 1.7 s, past the ≈ 1.5 s after which the turbo run lost commands; three answer them in ≈ 0.6 s. Cost ≈ 0.7 GB per copy (2.1 GB); `VOICEPUNCHER_COMMAND_STT_INSTANCES=1` saves 1.4 GB, unmeasured.
- **Threads** cores ÷ copies per service, as for dictation: `-t 3` for each of the six copies on the 10-core M1 Max, as measured.
- **Flags** the dictation flags (`whisper_args` takes the model as an argument): greedy, `-mc 0`, VAD, `-nlp` — without language probabilities whisper.cpp 1.9.5 does not crash on a detailed-format confirmation of audio without speech.
- **No health URL**, as for the dictation copies: HAProxy routes around a copy that stops answering, and the command spotting service gives up a confirmation at its 3 s deadline.

### D5. The command spotting service confirms with the commands service

`start-model-servers` passes `--transcription-url http://127.0.0.1:<commands port>/v1`, and starts by default in the order `whisper-server`, `whisper-server-commands`, `llama-server`, `voicepuncher-command-spotting`, so the first confirmation finds its service ready. The app's default address stays 8081.

### D6. The service scripts

- **Names:** `whisper-server` stays the dictation service (the app's; README checklist and steps keep working); `whisper-server-commands` is the commands service. `start-model-servers` and `stop-model-servers` take it, start or stop all four when none is named, and list four in the unknown-name error.
- **Start:** `start_whisper` becomes one function of name, port, copies, model variable and label; `haproxy_config` takes the frontend port. PID, log, copy-list and HAProxy files stay named by port (`whisper-server-<port>.*`, `haproxy-<port>.*`), so the two services never share a file. A copy or HAProxy that cannot start stops only its own service. Output names the role ("speech recognition service for commands ready on port 8085 (3 copies)").
- **Supervision:** the existing `supervise` loop for every copy and HAProxy: restart 1 s after an exit, doubling up to 30 s; an exit before ready is a start failure with the last log lines.
- **Stop:** `stop_speech_recognition` takes the port: HAProxy and the copies from that port's list, or the one whisper-server.
- **E2E helpers:** `_start-model-servers-for-e2e` and `_stop-model-servers-for-e2e` list `whisper-server-commands:<port>`, so every E2E recipe and `_model-stages` reuse a running commands service or start it and stop it afterwards. `run` restarts all four.
- **Exports:** `_e2e-smoke` exports `VOICEPUNCHER_COMMAND_TRANSCRIPTION_URL` (the commands service); `placed_commands_it` reads it instead of `VOICEPUNCHER_TRANSCRIPTION_URL`, so its in-process engine confirms as the command spotting service does (7/7 on small).
- **`_e2e-resilience`:** unchanged — it starts no service; through `_require-models` it now needs both Whisper models.

### D7. Test steps: ports of the scenario's own, and which service each scenario uses

- **Ports:** `ScenarioServices` gets a free port for the commands service and passes it as `VOICEPUNCHER_COMMAND_STT_PORT` on every `just` call, also in scenarios that never start it: its `stop-model-servers` on drop then never stops the shared commands service (or the one on a worktree's own port). Drop also kills what is left on that port and removes its files. It takes an optional number of commands copies, and removes or sets the two model variables per scenario (removed: the scripts' own default; set: the user names a model).
- **Which service:** each scenario starts the service its requirement names — the dictation service where the app's requests are meant, the commands service wherever the command spotting service confirms (Test plan table). Where a pause must stall every confirmation, the commands service runs as one copy: behind HAProxy three copies would route around the paused one. The two voice-control outage scenarios outside the deltas ("Commands work again after the command spotting service restarts", "\"stop dictation\" ends a dictation after the service is back") share the Given that starts the services, so they start the commands service too; the other local-services scenarios keep the dictation service.
- **A home of their own for the model scenarios:** the Given "the models that the service scripts download are on disk" prepares a temporary home whose `.voicepuncher/models/` holds links to the downloaded Whisper models under their download names — each found as the run's model variable when that names a file of this name (the checked copy in `models/research/` until the merge), else in `~/.voicepuncher/` — beside a larger `ggml-*.bin` that is no model (a sparse file). The two speech-recognition scenarios start the two speech recognition services with that home (logs, PID files and the stop follow it), so the default is exercised before turbo sits in the shared home, and the old largest-file rule would take the larger file and fail: their Red, and a lasting guard of the default. The routing scenario starts all services in the real home (the command spotting service is built there) with the commands model variable removed, so small comes from the scripts' default.

### D8. `recognition_paths` stays as it is

The justfile is on it whole (models, service settings), so this branch's `full-check` runs the 100 recordings ("recognition paths changed: justfile"), and so does any later change of either default model. The other files this change touches — local-services and speech-recognition steps and features, `placed_commands_it` — are not run by the `@accuracy` scenarios. A model swapped outside the repository (environment, `.env`, a replaced file) still needs `VOICEPUNCHER_ACCURACY_SAMPLE=100`.

### D9. Terms and docs

- **Glossary, first:** "Speech recognition service" stays the general term (a loopback process speaking the OpenAI Audio Transcriptions API, run as `whisper-server` copies behind one address) and gains two terms, worded as in the specs: **Speech recognition service for dictation** (what the app sends dictations and recording files to; large-v3-turbo q5_0; 8081) and **Speech recognition service for commands** (what the command spotting service confirms utterances with; small; 8085; the app never talks to it). Synonyms to avoid: "turbo server", "command whisper", "confirmation server", "second whisper". "Command spotting service" and "Confirmation deadline" name the service for commands.
- **README:** overview; hardware (working set ≈ 3 GB → ≈ 6 GB); Local services (both services, ports, copies, model variables, logs, `whisper-server-commands`); Command spotting; the Models table (turbo q5_0 for dictation, small for commands, both from `just models`, pinned and checked); Testing (models found by name); `just run` (four services, and the way to pick up this change: it downloads turbo and restarts the services).
- **AGENTS.md:** "What this project is" (two speech recognition services), "Models and corpus" (by file name), "Local services" (ports 8081, 8080, 8082, 8085, own ports include the commands port). After measuring (Test plan), the check durations there, in the README Testing table, the justfile comments and the `openspec/config.yaml` context.

### D10. Version

MINOR, 0.14.2 → 0.15.0 in `app/Cargo.toml` (proposal § Release): the specs promise another dictation model and a second service. The app binary itself does not change.

## Test plan

**Red**, on the branch's own ports, before the justfile change:

- The bug's reproduction exists: the `@seams` outline "Long speech under difficult recording conditions", red on main since 0.13.1 (7 of 22 over the bar with small, measured 2026-10-16); it is committed already, so there is no separate red commit — the PR cites that run.
- The three new scenarios, written from the delta specs, fail because the behaviour is missing: the default one finds the dictation service started on the larger file (the old largest-file rule, D7); the named one finds no service for commands; the routing one finds the confirmation answered by the dictation service.
- The modified scenarios keep their meaning and only name the service they start; their steps move to it in the task group of the justfile change.

**Green:** D2–D7, with the branch's model stages on ports of their own and the dictation model named as the checked copy in `models/research/`; turbo reaches its default path only at merge time (Migration Plan). Acceptance: `just e2e-seams` 29/29 with the seam sweep; corpus WER on the fixed 100 ≤ 0.116 (measured 0.077); `just full-check` passes but for the external `voice_forms_it` failure (Risks).

Every scenario of the change's delta specs, with its level:

| Spec scenario (capability) | Level | Test |
|---|---|---|
| A command is confirmed by the speech recognition service for commands (local-services, added) | E2E `.feature` | `local_services.feature`, new Rule, `@smoke @serial`: all services started in the real home, one copy each; "press enter" streamed and reported; the commands copy's log shows `ggml-small.bin` and the request, the dictation log none |
| Requests that arrive at once are shared by the copies (local-services) | E2E `.feature` | existing, dictation; Given worded "for dictation" |
| A short utterance overtakes long speech that waits (local-services) | E2E `.feature` | existing, dictation; Given worded "for dictation" |
| Only the speech recognition service is started (local-services) | E2E `.feature` | existing; starts the dictation service only; Then also checks the commands port |
| The command spotting service hangs (local-services) | E2E `.feature` `@resilience` | existing, unchanged: command spotting only |
| A busy service is not restarted (local-services) | E2E `.feature` `@resilience` | existing; the 4-second stand-in listens on the commands port |
| One confirmation hangs (local-services) | integration, plain Rust | `command-spotting/tests/server_it.rs` `should_keep_answering_the_app_while_a_confirmation_hangs`, unchanged: a stand-in that never answers plays the service for commands |
| Commands are spotted after a hung confirmation (local-services) | integration, plain Rust | `command-spotting/tests/engine_it.rs` `should_spot_a_command_after_a_hung_confirmation`, unchanged |
| Checks of a long utterance are skipped while the service is slow (local-services) | integration, plain Rust | `engine_it.rs` `should_skip_the_checks_of_a_long_utterance_while_one_waits_for_a_slow_service`, unchanged |
| A command said alone is confirmed by the second attempt (local-services) | integration, plain Rust | `engine_it.rs` `should_confirm_a_command_said_alone_by_the_second_attempt`, unchanged |
| Russian speech that makes speech recognition repeat itself (local-services) | E2E `.feature` `@resilience` | existing; commands service + command spotting service, Given worded as the spec |
| A command right after such speech is carried out (local-services) | E2E `.feature` `@resilience` | existing; the same Given |
| Dictation is recognized with the large-v3-turbo model by default (speech-recognition, added) | E2E `.feature` | `speech_recognition.feature`, new Rule, `@smoke @serial`, home of its own (D7): the dictation service started without a model loads the linked `ggml-large-v3-turbo-q5_0.bin` (command line, `loading model from`) and transcribes a short utterance |
| Dictation is recognized with the model the user names (speech-recognition, added) | E2E `.feature` | same Rule and home: the dictation service named a link to a model under another name loads it and transcribes; the commands service loads `ggml-small.bin` |
| Command confirmation of an utterance without speech (speech-recognition) | E2E `.feature` `@smoke` | existing; the commands service as one copy, Given worded as the spec |
| A confirmation that takes longer than 10 seconds keeps listening on (voice-control) | E2E `.feature` `@resilience` | existing in `voice_control.feature`; the commands service as one copy + command spotting service; the step pauses that copy for 15 s; Given worded as the spec |

Outside the deltas: the `@seams` outline (the Red test, unchanged, E2E); the two other voice-control outage scenarios (E2E `@resilience`, D7); `placed_commands_it` (integration, plain Rust), which reads the commands service's URL.

No unit tests: no Rust production code changes, and the behaviour lives in the service scripts, whose lowest test level here is E2E through the real `just` recipes, as for every local-services scenario. `just models` is not tested automatically (it needs the network): apply checks the pin with a HEAD request (`x-repo-commit`, `x-linked-etag`) and the placed file with `shasum -a 256`.

**Measured once at apply** (in the PR body):

1. **Confirmations under dictation load.** All services from the branch's justfile on its own ports; keep the three dictation copies busy with long speech (concurrent requests of the long synthesized recordings, or `e2e-bench` against the dictation service) while `commandspotting_websocket_it` and `voice_samples_it` run against the branch's command spotting service; read every `answered in` and `given up after` line of its log in that window. Bar: none given up, p90 ≤ 1.0 s, max < 1.5 s, and 13 of 13 in `commandspotting_websocket_it` — idle small answers at median 181 ms, p90 309 ms, and the turbo run lost commands from ≈ 1.5 s. A miss sends the change back to design with the numbers (candidates: the commands service on the CPU, `-ng`; fewer dictation copies).
2. **Memory:** peak footprint of the six copies (expected ≈ 2.2 + 2.1 GB).
3. **Durations** of `just check` and `just full-check`, for D9.

## Risks / Trade-offs

- [Confirmations slow down while turbo copies keep the GPU busy — not measured; turbo's encoder runs a full 30 s window per request, ≈ 1 s even for a 2.5 s clip] → measurement 1 and its bar; a miss returns to design.
- [Memory: six copies ≈ 4.3 GB against ≈ 2.0 GB today, plus ≈ 1 GB for text refinement] → README states it; `VOICEPUNCHER_COMMAND_STT_INSTANCES=1` (−1.4 GB) or fewer dictation copies lower it; measurement 2.
- [The first `just models` downloads ≈ 574 MB more (≈ 2.1 GB in all); an existing setup downloads it once, on its next `just run`] → pinned revision and checksum, so a cut or altered download never lands; the recipe comment and README give the sizes.
- [Every dictation request ≈ 0.5–0.8 s slower: stop→paste of a short dictation +0.7–0.8 s, of a long one +0.5 s] → accepted with the option, for WER 0.116 → 0.077.
- [E2E recipes reuse what answers on 8081 and 8082, and the running shared services keep small, with confirmations through 8081, until they are restarted] → the branch's checks use ports of their own; after the merge the user restarts the services from the main checkout (Migration Plan).
- [An older justfile takes the largest Whisper model: once turbo is in `~/.voicepuncher/models/`, an older checkout that starts its single service confirms with turbo (2 of 13 voice-control tests fail that way)] → turbo goes there only at merge time; branches in flight merge `main`; a rollback moves it out (Migration Plan 3, 5, 6).
- [`check` and `full-check` get slower (spike: `e2e-seams` ≈ 224 → 387 s, `e2e-bench` ≈ 117 → 311 s with the services' start), so the stated durations go wrong] → measurement 3, and D9 updates them.
- [With five or more dictation copies, the dictation copy on 18086 meets the first copy of the commands service] → the start fails naming that copy's log, as for any copy that cannot start; README states the limit.
- [External, not part of this change: `voice_forms_it` fails on the "restart" form (session-2, 27.23 s, "restart" → nothing) on main too, with small confirmations; `just check` and `just full-check` fail on it until it is fixed elsewhere] → named in the stage report and PR body; no bar loosened.
- [Overlap: `restore-recognition-prompt` (in progress) changes the confirmation and dictation requests and measures them on today's setup] → the dependencies stage settles the order; whichever merges second measures on the new setup.

## Migration Plan

1. **Own ports and the checked copy while the branch is open:** its model stages run with `VOICEPUNCHER_STT_PORT`, `VOICEPUNCHER_COMMAND_STT_PORT` and `VOICEPUNCHER_KWS_PORT` of its own and with `VOICEPUNCHER_WHISPER_MODEL` naming `~/.voicepuncher/models/research/ggml-large-v3-turbo-q5_0.bin`, all set in the environment of the check commands (`kit` passes them on). Not in `.env`: the model scenarios remove the model variables, and the scripts would read them back from `.env`. Text refinement stays shared on 8080.
2. **Red first:** the new scenarios run red before the justfile change (Test plan).
3. **Turbo reaches its default path only at merge time** (stage 11, before the services are restarted): the checked file is copied from `~/.voicepuncher/models/research/` to `~/.voicepuncher/models/ggml-large-v3-turbo-q5_0.bin` — the pinned hash, no new download; `just models` then reports it present. Not earlier: a justfile without this change takes the largest `ggml-*.bin` in `~/.voicepuncher/models/`, so every checkout — main included — that starts its speech recognition service would run turbo for command confirmations too, which failed 2 of 13 voice-control tests.
4. **After the merge** and the main checkout's update, the user restarts the services from the main checkout (`just run`, or `just stop-model-servers && just start-model-servers`): dictation on turbo at 8081, commands on small at 8085, command spotting confirming through 8085. The session's report says so.
5. **Branches in flight** merge `main` (`kit sync-base`) before their next model stage: from step 3 on, an older justfile starts its single service on turbo when it starts one itself.
6. **Rollback:** revert the squash commit, move turbo out of `~/.voicepuncher/models/` (or set `VOICEPUNCHER_WHISPER_MODEL` to small) — the old largest-model rule would pick it for confirmations too — and restart the services.

## Dependencies

Waits for `fail-hung-tests-fast`: it reworks the same `justfile` E2E recipes, the E2E harness (`e2e-test/tests/cucumber_e2e.rs`, `e2e-test/src/`) and `commandspotting_websocket_it` that this change extends for the speech recognition service for commands.
