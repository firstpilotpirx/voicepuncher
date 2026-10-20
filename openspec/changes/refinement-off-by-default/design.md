## Context

Text refinement runs on every dictation today unless `--no-llm` is passed. The app wires it in `main.rs` from one `no_llm` flag (refiner, pipeline stage, archive's model field, startup check). This change flips the default and adds an opt-in; the wiring stays.

## Decisions

### One switch: `[refinement] enabled`, set by the config and the two flags

`RefinementCfg` gets `enabled: bool` (serde default `false`). `Cli::overlay` sets it to `true` for `--llm` and to `false` for `--no-llm`, applied after `--llm`, so `--no-llm` wins over both the config and `--llm`. `main.rs` reads `!config.refinement.enabled` after the overlay instead of `cli.no_llm`; everything downstream of that boolean is unchanged. Rejected: a tri-state CLI option (`--refinement on|off`) — it breaks the existing `--no-llm` commands for no gain.

### `just run` decides from its own arguments

`run` is turned into a shell recipe: when its arguments contain `--llm`, it downloads every model and starts all four services as today; otherwise it skips the refinement model download and starts every service except `llama-server`. `models` gets an optional `refinement` parameter (default `yes`) so that other callers keep downloading everything. `run` still stops all services first, so a llama-server left from an earlier run with `--llm` is stopped. A user who turns refinement on in the config only must pass `--llm` to `just run` as well; the README says so. Rejected: reading the config file from the justfile — it would duplicate the app's config parsing in bash. `start-model-servers` without arguments still starts llama-server, since checks and benchmarks need it.

### Tests that need refinement ask for it

The E2E steps that run the app with refinement pass `--llm` instead of relying on the default; steps without refinement keep passing `--no-llm`, and a third mode passes neither, for the default-settings scenario.

## Test levels

| Scenario | Level |
|---|---|
| Default settings paste the raw transcript | E2E `text_refinement.feature` (@smoke) |
| Refinement turned on in the configuration | E2E `text_refinement.feature` (@smoke) |
| Turning refinement off on the command line wins over the configuration | E2E `text_refinement.feature` (@smoke) |
| One command without refinement | manual run |
| One command with refinement turned on | manual run |
| Service is not running at startup (modified) | E2E `text_refinement.feature`, existing scenario, now with `--llm` |
| Configured model is not offered (modified) | E2E `text_refinement.feature`, existing scenario, now with `--llm` |

The flag parsing and the overlay precedence (`--llm`, `--no-llm`, config) also get unit tests in `cli/mod.rs`, and the config default a unit test in `config/refinement.rs`.

## Risks

- A user who relied on refinement silently loses it after the update; the README's run section and the release notes name `--llm`.
- `just run --llm` with the refinement model missing downloads it (~2.5 GB) as before.
