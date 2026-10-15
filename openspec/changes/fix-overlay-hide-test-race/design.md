# Design

## Context

See proposal.md § Why for the race. What shapes the approach:

- **One log per step file, written from the pipeline's thread.** `InsertionRun` (`text_insertion.rs`), `StagesRun` (`processing_stages.rs`) and `VoiceRun` (`voice_control.rs`) each own an ordered log (`Arc<Mutex<Vec<Event>>>`, with an `Event` type of their own) that their stand-ins append to, and waits of their own: re-read the log every 5 ms, give up after the file's `REACTION` (1 s, 3 s, 2 s). `voice_control.rs` can also wait for an event after a log index (`wait_since_mark`); `text_insertion.rs` waits only for an event anywhere in the log.
- **One thread records each reaction, in a fixed order**, so the reaction's last event implies the earlier ones:

| Step | Reads the log after | The reaction, in the pipeline's order | Its last event, recorded by |
|---|---|---|---|
| `overlay_hides` | `Pasted` or `LeftInClipboard` | paste or clipboard hand-off → echo to stderr (after a paste) → hide | `Hidden`: `RecordingIndicator::hide` |
| `start_recording` | `RecordingStarted` | start the recorder → start live parts (in parts only) → show the recording | `Overlay(Recording)`: `NotingOverlay::show_recording` |
| `listening_on_given` | `ListeningStarted` | start listening → play the listening-on sound | `Sound("listening on")`: `RecordingSounds::play_listening_on` (`voice_control/doubles.rs`) |

- **How soon a step reads.** A step reads the log within one 5 ms poll of the event it waited for, plus however late its thread wakes: at most 14.3 ms (p99 13.5 ms) with the CPU twice oversubscribed (archived `test-without-machine-lock` design, Context). So it reads within about 20 ms of the first event of the reaction.

## Goals / Non-Goals

**Goals:**
- Each step reads the log only once its reaction has recorded its last event, through its own file's waits and bounds.
- A step that reads one of these three events too early fails on every run, not now and then: these three steps and any later one.

**Non-Goals:**
- A polling helper shared by the step files, or rewriting the other waits in them (D1).
- A lag on any other event or stand-in.

## Decisions

### D1. Each step adds one wait, with its file's own helper; no shared helper

- `start_recording` also waits for `Overlay(Recording)` after `RecordingStarted` (`StagesRun::wait_for`).
- `listening_on_given` also waits for `Sound("listening on")` after `ListeningStarted` (`VoiceRun::wait_for`).
- `overlay_hides` finds the outcome as today, then waits until `Hidden` follows it in the log, re-reading every 5 ms as `wait_since_mark` does; its failure message stays `overlay not hidden after {outcome}: {events}`.
- Every added wait gives up after its file's `REACTION`, like every other wait in these files; no bound changes. A healthy run waits only as long as the reaction takes; a broken one fails after at most one more `REACTION`, listing the log.
- The first wait stays, so a recorder or listening that never starts is still reported as such.

Rejected:
- **A shared helper** (in `steps/mod.rs`, generic over the log): each file already has its waits, and two of the three fixes are one more call of them. A new helper would leave two wait styles in each file, or rewrite the waits of the steps found safe, which the proposal leaves alone and `show-spotting-away` is reworking in `voice_control.rs`.
- **Making the `Then` steps tolerate the late event** (skip a leading `Overlay(Recording)` after the stop; ignore the listening-on sound in "no sound plays"): they would then miss the same signal caused by the `When`.
- **Sleeping `SETTLE` before the read:** slower on every run and still a race under load; `SETTLE` is for checks of what must not happen.
- **Waiting for the pipeline to be idle:** the app reports no such moment, and the change adds no app code.

### D2. The reproduction: the stand-in lags on the pipeline's thread, then records

`RecordingIndicator::hide`, `NotingOverlay::show_recording` and `RecordingSounds::play_listening_on` sleep a lag, then record their event. Every other method and stand-in stays as it is.

The lag runs on the pipeline's thread, between the two events of the reaction, which is what a busy CPU does to that thread. The log keeps the pipeline's order, and the pipeline takes its next input late, as a slow thread would.

The lag stays after the fix: a step that reads these events right after an earlier one fails on every run instead of now and then. A doc comment on the lag says so.

Rejected:
- **Recording from a spawned thread after the lag:** the event could land after later ones, an order the app never produces, and threads would outlive their scenario.
- **A lag only in the Red commit, or behind an environment variable:** a reproduction that is off by default guards nothing.
- **Loading the CPU:** not deterministic.

### D3. The lag is 50 ms

- **Long enough to fail every run:** a step reads within about 20 ms of the first event (Context); 50 ms is 2.5 times that, with margin for more load than measured.
- **Short enough to pass every run:** far below each `REACTION` (1 s, 3 s, 2 s), and 250 ms inside voice control's `SETTLE` (300 ms), which the out-of-scope `service_went_away_and_is_back` relies on for the listening-on sound at the service's return, which lags too.
- **Cost:** one lag per scenario that waits for a lagged event (37 voice-control, 7 processing-stages, 4 text-insertion scenarios): about 2.5 s more for `cucumber_integration`'s serial scenarios, off `just test`'s critical path (the 60 s outage test).
- One named constant per file, next to its stand-in (`PIPELINE_LAG`), as each file keeps its own `REACTION` and `SETTLE`.

### D4. Red, then Green: one commit each

- **Red:** the three lags, the steps unchanged. Each scenario named in proposal.md § Scope runs 10 times and fails every time, with the message of the observed flake for its step (Test levels). `just test` fails only in scenarios that use the three steps. Committed alone as `test: reproduce steps reading the pipeline log too early` (intentionally red).
- **Green:** the three waits (D1). The same scenarios pass 10 of 10, and `just test` passes; with the lags in place, that run also shows that no other step reads these events too early.
- One Red and one Green commit for all three steps: one cause and one fix in three independent files. Each scenario's message proves its own race, and `just check` runs once, after the Green commit.
- **The 10 runs:** the harness filtered by scenario name, `cargo test -p voicepuncher --test cucumber_integration -- --name '<scenario>'` (through `kit run`). The `test` recipe cannot select a scenario and this change adds no recipe; the binary uses no shared service, port or GPU, which is why `just test` runs unlocked. The form keeps working once `fail-hung-tests-fast` lands (its D4). The lag, not the load, makes today's steps fail, so nothing needs to run beside the 10 runs.

### D5. What stays

No `.feature` text changes, header comments included: they say the stand-ins record what they are asked to do, which stays true. No timing constant changes (`REACTION`, `SETTLE`, `SERVICE_TIME`, `FOCUS_WAIT`, `WAIT_TIME`, `TRANSCRIPTION_TIME`, the poll intervals). No scenario is dropped, tagged `@wip` or retried. No app code, harness or recipe changes.

## Test levels

No spec scenario (`skip_specs: true`) and no app behavior change: no unit, adapter integration or E2E test is added, and no manual run, since hotkey, recording, overlay, paste and permissions are untouched. Each acceptance item of proposal.md § Scope:

| Acceptance item | Level | How |
|---|---|---|
| "Paste once the user clicks into a text input on the web page" fails 10 of 10 with the lag and today's `overlay_hides`, and passes 10 of 10 after the fix | slice `.feature` (`text_insertion.feature`), unchanged | 10 runs by name at Red and at Green (D4); Red fails with `overlay not hidden after Pasted("Meeting moved to Friday."): [RecordingStarted, WaitingShown, Pasted("Meeting moved to Friday.")]` |
| "A hotkey dictation shows speech recognition after the stop", the same for `start_recording` | component `.feature` (`processing_stages.feature`), unchanged | the same; Red fails with `the stop did not show speech recognition first: [Overlay(Recording), Overlay(RecognizingSpeech), …]` |
| "Stop dictation without a dictation is ignored", the same for `listening_on_given` | component `.feature` (`voice_control.feature`), unchanged | the same; Red fails with `a sound played: Sound("listening on"); …` |
| No bar is loosened | review | `git diff origin/main -- app/tests/features` is empty; no timing constant in the diff; no `@wip`, no retry |
| `just test` passes; `just check` after each task group; `just full-check` on the final commit | recipe runs | stages 5 and 8 |

## Risks / Trade-offs

- [Under more load than measured, a Red run passes because the step read later than the lag] → 50 ms is 2.5 times the measured worst; the lag may go up to 100 ms, still 200 ms inside voice control's `SETTLE`; no bound moves.
- [The lag catches a step the proposal found safe] → the Context table finds none; if Green's `just test` fails on one, it fails on every run, and it is a scope question for the user, not a quiet fix.
- [`show-spotting-away` edits `voice_control/doubles.rs` too] → this change touches one method of `RecordingSounds` and adds a constant; whichever lands second merges it by hand. Once that change drops the sound at the service's return, the lag still guards the start sound that `listening_on_given` waits for.
- [The Red commit fails `just test`] → it lives only on the branch; the squash merge keeps it off `main`.
