# Design

## Context

`_stages` runs a check's build stages (fmt, lint, test, metrics, build, bundle, build-e2e) without the machine lock at the utility QoS clamp, then its model stages in a light and a heavy phase under the machine lock shared (`split-machine-lock`). The exclusive mode keeps locked runs out, not those build stages: on 2026-10-22 another checkout's builds ran beside `e2e-seams` and starved the whisper-small copies (confirmations of 1–1.4 s instead of 0.15–0.27 s); the cue came 1.8 s late. The clamp is not enough: a utility storm still takes a third of the CPU (`test-without-machine-lock`). Every cue-timing scenario lives in one `Rule:` of `voice_control.feature`, tagged `@seams`.

## Decisions

### 1. The cue-timing scenarios become the stage `e2e-cues`

The `Rule:` "A voice start gives its start cue within 0.7 s" gets the tag `@cues`; a nextest profile and a recipe `just e2e-cues` run it; `full-check` runs it last, so its exclusive wait comes once everything else passed. `e2e-seams` keeps the seam sweep, the matrix and the `@seams` speech scenarios. Rejected: all of `e2e-seams` exclusive (7 min of seams that need no idle machine).

### 2. A third lock class, `exclusive`

`exclusive_stages := "e2e-cues"`; `_lock-phases` prints an `exclusive` phase after light and heavy. `_class-lock exclusive` takes the machine lock exclusively (outer wrapper, deadline = lock wait + stage deadline; the kit keeps every shared phase out and lets a waiting exclusive run go first), then the quiet lock of decision 3 exclusively (inner wrapper, the stage deadline). Machine lock first: holding the quiet lock while queuing for the machine would stall other checks' builds for the queue's length; this order stalls them at most for the stage's hold (~4 min). The phase uses the heavy ports, free while it holds the machine lock.

### 3. Other checks' build stages take a quiet lock, shared

The build stages of `_stages` run through the wrapper on `<machine lock dir>-voicepuncher-quiet` in the shared mode instead of a private directory: same time limit, they still start at once beside everything except a running or waiting `e2e-cues` stage, which they wait out. Standalone `just test`, `just release`, the hosted CI and the user's `cargo watch` take no lock; a cue timed beside them names them (decision 4). Rejected: builds shared on the machine lock itself (they would queue behind every exclusive run of any project and hold back exclusive waiters); a lower QoS (insufficient, Context); timing the cue against the confirmation's own time (hides the slowness the bound guards).

### 4. A late cue names its load

When a cue misses its bound, or fewer than 9 of 10 recorded starts make it, the failure appends a load report sampled right then: the five busiest processes (`ps` by CPU share, pid and command) and every live holder of a lock under `~/.cache/machine-lock*` (checkout, command and mode from the kit's holder files; a holder whose process is gone is skipped). `e2e-test/src/machine_load.rs`, unit-tested on a scratch lock directory and sample `ps` rows; the cue steps only append it.

### 5. Light-phase cue scenarios: none

Every cue-timing scenario is in the `Rule:` of decision 1; the light stages time no cue. The proposal's second open point closes with nothing to move.

### 6. Measured acceptance

Three `just e2e-cues` runs while another worktree runs a check's build stages (`VOICEPUNCHER_REUSE_STAGES=0 just lint test metrics build build-e2e`, started a minute earlier): each stage table shows the exclusive lock's wait and every cue scenario passes; the measured times set the nextest limits of `e2e-cues` and the shortened `e2e-seams`. One `full-check` is the merge gate.

## Test levels

| Spec scenario / behavior | Level |
|---|---|
| The start cue is timed once the other checks' builds have ended | recipe self-tests `_test-class-locks` (an exclusive run waits for a shared quiet holder, keeps a new one out, excludes light and heavy) and `_test-lock-phases` (the exclusive phase, last); the runs of decision 6 |
| A late start cue names what ran beside it | unit tests of `machine_load.rs`; the step's message is plain Rust |
| Start start followed by the dictation at once gives its cue at once | E2E `.feature` (`e2e-cues`), unchanged in meaning |
| Start followed by the dictation at once gives its cue at once | E2E `.feature` (`e2e-cues`), unchanged |
| Start said on its own gives its cue at once | E2E `.feature` (`e2e-cues`), unchanged |
| Restart restart followed by the new dictation at once gives its cue at once | E2E `.feature` (`e2e-cues`), unchanged |
| Restart said on its own gives its cue at once | E2E `.feature` (`e2e-cues`), unchanged |
| The user's recorded starts give their cue at once | E2E `.feature` (`e2e-cues`), unchanged |
| Start please followed by the dictation at once gives its cue at once | E2E `.feature` (`e2e-cues`), unchanged |
| Start please said on its own gives its cue at once | E2E `.feature` (`e2e-cues`), unchanged |
| A faster start keeps the dictation where the command's words end | E2E `.feature`, unchanged, stays in its stage |
| A faster start adds no false start | E2E `.feature`, unchanged, stays in its stage |
| A brief dip inside the command words starts nothing early | E2E `.feature`, unchanged, stays in its stage |

No manual run: hotkey, recording, overlay, paste and permissions are untouched.

## Risks / Trade-offs

- [~4 min of exclusive hold per `full-check`, after every shared holder] → it runs last; `kit land` runs one `full-check` per group.
- [Another check's build stage waits while a cue stage runs] → at most the stage's hold, shown inside that stage's time.
- [A user's `cargo watch` or standalone `just test` still loads the machine] → the failure names it.

## Dependencies

None. `measure-test-strength` edits `_stages` too and `voice-commands-under-load` edits `voice_control.feature`: shared files, taken in parallel; the second to merge resolves the conflicts from both designs.
