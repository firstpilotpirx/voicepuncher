# voicepuncher — AI agent instructions

## What this project is

Push-to-talk voice-to-text CLI for macOS Apple Silicon. Single Rust crate at `app/`. Fully offline: two local whisper.cpp speech recognition services — the one for dictation (`whisper-server`, large-v3-turbo q5_0, port 8081, what the app uses) and the one for commands (`whisper-server-commands`, small, port 8085, used only by the command spotting service to confirm utterances) — and a local llama.cpp `llama-server` (Qwen3-4B-Instruct-2507) for text cleanup, all reached over loopback HTTP and running on Metal, plus the command spotting service `voicepuncher-command-spotting` (crate `command-spotting/`, sherpa-onnx keyword spotting confirmed by the speech recognition service for commands) reached over a loopback WebSocket for voice control; `just start-model-servers` starts all four (or the named ones) and restarts a service that crashes, until `just stop-model-servers`; each speech recognition service runs as three `whisper-server` copies behind HAProxy on its port (`VOICEPUNCHER_STT_INSTANCES`, `VOICEPUNCHER_COMMAND_STT_INSTANCES`). No internet at runtime; loopback connections only.

**Product lifecycle stage:** PoC (Proof of Concept). The core workflow is demonstrated but the codebase is not yet hardened for daily use.

## Repository layout

```
voicepuncher/
  app/          ← Rust CLI crate (hexagonal architecture)
  command-spotting/ ← Rust binary: the command spotting service (voice control)
  e2e-test/     ← Rust crate: accuracy benchmark + smoke E2E
  docs/         ← Architecture documentation + ubiquitous language
  Cargo.toml    ← Cargo workspace (members: app, command-spotting, e2e-test)
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
| 5 | Apply | `/opsx:apply`, test-first (§ TDD) | every task in `tasks.md` is checked; `just test` after every edit; one Conventional Commit after every task group; `just check` once, after the last group, when the branch changed a build input |
| 6 | Verify | `/opsx:verify`; then the `spec-test-reviewer` subagent once, only for a change that is not small (`~/.claude/rules-custom/openspec.md`: more than one added or modified requirement, or more than 10 tasks) (§ Gherkin) | no CRITICAL issues; for a reviewed change, `RESULT: PASS` — fix every PARTIAL, CONTRADICTS, UNTRACED, ORPHAN and CONFLICT entry and rerun it only when the fix touched tests or OpenSpec documents |
| 7 | Archive | `/opsx:archive`, then commit | specs merged into `openspec/specs/`, change moved to `openspec/changes/archive/` |
| 8 | Full check | `just full-check`, only when the branch changed a build input | passes on the final commit |
| 9 | Pull request | `git push -u origin HEAD`, `gh pr create` | the PR body lists the checks that ran |
| 10 | Manual run | the user, only when hotkey, recording, overlay, paste or permissions changed (§ How to verify a change) | the user pasted a healthy log |
| 11 | Merge | `gh pr merge --squash --delete-branch`, then update the main checkout (below) | the change is on `main` and in the main checkout; if `main` moved after stage 8, merge it with `kit sync-base` and run the check its last line names before `kit land` (§ How to verify a change) |
| 12 | Release | `just release` on `origin/main` (§ Release), only when stage 5 bumped the version | the GitHub Release `vX.Y.Z` exists with downloadable artifacts |

- **Never leave work uncommitted** (`~/.claude/rules-custom/atomic-commits.md`): commit and push after every finished piece of work, without asking.
- Stages 4–12 are pre-authorized: commit, push, open the PR, squash-merge and release without asking.
- **Filed changes land on `main` at once.** `/create-change`, and any later edit to a change that implements nothing (notes, artifacts, a reworded proposal), ends with a PR squash-merged without asking, then the main checkout update below. `openspec list` on `main` is the backlog; never leave a filed change only on a branch. Stop only at stage 3, stage 10, a failed check you cannot fix, or a scope question the specs do not settle.
- **Update the main checkout after every merge.** The user runs the app from the main checkout (`cargo watch`), not from a worktree, and a merge on GitHub does not change it. Right after `gh pr merge` run `git -C "$(git worktree list | head -1 | cut -d' ' -f1)" pull --ff-only origin main` when that checkout is on `main` and clean; otherwise tell the user it is behind. Say in the report that it is updated.
- A change is not done until stage 11 (or 12, when the version moved) is done. Report which stages ran.

Skills: `/create-change` files a change for later (stages 2–3 only, squash-merged to `main` without asking); `/proceed-change <name>` runs stages 1–12 for one change, stopping only at the gates picked at start. Both skills, the `spec-test-reviewer` agent and the shared OpenSpec rules (generated `opsx:*` skills stay untouched, one session per change) come from agent-kit (`~/.claude/skills/`, `~/.claude/agents/`, `~/.claude/rules-custom/openspec.md`); this file supplies their project values.

**Several changes at once:** `/orchestrate` (agent-kit skill) starts each ready change in a session and worktree of its own running `/proceed-change`, and a change whose `openspec/changes/<name>/orchestration.yaml` lists `depends_on` only after those changes are archived on `main`; `autonomy` (`auto`, `review`, `explore`) sets how much the change needs the user, and `kit backlog` shows each change as ready, blocked, running, needs you or invalid. A change the user approved when it was filed runs with `autonomy: auto` (`gates=none`): no gate stops it, and its calendar time is its work time; `review` is for a change whose proposal the user has not read, and the audit of 2026-10-16 measured gates and questions at half the calendar time of a change. Limits of this project:

- E2E needs no extra care: the model stages of `check` and `full-check`, and E2E, build and benchmark recipes run on their own, wait for the machine-wide test lock, so parallel changes run them one at a time (the lock is the GPU's throughput: starting fewer changes at once does not shorten the batch; waiting must cost nothing — a check runs in the background and wakes the session when it ends, never in a polling loop); the builds and unit tests of `check` and `full-check`, `just test` and `just release` do not wait, so coding, builds and tests go on in every session at once.
- Two changes that bump `version` from the same value conflict in `app/Cargo.toml` when the second merges `origin/main`; that change takes the next version above `main` and corrects the target version in its proposal (`kit version-check`).
- Changes that touch the same code (chunk seams, the command spotting protocol, the same recipe) depend on each other even when no proposal says so: declare it in `orchestration.yaml` when filing the change; the orchestrator does not guess overlaps.

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
- Each `.feature` scenario matches a spec scenario by meaning, not wording (`~/.claude/rules-custom/gherkin.md` § Traceability).
- Tag the feature with its capability (`@file_transcription`) and level (`@component`, `@slice`, `@e2e`); `@wip` scenarios are not run; `@smoke` / `@accuracy` / `@seams` / `@resilience` pick the E2E subsets `just e2e-smoke` / `just e2e-quick` / `just e2e-seams` / `just e2e-resilience` run.
- Steps hold no assertions in `Given`/`When`; types, flags, env vars and file paths live in step code, never in the `.feature` text.
- The `spec-test-reviewer` subagent (`~/.claude/agents/spec-test-reviewer.md`) compares meaning: it reads the change's OpenSpec documents (proposal, specs, design, tasks) and every test in scope — `.feature` scenarios with step code and plain Rust tests — and reports tests that miss or contradict a document, orphan `.feature` scenarios and conflicts between documents. Run it once at stage 6 for a change that is not small, and again only when a fix after it touched tests or OpenSpec documents.

## Release

- **Version:** `version` in `app/Cargo.toml`, SemVer. The proposal states the bump; the last task group of `tasks.md` bumps it. Pre-1.0: breaking change or new capability → MINOR, bug fix → PATCH. Changes without behavior change keep the version and get no release.
- **`just release`** runs after the merge, on a clean checkout of `origin/main` (in a worktree: `git fetch origin && git switch --detach origin/main`). It refuses to run otherwise, and when the tag already exists. It runs `cargo clean`, the tests and the release build from scratch (`_release-package`, without the machine-wide lock: it uses no shared service, port or GPU), then tags `vX.Y.Z`, pushes the tag and creates the GitHub Release with `voicepuncher-vX.Y.Z-aarch64-apple-darwin.tar.gz` (binary + README) and its `.sha256`; release notes are generated from the merged PRs. `cargo clean` deletes `target/release/voicepuncher`, which the user may be running: in the main checkout, ask first.

## How to verify a change

All checks are `just` recipes (see `justfile`; no other scripts). A change is "verified" only when every level that applies has run and passed — say which levels ran and which did not.

| When | Command | What it runs |
|---|---|---|
| After every edit | `just test` (~1 min, without the machine-wide lock) | unit + architecture + integration tests |
| Once per change, after the last task group and before verify (and after `kit sync-base` when its last line names it) | `just check` (~8.5 min; machine lock held ≈ 7.3 min) | fmt, clippy `-D warnings`, `test`, release build, E2E smoke, WER accuracy on 10 samples, refinement evaluation; incremental, the machine lock only for the model stages |
| Before merging a branch, before a release | `just full-check` (~15.5–17.5 min; machine lock held ≈ 14.5 min with 10 recordings, ≈ 16.3 min with 100) | everything in `check` (incremental, no `cargo clean`), plus the chunk seams (`e2e-seams`: seam sweep and seam matrix) and the outages (`e2e-resilience`: services paused, killed and restarted while voice control runs, on ports of their own), with WER accuracy on a fixed sample of 100 recordings when the branch changes the recognition path, else on 10 (below) |
| Before merging a change to hotkey, recording, overlay, paste, permissions | manual run by the user | `README.md` § Release checklist |

Build inputs are `app/`, `command-spotting/`, `e2e-test/`, `.config/` (`nextest.toml`), `Cargo.toml`, `Cargo.lock` and `justfile`. A branch that changes none of them (docs, `AGENTS.md`, `openspec/`, `.claude/`) needs `just test` only; never `just check` or `just full-check` for it. Do not merge a branch that changed a build input until `just full-check` has passed on it; say in the PR which checks ran. When `main` moves after that, merge it with `kit sync-base` rather than rerunning `just full-check` by habit: a passing full check carries over when no build input changed on both sides beyond the version, and the final commit then needs `just check` (`checks.disjoint_merge` in `.claude/kit.yaml`, default `after_group`). `kit sync-base`'s last line names the check to pass before `kit land`; run exactly that one.

`just bench` is a model comparison, not a gate — only when a model, decoding params or the prompt changed.

- **Models and corpus** are auto-discovered by the `justfile`: each Whisper model by its file name in `~/.voicepuncher/` or `~/.voicepuncher/models/` (`ggml-large-v3-turbo-q5_0.bin` for dictation, `VOICEPUNCHER_WHISPER_MODEL`; `ggml-small.bin` for commands, `VOICEPUNCHER_COMMAND_WHISPER_MODEL`), the text refinement model by its file name too (`Qwen_Qwen3-4B-Instruct-2507-Q4_K_M.gguf`, `VOICEPUNCHER_LLM_MODEL`); a default not on disk fails with "Run 'just models' or set it in .env"; corpus in `e2e-test/corpus/`. The corpus is gitignored, so in a worktree set `VOICEPUNCHER_BENCH_CORPUS` to the main checkout's `e2e-test/corpus`.
- **Local services:** the E2E recipes start `whisper-server`, `whisper-server-commands`, `llama-server` and `voicepuncher-command-spotting` (`just start-model-servers`) and stop what they started afterwards; `just test` needs no service. The user's services run on ports 8081, 8085, 8080 and 8082; every run under the machine-wide lock (the model stages of `check` and `full-check`, every E2E recipe) starts its own services from the checkout's code and models on test ports 28081, 28085, 28080 and 28082 (`VOICEPUNCHER_TEST_STT_PORT` and siblings), never reuses or stops a service it did not start, restarts one a killed locked run left there, and fails before its first stage, naming the process, when anything else holds a test port; a branch or test that needs services of its own sets `VOICEPUNCHER_STT_PORT`, `VOICEPUNCHER_COMMAND_STT_PORT` and `VOICEPUNCHER_KWS_PORT` together (the `@local_services` and `@resilience` steps pick free ones for all three), so it never starts, reuses or stops a shared speech recognition service; the model stages of `check` and `full-check`, and every build, E2E and benchmark recipe run on its own, wait for the machine-wide test lock (`~/.claude/rules-custom/machine-lock.md`), so runs from parallel checkouts queue instead of colliding; `check` and `full-check` start the services inside the lock and stop them before releasing it. `just test`, `just release` and the stages of `check` and `full-check` before their model stages (fmt, lint, test, build, build-e2e) use no shared service, port or GPU and start at once, beside any locked run (the `justfile` comments say why). E2E business scenarios synthesize speech with macOS `say`.
- **Test limits:** every Rust test runs under cargo-nextest 0.9.146 (`cargo +stable install --locked cargo-nextest@0.9.146`); the recipes' guard `_require-nextest` stops with that command when it is missing and installs nothing. Each test has a time limit in `.config/nextest.toml`: 2 min by default, one per E2E suite (a Gherkin harness run is one test). A test over its limit is stopped and reported `TIMEOUT`: it hung — report it, never rerun it unchanged, never raise a limit to let a hang pass. A limit is the smallest whole number of minutes that is at least twice the longest run, never below 2 min; a suite that grows past half its limit gets a new limit, twice its new longest run, in the same change. `VOICEPUNCHER_HANG_PROBE=1 just test` (stopped at 2 min) and `VOICEPUNCHER_HANG_PROBE=1 just e2e-quick` (3 min) prove the limits still work, e.g. after a nextest bump.
- **Flakes and the same worktree:** a test that fails and passes on one unchanged rerun is a flake, not a pass: rerun once, never more; name the flake — test, recipe, what ran beside it (other sessions' runs, the `[machine-lock]` holder) — in the stage report and the PR body; when the same test flakes again, `/create-change` for its cause. No bar is loosened, no test dropped; exit 124 is a hang, reported and never rerun unchanged. Let this worktree's `check` or `full-check` finish before running `just test`, another `check` or `full-check` there: they share `target/`, and the lock no longer keeps them apart.
- **The stage journal:** every run of `test`, `check`, `full-check` and the E2E recipes goes through `_stages`, prints its stage table and appends one JSON line to `~/.cache/voicepuncher/stage-times.jsonl` (`VOICEPUNCHER_STAGE_JOURNAL`; failed and interrupted runs too, not a run killed with KILL): recipe, branch, commit, each stage with its time and result, lock wait and hold, the accuracy sample, the failed tests. `just stage-times [7d|2026-10-01]` (default 30 days) summarizes it; use it, not session logs, to tell where test time goes and whether a speed-up worked.
- **Synthesized speech is cached** in `~/.cache/voicepuncher/speech/` (or `VOICEPUNCHER_SPEECH_CACHE`), shared by every checkout and keyed by macOS build, voice, rate, audio format and text: `say` runs once per phrase per machine. Delete the directory to synthesize everything again.
- **The accuracy sample of `full-check`:** 100 recordings when the branch changes a path of `recognition_paths` in the `justfile` (or a Cargo file beyond its version), on `main` itself and when the changed paths cannot be read; else 10. It prints its choice before its first stage (`● accuracy: …`). Force 100 with `VOICEPUNCHER_ACCURACY_SAMPLE=100` (also for `kit verify`: `VOICEPUNCHER_ACCURACY_SAMPLE=100 kit verify before-merge`) after a change outside the repository — a model, `.env`, a service binary. A branch that adds a file on the recognition path adds it to `recognition_paths`.
- **The user's voice:** recordings of the user's own voice commands live in `~/.voicepuncher/voice-samples/` (never committed; README § Testing); `voice_samples_it` and the `@voice_samples` E2E scenarios run on them when they exist. When voice recognition fails for the user, ask for a new recording session there and add it rather than tuning on synthesized speech alone.
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
- No internet at runtime; loopback connections only — all inference is local, in separate local services (speech recognition, text refinement, command spotting).
- Three macOS TCC permissions required: Microphone, Accessibility, Input Monitoring.
