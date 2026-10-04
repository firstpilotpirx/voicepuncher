# voicepuncher — AI agent instructions

## What this project is

Push-to-talk voice-to-text CLI for macOS Apple Silicon. Single Rust crate at `app/`. Fully offline: a local whisper.cpp `whisper-server` for transcription and a local llama.cpp `llama-server` (Qwen2.5) for text cleanup, both reached over loopback HTTP, started with `just start-model-servers` and running on Metal. No internet at runtime; loopback connections only.

**Product lifecycle stage:** PoC (Proof of Concept). The core workflow is demonstrated but the codebase is not yet hardened for daily use.

## Repository layout

```
voicepuncher/
  app/          ← Rust CLI crate (hexagonal architecture)
  e2e-test/     ← Rust crate: accuracy benchmark + smoke E2E
  docs/         ← Architecture documentation + ubiquitous language
  Cargo.toml    ← Cargo workspace (members: app, e2e-test)
  justfile      ← Task runner: all checks, E2E, benchmarks (`just` lists recipes)
```

## Workflow — OpenSpec

Every feature, bug fix and behavior change runs through all stages below, in order — do not skip or reorder a stage. Changes without behavior change (typos, formatting, renames, docs, dependency bumps, tooling) skip stages 1–3 and 6–7 and leave the version unchanged; they still go through a branch, the checks of § How to verify a change, a PR and a merge. When unsure, use OpenSpec.

| # | Stage | How | Done when |
|---|---|---|---|
| 1 | Explore (optional) | `/opsx:explore` | the idea is clear; no files written |
| 2 | Propose | `/opsx:propose "<idea>"` creates every artifact in one go. Artifact by artifact instead: `/opsx:new`, then `/opsx:continue` once per artifact (proposal → specs → design → tasks), or `/opsx:ff` for the rest | `openspec validate <name>` passes and `openspec status --change <name>` shows every artifact done |
| 3 | Approval | show the proposal to the user | the user approved it — **stop and wait**; no code before this |
| 4 | Branch | `git fetch origin && git switch -c <type>/<name> origin/main` (`feat`, `fix`, `chore`) | |
| 5 | Apply | `/opsx:apply`, test-first (§ TDD) | every task in `tasks.md` is checked; `just test` after every edit; one Conventional Commit after every task group, with `just check` when the group changed a build input |
| 6 | Verify | `/opsx:verify`, then the `spec-test-reviewer` subagent on the change (§ Gherkin) | no CRITICAL issues and the reviewer reports `RESULT: PASS`; fix every PARTIAL, CONTRADICTS, UNTRACED, ORPHAN and CONFLICT entry and rerun it |
| 7 | Archive | `/opsx:archive`, then commit | specs merged into `openspec/specs/`, change moved to `openspec/changes/archive/` |
| 8 | Full check | `just full-check`, only when the branch changed a build input | passes on the final commit |
| 9 | Pull request | `git push -u origin HEAD`, `gh pr create` | the PR body lists the checks that ran |
| 10 | Manual run | the user, only when hotkey, recording, overlay, paste or permissions changed (§ How to verify a change) | the user pasted a healthy log |
| 11 | Merge | `gh pr merge --squash --delete-branch`, then update the main checkout (below) | the change is on `main` and in the main checkout; if `main` moved after stage 8, merge it in and rerun stage 8 first |
| 12 | Release | `just release` on `origin/main` (§ Release), only when stage 5 bumped the version | the GitHub Release `vX.Y.Z` exists with downloadable artifacts |

- **Never leave work uncommitted.** After every finished piece of work — a task group, a fix, a doc or config edit — commit and push to the current branch without asking. Before ending a turn, `git status` is clean and the branch is pushed. On `main`, first create a `<type>/<slug>` branch from `origin/main`. This beats any "commit only when asked" rule.
- Stages 4–12 are pre-authorized: commit, push, open the PR, squash-merge and release without asking. Stop only at stage 3, stage 10, a failed check you cannot fix, or a scope question the specs do not settle.
- **One session per change.** After stage 11 (or 12) end the session and start the next change in a new one with a handoff of ≤ 10 lines (branch, PR, what is left). Do not resume a session idle for more than 1 h; start a new one with a handoff. A long session re-reads its whole history on every call.
- **Update the main checkout after every merge.** The user runs the app from the main checkout (`cargo watch`), not from a worktree, and a merge on GitHub does not change it. Right after `gh pr merge` run `git -C "$(git worktree list | head -1 | cut -d' ' -f1)" pull --ff-only origin main` when that checkout is on `main` and clean; otherwise tell the user it is behind. Say in the report that it is updated.
- A change is not done until stage 11 (or 12, when the version moved) is done. Report which stages ran.

Skills: `/create-change` files a change for later (stages 2–3 only, landed on `main`); `/proceed-change <name> [--stop <gates>]` runs stages 1–12 for one change with gates (`.claude/skills/`).

CLI: `openspec list` (changes), `openspec list --specs`, `openspec status --change <name>`, `openspec view` (dashboard), `openspec validate`.

## TDD — Red, Green, Refactor

Every behavior change in stage 5 is test-first (`tdd` skill):

1. **Red** — write the test for the new behavior and run it; it fails because the behavior is missing.
2. **Green** — write the smallest code that makes it pass; `just test` is green.
3. **Refactor** — clean up with every test green.

- **Bug fix:** first write a test that reproduces the bug and watch it fail, before touching the code under test. Commit that failing test on its own (`test: reproduce <bug>` — the commit message states it is intentionally red), then fix it in the next commit. The red commit proves the bug existed; the green one proves the fix.
- A test that passes on its first run proves nothing: change it until it fails for the expected reason, then go green.
- `tasks.md` lists the Red task before the Green task for each behavior.
- At slice, component and E2E level, the Red test is the `.feature` scenario written from the spec (§ Gherkin).

## Gherkin — business tests mirror the specs

Business behavior is tested in Gherkin `.feature` files executed by cucumber-rs, so each test reads like its OpenSpec scenario.

| Test | Form |
|---|---|
| Unit (`#[cfg(test)]`) | plain Rust; never Gherkin |
| Adapter integration (`app/tests/<adapter>_it.rs`), architecture | plain Rust |
| Slice / component (`app/tests/features/{slice,component}/<capability>.feature`) | Gherkin, steps in `app/tests/steps/<capability>.rs`, harness `app/tests/cucumber_integration.rs` |
| E2E (`e2e-test/tests/features/<capability>.feature`) | Gherkin, steps in `e2e-test/tests/steps/<capability>.rs`, harness `e2e-test/tests/cucumber_e2e.rs` |

- One `.feature` per OpenSpec capability, named after it with `-` and `/` → `_` (`file-transcription` → `file_transcription.feature`); `Feature:` is the capability id, one `Rule:` per `### Requirement:`.
- Write each `.feature` scenario from a spec scenario: same precondition, same user action, every outcome asserted. Wording, step split and level of detail may differ; meaning may not. Reuse the spec's scenario name unless a clearer one helps. This beats `~/.claude/rules-custom/gherkin.md` § Traceability, which demands verbatim copies.
- Never add a `.feature` scenario that no spec scenario covers; when the test needs new wording, change the spec first.
- Tag the feature with its capability (`@file_transcription`) and level (`@component`, `@slice`, `@e2e`); `@wip` scenarios are not run; `@smoke` / `@accuracy` / `@seams` pick the E2E subsets `just e2e-smoke` / `just e2e-quick` / `just e2e-seams` run.
- Steps hold no assertions in `Given`/`When`; types, flags, env vars and file paths live in step code, never in the `.feature` text.
- The `spec-test-reviewer` subagent (`.claude/agents/spec-test-reviewer.md`) compares meaning: it reads the change's OpenSpec documents (proposal, specs, design, tasks) and every test in scope — `.feature` scenarios with step code and plain Rust tests — and reports tests that miss or contradict a document, orphan `.feature` scenarios and conflicts between documents. Run it at stage 6 and again before stage 7 when tests or OpenSpec documents changed after it.

## Release

- **Version:** `version` in `app/Cargo.toml`, SemVer. The proposal states the bump; the last task group of `tasks.md` bumps it. Pre-1.0: breaking change or new capability → MINOR, bug fix → PATCH. Changes without behavior change keep the version and get no release.
- **`just release`** runs after the merge, on a clean checkout of `origin/main` (in a worktree: `git fetch origin && git switch --detach origin/main`). It refuses to run otherwise, and when the tag already exists. It runs `just test`, builds the release binary, tags `vX.Y.Z`, pushes the tag and creates the GitHub Release with `voicepuncher-vX.Y.Z-aarch64-apple-darwin.tar.gz` (binary + README) and its `.sha256`; release notes are generated from the merged PRs.

## How to verify a change

All checks are `just` recipes (see `justfile`; no other scripts). A change is "verified" only when every level that applies has run and passed — say which levels ran and which did not.

| When | Command | What it runs |
|---|---|---|
| After every edit | `just test` (~1 min) | unit + architecture + integration tests |
| After every task, before every push | `just check` (~6 min) | fmt, clippy `-D warnings`, `test`, release build, E2E smoke (with the chunk seam sweep against the real service), WER accuracy on 10 samples |
| Before merging a branch, before a release | `just full-check` (~35 min) | `cargo clean` + everything in `check`, plus the chunk seam matrix (`e2e-seams`), with WER accuracy on the whole corpus |
| Before merging a change to hotkey, recording, overlay, paste, permissions | manual run by the user | `README.md` § Release checklist |

Build inputs are `app/`, `e2e-test/`, `Cargo.toml`, `Cargo.lock` and `justfile`. A branch that changes none of them (docs, `AGENTS.md`, `openspec/`, `.claude/`) needs `just test` only; never `just check` or `just full-check` for it. Do not merge a branch that changed a build input until `just full-check` has passed on its final commit; say in the PR which checks ran.

`just bench` is a model comparison, not a gate — only when a model, decoding params or the prompt changed.

- **Models and corpus** are auto-discovered by the `justfile` (largest model in `~/.voicepuncher/` or `~/.voicepuncher/models/`; corpus in `e2e-test/corpus/`). The corpus is gitignored, so in a worktree set `VOICEPUNCHER_BENCH_CORPUS` to the main checkout's `e2e-test/corpus`.
- **Local services:** the E2E recipes start `whisper-server` and `llama-server` (`just start-model-servers`) and stop what they started afterwards; `just test` needs no service. The services are shared across checkouts on ports 8081 and 8080, so do not run E2E in two checkouts at once. E2E business scenarios synthesize speech with macOS `say`.
- **`full-check` runs `cargo clean`** and deletes `target/release/voicepuncher`, which the user may be running. In the main checkout, ask first; in a worktree it only cleans that worktree's `target/`.
- **The manual run cannot happen in an agent sandbox**: it lacks macOS TCC permissions (`permission denied for accessibility`). Ask the user to run `cargo run --release -p voicepuncher` (with and without `--no-llm` when refinement changed) and paste the log; `just start-model-servers` must run first. A healthy cycle logs `pipeline recv Pressed` → `chunk 1 | … transcribed` → `pasting…` → `stop→paste: … total=…ms`.

## Test layout (inside `app/`)

| Layer | Location | Naming |
|---|---|---|
| Unit | `#[cfg(test)]` blocks inside each `*.rs` | `mod tests { fn should_*` } |
| Integration | `app/tests/*_it.rs` | `<adapter>_it.rs` |
| Slice / component | `app/tests/features/{slice,component}/*.feature` | `<capability>.feature` (§ Gherkin) |
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
- No internet at runtime; loopback connections only — all inference is local, in separate local services (speech recognition, text refinement).
- Three macOS TCC permissions required: Microphone, Accessibility, Input Monitoring.
