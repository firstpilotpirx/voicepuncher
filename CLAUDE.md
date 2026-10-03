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
  scripts/      ← Repo-level automation
  Cargo.toml    ← Cargo workspace (members: app, e2e-test)
  justfile      ← Task runner (build, bench, corpus, e2e)
```

## Workflow — OpenSpec

Every feature, bug fix and behavior change goes through OpenSpec. Skip it only for changes without behavior change: typos, formatting, renames, docs, dependency bumps, tooling. When unsure, use OpenSpec.

1. `/opsx:explore` — optional: think the idea through without creating files.
2. `/opsx:propose "<idea>"` — creates `openspec/changes/<name>/` with proposal, specs, design, tasks. Step by step instead: `/opsx:new`, then `/opsx:continue` per artifact or `/opsx:ff` for all.
3. Get the user's approval of the proposal before step 4.
4. `/opsx:apply` — implement `tasks.md` on a `<type>/<name>` branch.
5. `/opsx:verify` — check the code against the change specs, then run § How to verify a change.
6. `/opsx:archive` — merge the change specs into `openspec/specs/` and archive the change, in the same PR as the code.

CLI: `openspec list` (changes), `openspec list --specs`, `openspec view` (dashboard), `openspec validate`.

## How to verify a change

Run these levels in order. A change is "verified" only when every level that applies has run and passed — say which levels ran and which did not.

### Environment

- Models live in `~/.voicepuncher/` (not `~/.voicepuncher/models/`). Always export them explicitly; without them model-dependent tests print `GATED: … skipping` and pass without testing anything:
  ```bash
  export VOICEPUNCHER_WHISPER_MODEL=$HOME/.voicepuncher/ggml-large-v3-turbo.bin
  export VOICEPUNCHER_LLM_MODEL=$HOME/.voicepuncher/Qwen2.5-1.5B-Instruct-Q4_K_M.gguf
  export VOICEPUNCHER_BENCH_CORPUS=$PWD/e2e-test/corpus   # directory with manifest.jsonl, absolute path
  ```
- The benchmark corpus `e2e-test/corpus/` is gitignored: it exists only in the main checkout (or after `just corpus`). In a worktree, point `VOICEPUNCHER_BENCH_CORPUS` at the main checkout's `e2e-test/corpus`.
- If `llama-cpp-sys` fails with `'openssl/err.h' file not found`, export Homebrew OpenSSL for the build:
  ```bash
  export CPATH=/opt/homebrew/opt/openssl@3/include LIBRARY_PATH=/opt/homebrew/opt/openssl@3/lib
  ```

Before a release run all three: `just release-check` (levels 1–2) plus the manual checklist in `README.md` § Release checklist (level 3). `just bench` is a model comparison, not a gate — only when a model, decoding params or the prompt changed.

### Level 1 — code checks (every change)

`just check` — runs:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p voicepuncher --tests --no-fail-fast   # unit + architecture + integration
cargo build --release
```

`./scripts/full-check-fresh.sh` and `just release-check` start with `cargo clean`: it deletes `target/release/voicepuncher`, which the user may be running. Ask before running it; prefer the commands above.

### Level 2 — E2E on real models (changes to transcription, refinement, chunking, pipeline)

`just e2e` — runs:

```bash
cargo test -p voicepuncher-e2e-test --test smoke --release -- --nocapture             # models load, file mode works
cargo test -p voicepuncher-e2e-test --test pipeline_accuracy --release -- --nocapture # WER on corpus, fails if mean WER > 30 %
```

Both drive the release binary in file mode (`--input-file`), so build it first. Model comparison (`just bench`, `just bench-smoke`) is described in `README.md` § End-to-end tests; it downloads missing models.

### Level 3 — manual run (changes to hotkey, recording, overlay, paste, permissions)

Hotkey capture, microphone, overlay and paste need macOS TCC permissions that an agent sandbox does not have — `voicepuncher` exits with `permission denied for accessibility` there. Ask the user to run it from their terminal, walk through `README.md` § Release checklist, and paste the log:

```bash
cargo run --release -p voicepuncher            # add --no-llm to skip the LLM
```

A healthy cycle logs `pipeline recv Pressed` → `chunk 1 | … transcribed` → `pasting…` → `stop→paste: … total=…ms`. Test with and without `--no-llm` when refinement changed.

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
