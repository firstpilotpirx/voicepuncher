## Why

The repository has no `.github/`: the only gate is the owner's machine and the landing queue there (audit of 2026-10-22). A PR opened from another machine or a cloud session has no check at all; a toolchain drift (`rust-toolchain.toml` pins 1.88, `rust-version` too) is caught only locally; a fresh clone proving that `just test` passes depends on one laptop.

## What Changes

- A GitHub Actions workflow on every PR and on `main` runs on a macOS Apple Silicon runner: `cargo fmt --check`, `just lint` (clippy and the analyzers of `adopt-static-analyzers`), `just test` with nextest, and the `just metrics` ratchet — no model, no service, no GPU, no machine lock; ≈ 5 min.
- The workflow caches `target/` and the cargo registry by lockfile hash; it installs the analyzers with the same `just tools` recipe the developer uses, `just` and the pinned nextest with a pinned install action, shellcheck from Homebrew.
- Without the agent-kit machine lock (absent on the runner) the recipes run their bodies directly (`VOICEPUNCHER_NO_MACHINE_LOCK=1`); a nextest profile `ci` leaves out the two tests that need the owner's desktop session or voices; `just test` unsets empty model variables, so model tests skip on a machine without models.
- The branch protection on `main` requires the workflow (set by the owner; the change documents it in `README.md` § Release checklist).
- `AGENTS.md` § How to verify a change states that the landing queue's `full-check` remains the merge gate for build inputs and CI is the floor every PR has.

Current stage: PoC. Target stage: PoC.
Scope: `.github/workflows/ci.yml`, `justfile` (`machine_lock`, `_test`, `_test-lock-phases`), `.config/nextest.toml`, `README.md`, `AGENTS.md`.
Acceptance bar: a PR with a formatting error or a failing unit test shows a red check on GitHub (a clippy error fails the same `just lint` step as locally); a clean PR shows green within 10 min.
Out of scope: E2E, models, releases in CI.

## Capabilities

### Modified Capabilities

- `change-workflow`: every PR has a hosted check.

## Impact

- `.github/workflows/ci.yml`, `justfile`, `.config/nextest.toml`, `README.md`, `AGENTS.md`.

## Open points

- Runner: `macos-14` (Apple Silicon) proposed; if the free minutes are a concern, `just test` only on PRs and lint on `main`.

## Release

SemVer: no release.
Target version: unchanged.
