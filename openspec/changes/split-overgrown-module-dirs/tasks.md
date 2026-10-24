# Tasks

## 1. Clippy autofix and lint configuration

- [ ] 1.1 `cargo clippy --fix` with the ratchet's lint set over the workspace; `just fmt`
- [ ] 1.2 Turn off `clippy::cast_precision_loss` and `clippy::suboptimal_flops` in `.config/analyzers.toml` with their reasons
- [ ] 1.3 `just analyzers-baseline`; `just test`; commit

## 2. Test steps of `app/` within budget

- [ ] 2.1 Move each `app/tests/steps/<capability>.rs` beside its folder into `<capability>/mod.rs`; group the loose step files into folders
- [ ] 2.2 Split every listed file under `app/tests/steps/` along its scenarios' concerns to at most 300 code lines
- [ ] 2.3 `just size-baseline`; `just test`; commit

## 3. Integration tests of `app/` within budget

- [ ] 3.1 Group `app/tests/it/*.rs` into concern folders, listed in `main.rs`; `test_layout` accepts the folders
- [ ] 3.2 Split `architecture.rs`, `commandspotting_reconnect.rs`, `commandspotting_websocket.rs`, `transcription_openai.rs` and the listed `main_thread` files
- [ ] 3.3 `just size-baseline`; `just test`; commit

## 4. `core/application` and `core/domain` within budget

- [ ] 4.1 Move `chunking.rs` and `chunking_seam_cases.rs` into `chunking/`, `model_download.rs` and `text_insertion.rs` into their folders; ports stay flat
- [ ] 4.2 Split the listed `core/application` files (chunking, seam cases, model download tests, voice control tests, text insertion)
- [ ] 4.3 Move the non-port modules of `core/domain` into folders until it fits
- [ ] 4.4 Update `recognition_paths`, `chunking_paths` and `stage_inputs` in `.config/stages.toml`
- [ ] 4.5 `just size-baseline`; `just test`; commit

## 5. macOS adapters within budget

- [ ] 5.1 Split `visualfeedback/macos` into view folders and its listed files (`mod.rs`, `glass.rs`, `pill_content.rs`, `pill_view.rs`)
- [ ] 5.2 Split `settings/macos` until it fits
- [ ] 5.3 Split the listed menu bar, model download, permissions files
- [ ] 5.4 `just size-baseline`; `just test`; commit

## 6. Other `app/src` files within budget

- [ ] 6.1 Split `localservices/catalog`, `commandspotting/websocket`, `transcription/openai`, `config.rs`, `config/transcription.rs`, `runtime/mod.rs`
- [ ] 6.2 `just size-baseline`; `just test`; commit

## 7. `command-spotting` and `e2e-test` within budget

- [ ] 7.1 Split `command-spotting/src/engine.rs`, `matcher/placed.rs`, `tests/it/engine.rs`, `tests/it/placed_commands.rs`; the `tests/it` folder fits
- [ ] 7.2 Split `e2e-test/src/harness_args.rs`, `speech.rs` and the listed step files; `e2e-test/tests/steps` fits
- [ ] 7.3 `just size-baseline`; `just test`; commit

## 8. Functions, nesting, parameters and structs within budget

- [ ] 8.1 Refactor the listed units of `app/src`; drop their `#[expect(..., reason = "size baseline")]`
- [ ] 8.2 Refactor the listed units of `app/tests`
- [ ] 8.3 Refactor the listed units of `command-spotting/`, `e2e-test/`, `dev-tools/`
- [ ] 8.4 `just size-baseline` leaves the file empty; `just test`; commit

## 9. Casts and the remaining clippy findings

- [ ] 9.1 Fix the cast findings with `try_from` or named conversion helpers
- [ ] 9.2 Fix every other listed finding (`needless_pass_by_value`, docs, `expect_used`, `float_cmp`, …)
- [ ] 9.3 `just analyzers-baseline` leaves `clippy.toml` empty; `just test`; commit

## 10. Metrics

- [ ] 10.1 Bring every function over cyclomatic 10 under it
- [ ] 10.2 Break the import cycles of `core/application`, `command-spotting/src` and `dev-tools/src`
- [ ] 10.3 Remove the duplicated lines into shared helpers
- [ ] 10.4 Fix the two shellcheck findings of the recipes
- [ ] 10.5 `just analyzers-baseline` leaves only `unsafe app`; `just test`; commit

## 11. Chunk request hang

- [ ] 11.1 Red: unfreeze "The first request of a chunk hangs"; run it with `just test-one`, find the cause of the second hung attempt
- [ ] 11.2 Green: fix it; the scenario passes repeatedly; commit

## 12. Voice control scenarios

- [ ] 12.1 Red: enable "The user's recorded restart restart followed by the new dictation at once" and the frozen regression examples; run them in their E2E recipe, find the cause
- [ ] 12.2 Green: fix it or delete the scenario, naming the covering test in `design.md`
- [ ] 12.3 Red: enable "A chain ending in start gives its cue at once"; run it, see it fail
- [ ] 12.4 Green: fix the early checks' hearing of "Enter" or delete the scenario, naming the covering test
- [ ] 12.5 Red: unfreeze "Restart restart followed by the new dictation at once gives its cue at once"; run `just e2e-cues`
- [ ] 12.6 Green: fix the slow path within the 700 ms bound; commit

## 13. Service and bundle scenarios

- [ ] 13.1 Red: enable "Russian speech that makes speech recognition repeat itself"; run `just e2e-resilience`, find the cause
- [ ] 13.2 Green: fix it or delete the scenario, naming the covering test
- [ ] 13.3 Red: enable "The pinned engines are staged"; run the bundle stage, find the hang
- [ ] 13.4 Green: fix it; `just frozen-tests` lists none; commit

## 14. Docs, version and check

- [ ] 14.1 `AGENTS.md`: the size baseline and analyzer baselines are empty; moved paths named in docs
- [ ] 14.2 Bump `version` in `app/Cargo.toml` to 0.38.6 only when groups 11–13 changed app code
- [ ] 14.3 `just test`; `just check` once; commit
- [ ] 14.4 Manual checks: none (no hotkey, recording, overlay, paste or permission behavior changes)
