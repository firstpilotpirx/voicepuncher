# Tasks

No app behavior changes (`skip_specs: true`), so no Red/Green unit, `.feature` or E2E test is added. Each justfile change gets an observable check instead, run once before the change (it fails) and once after (it passes). `just check` waits for the machine-wide lock like every locked recipe.

## 1. `just test` on a lock of its run's own

- [x] 1.1 Before: with another worktree's `just check` or `just full-check` holding the lock, start `just test` here; it prints `[machine-lock] held by …` and waits. This is the failing check of 1.2.
- [x] 1.2 `test` in `justfile` runs `_test` through the wrapper with `MACHINE_LOCK_DIR` set to a `mktemp -d` directory, removed when the run ends (design decision 1); `_test`, `_stages` and `_release` stay as they are. Rerun 1.1: `just test` starts at once, prints no `[machine-lock]` line and exits 0. `MACHINE_LOCK_TIMEOUT=5 just test` on a warm build exits 124 after ~5 s.
- [x] 1.3 The comment of `test` and the comment on the lock (design decision 7): which recipes run under the machine-wide lock, why `test` does not, that its own lock only gives it the wrapper's time limit (exit 124), and that it departs from agent-kit `machine-lock.md` on purpose until the kit's exception lands. `just --list` shows the comment of `test`.
- [x] 1.4 `just check` passes; commit `chore(just): run just test without the machine-wide lock`.
- [x] 1.5 Fix found at verify (design § Risks): before, a TERM sent to `just` alone stopped at the recipe's shell, which waited for the wrapper, so the run went on to its end (still running after 20 s). The shell now runs the wrapper in the background and passes TERM, INT and HUP on to it; after, a TERM to `just` alone exits 143 within 1 s and leaves no test binary, `MACHINE_LOCK_TIMEOUT=5 just test` exits 124 and a full run exits 0. Commit `fix(just): pass a signal to just test on to the whole run`.

## 2. AGENTS.md

- [x] 2.1 § Workflow, "Several changes at once", first bullet: checks, builds and E2E queue on the machine-wide lock; `just test` does not, so coding and its tests go on in every session at once (design decision 7).
- [x] 2.2 § How to verify a change, "Local services": the same split instead of "every check, test and benchmark recipe waits"; a new bullet next to it holds the flake rule (one unchanged rerun, flake named in the stage report and PR body, `/create-change` on a repeat; design decision 3) and the same-worktree rule (let this worktree's `check` or `full-check` finish before `just test`; design decision 4). `grep -n "lock" AGENTS.md` shows no line left that says `just test` waits.
- [x] 2.3 `just test` passes (no build input changed in this group); commit `docs: just test runs without the machine-wide lock`.

## 3. Acceptance checks

Run with a second worktree on this branch's commit, its `VOICEPUNCHER_BENCH_CORPUS` set to the main checkout's `e2e-test/corpus` (design.md § How the acceptance bar is checked).

- [x] 3.1 `just full-check` in the second worktree; `just test` here once during its `test` stage, once during its `build` stage, once during an E2E stage: no `[machine-lock]` line and exit 0 each time; the `full-check` exits 0. So far: the three `just test` runs exited 0 with no `[machine-lock]` line; the `full-check` failed only in `e2e-seams`, on the known failure that also fails on `main` (backlog change `raise-accuracy-under-hard-conditions`). The user accepted this on 2026-10-15 and decided to merge with that known failure, as for PR #77; the shared services were restarted from `main` first, since the command spotting service still ran a build from 2026-10-14.
- [x] 3.2 `just test` started at once in both worktrees: both exit 0. Rerun on 141aa58: the first pair failed in both attempts (flakes in 3.5), each unchanged rerun of both passed. Not met cleanly; the user accepted it on 2026-10-15 with both flakes named here and in the PR.
- [x] 3.3 `just --dry-run` of `check`, `full-check`, `release`, `build`, `fmt`, `lint`, `e2e`, `e2e-smoke`, `e2e-quick`, `e2e-seams`, `e2e-resilience`, `e2e-bench`, `refine-eval`, `test-real-keyboard`, `bench` and `bench-smoke` shows the wrapper without `MACHINE_LOCK_DIR`; `just fmt` started while the second worktree's `full-check` holds the lock waits (`[machine-lock] held by …` after 30 s).
- [x] 3.4 `MACHINE_LOCK_TIMEOUT=5 just test` on a warm build: exit 124 after ~5 s, and `pgrep -f` finds no process of the run after the 10 s grace. Rerun on 141aa58: exit 124 after ~6 s; `pgrep` found no process of the run after the 10 s grace.
- [x] 3.5 Any flake seen in 3.1–3.4 is named in the stage report and the PR body (design decision 3). Seen once, in 3.2: `http_failure_it` `should_tell_a_busy_answer_as_busy`, in `just test` beside the second worktree's `just test`, got `Failure::Dropped` instead of `Failure::Busy(503)` — a refused, reset or broken connection, not its 300 ms deadline (that reads `TimedOut`); the unchanged rerun of both runs passed. The test's first flake, so no change is filed. Seen again on 141aa58, in 3.2: `text_insertion.feature` "Paste once the user clicks into a text input on the web page" — overlay not hidden after `Pasted`; it had flaked once before on `main`'s code (during this change's filing), so its cause gets a change of its own (`/create-change`), proposed as a follow-up: the step `overlay_hides` (`app/tests/steps/text_insertion.rs`) reads the events right after `Pasted` instead of waiting for `Hidden`.
- [x] 3.6 `just check` passes on the final commit; commit the checked tasks (and the flake notes, if any) as `chore(openspec): record the acceptance checks of test-without-machine-lock`. On 3550089 `just full-check` passed fmt, lint, test, build, e2e-resilience, e2e-bench and refine-eval; it failed in e2e-seams (the known failure of 3.1) and in e2e-smoke `voice_forms_it` (the user's recorded "restart", session-2 at 27.23 s, not carried out, 2 of 2 runs; also on `main`, likely from #78). The user decided on 2026-10-15 to merge with both.

## 4. Version and manual checks

- [x] 4.1 No version bump: no release (proposal § Release); `version` in `app/Cargo.toml` keeps its value. Manual checks by the user: none.
