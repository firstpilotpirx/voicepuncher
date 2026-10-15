# Design

## Context

Motivation and measurements: proposal.md § Why and `research/README.md`. The facts that shape the approach:

- **The lock wrapper** (`with-machine-lock`, agent-kit `kit/internal/lock` and `proc`) takes an exclusive `flock` on `$MACHINE_LOCK_DIR/tests.lock` (default `~/.cache/machine-lock`), polled every 200 ms: waiters form no queue. A waiter prints nothing for 30 s, then `● waiting for the machine-wide test lock` and `held by …; waiting Ns` every 30 s; it prints nothing when it gets the lock and never a total. After `MACHINE_LOCK_WAIT` (3600 s) it exits 75 without running the command. The command runs in a process group of its own, with `MACHINE_LOCK_HELD=<lock file>` added to its environment; a nested wrapper whose `MACHINE_LOCK_HELD` equals its own lock file runs its command at once. INT, TERM and HUP become TERM to the group; after `MACHINE_LOCK_TIMEOUT` (3600 s) the group gets TERM and the wrapper exits 124; whenever the command exits non-zero, what is left of the group gets TERM and, 10 s later, KILL.
- **`kit verify`** runs `sh -c "cd <root> && just full-check"` through `kit run`, without `--lock` and with kit's own environment (`exec.Command` without `Env`); the wrapper and `just` pass the environment on too (plus `.env`). A variable set for `kit verify` reaches every recipe.
- **Today** `check` and `full-check` run `{{locked}} _check` / `_full-check`: every stage runs in one lock. `_stages` starts the services it does not find running before the first `e2e*` or `refine-eval` stage, sets `VOICEPUNCHER_E2E_SERVICES_HELD=1` so the E2E recipes neither start nor stop one, and stops what it started in its `EXIT` trap. Which services a run started is marked per service and port, not per run.
- **What compiles under the lock today.** In the `full-check` log of 2026-10-15 15:38 (after `cargo clean`), the model stages compiled only `cucumber_e2e` (6 s), not the `*_it` binaries `research/README.md` lists: `test` builds every app test target and every command spotting target, which covers what `e2e-smoke` and `e2e-seams` run. `start-model-servers` builds the service with `cargo build --release -p voicepuncher-command-spotting`, which the workspace's `cargo build --release` does not cover: alone the service resolves `libc` without `std` and `ureq` without its default features, in the workspace with them (`cargo tree`), so cargo builds them and the binary a second time — under the lock whenever a run starts the service on a fresh `target/`. The `@local_services` and `@resilience` scenarios run `just start-model-servers` too. No crate has a build script or reads a variable the recipes set at compile time, so the URLs and limits the E2E recipes export rebuild nothing.
- **Scheduling on this machine** (M1 Max: 8 performance and 2 efficiency cores, 64 GB). `nice` does nothing: `ps` shows priority 31 under `nice -n 10`, as without it, and the separately filed `test-without-machine-lock` measured equal CPU shares at nice 0, 10 and 20. The utility QoS clamp reaches every process a command starts (a grandchild of `taskpolicy -c utility` runs at priority 20); that change measured clamped work at full speed alone and yielding beside default-QoS work (which then wakes within 5 ms and gets 3× the CPU), clamped timer waits 60–120 ms late under load, and the background clamp (`taskpolicy -b`) at 15 % of full speed.
- **The accuracy run.** Two `@accuracy` scenarios change with the sample. "Benchmark corpus is transcribed within the accuracy bar" runs `target/release/voicepuncher --config /dev/null --input-file <wav> --transcription-url … --refinement-url …` on 100 corpus recordings, two at a time, scored by `e2e-test/src/wer.rs`. "Ordinary speech triggers no command" streams 20 corpus recordings through the app's `SpottingClient` to the command spotting service, which confirms with the speech recognition service; the app's `VoiceCommandRecognizer` must find no command. `e2e-quick` caps both at 10 (`VOICEPUNCHER_BENCH_LIMIT=10`); its comment's "20 recordings of ordinary speech" is wrong.
- **Version bumps touch Cargo files**: the bump in `app/Cargo.toml` also changes the `voicepuncher` entry of `Cargo.lock`, in `version = "…"` lines only (796723e). Since 2026-09-15, 35 merges changed a build input: 24 a Cargo file (11 of them a dependency), 22 the justfile.
- **Shell**: `#!/usr/bin/env bash` is macOS's bash 3.2 here (no other bash on `PATH`): no bash 4 features, no empty `"${array[@]}"` under `set -u`. `git diff --name-only` detects renames and then lists only the new path.

## Goals / Non-Goals

**Goals:**
- `check` and `full-check` hold the machine lock only for their model stages and compile nothing while they hold it; a run whose earlier stage fails never waits for it.
- One stage table per run, with the lock wait and the lock hold as rows of their own.
- Every stage keeps a time limit and exit 124, inside the lock or not.
- `full-check` picks its accuracy sample offline from the branch's changed paths, says which and why, and runs 100 recordings whenever it cannot tell.

**Non-Goals:**
- Sharing services between runs: only measured (decision 7).
- A fair queue: who gets the lock after a release stays a race between pollers, as today.
- Recipes run on their own — `just test` is `test-without-machine-lock`'s; `fmt`, `lint`, `build`, the E2E recipes, `e2e` and `release` keep their lock — and any kit change.

## Decisions

### 1. `_stages` runs in two phases

`_stages` splits its stage list at the first model stage: a stage named `e2e*` or `refine-eval`, `e2e-resilience` included (its services listen on ports of their own, but use the same GPU and CPU). The stages before it run one by one without the machine lock (decisions 3 and 5). The first model stage and every stage after it run in one private recipe, `_model-stages`, which `_stages` starts through `{{locked}}`: it starts the services it does not find running, exports `VOICEPUNCHER_E2E_SERVICES_HELD=1`, runs its stages and stops what it started in its `EXIT` trap — today's `_stages` logic, now after taking the lock and before giving it back. Its trap stops what it started even when the start itself is interrupted (the markers exist before the start). The supervisors of the services it starts belong to the lock's process group, so a block that fails or is killed takes them down with it.

- A failing unlocked stage ends the run with its exit code; the lock is never requested.
- Exit 75 (the wait passed `MACHINE_LOCK_WAIT`): no model stage ran; `_stages` exits 75.
- Exit 124 (the block passed `MACHINE_LOCK_TIMEOUT`): the block's trap marks the running stage failed and stops its services; `_stages` exits 124, which agents report as a hang (`machine-lock.md`).
- Ctrl-C: in an unlocked stage, the stage stops; during the wait, the wrapper dies with INT and its `flock` with it; in the block, the wrapper turns INT into TERM for the block. Each time `_stages` prints the table and exits non-zero; no lock stays held and no service the run started stays up.
- A caller that already holds the lock (`kit run --lock just check`) runs everything inside it: the nested wrapper starts the block at once.

`check` runs `_stages fmt lint test build build-e2e e2e-smoke e2e-quick refine-eval`; `full-check`, after its model and corpus guards as today, runs `_stages clean fmt lint test build build-e2e e2e-smoke e2e-seams e2e-resilience <accuracy stage> refine-eval`, with `cargo clean` as the stage `clean`, so the table accounts for it. `_check` and `_full-check` go away.

*Alternatives:* a lock per model stage (the services would stop and start between stages, or run unowned between two locks, where another run could stop them: the markers are per service); moving only the build out of the lock (a run whose tests fail would still wait first).

### 2. One table with the lock rows

`_stages` creates a report file and hands it to `_model-stages` in `VOICEPUNCHER_STAGES_REPORT`; both append one row per stage (mark, name, seconds). Right before the wrapper, `_stages` notes the time and prints `→ HH:MM:SS waiting for the machine lock: <model stages>`; `_model-stages` first appends `locked <epoch>` and prints `● HH:MM:SS machine lock taken after N s`. When the wrapper returns, `_stages` adds `lock wait` (taken − requested; without a `locked` row, the time until the wrapper gave up or stopped, marked ✗) and `lock held` (returned − taken: the services' start and stop and every model stage). Its `EXIT` trap prints the unlocked stages, `lock wait`, the model stages, `lock held` and `total` in today's format: the `✓ ✗ ●` symbols of `terminal-output.md`, aligned columns wide enough for `e2e-resilience`, no colors (agents read it in `kit run` logs, not in a terminal). Under a caller's lock, the two rows say so instead of a time.

*Alternatives:* the wrapper's own output (nothing under 30 s, never a total); one table per phase (the acceptance bar reads one).

### 3. Unlocked stages keep a time limit

Today the lock's 3600 s limit bounds the whole run; outside it, a hung test binary would hold the run, and the agent's session waiting on it, for good. `_stages` therefore runs each unlocked stage through the wrapper too, with a lock directory of the run's own (`MACHINE_LOCK_DIR=$(mktemp -d)`, removed in its `EXIT` trap): no one else takes that lock, so the stage starts at once with all the wrapper gives — a process group, signals passed on, the `MACHINE_LOCK_TIMEOUT` limit, exit 124. `_model-stages` gets the caller's `MACHINE_LOCK_DIR`, so it takes the machine-wide lock. `test-without-machine-lock` gives `just test` its limit the same way. `MACHINE_LOCK_TIMEOUT` now bounds each unlocked stage and the block separately.

*Alternatives:* no limit outside the lock; GNU `timeout` (not on stock macOS); `perl alarm` (kills `just`, not the stage's processes); a time-limit-only mode of `kit lock` (the right home, an agent-kit follow-up; kit changes are out of scope).

### 4. `build-e2e` compiles the model stages' binaries before the lock

A new unlocked stage, `build-e2e` (private recipe `_build-e2e`), runs right after `build`:

- `cargo build --release -p voicepuncher-command-spotting`, the line `start-model-servers` runs — a third build next to the two the proposal names (Context);
- `cargo test <selection> --no-run` for each selection the model recipes run: the app integration tests of `e2e-smoke`, those of `e2e-seams`, `placed_commands_it`, and `cucumber_e2e` (on top of `fail-hung-tests-fast`, their nextest runs with `--no-run`: decision 8).

Each of these lines becomes a justfile variable that the E2E recipes (or `start-model-servers`) and `_build-e2e` share, so they cannot drift: another package or feature selection builds other artifacts (Context). This edits the E2E recipes and `start-model-servers` beyond the recipes proposal.md § Impact names, without changing what they run. Warm, the stage is five no-op cargo calls (≈ 2 s); after `cargo clean`, it builds `cucumber_e2e` and the service's own copies of its dependencies outside the lock. The E2E recipes keep their `_build` dependency, a no-op inside the block, so they still work on their own. No test moves: the model stages built these binaries themselves until now. The name does not start with `e2e`, so the stage is no model stage.

*Alternatives:* fold it into `build` (`_build` is also the dependency of every E2E recipe and of `release`); `cargo test --workspace --no-run` (unifies features across the workspace: other artifacts); keep compiling under the lock (6 s warm, more after an edit of `e2e-test/` or on a fresh `target/`).

### 5. Unlocked builds yield the CPU: the utility QoS clamp

`_stages` runs `clean`, `fmt`, `lint`, `build` and `build-e2e` under `taskpolicy -c utility`, and `_test` runs its three `cargo test --no-run` builds under it (its nextest build: decision 8); the test binaries keep the default QoS. Alone, the clamp costs nothing; beside another run's model stages, the builds yield to them. That is the side to slow down: the model stages hold the lock every queued run waits for, and a run that is still building could not take the lock before they end anyway.

`nice` is not used: it does nothing on this macOS (Context), which corrects the proposal's "lower CPU priority". The background clamp would confine builds to the two efficiency cores. Test binaries stay unclamped: clamped timer waits run 60–120 ms late under load, beyond what `http_failure_it`'s 300 ms deadline absorbs; `test-without-machine-lock` keeps them at the default QoS for the same reason and names this clamp of the build phase as its fallback.

The acceptance runs check this default twice: the unlocked stages of `check` alone take no longer than their medians in `research/README.md`, and the model stages stay within the 15 % bar beside a build from scratch. If the bar fails, the session reports the numbers and which unlocked stage of the other run overlapped (the `→` and `●` lines carry times); the next lever is fewer parallel build jobs, not a looser bar.

*Alternatives:* no clamp (another worktree's build takes half the CPU from the model stages); clamping the `test` stage whole (its timing tests).

### 6. The accuracy sample of `full-check`

`full-check` first calls a private recipe, `_accuracy-stage base="origin/main"`, which prints `e2e-bench` (100 recordings) or `e2e-quick` (10) and the reason; `full-check` prints `● accuracy: N recordings (<stage>) — <reason>` before its first stage and repeats it in the line that ends a passing run. The rules, in order:

1. `VOICEPUNCHER_ACCURACY_SAMPLE=100` (env or `.env`) → 100, "forced"; it passes through `kit verify` (Context). Any other value fails at once: the switch can only add recordings.
2. The changed paths, offline, as `kit checks-needed` collects them plus `--no-renames`: `git merge-base HEAD <base>` with the local ref (no fetch), then `git diff --name-only --no-renames <merge base>` (committed and uncommitted changes; a file moved off a listed path still counts) and `git ls-files --others --exclude-standard`. If any of these fails (no `origin/main`, a shallow clone, no repository) → 100, "cannot tell what the branch changed".
3. No changed path at all → 100: that is `main` itself, e.g. a check on `origin/main` before a release.
4. A changed path matches `recognition_paths` → 100, naming up to three matches; else → 10, "no recognition path among the N changed files".

`recognition_paths` is a justfile variable next to `full-check`: humans read it there, tools with `just --evaluate recognition_paths`; kit needs nothing but the run's output. It holds glob patterns, written out one path each, matched with bash `[[ == ]]` (`*` spans `/`, pathname expansion off). The list follows what the two `@accuracy` scenarios run (Context):

| Area | Paths |
|---|---|
| File mode, transcription, chunking | `app/src/main.rs`, `app/src/adapters/primary/cli/**`, `app/src/config.rs`, `app/src/config/**` (defaults and prompts), `app/src/adapters/secondary/{recording/filesystem,transcription,http,textpasting/stdio}/**`, `app/src/core/application/{pipeline,chunking,transcriber,transcript_filter}.rs`, `app/src/core/domain/{audio_buffer,raw_transcript}.rs` |
| Text refinement | `app/src/adapters/secondary/textrefinement/**`, `app/src/core/application/text_refiner.rs`, `app/src/core/domain/{cleaned_text,filler_sounds,normalization_check,command_words}.rs` |
| Command spotting, voice-command matching | `command-spotting/src/**`, `app/src/adapters/secondary/commandspotting/websocket/client.rs`, `app/src/core/domain/{voice_command,voice_command_recognizer}.rs` |
| WER and corpus test code | `e2e-test/src/{corpus,wer,concurrency,speech,voice_commands}.rs`, `e2e-test/tests/cucumber_e2e.rs`, `e2e-test/tests/steps/{mod,file_transcription,voice_control}.rs`, `e2e-test/tests/features/{file_transcription,voice_control}.feature` |
| Models, service settings | `justfile`, whole |
| Dependencies | `Cargo.lock`, `Cargo.toml`, `*/Cargo.toml`, when their diff changes a line other than `version = "…"` |

- **The justfile counts whole.** The service command lines, the model lookup and the WER recipes are recipes and variables that call each other (`start-model-servers`, `_start-model-servers-for-e2e`, `_e2e-bench`, the model variables); a check by recipe name would have to follow those calls to stay safe. The cost is small: of the 22 merges since 2026-09-15 that changed the justfile, 6 changed no other listed path, and one of those (HAProxy) changed recognition through the justfile alone.
- **Cargo files count unless only version lines change**: a release bump does not force the 100 recordings; a dependency added, removed, bumped or given other features does (a registry bump also changes its `checksum` line).
- `pipeline.rs`, `main.rs`, the CLI and the config count whole: `run_once`, the wiring of file mode, its flags and its defaults live there, next to live-mode code.
- A branch that adds a file on this path adds it to the list; the variable's comment and AGENTS.md say so.

*Alternatives:* `app/**` whole (the choice would never pick 10 for an app change); a recipe-level justfile check (above); the list in a file of its own (one more build input, apart from the recipe that reads it); `.claude/kit.yaml` (kit config; kit changes are out of scope); a switch that also forces 10 (it would loosen the bar; the 10-recording timing of the acceptance bar is measured through `_stages`, see Test levels).

### 7. The measurement of shared services

At the end of stage 5, from two worktrees A and B on the change's final commit, each prebuilt (`just build`, `just _build-e2e`), with `VOICEPUNCHER_BENCH_CORPUS` pointing at the main checkout's corpus:

- **A run** is `check`'s model stages in one worktree — `just _e2e-smoke`, `just _e2e-quick`, `just _refine-eval` — timed per stage. **A pair** is A and B one after the other (S) or started together (C). Eight pairs, S C S C S C C C (3 sequential, 5 concurrent; the alternation spreads drift), each inside one `with-machine-lock` call, so no third run overlaps a pair and queued runs get the lock between pairs.
- **Services** are started once, outside both runs: the first pair runs `just start-model-servers` unless they already run; every run gets `VOICEPUNCHER_E2E_SERVICES_HELD=1`, so no run starts or stops one (the markers are per service, not per run); after the last pair, the script stops what it started. No `*.e2e` marker may be left in `~/.voicepuncher/run/` before the first pair, and the user does not dictate meanwhile.
- **Recorded:** per run, the exit status, every failing test or scenario by name, stage and run times, the mean WER of the 10 recordings and the "no command" result; per pair, the wall time from the first start to the last end; from the services' logs, every supervisor restart (`exited with code`, `stopped answering its health check`).
- **Expected bottlenecks:** the three speech recognition copies (one request each, the rest queued in HAProxy, short confirmations first); `llama-server`'s single slot (`-np 1`); command spotting confirmations queued behind corpus transcriptions against the client's 10 s silence limit — how the `@accuracy` scenarios failed when they overlapped other scenarios (speed-up-checks); 4 scenarios at once per run make 8 clients per pair.
- **Sharing works** when all three hold: no concurrent run fails a test that passed in every sequential run, and no supervisor restarts during a concurrent pair; each concurrent run's mean WER is within 0.01 of the sequential median (sharing must not change what the runs measure); the median concurrent pair takes at most 75 % of the median sequential pair. Then a follow-up change is filed with `/create-change`; per-run ownership of the services, a lock that admits two model blocks and `full-check`'s heavier stages are its to design and measure. Otherwise the finding names the bottleneck that failed.
- The finding, with its numbers and the commands to repeat it, goes into `research/README.md`, together with what sharing cannot cover: a branch that changes a service (`command-spotting/`, or a service's command line or model in the justfile) needs services of its own, on ports of its own; the `@local_services` and `@resilience` scenarios start their own anyway.

### 8. Docs, `test-without-machine-lock` and `fail-hung-tests-fast`

- **justfile:** the lock comment at the top (recipes run on their own take the lock whole; `check` and `full-check` take it for their model stages only); the comments of `check` and `full-check` (measured time and lock hold, the accuracy choice and its switch), `_stages`, `_model-stages`, `_build-e2e`, `_accuracy-stage` and `recognition_paths` (with the rule for new files); `e2e-quick`'s 10 recordings of ordinary speech.
- **AGENTS.md:** § Workflow, "Several changes at once" (builds and unit tests run at once in every session; only the model stages queue); § How to verify a change: the `check` and `full-check` rows, "Local services" (the services start and stop inside the lock), the accuracy choice and its switch for `kit verify`, and that two checks of one worktree must not overlap.
- **README.md:** § Testing (table and paragraph: the lock, the accuracy choice and its switch); § Release checklist, step 1 (on `origin/main` itself, `full-check` runs the 100 recordings).
- **openspec/config.yaml** `context`: the measured times, and that `full-check` runs the 100 recordings when recognition changes.

`test-without-machine-lock` (filed separately, not merged) composes with this change:

- It unlocks `just test` run on its own; this change unlocks the `test` stage inside `check` and `full-check`. Both take the time limit from the wrapper with a lock directory of the run's own (decision 3): one device for both. Its acceptance item "`check` and `full-check` still wait for the lock" reads "their model stages" once this change is on `main`.
- Both rewrite the justfile's lock comment and the lock sentences of AGENTS.md ("Several changes at once", "Local services"); the second to merge takes the other's wording along: recipes run on their own take the lock, `test` does not, `check` and `full-check` take it for their model stages.
- `_test`'s build phase gets the utility clamp here (decision 5): the fallback of its decision 2, with its test binaries untouched. Its rule for one worktree (let your own `check` finish before `just test`) extends to two `check` or `full-check` runs of one worktree, which the lock no longer keeps apart.

`fail-hung-tests-fast` (filed on `main`) moves `_test` and the bodies of the model recipes to cargo nextest with a limit per test. This change depends on it and on `test-without-machine-lock` and is implemented on top of both:

- `build-e2e` prebuilds what the model recipes' nextest runs build: the shared selection variables hold each nextest invocation's package and target selection, and `_build-e2e` runs the same invocation with `--no-run` ("compile, but don't run tests"), so the artifacts match whatever the selection; its `cargo nextest run --workspace` in `_test` unifies features across the workspace and does not cover per-package selections. `_build-e2e` gets its `_require-nextest` guard.
- `_test` keeps the clamp on its build only: `taskpolicy -c utility cargo nextest run <its arguments> --no-run`, then the run itself, whose build is then a no-op.
- Limits per test do not replace decision 3: `clean`, `fmt`, `lint`, `build` and `build-e2e` are no tests, and that change leaves builds out of scope. Inside the lock, a hung test now fails at its nextest limit and the block ends long before `MACHINE_LOCK_TIMEOUT`.
- Its "every check still runs under `with-machine-lock`" (every public recipe `{{locked}} _<name>`) reads "the model stages of `check` and `full-check`" once both are on `main`.

## Test levels

No spec scenario (`skip_specs: true`) and no Rust code change, so no unit, integration or `.feature` test is added. The session checks each behavior by running the recipes in stage 5; scratch edits and commits are throwaway and never pushed. No manual run by the user: hotkey, recording, overlay, paste and permissions are untouched.

| What | How it is checked |
|---|---|
| Two runs build at once; each waits for the lock only after `build-e2e` | two `just check` runs after an edit, started together in two worktrees: both logs show the unlocked stages overlapping and the `→ waiting` line after `build-e2e` |
| A failing unlocked stage never waits | a scratch formatting error: `just check` ends at `fmt` ✗, without a `→ waiting` line, exit non-zero |
| Lock rows | every acceptance run: `lock wait` matches the `→` and `●` times, `lock held` the block |
| Exit 75 | `MACHINE_LOCK_WAIT=5 MACHINE_LOCK_REPORT_EVERY=1 just _stages e2e-quick` while another run holds the lock (the wrapper checks its wait limit only when it reports): exit 75, ✗ `lock wait`, no stage ran |
| Exit 124 of the block | `MACHINE_LOCK_TIMEOUT=60 just _stages e2e-smoke` with the service ports set to free ones (as `@local_services` does), so the run starts its own services: exit 124, ✗ `e2e-smoke`, nothing left on those ports, no pid file left |
| Exit 124 of an unlocked stage | `MACHINE_LOCK_TIMEOUT=5 just _stages lint` after touching a source file: exit 124 after ≈ 5 s, ✗ `lint`, no cargo process of the run left |
| Ctrl-C | INT to the run's process group in `test`, during the wait, and in `e2e-smoke` on free ports: the table is printed, another `with-machine-lock true` starts at once, nothing is left on the run's ports |
| Nothing compiles under the lock | no `Compiling` line after `● … machine lock taken` in a `check` after an edit of `e2e-test/src/`, nor in the final `full-check` |
| Accuracy choice | `just _accuracy-stage base=<scratch base>` on throwaway commits: listed file → 100; unlisted file → 10; renamed listed file → 100; version-only bump → 10; dependency line → 100; `base=no-such-ref` → 100; a detached `origin/main` without changes → 100; `VOICEPUNCHER_ACCURACY_SAMPLE=10` → error; the final `full-check` run as `VOICEPUNCHER_ACCURACY_SAMPLE=100 kit verify before-merge` → "forced" |
| Lock hold bars | read from the rows: `check` warm and after an edit, three runs each (median); `full-check` with 100 recordings (the final run); with 10, `cargo clean` included, `just _stages clean fmt lint test build build-e2e e2e-smoke e2e-seams e2e-resilience e2e-quick refine-eval` — this branch changes the justfile, so `full-check` itself picks 100 here |
| CPU clamp | the unlocked stages of the `check` runs alone against the research medians; three `check` runs whose model stages overlap another worktree's `full-check` from `clean` to `build-e2e`, against three alone the same day: all pass, at most 15 % slower |
| Sharing | the measurement of decision 7, written into `research/README.md` |
| Nothing else broke | `just check` after every task group, `just full-check` on the final commit |

## Risks / Trade-offs

- [The accuracy choice rarely picks 10 here: applied to the 35 merges since 2026-09-15 that changed a build input, the list picks 100 for 34 — 28 through an app, service or test path or a dependency, 6 through the justfile alone] → the lock split carries most of this change's saving; the choice still saves ≈ 100 s on UI and tooling branches that leave the listed files alone (the web-content focus fix, 0.13.2, was one), and it can be dropped later without touching the rest.
- [A new file on the recognition path is missing from the list] → the branch that adds it edits a listed caller and runs 100; the rule asks it to add the file; a later miss costs one `full-check` on 10 recordings, which still checks the bar.
- [Changes outside the repository — a model file, `.env`, a Homebrew service binary — are invisible to the choice] → `VOICEPUNCHER_ACCURACY_SAMPLE=100`; `just bench` compares models.
- [A stale or missing local `origin/main`] → stale: an older merge base adds paths, never hides one; missing: 100.
- [Another run's test binaries (default QoS, ≈ 20–60 s) and other unclamped work beside the model stages] → measured by the 15 % bar; the clamp covers every build; the next lever is named in decision 5.
- [Two runs of one worktree are no longer kept apart] → cargo's build lock covers builds, not `cargo clean`; the workflow runs one check per worktree at a time, and AGENTS.md says so.
- [Unlocked stages borrow the wrapper: a timeout prints `[machine-lock]`, and a run killed hard leaves an empty directory in `$TMPDIR`] → the `_stages` comment says so; macOS clears its temporary directories; a time-limit-only mode of `kit lock` would replace it.
- [`check` and `full-check` depart further from `machine-lock.md`, which wraps every check recipe whole] → the agent-kit amendment named in proposal.md § Scope.
