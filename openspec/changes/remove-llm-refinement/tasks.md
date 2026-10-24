## 1. Paste the transcript without refinement (core)

- [ ] 1.1 Red: unit tests of `text_to_paste` in `app/src/core/domain/filler_sounds.rs` — filler sounds dropped ("Эм, давай, э-э, созвонимся завтра утром." → "давай, созвонимся завтра утром."), a transcript without them unchanged, a filler-only transcript returned as it was
- [ ] 1.2 Green: `text_to_paste` in the domain; the pipeline (`processing.rs` `finish`, `run_once`) pastes it; remove `text_refiner.rs` (port, `refine_or_raw`, `RefinedText`), `adapters/secondary/textrefinement/`, `normalization_check.rs`, `respelling.rs`, the `refiner` part of the pipeline and its wiring
- [ ] 1.3 Remove the refining overlay stage: `VisualIndicator::show_refining_text`, `cfg.shows_refinement`, the sparkles stage of the pill (`pill_content.rs`), the `refine=` lap of the stage clock
- [ ] 1.4 Remove `FailureNotice::PastedUnrefined` and `FailingPart::TextRefinement` (REFINE) with their notice texts and tests
- [ ] 1.5 Archive entry: drop `RefinementOutcome`, the entry's `refinement` and `refinement_model` fields and `trace_refinement`; bump the `entry.json` layout version; update the archive unit tests
- [ ] 1.6 Update the pipeline unit tests and the slice/component scenarios and steps of `processing_stages`, `failure_notices`, `dictation_archive`, `visual feedback` for the removed stage, notice and fields (`app/tests/features/component/*.feature`, `app/tests/steps/`)
- [ ] 1.7 `just test`; commit

## 2. Configuration, command line and settings window

- [ ] 2.1 Red: `app/tests/it/config_loader.rs` — a file with `[refinement]`, `[llm]` and `services.refinement_copies` loads and their values are ignored; Red: `app/tests/it/cli.rs` — `--no-llm` exits with code 2 naming the option
- [ ] 2.2 Green: the loader drops the retired `refinement`/`llm` tables and `services.refinement_copies` with one warning; remove `RefinementCfg`, `config/refinement.rs`, `refinement_copies`, the `[refinement]` section of `default.toml` and `config.example.toml`, the `--llm`/`--no-llm` flags and their overrides
- [ ] 2.3 Settings window: remove the Refinement tab, its fields, `examples.rs`, the override note; update `settings_window.feature` and its steps (the retired settings survive a Save)
- [ ] 2.4 `just test`; commit

## 3. Local services, service scripts, engines and recipes

- [ ] 3.1 Remove `LocalService::Refinement`, `ModelUse::Refinement`, `ModelNeeds::refinement`, the Qwen model of the catalog, the refinement command line of `localservices/catalog`, its runtime wiring and tests (`catalog/tests.rs`, `command_lines.rs`, `menu_bar_presence`, `app_runtime`)
- [ ] 3.2 `dev-tools/src/services/`: drop `llama-server` (start, ready, stop, health check, copies, e2e ports 28080 / 27080) and its tests; `copy-pool` address notes
- [ ] 3.3 `dev-tools/src/engines/` and `app/packaging/engines.lock`: drop the `[llama]` engine, the `github-release` source kind and its tests; drop the llama.cpp license from the bundle resources
- [ ] 3.4 `justfile` and `.config/`: remove `VOICEPUNCHER_LLM_MODEL`, the Qwen download in `just models`, the refinement evaluation, `e2e-smoke-refinement`, the 20-run series and their `stage_inputs` lines, stage lists, nextest limits; `dev-tools` stage tests
- [ ] 3.5 `just test`; commit

## 4. E2E tests and benchmark

- [ ] 4.1 Remove `e2e-test/tests/features/text_refinement.feature`, `e2e-test/tests/steps/text_refinement/`, `e2e-test/src/refinement_eval.rs`, `e2e-test/src/refinement_service_stub/`, `app/tests/it/textrefinement_*.rs`; their harness registrations
- [ ] 4.2 Update the E2E scenarios and steps that turn refinement on or off (`file_transcription`, `speech_recognition`, `local_services`, `app_distribution`, `voice_control`) and `e2e-test/scripts/bench.py`
- [ ] 4.3 `just analyzers-baseline` and `just size-baseline` to drop the entries of deleted code; check that no entry was added or raised
- [ ] 4.4 `just test`; commit

## 5. Documentation and version

- [ ] 5.1 `AGENTS.md`, `README.md`, `docs/` (architecture, ubiquitous language, project map, benchmark docs): no text refinement, three services, the paste right after speech recognition
- [ ] 5.2 `git grep -i 'llama\|llm\|refine'` outside `openspec/changes/archive/` and dated acceptance-run notes finds no live reference
- [ ] 5.3 Bump `version` in `app/Cargo.toml` to 0.38.0
- [ ] 5.4 After archive: delete `openspec/specs/text-refinement/` and update the Purpose of `processing-stages` and `local-services`
- [ ] 5.5 `just test`; commit
