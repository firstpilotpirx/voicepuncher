# Tasks

No app behavior changes (`skip_specs: true`), so no Red/Green unit, `.feature` or E2E test is added. Each justfile change gets an observable check instead, run once before the change (it fails) and once after (it passes). `just check` waits for the machine-wide lock like every locked recipe.

## 1. `just test` on a lock of its run's own

- [ ] 1.1 Before: with another worktree's `just check` or `just full-check` holding the lock, start `just test` here; it prints `[machine-lock] held by …` and waits. This is the failing check of 1.2.
- [ ] 1.2 `test` in `justfile` runs `_test` through the wrapper with `MACHINE_LOCK_DIR` set to a `mktemp -d` directory, removed when the run ends (design decision 1); `_test`, `_stages` and `_release` stay as they are. Rerun 1.1: `just test` starts at once, prints no `[machine-lock]` line and exits 0. `MACHINE_LOCK_TIMEOUT=5 just test` on a warm build exits 124 after ~5 s.
- [ ] 1.3 The comment of `test` and the comment on the lock (design decision 7): which recipes run under the machine-wide lock, why `test` does not, that its own lock only gives it the wrapper's time limit (exit 124), and that it departs from agent-kit `machine-lock.md` on purpose until the kit's exception lands. `just --list` shows the comment of `test`.
- [ ] 1.4 `just check` passes; commit `chore(just): run just test without the machine-wide lock`.

## 2. AGENTS.md

- [ ] 2.1 § Workflow, "Several changes at once", first bullet: checks, builds and E2E queue on the machine-wide lock; `just test` does not, so coding and its tests go on in every session at once (design decision 7).
- [ ] 2.2 § How to verify a change, "Local services": the same split instead of "every check, test and benchmark recipe waits"; a new bullet next to it holds the flake rule (one unchanged rerun, flake named in the stage report and PR body, `/create-change` on a repeat; design decision 3) and the same-worktree rule (let this worktree's `check` or `full-check` finish before `just test`; design decision 4). `grep -n "lock" AGENTS.md` shows no line left that says `just test` waits.
- [ ] 2.3 `just test` passes (no build input changed in this group); commit `docs: just test runs without the machine-wide lock`.

## 3. Acceptance checks

Run with a second worktree on this branch's commit, its `VOICEPUNCHER_BENCH_CORPUS` set to the main checkout's `e2e-test/corpus` (design.md § How the acceptance bar is checked).

- [ ] 3.1 `just full-check` in the second worktree; `just test` here once during its `test` stage, once during its `build` stage, once during an E2E stage: no `[machine-lock]` line and exit 0 each time; the `full-check` exits 0.
- [ ] 3.2 `just test` started at once in both worktrees: both exit 0.
- [ ] 3.3 `just --dry-run` of `check`, `full-check`, `release`, `build`, `fmt`, `lint`, `e2e`, `e2e-smoke`, `e2e-quick`, `e2e-seams`, `e2e-resilience`, `e2e-bench`, `refine-eval`, `test-real-keyboard`, `bench` and `bench-smoke` shows the wrapper without `MACHINE_LOCK_DIR`; `just fmt` started while the second worktree's `full-check` holds the lock waits (`[machine-lock] held by …` after 30 s).
- [ ] 3.4 `MACHINE_LOCK_TIMEOUT=5 just test` on a warm build: exit 124 after ~5 s, and `pgrep -f` finds no process of the run after the 10 s grace.
- [ ] 3.5 Any flake seen in 3.1–3.4 is named in the stage report and the PR body (design decision 3).
- [ ] 3.6 `just check` passes on the final commit; commit the checked tasks (and the flake notes, if any) as `chore(openspec): record the acceptance checks of test-without-machine-lock`.

## 4. Version and manual checks

- [ ] 4.1 No version bump: no release (proposal § Release); `app/Cargo.toml` keeps 0.13.2. Manual checks by the user: none.
