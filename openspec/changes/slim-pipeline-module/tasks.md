# Tasks

## 1. Ports without default bodies

- [x] 1.1 Remove the default bodies of `truncate_at`, `begin_with`, `recording` in `app/src/core/application/audio_recorder.rs`, of `show_command_sign`, `show_commands_unavailable`, `hide_commands_unavailable`, `live_transcript_view` in `visual_indicator.rs` and of `heard_since` in `command_spotter.rs`
- [x] 1.2 Implement them explicitly in every implementor the compiler names: `adapters/secondary/recording/{cpal,filesystem}`, `adapters/secondary/visualfeedback/macos`, `adapters/secondary/commandspotting/websocket`, `runtime/overlay.rs`, `main.rs`, `app/tests/steps/**`, `e2e-test/tests/steps/voice_control/{session,hands_free}.rs`
- [x] 1.3 `just test`; commit `refactor(ports): no default bodies on the recorder, overlay and spotter ports`

## 2. One constructor value

- [ ] 2.1 Add `PipelineParts` (the six ports) and `Pipeline::new(parts, cfg)`; move `stop_wait` and `shows_refinement` into `PipelineConfig`, keep `with_stop_wait` and `without_text_refinement_stage`; make every field private
- [ ] 2.2 Sweep the 22 call sites of `Pipeline::new` (wiring, main, runtime tests, component steps, `runtime_signal_it.rs`, `architecture.rs`, the E2E harness)
- [ ] 2.3 `just test`; commit `refactor(pipeline): build the pipeline from one parts value`

## 3. The dictation as a value, two states, voice control without Option

- [ ] 3.1 Add `pipeline/dictation.rs`: `Dictation` (live session, command words, pending notice, trace) and `without_command_words`; replace `state`, `live_session`, `command_words`, `pending_notice`, `trace` with `dictation: Option<Dictation>`; delete `PipelineState` and the tests `should_ignore_pressed_signal_while_transcribing` and `should_ignore_released_signal_while_pasting`
- [ ] 3.2 Replace `voice: Option<VoiceControl>` with `Voice::{Off, On(VoiceControl)}` in `pipeline/voice_control.rs`; match once in the dispatch; drop `expect("voice control is on")`
- [ ] 3.3 Group `focus_probe`/`focus_wait` into `FocusWait` and `inputs`/`pending` into `Inbox`; `Pipeline` at 12 fields
- [ ] 3.4 `just test`; commit `refactor(pipeline): hold the dictation as one value and voice control as off or on`

## 4. One processing path, one abandon, one close

- [ ] 4.1 Red: unit test that `run_once` returns the pasted text and the pipeline prints nothing (`pipeline/tests/`)
- [ ] 4.2 Add `pipeline/processing.rs`: `StageClock`, `finish`, `end_without_text`, `transcribe_recording`, `refine`, `deliver`; delete `run_transcribe_refine_paste` and `run_refine_paste`; `run_once` returns `CleanedText`, `run_file_mode` in `main.rs` prints it; remove both `eprintln!`
- [ ] 4.3 Add `pipeline/recording.rs`: `start_recording_from`, `stop_recording`, `cancel_recording`, `abandon(Awaits)` replacing `drop_recording`/`discard_recording`, `stop_recorder_quietly` replacing every `let _ = self.recorder.stop()`; `close_dictation(spoken)` in `voice_control.rs` for StopDictation and PressEnter
- [ ] 4.4 Move `run_live_worker` to `pipeline/live_worker.rs`; log a result nobody receives at debug
- [ ] 4.5 `just test`; commit `refactor(pipeline): one processing path, one abandon, one close of a dictation`

## 5. Pipeline unit tests on one builder

- [ ] 5.1 Move the tests of `pipeline.rs` to `pipeline/tests/{mod,recording,processing,overlay,activity}.rs` with a `PipelineUnderTest` builder (no-op doubles by default)
- [ ] 5.2 Red: test asserting the `stop→paste: … total=…ms` line through a capturing logger; Green: keep the line
- [ ] 5.3 Replace every assertion on the private state by the pasted text, the overlay calls or the published activity
- [ ] 5.4 `just test`; commit `test(pipeline): one builder, assertions on observable effects`

## 6. Runtime idle wait and main

- [ ] 6.1 Red: unit test in `runtime/activity.rs` that `wait_idle` returns as soon as `Idle` is published and after the bound otherwise
- [ ] 6.2 Green: `ActivityRegister` on a mutex and a condvar; `wait_for_idle` in `runtime/mod.rs` blocks on `wait_idle` instead of sleeping
- [ ] 6.3 Add `end_pipeline(config)` shared by `reload` and `wait_for_models`
- [ ] 6.4 `fn main() -> ExitCode` calling `run(cli) -> Result<ExitCode>` in `main.rs`; one error print, one exit code 2; `cli_it.rs` stays green
- [ ] 6.5 `just test`; commit `refactor(runtime): wait for idleness on a condvar; main returns an exit code`

## 7. Budgets and baselines

- [ ] 7.1 Every new file under 300 code lines, every function within the budgets; `just size-baseline` and `just analyzers-baseline` drop the gone or shrunk entries; no entry added or raised (diff of `app/tests/size_baseline.toml`, `.config/analyzers/metrics.toml`)
- [ ] 7.2 `just test`; `just check`; commit `chore(baselines): drop the pipeline entries the refactor retired`
- [ ] 7.3 Version unchanged (no release); manual checks: the areas `kit checks-needed` names, else none
