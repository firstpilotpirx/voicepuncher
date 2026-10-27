## Why

`cargo deny` (part of `just lint`) carries 7 RUSTSEC ignores in `deny.toml`. Four of them (RUSTSEC-2026-0391, -0398, -0403, -0410) were added during `log-voice-command-history` because new zstd advisories broke `full-check` lint on `main` as well; three are older. All seven come from one build-only chain: `sherpa-onnx` 1.13.8 → `sherpa-onnx-sys` → `zip` 2.4 → `zstd` 0.13, which extracts sherpa-onnx's own pinned archive at build time. zstd 0.13 has no fixed release, so the ignores can only go away by moving off that chain. `sherpa-onnx-sys` 2.0.1 depends on `zip` ^9, which no longer pulls the vulnerable zstd. The user wants the dependency upgraded so the vulnerabilities disappear instead of being ignored.

## What Changes

- The command spotting service builds against `sherpa-onnx` 2.0.1 (and `sherpa-onnx-sys` 2.0.1), pinned exactly, with `Cargo.lock` updated.
- Any API change of the 2.x crate is adapted in `command-spotting/` (keyword spotter, online stream, results with keyword, tokens and timestamps).
- The 7 RUSTSEC ignores tied to the old chain are removed from `deny.toml`; `cargo deny` passes without them.
- Voice command recognition is shown not to regress: same commands spotted, same placements, same start latency.

Current stage: PoC. Target stage: PoC.
Scope: the sherpa-onnx dependency of `command-spotting/`, its adaptation, the `deny.toml` ignores, and the accuracy check of voice control.
Acceptance bar: `cargo deny` passes with none of the 7 ignores; `just full-check` passes, including every voice control scenario (`e2e-smoke-voice`, `e2e-regression`), the `@voice_samples` scenarios and `voice_samples` integration tests on the user's recordings, `e2e-resilience` and `e2e-bundle` (the bundled engine runs on its own); the command history and the start cue timings stay within their current bounds.
Out of scope: changing keyword models, phrases, thresholds or the command spotting protocol; other dependency upgrades.

## Capabilities

### Modified Capabilities

- `voice-control`: no requirement changes; the engine underneath is upgraded and must keep every existing scenario passing.

## Impact

- `command-spotting/Cargo.toml`, `Cargo.lock`, the engine code under `command-spotting/src/engine/`.
- `deny.toml`.
- The bundle stage, if the 2.x crate ships or links its native libraries differently.

## Open points

- Whether 2.0.1 changes the native library layout or Metal/CPU provider handling, which would touch the bundle and `e2e-bundle`; proposed: check in design and adapt if needed.
- Whether the 2.x API renames keyword spotter types or result fields; proposed: adapt in place, no protocol change.
- The three older RUSTSEC ignores: proposed to remove them only if `cargo deny` confirms they come from the same chain; any that remain for another reason stay with their reason.
- Release: proposed PATCH, since the shipped binary changes though behavior does not.

## Release

SemVer: PATCH — the shipped command spotting engine changes, no behavior change intended.
Target version: 0.39.1.
