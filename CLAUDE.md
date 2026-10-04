# voicepuncher — AI agent instructions

## What this project is

Push-to-talk voice-to-text CLI for macOS Apple Silicon. Single Rust crate at `app/`. Fully offline: whisper.cpp for transcription, Qwen2.5 (via llama.cpp) for text cleanup, both running on Metal. No network calls at runtime.

**Product lifecycle stage:** PoC (Proof of Concept). The core workflow is demonstrated but the codebase is not yet hardened for daily use.

## Repository layout

```
voicepuncher/
  app/          ← Rust CLI crate (hexagonal architecture)
  e2e-test/     ← Rust crate: accuracy benchmark + smoke E2E
  docs/         ← Architecture documentation + ubiquitous language
  Cargo.toml    ← Cargo workspace (members: app, e2e-test)
  justfile      ← Task runner: all checks, E2E, benchmarks (`just` lists recipes)
```

## Workflow — OpenSpec

Every feature, bug fix and behavior change goes through OpenSpec. Skip it only for changes without behavior change: typos, formatting, renames, docs, dependency bumps, tooling. When unsure, use OpenSpec.

1. `/opsx:explore` — optional: think the idea through without creating files.
2. `/opsx:propose "<idea>"` — creates `openspec/changes/<name>/` with proposal, specs, design, tasks. Step by step instead: `/opsx:new`, then `/opsx:continue` per artifact or `/opsx:ff` for all.
3. Get the user's approval of the proposal before step 4.
4. `/opsx:apply` — implement `tasks.md` on a `<type>/<name>` branch; `just test` after every edit, `just check` after every task group.
5. `/opsx:verify` — check the code against the change specs, then run `just full-check` (§ How to verify a change).
6. `/opsx:archive` — merge the change specs into `openspec/specs/` and archive the change, in the same PR as the code.

CLI: `openspec list` (changes), `openspec list --specs`, `openspec view` (dashboard), `openspec validate`.

## How to verify a change

All checks are `just` recipes (see `justfile`; no other scripts). A change is "verified" only when every level that applies has run and passed — say which levels ran and which did not.

| When | Command | What it runs |
|---|---|---|
| After every edit | `just test` (~1 min) | unit + architecture + integration tests |
| After every task, before every push | `just check` (~4 min) | fmt, clippy `-D warnings`, `test`, release build, E2E smoke, WER accuracy on 10 samples |
| Before merging a branch, before a release | `just full-check` (~30 min) | `cargo clean` + everything in `check`, with WER accuracy on the whole corpus |
| Before a release, and after changes to hotkey, recording, overlay, paste, permissions | manual run by the user | `README.md` § Release checklist |

Do not merge a branch until `just full-check` has passed on its final commit; say in the PR which checks ran.

`just bench` is a model comparison, not a gate — only when a model, decoding params or the prompt changed.

- **Models and corpus** are auto-discovered by the `justfile` (largest model in `~/.voicepuncher/` or `~/.voicepuncher/models/`; corpus in `e2e-test/corpus/`). The corpus is gitignored, so in a worktree set `VOICEPUNCHER_BENCH_CORPUS` to the main checkout's `e2e-test/corpus`.
- **`full-check` runs `cargo clean`** and deletes `target/release/voicepuncher`, which the user may be running. In the main checkout, ask first; in a worktree it only cleans that worktree's `target/`.
- **The manual run cannot happen in an agent sandbox**: it lacks macOS TCC permissions (`permission denied for accessibility`). Ask the user to run `cargo run --release -p voicepuncher` (with and without `--no-llm` when refinement changed) and paste the log. A healthy cycle logs `pipeline recv Pressed` → `chunk 1 | … transcribed` → `pasting…` → `stop→paste: … total=…ms`.

## Test layout (inside `app/`)

| Layer | Location | Naming |
|---|---|---|
| Unit | `#[cfg(test)]` blocks inside each `*.rs` | `mod tests { fn should_*` } |
| Integration | `app/tests/*_it.rs` | `<adapter>_it.rs` |
| Architecture | `app/tests/architecture.rs` | single file, import-graph assertions |

## Ubiquitous language

All domain terms are defined in `docs/ubiquitous-language.md`. Read it before naming any type, method, event, or test. The glossary is the contract — code must match it.

## Architecture rules

- Dependency direction: `adapters → core/application → core/domain`. Reverse direction is forbidden.
- Ports (traits) live flat in `app/src/core/application/`.
- No framework imports inside `app/src/core/`.
- Adapter placement: `app/src/adapters/primary/<tech>/` and `app/src/adapters/secondary/<tech>/`.

## Key constraints

- macOS + Apple Silicon only (Metal GPU, CoreAudio, macOS TCC permissions).
- Zero network I/O at runtime — all inference is local.
- Three macOS TCC permissions required: Microphone, Accessibility, Input Monitoring.
