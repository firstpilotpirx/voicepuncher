## Why

The `justfile` has become a program (audit of 2026-10-22): 2 390 lines, 41 bash recipes and 4 Perl recipes totalling 524 lines of Perl — `_stage-keys` (`justfile:442–602`) hashes every tracked file with `git hash-object`, parses `Cargo.lock`, fingerprints the toolchain and the service binaries; `_stage-journal-append` (`:867`), `stage-times` (`:167`) and `_engines` (`:1765`, relocating dylibs and verifying sha256) are programs in their own right; a 70-row `stage_inputs` table (`:372–441`) maps globs to stages; 190 lines of the file test the file itself (`_test-stage-keys`, `_test-lock-phases`, `_test-class-locks`); 25 `|| true` hide helper failures; the `_stages → _lock-phases → _class-lock → _model-stages → _<stage>` chain cannot be followed by a newcomer. The lock itself is a tool outside the repository, `~/.claude/bin/with-machine-lock` (`justfile:100`), with no guard recipe, so a fresh clone cannot run `just check`. Over 30 days the verification machinery was the project's costliest and least reliable part: 657 runs, 266 failed, 35 interrupted; 22 of 94 archived changes were pure tooling, 9 flake fixes, 27 named check/test/lock/stage.

## What Changes

- The stage runner — stage keys and pass reuse, the journal and `stage-times`, the lock classes and phases, the test-port sets, the engines staging — moves into one Rust binary in the workspace (`dev-tools/` or `e2e-test/src/bin/stages.rs`) with unit tests in Rust; the `justfile` keeps thin recipes that call it, under the kit's budgets for a recipe body.
- `with-machine-lock` is either vendored (a copy in the repository with its version) or guarded by `_require-machine-lock` that names the install command, as `_require-nextest` does; the lock protocol it implements is documented in `AGENTS.md`.
- No `|| true` without a comment stating what failure is tolerated and why; helper failures surface.
- The `justfile` self-tests move to the binary's tests; the stage table, journal line format and pass records stay byte-compatible so `just stage-times` history survives.

Current stage: PoC. Target stage: PoC.
Scope: the `justfile` recipes named, the new binary, the guard, `AGENTS.md` § How to verify a change.
Acceptance bar: `just check` and `just full-check` produce the same stage table and journal line as before (compared on one commit); the `justfile` under 800 lines with no Perl; a fresh clone without the lock tool gets the install command from `just check`; the binary's tests cover keys, reuse, lock phases and the journal.
Out of scope: changing which stages run or their order; CI (`add-ci-smoke`).

## Capabilities

### Modified Capabilities

- `change-workflow`: the check machinery is a tested program; the lock tool is guarded.

## Impact

- `justfile`, a new workspace member (`dev-tools/`) or `e2e-test/src/bin/`, `Cargo.toml` (members), `AGENTS.md`, `.config/nextest.toml` (the new binary's tests).

## Open points

- Whether the runner becomes a `kit` command (every project with stages would use it) or stays in this repository; proposed: this repository first, since the stage set is project-specific; `kit-placement.md` decides later.

## Release

SemVer: no release — tooling only.
Target version: unchanged.
