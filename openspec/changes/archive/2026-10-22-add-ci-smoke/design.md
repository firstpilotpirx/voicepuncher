## Context

Every check runs on the owner's Mac: `just fmt` and `just lint` go through the agent-kit machine lock (`~/.claude/bin/with-machine-lock`, a binary of the private agent-kit repository), and the `test` stage's `_test-class-locks` self-test drives that lock directly. A GitHub runner has neither, and agent-kit is private, so CI cannot install it.

## Goals / Non-Goals

**Goals:** one hosted workflow that runs the same recipes the developer runs — `just fmt`, `just lint`, `just test` — on every PR and on `main`.

**Non-Goals:** E2E, models, services, releases, accuracy in CI; replacing the landing queue's `full-check`.

## Decisions

- **`VOICEPUNCHER_NO_MACHINE_LOCK=1` instead of a fake lock.** With it, `machine_lock` evaluates to `/usr/bin/env`, so `{{locked}} _fmt` becomes `env <ports> /usr/bin/env just … _fmt` — the private recipe body, unchanged. `_test` skips `_test-class-locks` (it tests the lock itself) and prints `⊘ machine lock self-test skipped (VOICEPUNCHER_NO_MACHINE_LOCK=1)`; every other self-test runs. Alternatives: checking out agent-kit with a deploy token (needs an owner secret, couples CI to a private repo); calling `just _fmt` directly (CI would then not run the public recipes). The variable is a `VOICEPUNCHER_*` one, so it changes stage keys: a CI pass never stands in for a local one.
- **Runner `macos-15`** (Apple Silicon; the proposal's `macos-14` is older and gains nothing). `taskpolicy` and the toolchain need macOS.
- **Triggers:** `pull_request` (drafts included, so the batch sessions' draft PRs get the floor too) and `push` to `main`; `concurrency` per ref with `cancel-in-progress`, so a pushed fix-up cancels the stale run and saves runner minutes.
- **Tools:** `just` and `cargo-nextest` from `taiki-e/install-action` at the versions the project pins (nextest 0.9.146; just 1.59.0, the local one); shellcheck from Homebrew; the rest from `just tools`. `~/.cargo/bin`, `~/perl5` and the npm global prefix are cached under a key hashed from the `justfile`'s `*_version :=` lines, so `just tools` runs only when a pin moves. `Swatinem/rust-cache` caches `target/` and the registry by lockfile, also after a failed run; the analyzers' cache is saved right after their install.
- **Nextest profile `ci`** (`.config/nextest.toml`, `NEXTEST_PROFILE=ci`): its default filter leaves out the two tests that need the owner's Mac — the overlay drawing test (a running window server) and the check that synthesized speech is over 30 s (the owner's `say` voices; the runner's are faster). It inherits every limit and override of the default profile.
- **An empty model variable means no model.** The `justfile` exports a model path it did not find as `""`; the command spotting model tests (`engine_it`, `placed_commands_it`, `server_it`) read only an unset variable as absent, so on a machine without models they failed loading `""` instead of skipping. `_test` now unsets every empty `VOICEPUNCHER_*` variable before the tests run, so they skip — the first run of this workflow found it. (Fixing the three test files instead would grow units listed in `size_baseline.toml`.)
- **Every check step runs after a failed one** (`if: success() || failure()`), so a PR sees its formatting, lint and test failures in one run.
- **`fetch-depth: 0`**, because `just lint` runs semver-checks against `origin/main`.
- Actions are pinned to exact release tags (`exact-versions.md`).

## Test levels

| Scenario | Level |
|---|---|
| A formatting error turns the check red | the hosted workflow itself: a commit with a misformatted line pushed to this PR shows the `fmt` step red, then is reverted |
| A failing unit test turns the check red | the hosted workflow: `just test` fails on any failed nextest test (exit status), proven by the same red commit with a failing unit test |
| A clean pull request turns the check green | the hosted workflow on this PR's final commit, with the run time read from `gh run view` |
| Model tests skip on a machine without models | the hosted workflow, whose runner has no models (its first run failed 49 such tests on `""`) |
| Without the machine lock the recipes run directly | justfile self-test in `_test-lock-phases` (`just test`), and `VOICEPUNCHER_NO_MACHINE_LOCK=1 just fmt` by hand |

## Risks / Trade-offs

- [macOS runner minutes cost 10× Linux minutes on a private repository] → cancel superseded runs; the owner can restrict triggers later.
- [Homebrew's shellcheck may differ from the pinned 0.11.0] → `_require-tools` fails naming the version; then pin it through a release download.
- [A cold cache: installing every analyzer from source takes ~15 min] → only on a pin change; the 10-minute bar applies to warm caches.

## Migration Plan

Merge, then the owner adds the `ci` check as required in the branch protection of `main` (README § Release checklist).
