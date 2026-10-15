# Design

## Context

The motivation is in proposal.md § Why. The facts that shape the approach:

- **Wiring.** `test` runs `{{locked}} _test`: agent-kit's `with-machine-lock` (a link to `kit lock`) calling `just _test`. `check` and `full-check` run `_test` as a stage of `_stages`, and `_release` depends on it; both run inside the lock their own recipe holds.
- **The wrapper** (agent-kit `kit/internal/lock`, `kit/internal/proc`) takes an exclusive `flock` on `$MACHINE_LOCK_DIR/tests.lock` (default `~/.cache/machine-lock`), waiting at most `MACHINE_LOCK_WAIT` (3600 s, then exit 75). It runs the command in a process group of its own, passes INT, TERM and HUP on to the group as TERM, TERMs the group after `MACHINE_LOCK_TIMEOUT` (3600 s), KILLs what is left 10 s later and exits 124. The next taker of a lock kills what a holder that died left behind. It has no time-limit-only mode: its one way past the lock, a nested call inside a locked run (`MACHINE_LOCK_HELD` naming the lock file), runs the command with no limit and no process group of its own.
- **Stock macOS has no `timeout`:** neither `timeout` nor `gtimeout` exists on macOS 27.0.1; `perl` does, and the justfile already uses it.
- **Cargo 1.88 in one `target/`**, tried on a scratch crate: a second build prints `Blocking waiting for file lock on build directory` and waits for the first. Cargo holds that lock only while it builds: a build started while `cargo run`'s program ran finished at once. `cargo clean` does not take it: it removed `target/` in 0 s under a running build, which failed with `failed to write …: No such file or directory`. `_test` builds with `cargo test --no-run`, then runs the test binaries itself, all at once, outside cargo.
- **The speech cache** (`e2e-test/src/speech_cache.rs`, `command-spotting/tests/common/mod.rs`): on a miss, `say` writes into a temporary file of its own in the cache directory, which is then renamed into place (`tempfile_in`, `persist`). In `just test`, `engine_it` and `server_it` synthesize speech before any timed part; `chunk_cuts_it` and two unit tests of the E2E harness (`e2e-test/src/speech.rs`) synthesize it through the same cache and time nothing.
- **Scheduling on this machine** (10 cores: 8 performance, 2 efficiency), measured under the machine-wide lock with busy perl loops — a *storm* is 20 of them — and a *probe* that sleeps 10 ms in a loop and records how late it wakes:

| Measured | Result |
|---|---|
| CPU share: 10 loops at nice 0 beside 10 at nice 10, or at nice 20 | equal: 29.0 M vs 29.2 M loops; 25.7 M vs 26.3 M |
| Probe on an idle machine | at most 12.8 ms late |
| Storm and probe at one priority | p99 13.5 ms, at most 14.3 ms late |
| Utility QoS (`taskpolicy -c utility`) alone; 8 loops at once | full speed: 43.2 M vs 43.3 M loops; 69.7 M vs 67.9 M |
| Utility storm, probe at default QoS | at most 5.0 ms late; default loops get 3× the CPU of utility ones (50.7 M vs 16.3 M) |
| Storm at default QoS, utility probe | p50 59.7 ms, at most 118.8 ms late |
| Background QoS (`taskpolicy -b`) alone | 15 % of full speed (6.7 M vs 43.3 M loops) |

## Goals / Non-Goals

**Goals:**
- `just test` starts at once, with today's body and bars, and keeps today's limit: after 3600 s the whole run is killed and the recipe exits 124.
- `check`, `full-check` and `release` run `_test` exactly as today, inside their lock.
- No new tool, script or dependency.

**Non-Goals:**
- Making `just test` and a locked run independent of each other's CPU load: the design measures the margins and sets a flake rule (decisions 2 and 3).
- A guard for `just test` beside its own worktree's `check` or `full-check` (decision 4).
- Any agent-kit change: the rule's exception and a time-limit-only mode are its follow-up (decision 6).

## Decisions

### 1. `test` keeps the wrapper, on a lock of its run's own

`test` still runs `_test` through the wrapper, but gives it a lock directory of the run's own: a temporary directory (`mktemp -d`) passed as `MACHINE_LOCK_DIR` and removed when the run ends. Nothing else ever takes that lock, so `just test` starts at once beside anything on the machine, another `just test` of the same worktree included, and it keeps everything the wrapper does for a run: its own process group, INT, TERM and HUP passed on as TERM, the 3600 s limit (`MACHINE_LOCK_TIMEOUT`, unchanged, and the way to shorten it), the kill of the whole run, exit 124. `_test` does not change; `_stages` and `_release` keep running it inside the machine-wide lock.

*Alternatives:* GNU `timeout` (not on stock macOS; a Homebrew dependency for one recipe); `perl -e 'alarm …; exec …'` (the alarm kills only `just`: the `_test` script and its test binaries keep running, and the exit is 142, not 124); a perl supervisor with a process group, signal forwarding and a grace kill (about twenty lines of justfile re-implementing the wrapper's tested code); a deadline in `_test`'s wait loop (misses a hung build, applies inside `check` and `release` too, and a script without job control cannot reliably kill the binaries and what they start); a lock directory per worktree (two `just test` runs of one worktree would queue, which the proposal rules out); a time-limit-only mode of `kit lock` (the right home, part of the follow-up of decision 6, but kit changes are out of scope).

### 2. `just test` runs at the default priority, all binaries at once

No `nice`, no QoS clamp, no cap on parallel binaries or test threads. The measurements decide it:

- `nice` changes nothing on this macOS (Context), so it cannot give the locked run priority.
- The utility QoS clamp would: default work keeps waking within 5 ms and gets 3× the CPU. But `just test`'s own timing tests would then wake 60–120 ms late while the locked run builds or lints, and their tightest bound does not hold that: `http_failure_it`'s 300 ms deadline spans three wake-ups (the stub accepts, its handler thread starts, the client reads the answer), 180–360 ms under the clamp. The background clamp is slow even alone.
- At one priority, a waking thread is at most ~15 ms late with the CPU twice oversubscribed, on either side. The timing tests of both runs are bounded by wake-ups with little CPU work between them, not by CPU time. In `just test`: `http_failure_it` (300 ms over three wake-ups), the `@serial` scenarios of `text_insertion`, `voice_control` and `processing_stages` (reactions within 1–3 s), `commandspotting_client_it` (lower bounds of 12 s and 4.5 s that slowness cannot break; an upper bound of 7 s, 2 s past the client's 5 s limit), `commandspotting_reconnect_it` (back within 5 s), `server_it` (pongs at most 1 s apart while the in-process service decodes 100 ms frames, each in a fraction of its length) and `engine_it` (audio fed as fast as it goes; confirmation deadlines of 800 ms to 5 s against stand-ins that answer at once, or sleep 4 s under a 5 s deadline). The locked run's tests against the live services do their heavy work in speech recognition and text refinement on the GPU, which `just test` never touches. CPU-bound work slows by the oversubscription factor, which these bounds absorb.
- Fewer binaries or test threads at once would slow every `just test` on an idle machine, the common case, to soften contention the bounds already absorb.

Fallback, only once a locked run's real-time test is shown to fail beside `just test`: a change of its own clamps the build phase of the unlocked `just test` to utility QoS — it is not timing-sensitive, and the clamp costs nothing on an idle machine — never its test binaries.

### 3. Flakes: one rerun, named, a change on repeat

A test that fails and passes on one unchanged rerun is a flake, not a pass, in `just test` and in a locked run alike. The agent reruns once, never more; names the flake — test, recipe, what ran beside it (other sessions' runs, the `[machine-lock]` holder) — in its stage report and in the PR body; and when the same test flakes again, files a change for its cause with `/create-change`. No bar is loosened and no test dropped. A failure on the rerun is a real failure; exit 124 is a hang, reported and never rerun unchanged (`machine-lock.md`).

### 4. The same worktree: cargo's lock, a rule, no guard

`just test` beside its own worktree's `check` or `full-check` shares one `target/`. Cargo's build-directory lock keeps their builds apart (Context): no artifact is written by two builds at once. It does not cover `full-check`'s first step: `cargo clean` deletes `target/` under a `just test` that is building or running there, which then fails on missing files. The clash shows as a failed run, never as a pass on stale or half-built artifacts, and the workflow does not produce it: `proceed-change` waits for `kit verify after-group` before the next task group, and `full-check` runs after the last one. `AGENTS.md` tells the agent to let its own `check` or `full-check` finish before it runs `just test` in that worktree.

*Alternatives:* `test` waits while the machine-wide lock's holder is its own worktree (reads the kit's private holder file, and brings back the waiting this change removes); a worktree lock also taken by `check` and `full-check` (changes those recipes, and `cargo clean` would delete a lock kept in `target/`).

### 5. The speech cache stays as it is

Parallel runs never read half an entry: an entry joins the cache by renaming a complete file (Context). Two runs that miss the same phrase at once both run `say` and both rename; the second replaces the first with the same speech (one key: macOS build, voice, rate, format, text), and a reader that opened the first keeps reading it. Parallel `say` calls only slow each other, and only on a miss: once per phrase per machine, before any timed part.

### 6. The agent-kit rule: a deliberate departure until its exception lands

`~/.claude/rules-custom/machine-lock.md` says to wrap every public recipe that builds, tests, checks or benchmarks in `with-machine-lock`, and the agent-kit README (§ Machine-wide test lock) says the same. Both need a matching exception: a test recipe that uses no shared port, service, GPU or exclusive device may run without the lock. That is an agent-kit follow-up, to be filed in agent-kit, not part of this change; with it, `kit lock` can offer a time-limit-only mode, which `test` then uses instead of its own lock directory. Until then `test` departs from the rule on purpose, and its justfile comment says so.

### 7. Docs

- `justfile`, the comment on the lock (above `machine_lock`): every check, build, E2E, benchmark, release and real-keyboard recipe runs its private body under the machine-wide lock; `test` does not (see its comment).
- `justfile`, the comment of `test`: it runs at once, without the machine-wide lock, because it uses no shared service, port, GPU or exclusive device; the lock of its run's own only gives it the wrapper's time limit (exit 124); it departs from agent-kit `machine-lock.md` on purpose until the kit's exception lands.
- `AGENTS.md` § Workflow, "Several changes at once", first bullet: checks, builds and E2E queue on the machine-wide lock; `just test` does not, so coding and its tests go on in every session at once.
- `AGENTS.md` § How to verify a change, "Local services": the same split instead of "every check, test and benchmark recipe waits"; a new bullet next to it holds the flake rule (decision 3) and the same-worktree rule (decision 4).
- `openspec/config.yaml`, `README.md` and `.claude/kit.yaml` mention no lock and stay as they are.

## How the acceptance bar is checked

No spec scenario (`skip_specs: true`) and no app behavior change, so no unit, integration, `.feature` or E2E test is added, and no manual run by the user: hotkey, recording, overlay, paste and permissions are untouched. The session checks each item of proposal.md § Scope at stage 5, with a second worktree on the change's commit (its `VOICEPUNCHER_BENCH_CORPUS` pointing at the main checkout's corpus):

| Item | Check |
|---|---|
| `just test` starts at once and passes beside another worktree's `full-check`, in its `test`, `build` and an E2E stage; that `full-check` passes | `just full-check` in the second worktree; `just test` here once in each of those stages: no `[machine-lock]` line, exit 0 each time; the `full-check` exits 0 |
| Two `just test` runs from two worktrees, started together, pass | both started at once; both exit 0 |
| Every other recipe still waits for the lock | `just --dry-run` of `check`, `full-check`, `release`, `build`, `fmt`, `lint`, `e2e`, `e2e-smoke`, `e2e-quick`, `e2e-seams`, `e2e-resilience`, `e2e-bench`, `refine-eval`, `test-real-keyboard`, `bench` and `bench-smoke` shows the wrapper without `MACHINE_LOCK_DIR`; `just fmt`, started while the `full-check` holds the lock, waits (`[machine-lock] held by …` after 30 s) |
| A hung `just test` is killed at its limit with exit 124 | `MACHINE_LOCK_TIMEOUT=5 just test` on a warm build (the run needs ~1 min, so the limit cuts it as it would a hang): exit 124 after ~5 s, and no process of the run is left after the 10 s grace |
| `just check` and `just full-check` pass on the final commit | stages 5 and 8 |

## Risks / Trade-offs

- [A timing test flakes under contention, in `just test` or in a locked run] → the margins are measured (decision 2); the flake rule names it, a repeat files a change, and the fallback clamps the unlocked build phase; no bar moves.
- [Several sessions' `just test` runs at once oversubscribe the CPU several times over; CPU-bound parts slow in proportion] → each run takes ~1 min, most of it `commandspotting_reconnect_it`'s 60 s outage, which needs little CPU (a frame every 100 ms); wake-ups stay prompt at one priority; the acceptance runs put two `just test` runs and a `full-check` side by side.
- [`test` borrows the lock wrapper for its limit: its timeout message says `[machine-lock]`, and a run killed before its cleanup leaves a directory in `$TMPDIR` that holds only `tests.lock`] → the `test` comment names the `[machine-lock]` message; macOS clears its temporary directories; the kit's time-limit-only mode (decision 6) replaces the borrowed lock.
- [A wrapper killed outright (SIGKILL) leaves the run's test binaries running, and no later run reaps them: nobody takes that lock directory again, while the next taker of the machine-wide lock kills a dead holder's leftovers (Context)] → INT, TERM and HUP still stop the whole run; only a SIGKILL of the wrapper skips that; the time-limit-only mode of decision 6 is where reaping belongs.
- [The recipe's shell, there for the `EXIT` trap that removes the lock directory, sits between `just` and the wrapper, so a TERM sent to `just` alone would not reach the wrapper] → the shell runs the wrapper in the background and passes TERM, INT and HUP on to it as TERM: a TERM to `just` alone stops the whole run at once (tried: exit 143 within 1 s, no test binary left); the time-limit-only mode of decision 6 removes the shell.
- [The measurements come from one machine and one macOS version] → they only justify leaving the priority alone, which needs no macOS feature; the acceptance runs on this machine confirm it.
