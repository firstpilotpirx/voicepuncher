# Tasks

No spec scenario (`skip_specs: true`): each behavior is checked by running the recipes (design.md § Test levels). The Red item shows the failing check, the Green item makes it pass. In a worktree, set `VOICEPUNCHER_BENCH_CORPUS` to the main checkout's `e2e-test/corpus` for every check run.

## 1. Upgrade sherpa-onnx and drop the advisory ignores

- [x] 1.1 Red: remove the 7 `ignore` entries from `[advisories]` in `deny.toml`; `cargo deny check advisories` fails on the zip 2.4 and zstd 0.13 advisories.
- [x] 1.2 Green: `sherpa-onnx = "=2.0.1"` in `command-spotting/Cargo.toml`; `cargo update -p sherpa-onnx`, then `cargo update aes --precise 0.9.2` (design.md § Decisions); `cargo deny check advisories` passes.
- [x] 1.3 Red: `cargo deny check licenses` fails on `libbz2-rs-sys` (`bzip2-1.0.6`).
- [x] 1.4 Green: add `bzip2-1.0.6` to `[licenses] allow` in `deny.toml` with a comment naming its build-only source; `cargo deny check` passes.
- [x] 1.5 The release binary of the command spotting service links only system libraries (`otool -L`); `just static` and `just test` pass; commit `fix(deps): upgrade sherpa-onnx to 2.0.1 and drop the zip/zstd advisory ignores`.

## 2. Version

- [x] 2.1 Bump `version` in `app/Cargo.toml` to 0.39.1; `just test` passes; commit `chore: bump version to 0.39.1`.
- [x] 2.2 `just check` passes once (`kit verify`), then `just full-check` in the landing group, with every voice control scenario, the `@voice_samples` scenarios, `voice_samples` tests, `e2e-resilience`, `e2e-bundle` and `e2e-cues`.
- [x] 2.3 Manual checks by the user: none (no hotkey, recording, overlay, paste or permissions change).
