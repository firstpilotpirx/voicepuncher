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

## Clean-slate validation command

```bash
VOICEPUNCHER_WHISPER_MODEL=<path> VOICEPUNCHER_LLM_MODEL=<path> ./scripts/full-check-fresh.sh
```

Both env vars must point to downloaded model files. The script runs from the repo root:
`cargo clean → fmt check → clippy -D warnings → architecture test → lib tests → integration tests → release build`.

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
