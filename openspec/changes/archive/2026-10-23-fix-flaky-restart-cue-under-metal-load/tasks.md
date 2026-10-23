# Tasks

## 1. The warm-up utterance and the service scripts (decisions 1, 2)

- [x] 1.1 Red: scenario "The service scripts report a copy ready after its first transcription" in `e2e-test/tests/features/local_services.feature` (Rule of the added requirement) with its steps in `e2e-test/tests/steps/local_services/warm_up.rs`; run it against the current scripts, it fails (no first-transcription time in the ready lines).
- [x] 1.2 Green: `app/assets/warm-up.wav`; `whisper_ready` in the `justfile` as the readiness check of every whisper copy, the ready line with the first transcription's time; rerun the scenario; `just test`; commit.

## 2. The app's service start (decision 3)

- [x] 2.1 Red: unit tests in `app/src/core/application/local_services/supervision_tests.rs` — a speech recognition copy stays Starting while its warm-up is refused and is Ready once it is accepted, a copy whose warm-up never passes within the start limit is restarted, a Ready copy's checks warm nothing, a copy started again after an exit is warmed again; `FakeProbe` records warm-ups; run them, they fail.
- [x] 2.2 Green: `ServiceProbe::warm_up`, `watch_start` in `supervision.rs`, the ready log line with the first transcription's time; `just test`; commit.
- [x] 2.3 Red: integration tests in `app/tests/it/localservices_process/probe.rs` (the `HttpProbe` tests move there, so `app/tests/it/` shrinks) — `HttpProbe::warm_up` posts the utterance to the transcription path of a stub server and is true on 200, false on an error answer, true at once for a non-whisper kind; run them, they fail.
- [x] 2.4 Green: `HttpProbe::warm_up` in `app/src/adapters/secondary/localservices/process/probe.rs` with the embedded utterance; `just test`; commit.

## 3. A missing cue names the confirmations cut short (decision 4)

- [x] 3.1 Red: unit tests in `e2e-test/src/stalled_copies/tests.rs` — the cut-short lines of a sample log since a given stream position, as "copy <port>: cut short after N ms", none when nothing was cut short; run them, they fail.
- [x] 3.2 Green: `e2e-test/src/stalled_copies.rs` with its module directory; `check_within` in `e2e-test/tests/steps/voice_control/session/start_cue.rs` appends the cut-short report before the machine load; `just test`; commit.

## 4. The series and the check (decision 5)

- [x] 4.1 Ten `VOICEPUNCHER_REUSE_STAGES=0 just e2e-cues` runs from freshly started services, in the background; their results into `design.md` § Series; commit.
- [x] 4.2 `just check`; commit.

## 5. Docs and version

- [x] 5.1 `AGENTS.md` § What this project is (a copy is ready after its warm-up transcription), `README.md` where it describes the services' start, the `justfile` comments; commit.
- [x] 5.2 `version` in `app/Cargo.toml` → 0.37.6; manual checks: none (the service start is not a manual-run area).
