## Context

The project has ≈ 1 440 tests and no measure of their strength: nothing records coverage or whether a test would catch a regression. `adopt-static-analyzers` (archived) gave the pattern this change follows: a pinned tool, a subcommand of `voicepuncher-analyzers` that runs it and compares with a baseline in `.config/analyzers/`, a stage of `_stages` with its key and journal row.

Measured on this machine on 2026-10-22 (worktree, other sessions' checks running beside):

| What | Result |
|---|---|
| `cargo llvm-cov nextest` over the workspace without the E2E harness (1 440 tests) | 2 min; lines: app 85.5 %, copy-pool 98.5 %, command-spotting 52.2 %, analyzers 73.0 %; regions 1–2 points lower |
| `cargo mutants -p voicepuncher-copy-pool` | 151 mutants in 2 min: 94 caught, 5 timeouts, 16 missed, 36 unviable → 86 % caught |
| `cargo mutants -f 'app/src/core/**'` | 1 018 mutants; a shard of 51 took 17 min: a mutant rebuilds the `app` lib test binary (median 50 s under load) + 9 s of unit tests → the whole core takes 2–4 h |
| `cargo +nightly miri nextest run -p voicepuncher --lib -E 'test(/^core::/)'` | builds and runs; 331 tests in 18 min; 10 tests over the 2 min limit (the pipeline's voice control tests sleep real time, three normalization checks are heavy), one fails on `Instant` under isolation |
| miri over copy-pool | its tests open sockets, which miri cannot interpret |
| `RUSTFLAGS=-Zsanitizer=address cargo +nightly nextest run -p voicepuncher --target aarch64-apple-darwin` | builds in 100 s; the sanitized tests pass |
| branch coverage | needs nightly (`-Z coverage-options=branch`); regions are the stable alternative |

## Goals / Non-Goals

**Goals:** `full-check` reports coverage and the mutation score and fails under floors kept in a baseline with their commit; the survivors are listed; miri and the address sanitizer run on demand on a pinned nightly; the first survivors of `core/domain` are killed by new unit tests.

**Non-Goals:** coverage of the AppKit adapters (manual run); restructuring tests; running the whole core's mutants in every check (the scope call of 2026-10-22 chose the branch's changed lines); CI changes (its `just tools` and cache stay as they are).

## Decisions

### Two analyzers more: `coverage` and `mutants`

`voicepuncher-analyzers coverage` runs `cargo llvm-cov nextest --workspace --exclude voicepuncher-e2e-test --json --summary-only --output-path <strength>/coverage.json` (the test selection of `_test`: no E2E harness, no doctests), sums the lines and regions of every source file per crate (the first path segment; files under `tests/` do not count) and prints one row per crate. `voicepuncher-analyzers mutants` runs `cargo mutants` once per entry of `[[mutants.crates]]` in `.config/analyzers.toml` (`name`, `package`, `files` glob, `test_args`, `whole`): copy-pool whole, app-core (`app/src/core/**/*.rs`, `-- --lib`, so only the unit tests run per mutant) in the branch's diff when the stage passes `--diff <patch file>` (`cargo mutants --in-diff`); `--output <strength>/mutants-<name>`, `--test-tool nextest`, `--jobs 4`. It reads `mutants.out/outcomes.json`: caught = caught + timeout, tried = caught + missed (unviable mutants do not count), score = caught / tried; prints per crate the tried, caught and survived counts and the score, then every survivor as `<file>:<line>: <change>` (the lines of `missed.txt`). A crate with no mutant in the diff prints `no mutants in the changed lines` and passes. `<strength>` is `~/.cache/voicepuncher/strength/` (`VOICEPUNCHER_STRENGTH_DIR`), beside the stage journal.

*Alternative:* running the tools straight from the recipes and parsing in Perl, as `_stage-journal-append` does. Rejected: the floors need unit-tested parsing and comparison, which `analyzers/` already has the shape for.

### Floors in one baseline file, ratcheted upwards

`.config/analyzers/strength.toml` holds `commit = "<sha>"`, `[coverage.<crate>] lines = …, regions = …` and `[mutants] <name> = <score>`; every value a percentage with one decimal. A new module `analyzers/src/floors.rs` compares: a value under its floor is a breach (`✗ under: coverage app lines (floor 85.0, now 83.9)`), a value at or above it passes, a crate without an entry passes with `● no floor for <crate>: just strength-baseline`. `--write-baseline` writes every measured value minus 0.5 points (rounded to one decimal, never below 0) with the current commit: the half point absorbs the jitter of tests that skip on a machine without models or voices. `baseline.rs` (counts that only shrink) stays as it is.

### `just strength-baseline` rewrites the floors from whole runs

The recipe runs `coverage --write-baseline` and `mutants --whole --write-baseline`: copy-pool whole (2 min) and the whole app core (1 018 mutants, 2–4 h, with `taskpolicy -c utility` so it yields to the model stages of other runs), in the background by the session that calls it. It is the only run that measures the core's full score; `full-check` compares the branch's mutants with that floor. A rewrite that lowers a floor is reviewed like code (`just analyzers-baseline` stays for the three cheap analyzers; it does not run the two slow ones).

### `full-check` gets the trailing unlocked stages `coverage` and `mutants`

`_stages` gets `trailing_stages := "coverage mutants"`: a stage named there is unlocked (no service, port or GPU) and runs after the model phases returned their locks, so the lock is held no longer than today and the voice control stages still fail first; a stage not named there keeps today's rule (e2e* and refine-eval are model stages, every other stage is unlocked and runs first). `full-check` lists them last: `… build-e2e <model stages> coverage mutants`; `check` keeps its ≈ 8.5 min and does not run them. The `mutants` stage writes `git diff --merge-base origin/main HEAD -- app/src/core` to a patch file and passes it as `--diff`; a stage key over the tree is enough: the same tree against a base that moved has a diff no larger than before. Both stages are `code` stages of `stage_inputs` (no line names them, so every file feeds them). `_test-lock-phases` learns the new lists; the self-test that pins the stage sets names them.

*Alternative:* in the unlocked list before `build`. Rejected: 4–6 min more before the lock is requested, and a free lock would wait on CPU work that needs no lock.

### The journal carries the strength rows

Each analyzer prints machine rows `strength coverage <crate> lines=<v> regions=<v>` and `strength mutants <name> tried=<n> caught=<n> score=<v>` to `$VOICEPUNCHER_STAGES_REPORT` when it is set; `_stage-journal-append` collects them into `strength: { coverage: { <crate>: { lines, regions } }, mutants: { <name>: { tried, caught, score } } }`. The JSON reports stay in `<strength>/` (overwritten by every run; the journal holds the numbers).

### Per-mutant builds use a profile without debug info

`[profile.mutants]` in `Cargo.toml` inherits `test` with `debug = false`, and the mutants analyzer passes `--profile mutants`: a mutant's rebuild of the `app` lib test binary spends most of its time on debug info nobody reads. Measured in apply; the design keeps it even when the gain is small, as it costs nothing.

### `just mutants <path>` runs one path

`cargo mutants -f <path> --test-tool nextest --output <strength>/mutants-on-demand [-- --lib when the path is under app/]`, then `voicepuncher-analyzers mutants --report <strength>/mutants-on-demand` prints the stage's table and survivors without comparing or writing floors.

### `just sanitize`: miri and the address sanitizer on a pinned nightly

`nightly_date := "2026-10-15"` in the `justfile`; `just strength-tools` installs `nightly-<date>` with `miri`, `rust-src` and `llvm-tools` (`rustup toolchain install … --profile minimal -c …`), `llvm-tools` on the project's `1.88` toolchain, `cargo-llvm-cov` 0.9.1 and `cargo-mutants` 27.1.0 (`cargo +stable install --locked`); `_require-strength-tools` checks the four and names `just strength-tools`, installing nothing; `_require-tools`, `just tools` and CI stay unchanged. `rust-toolchain.toml` keeps `1.88`: the nightly is named only by `cargo +nightly-<date>` inside `sanitize`, so every other recipe and the hosted check build as before.

- miri: `MIRIFLAGS=-Zmiri-disable-isolation cargo +nightly-<date> miri nextest run -p voicepuncher --lib --profile miri -E 'test(/^core::/) & not test(/^core::application::pipeline::/)'`. `[profile.miri]` in `.config/nextest.toml` sets `slow-timeout = { period = "60s", terminate-after = 10 }` (10 min: miri runs ≈ 100× slower than native and the heaviest core test took under 2 min of native-equivalent work). The pipeline's voice control tests are excluded by the filter: they sleep real time on threads and need more than the limit without finding anything (no `unsafe` there); copy-pool is excluded: its tests open sockets, which miri cannot interpret. The recipe says both in a comment.
- address sanitizer: `RUSTFLAGS=-Zsanitizer=address cargo +nightly-<date> nextest run -p voicepuncher --target aarch64-apple-darwin -E 'test(/main_queue/) | binary(textpasting_macos_it) | binary(focus_macos_it)'`, a target directory of its own under `target/sanitize/` (`CARGO_TARGET_DIR`) so the sanitized build never mixes with the stable one. The pasteboard tests keep their own file lock, so the recipe is unlocked.

The recipe takes ≈ 25 min and runs on demand only (AGENTS.md names when: a change to `unsafe` code, a native adapter or the core's data structures).

### Tests against the survivors of `core/domain`

`just strength-baseline` runs once in apply (background); its `mutants-app-core` survivors under `app/src/core/domain/` are the first batch: every survivor that a unit test at the function's contract kills gets one (Red: the test fails on the mutant — `cargo mutants -f <file> --test-tool nextest -- --lib` reports it caught; Green: the production code is unchanged, the test passes on it). Survivors that no contract distinguishes (a `log` call replaced, a `Default` body) stay listed in the run's report, not in a baseline: the floor is the score, the survivor list is the to-do. The floors written after that batch are the baseline committed.

### AGENTS.md

§ How to verify a change gets a paragraph **Test strength** (the two stages, the floors, `just strength-baseline`, `just mutants <path>`, `just sanitize` and when to run it, `just strength-tools`) and the `full-check` row names the two stages and their time.

## Test levels

| Scenario | Level |
|---|---|
| A crate that lost coverage fails the full check | unit (`floors` compare) and unit (`coverage` parse of a saved llvm-cov JSON) |
| A crate that gained coverage keeps passing | unit (`floors` compare) |
| The coverage report reaches the journal | unit (`_stage-journal-append` self-test in the `justfile`, a report with strength rows) |
| A crate whose tests let more mutants through fails the full check | unit (`floors` compare) and unit (`mutants` parse of a saved `outcomes.json`) |
| Core mutants of unchanged lines do not run in the full check | unit (`mutants` builds the `cargo mutants` command with `--in-diff` for a diff crate and without for a whole one) |
| Survivors are listed for the next tests | unit (`mutants` report from a saved `missed.txt`) |
| Mutation testing of one path on demand | `just mutants copy-pool/src/pool/state.rs` by hand in apply (its output noted in tasks) |
| The floors are rewritten from a whole run | unit (`floors` write: value − 0.5, commit) and the `just strength-baseline` run of apply |
| Undefined behavior in a core test fails the recipe | `just sanitize` by hand in apply with a temporary test that reads uninitialized memory (removed before commit; the output noted in tasks) |
| A memory error in a native adapter fails the recipe | the same run with a temporary out-of-bounds read in a sanitized test |
| The project still builds on stable | `just test` (every task) |

## Risks / Trade-offs

- [The instrumented build doubles the target directory (`target/llvm-cov-target`), ≈ 4 GB more per worktree] → documented in AGENTS.md; `kit` worktree cleanup removes it with the worktree.
- [`full-check` grows by ≈ 4 min (coverage 2 min, copy-pool mutants 2 min) plus the branch's core mutants (≈ 1 min each, four at once)] → trailing stages, unlocked; a branch that changes many core lines pays more, measured in the stage journal.
- [A regression in unchanged core lines passes `full-check`] → caught only by `just strength-baseline`; the floor compares the branch's mutants, so a weakly tested change still fails.
- [The pinned nightly may be withdrawn from the mirrors] → `just strength-tools` fails naming the date; moving the date is a chore change.
- [Coverage jitters with skipped tests (no models, no voices)] → floors sit 0.5 points under the measured value; a machine without models measures lower and must not rewrite the baseline.

## Dependencies

`adopt-static-analyzers` (archived on `main`): the `analyzers/` crate, `just tools` and the baseline pattern.
