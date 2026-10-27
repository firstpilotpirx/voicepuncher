# Design

## Context

`sherpa-onnx` 2.0.1 builds against the code of `command-spotting/` unchanged: the keyword spotter, online stream and voice activity detector keep their names and fields. `sherpa-onnx-sys` 2.0.1 still links its native libraries statically (the release binary links only system libraries), and its `LICENSE` is identical to 1.13.8's, so the bundle, `app/packaging/engines.lock` and `e2e-bundle` need no change. Its build dependency `zip` 9 replaces `zip` 2.4 and `zstd` 0.13; every advisory the 7 ignores in `deny.toml` covered is gone, the three older ones (zip 2.4) included.

## Goals / Non-Goals

**Goals:** the command spotting service on `sherpa-onnx` 2.0.1; `cargo deny` without advisory ignores; voice control unchanged.

**Non-Goals:** keyword models, phrases, thresholds, the command spotting protocol, a toolchain bump, other dependency upgrades.

## Decisions

- **`aes` held at 0.9.2.** `zip` 9 pulls `aes` 0.9.3, which needs Rust 1.89; the workspace is on 1.88 (`rust-toolchain.toml`). `Cargo.lock` keeps `aes` 0.9.2 (Rust 1.85), the newest that builds on 1.88. Rejected: a toolchain bump (out of scope, touches every crate and CI); turning off `zip`'s `aes-crypto` feature (the feature is chosen by `sherpa-onnx-sys`, not by this workspace).
- **License `bzip2-1.0.6` allowed.** `zip` 9 brings `libbz2-rs-sys`, licensed under the bzip2 1.0.6 license, a permissive BSD-style license; it is a build-only dependency and ships in no binary. It is added to `[licenses] allow` in `deny.toml` with a comment saying where it comes from. Rejected: an exception for that one crate (the allow list is the project's single place for reviewed licenses).
- **No requirement changes** (`skip_specs: true`): the observable behavior of voice control is the contract, and the existing scenarios prove it.

## Test levels

| What | Level |
|---|---|
| `cargo deny` passes with no advisory ignore | `just lint` (lint stage) |
| Every voice control scenario, the `@voice_samples` scenarios and `voice_samples` tests | E2E `.feature` (`e2e-smoke-voice`, `e2e-regression`) and adapter integration, in `full-check` |
| Outages and the bundled engine | E2E (`e2e-resilience`, `e2e-bundle`) in `full-check` |
| Start cue timings | E2E (`e2e-cues`) in `full-check` |

## Risks / Trade-offs

- The 2.x native library may score keywords differently → caught by the voice control scenarios and the user's recordings in `full-check`; a regression stops the change.
- `aes` held below the newest patch until the toolchain moves to 1.89; harmless, it is build-only.
