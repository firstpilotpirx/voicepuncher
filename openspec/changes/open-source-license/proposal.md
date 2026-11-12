## Why

The owner publishes VoicePuncher as public open source. The repository carries no license, so nobody may legally use, copy or change the code, and the disk image ships no license of the application itself. The owner chose the Rust community's form, the one every public project of the owner uses: dual `MIT OR Apache-2.0`, with both full texts in the repository. The `.gitignore` also lacks the agent worktree folder `.claude/worktrees/`, so a stray `git add -A` could commit another session's checkout.

## What Changes

- The root holds `LICENSE-MIT` (2026, firstpilotpirx) and the unchanged official `LICENSE-APACHE`.
- Every workspace crate states `license = "MIT OR Apache-2.0"` (once, in `[workspace.package]`).
- The README ends with a `## License` section with the standard contribution clause.
- The bundle build copies both texts unchanged into `voicepuncher.app/Contents/Resources`, so every disk image carries the application's own license beside the engines' licenses.
- `.gitignore` is grouped and commented and ignores `.claude/worktrees/`.

Current stage: PoC. Target stage: PoC.
Scope: `LICENSE-MIT`, `LICENSE-APACHE`, the Cargo manifests, `README.md`, `.gitignore`, the `bundle` recipe, the `app-distribution` requirement on licenses and its E2E scenario.
Acceptance bar: the `@bundle` scenario "The bundle holds the application's own license" passes; `just check` passes.
Out of scope: license headers in source files; a contributor license agreement; changing the licenses of the bundled engines.
Interface: none.

## Capabilities

### Modified Capabilities

- `app-distribution`: the bundle also carries the application's own license texts.

## Impact

- `LICENSE-MIT`, `LICENSE-APACHE`, `Cargo.toml` and every member's `Cargo.toml`, `README.md`, `.gitignore`, `justfile` (`bundle`), `e2e-test/tests/features/app_distribution.feature`, `e2e-test/tests/steps/app_distribution/`.

## Release

SemVer: MINOR — a new license and a new bundle content.
Target version: 0.49.0.
