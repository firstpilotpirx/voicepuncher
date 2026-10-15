# Tasks

No Rust code changes and no spec scenario (`skip_specs: true`): each behavior is checked by running the recipes (design.md § Test levels). Where it means something, a Red item first shows today's behavior or a failing stub, then the Green item implements the behavior and runs its check. Scratch edits and throwaway commits are never pushed. Every check runs through `just` recipes. In a worktree, set `VOICEPUNCHER_BENCH_CORPUS` to the main checkout's `e2e-test/corpus` for every run below.

## 1. Lock only the model stages (decisions 1–3)

- [ ] 1.1 Red: with a scratch formatting error in an app source file, run `just check` while another `with-machine-lock sleep 60` holds the lock: the run waits for the lock before `fmt` and only then fails at `fmt` (today's whole-run lock). Keep the log excerpt for the PR; revert the scratch edit.
- [ ] 1.2 Green: in `justfile`, `_stages` splits its stage list at the first model stage (`e2e*` or `refine-eval`); the stages before it run one by one, each through `with-machine-lock` with `MACHINE_LOCK_DIR=$(mktemp -d)` (removed in its `EXIT` trap) so each keeps the `MACHINE_LOCK_TIMEOUT` limit and exit 124; the rest runs in the new private recipe `_model-stages` through `{{locked}}` with the caller's `MACHINE_LOCK_DIR`, which starts the services it does not find running, exports `VOICEPUNCHER_E2E_SERVICES_HELD=1` and stops what it started in its `EXIT` trap (also when the start is interrupted). `check` runs `_stages fmt lint test build build-e2e e2e-smoke e2e-quick refine-eval` (with `build-e2e` a no-op stub until group 2); `full-check` runs `_stages clean fmt lint test build build-e2e e2e-smoke e2e-seams e2e-resilience e2e-bench refine-eval` (accuracy choice in group 4); `_check` and `_full-check` go away. Check: the 1.1 run now ends at `fmt` ✗ with no `→ waiting` line and a non-zero exit.
- [ ] 1.3 Green: the stage table (decision 2): `_stages` creates the report file and passes it in `VOICEPUNCHER_STAGES_REPORT`; it prints `→ HH:MM:SS waiting for the machine lock: <model stages>` before the wrapper, `_model-stages` appends `locked <epoch>` and prints `● HH:MM:SS machine lock taken after N s`; the `EXIT` trap prints unlocked stages, `lock wait`, model stages, `lock held` and `total` with `✓ ✗ ●`, aligned for `e2e-resilience`, no colors; under a caller's lock both rows say so. Check: in a `just check` log, `lock wait` matches the `→` and `●` times and `lock held` the block; under `kit run --lock just check` the rows say the caller holds the lock.
- [ ] 1.4 Exit 75: `MACHINE_LOCK_WAIT=5 MACHINE_LOCK_REPORT_EVERY=1 just _stages e2e-quick` while another `with-machine-lock sleep 60` holds the lock exits 75 with ✗ `lock wait` and no stage run.
- [ ] 1.5 Exit 124 of the block: `MACHINE_LOCK_TIMEOUT=60 just _stages e2e-smoke` with the service ports set to free ones (as `@local_services` does) exits 124 with ✗ `e2e-smoke`; nothing listens on those ports and no pid file is left in `~/.voicepuncher/run/`.
- [ ] 1.6 Exit 124 of an unlocked stage: after touching a source file, `MACHINE_LOCK_TIMEOUT=5 just _stages lint` exits 124 after ≈ 5 s with ✗ `lint`; `pgrep cargo` shows no cargo process of the run left.
- [ ] 1.7 Ctrl-C: INT to the run's process group during `test`, during the lock wait, and during `e2e-smoke` on free ports: each time the table is printed, a following `with-machine-lock true` starts at once, and nothing is left on the run's ports.
- [ ] 1.8 `just check` passes and its table shows the lock rows; commit `chore(just): lock only the model stages of check and full-check`.

## 2. Build the model stages' binaries before the lock (decision 4)

- [ ] 2.1 Red: after an edit of `e2e-test/src/`, a `just check` log on group 1's commit shows `Compiling` lines (`cucumber_e2e`) after `● … machine lock taken`.
- [ ] 2.2 Green: justfile variables for each cargo selection the model stages build — `cargo build --release -p voicepuncher-command-spotting` (used by `start-model-servers`), and the `cargo test … --no-run` selections of `e2e-smoke`, `e2e-seams`, `placed_commands_it` and `cucumber_e2e` (used by the E2E recipes); the private recipe `_build-e2e` runs them all and replaces the stub stage `build-e2e`. The E2E recipes keep their `_build` dependency and run what they ran before. Check: warm, `just _build-e2e` takes ≈ 2 s; `just e2e-smoke` passes on its own.
- [ ] 2.3 Repeat the 2.1 run: no `Compiling` line after `● … machine lock taken`.
- [ ] 2.4 `just check` passes; commit `chore(just): build the E2E binaries before the machine lock`.

## 3. Unlocked builds yield the CPU (decision 5)

- [ ] 3.1 Red: during the `build` stage of `just check` after an edit, `ps -o pid,pri,comm` on the `rustc` processes shows the default priority (31), not the utility clamp.
- [ ] 3.2 Green: `_stages` runs `clean`, `fmt`, `lint`, `build` and `build-e2e` under `taskpolicy -c utility`; `_test` runs its three `cargo test --no-run` builds under it and its test binaries at the default QoS. Check: the 3.1 probe shows clamped `rustc` processes (priority 20) during `build` and unclamped test binaries during `test`; `http_failure_it` passes in `just test`.
- [ ] 3.3 `just check` passes; commit `chore(just): build at utility QoS outside the machine lock`.

## 4. The accuracy sample of `full-check` (decision 6)

- [ ] 4.1 Red: add `_accuracy-stage base="origin/main"` as an `exit 1` stub and the `recognition_paths` variable (empty), wire `full-check` to call it, and run `just _accuracy-stage base=<scratch base>` on the throwaway commits of 4.3: every case fails.
- [ ] 4.2 Green: `recognition_paths` holds the paths of design.md decision 6 (one glob per path, with the comment on adding new files); `_accuracy-stage` applies the rules in order — `VOICEPUNCHER_ACCURACY_SAMPLE=100` forces 100 and any other value fails, the changed paths from `git merge-base HEAD <base>`, `git diff --name-only --no-renames` and `git ls-files --others --exclude-standard` (failure → 100), no change → 100, a match → 100 naming up to three matches, Cargo files only when a non-`version = "…"` line changed, else 10 — and prints the stage and reason; `full-check` prints `● accuracy: N recordings (<stage>) — <reason>` before its first stage and in its passing end line, and runs the chosen stage in place of `e2e-bench`.
- [ ] 4.3 On throwaway commits against a scratch base: listed file → 100; unlisted file → 10; renamed listed file → 100; version-only bump of `app/Cargo.toml` and `Cargo.lock` → 10; dependency line → 100; `base=no-such-ref` → 100; a detached `origin/main` without changes → 100; `VOICEPUNCHER_ACCURACY_SAMPLE=10` → error. Each case shown once in the PR body.
- [ ] 4.4 `just check` passes; commit `chore(just): full-check runs 100 recordings only when recognition changes`.

## 5. Docs (decision 8)

- [ ] 5.1 `justfile` comments: the lock comment at the top (recipes run on their own take the lock whole; `check` and `full-check` only for their model stages); `check` and `full-check` (time, lock hold, accuracy choice and its switch); `_stages` (including the borrowed wrapper and its `[machine-lock]` timeout line), `_model-stages`, `_build-e2e`, `_accuracy-stage`, `recognition_paths`; `e2e-quick`'s 10 recordings of ordinary speech.
- [ ] 5.2 `AGENTS.md`: § Workflow "Several changes at once" (builds and unit tests run at once, only model stages queue); § How to verify a change — `check` and `full-check` rows, "Local services" (start and stop inside the lock), the accuracy choice and `VOICEPUNCHER_ACCURACY_SAMPLE=100` for `kit verify`, the rule for new recognition files, and that two checks of one worktree must not overlap.
- [ ] 5.3 `README.md` § Testing (table and paragraph: the lock, the accuracy choice and its switch) and § Release checklist step 1 (on `origin/main` itself `full-check` runs 100 recordings); `openspec/config.yaml` `context` gets the measured times from group 6 (placeholder wording until then) and that `full-check` runs 100 recordings when recognition changes. Check: `openspec validate --all` passes and every recipe named in the docs exists in `just --list --unsorted` or as a private recipe.
- [ ] 5.4 `just check` passes; commit `docs: the machine lock covers only the model stages`.

## 6. Measurements against the acceptance bar

- [ ] 6.1 Lock hold of `check`: three warm runs and three runs after an edit, median of the `lock held` row each; bar ≤ 345 s each. The unlocked stages of these runs alone take no longer than their medians in `research/README.md`.
- [ ] 6.2 Lock hold of `full-check` with 10 recordings: `just _stages clean fmt lint test build build-e2e e2e-smoke e2e-seams e2e-resilience e2e-quick refine-eval` (this branch changes the justfile, so `full-check` itself picks 100); bar ≤ 685 s. With 100 recordings: the final `full-check` of 8.2; bar ≤ 800 s.
- [ ] 6.3 Two parallel checks: two worktrees, each after an edit, `just check` started together; both logs show the unlocked stages overlapping and each `→ waiting` line after `build-e2e`.
- [ ] 6.4 CPU clamp: three `just check` runs whose model stages overlap another worktree's `full-check` from `clean` to `build-e2e`, against three runs alone the same day: all pass, model stages at most 15 % slower. If not, record which stage of the other run overlapped and stop for the next lever (fewer build jobs), not a looser bar.
- [ ] 6.5 Write the numbers of 6.1–6.4 with their commands into `research/README.md` next to today's; update the times in the `justfile` comments, `AGENTS.md`, `README.md` and `openspec/config.yaml` `context`. `just check` passes (the justfile changed); commit `docs: measured lock hold of check and full-check`.

## 7. Sharing the services between runs (decision 7)

- [ ] 7.1 Prepare two worktrees A and B on the change's final commit, each with `just build` and `just _build-e2e`, `VOICEPUNCHER_BENCH_CORPUS` at the main checkout's corpus; no `*.e2e` marker in `~/.voicepuncher/run/`; ask the user not to dictate meanwhile. Write the measurement script in the session's scratchpad (not committed).
- [ ] 7.2 Run eight pairs, S C S C S C C C, each inside one `with-machine-lock` call; a run is `just _e2e-smoke`, `just _e2e-quick`, `just _refine-eval` with `VOICEPUNCHER_E2E_SERVICES_HELD=1`; services started once before the first pair (unless running) and stopped after the last only if the script started them. Record per run: exit status, failing tests or scenarios by name, stage and run times, mean WER of the 10 recordings, the "no command" result; per pair: wall time; from the service logs: every supervisor restart.
- [ ] 7.3 Write the finding into `research/README.md` (replacing "not measured yet"): the numbers, the commands to repeat it, the verdict against the three criteria (no new failure and no restart in a concurrent pair; WER within 0.01 of the sequential median; median concurrent pair ≤ 75 % of the median sequential pair) or the bottleneck that failed, and what sharing cannot cover.
- [ ] 7.4 Only if all three criteria hold: file a follow-up with `/create-change` (per-run ownership of the services, a lock admitting two model blocks, `full-check`'s heavier stages), and name it in the finding and the PR body; else note "no follow-up" with the reason.
- [ ] 7.5 `just test` passes (only `research/README.md` and possibly `openspec/changes/` changed, no build input); commit `docs: measure sharing the model services between two runs`.

## 8. Version and manual checks

- [ ] 8.1 No version bump: the proposal says no release; `app/Cargo.toml` keeps its version (0.13.2 when filed).
- [ ] 8.2 `just full-check` on the final commit, run as `VOICEPUNCHER_ACCURACY_SAMPLE=100 kit verify before-merge`: passes (bar failures another filed change owns, named in the PR), prints `● accuracy: 100 recordings (e2e-bench) — forced`, and no `Compiling` line follows `● … machine lock taken`.
- [ ] 8.3 Manual checks by the user: none (hotkey, recording, overlay, paste and permissions are untouched).

## Workflow follow-up

- Amend `~/.claude/rules-custom/machine-lock.md` in agent-kit (lock the stages that use shared services, the GPU or shared ports; builds and unit tests may run unlocked): outside this repo, proposal.md § Scope.
- If `test-without-machine-lock` merges first, merge its lock wording into the justfile comment and AGENTS.md (design.md decision 8).
