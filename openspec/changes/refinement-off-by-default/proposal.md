## Why

The user asked whether text refinement (the local LLM, llama-server with Qwen3-4B) is worth keeping. An analysis of the 46 dictations in the dictation archive (`~/.voicepuncher/history`, 2026-10-19..20, app 0.24.2) compared the raw transcript with the pasted text:

- 2 of 46 fell back (refinement answer rejected, raw transcript pasted).
- Of the 44 refined: 11 were pasted unchanged, ~20 got only cosmetic edits (е → ё, one comma, a final period).
- ~10 helped: Whisper sometimes returns a chunk with no punctuation at all ("кстати еще бага когда идет запись…"); refinement added periods, commas and capitals there. A few correct word fixes too ("серби-то" → "Сербия", "госслугах" → "Госслугах", "минут" → "минуты").
- ~8 hurt, ~5 of them changed meaning: "проанализируй… выдвинуть предложения" → "проанализирую… выдвину" (a request became a promise); "добавь ещё такую таску в бэклог" → "добавь к этой задаче" (new task became an edit of an existing one); "на пасти других виджетов нет" lost "пасти"; "переповторы" → "переподключения"; "заблюренная" → "заблуждённая"; "агент кита… рост кода" (agent-kit, Rust) → "агента Кита… роста кода". It also replaced the user's own words ("таска" → "задача", "приложуха" → "приложение").
- It almost never fixes real recognition errors ("клото" for Claude stayed wrong).
- It costs 52 % of the stop → paste latency (median 1.5 s, max 9 s per dictation).

Conclusion: refinement gives little, sometimes corrupts the meaning, and doubles the wait. The user decided to turn it off by default.

## What Changes

- The app pastes the raw transcript by default, without calling the text refinement service; the overlay shows no "Refining text" step.
- Text refinement becomes opt-in: an explicit flag on the command line (proposed `--llm`) or a config setting (proposed `[refinement] enabled = true`) turns it back on; `--no-llm` keeps working as a no-op alias of the default so existing commands do not break.
- `just run` (the user's one command to run the app) does not start llama-server when refinement is off, and does not require the refinement model on disk; with the opt-in it starts it as today.
- The startup check that the text refinement service answers runs only when refinement is on.

Current stage: PoC. Target stage: PoC.
Scope: the default of text refinement in the app, its CLI and config switch, `just run` and the services it starts, the README and docs that describe the default.
Acceptance bar: a dictation with default settings is pasted as its raw transcript with no request to the text refinement service and no llama-server started by `just run`; with the opt-in, refinement runs as before; `just full-check` passes.
Out of scope: improving the refinement prompt or the normalization check; removing refinement or llama-server from the project; the refinement evaluation in `just check` and the E2E recipes, which keep exercising refinement explicitly.

## Capabilities

### Modified Capabilities

- `text-refinement`: refinement is off unless the user turns it on; how it is turned on.

## Impact

- `app/` CLI arguments and config (`[refinement]` section), the pipeline wiring of the text refinement stage, the startup service check.
- `justfile` recipe `run` and the services `start-model-servers` starts for it.
- `README.md`, `docs/ubiquitous-language.md` (text refinement: "Disabled with `--no-llm`" becomes the default).

## Open points

- Name of the opt-in: proposed `--llm` flag plus `[refinement] enabled = true` (default `false`); `--no-llm` stays accepted and wins over the config.
- Whether `just start-model-servers` without arguments still starts llama-server: proposed yes (tests and benchmarks need it), only `just run` skips it when refinement is off.
- Capability name: proposed `text-refinement`; use the existing spec that holds the `--no-llm` requirement if it is named differently.
- Later idea, not in this change: refine only chunks that came without punctuation.

## Release

SemVer: MINOR — pre-1.0 change of default behavior (refinement no longer runs unless opted in).
Target version: 0.25.0.
