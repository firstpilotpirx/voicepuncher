## 1. Refinement off by default in the app

- [x] 1.1 Red: unit tests in `app/src/config.rs` (default `enabled` is false, `enabled = true` parses) and `app/src/adapters/primary/cli/mod.rs` (`--llm` parses; overlay: `--llm` turns it on, `--no-llm` wins over `--llm` and over the config); run them, see them fail
- [x] 1.2 Green: add `enabled` to `RefinementCfg`, `--llm` to `Cli`, the precedence to `Cli::overlay`; `main.rs` reads `config.refinement.enabled` instead of `cli.no_llm`
- [x] 1.3 Red: in `e2e-test/tests/features/text_refinement.feature` replace `Running without refinement` with the scenarios `Default settings paste the raw transcript`, `Refinement turned on in the configuration` and `Turning refinement off on the command line wins over the configuration` under the new rule; steps in `e2e-test/tests/steps/text_refinement/mod.rs` (a refinement mode on, off, default for `run_app`, `--llm` for on); run them against the code before 1.2, see the default one fail
- [x] 1.4 Green: pass `--llm` wherever an E2E step runs the app with refinement (`text_refinement/mod.rs`, `file_transcription.rs`); the new scenarios pass
- [x] 1.5 `just test` and commit

## 2. `just run` and docs

- [x] 2.1 `justfile`: `models` takes an optional `refinement` parameter; `run` skips the refinement model and llama-server unless its arguments contain `--llm`
- [x] 2.2 `README.md` (options, run section, release checklist), `docs/ubiquitous-language.md` (text refinement), `docs/project-map.md` (passthrough adapter): refinement off by default, `--llm` and `[refinement] enabled = true` turn it on
- [x] 2.3 `e2e-test/scripts/bench.py`: pass `--llm` when it runs the app, so `just bench` keeps measuring refinement
- [x] 2.4 `just test` and commit

## 3. Version and manual checks

- [x] 3.1 Bump `version` in `app/Cargo.toml` to 0.30.0
- [x] 3.2 `just check` once, then commit
- [x] 3.3 (deferred by the user to the end of the batch) Manual run by the user: `just run` with the refinement model moved away → the app starts, no llama-server, the overlay shows no "Refining text", the raw transcript is pasted; `just run --llm` → llama-server starts and the pasted text is refined
