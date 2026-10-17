# Proposal

## Why

The slice scenario "Paste once the user clicks into a text input on the web page" (`app/tests/features/slice/text_insertion.feature`) failed twice on 2026-10-15 with `overlay not hidden after Pasted("Meeting moved to Friday."): [RecordingStarted, WaitingShown, Pasted("Meeting moved to Friday.")]`: once in `just test` on `main` after #77 was merged, once in task 3.2 of `test-without-machine-lock`, while two `just test` runs ran at once. Each unchanged rerun passed. By AGENTS.md's flake rule, a test that flakes a second time gets a change for its cause. Since #88 `just test` runs without the machine-wide lock, so runs beside other runs, where this race shows, are now the normal case.

The cause is in the test, not the app. The pipeline runs on a thread of its own, and its stand-ins record what they are asked to do in one shared log. The `Then` step "the overlay hides" (`overlay_hides` in `app/tests/steps/text_insertion.rs`) waits up to `REACTION` (1 s) only for the paste or the clipboard hand-off, then reads the log once and requires `Hidden` after it. The pipeline hides the overlay a moment after the paste (it echoes the pasted text to stderr first), so when a busy CPU slows its thread, that one read comes before `Hidden` is recorded. The app is right: the overlay hides after the paste.

## What Changes

- **"the overlay hides" waits for `Hidden`.** `overlay_hides` waits for `Hidden` after the paste or the clipboard hand-off within its existing `REACTION` bound, instead of reading the log once. This covers the three text-insertion scenarios that end with the step.
- **The same race in two more steps.** Every step of `text_insertion.rs`, `processing_stages.rs` and `voice_control.rs` (with `voice_control/`) was checked; two more take a reading right after the first event of a reaction while the app records the rest of it a moment later. Neither has been seen failing yet; each gets the same fix, a wait within its existing bound for the last event of that reaction:
  - `start_recording` (`processing_stages.rs`) waits for `RecordingStarted`, then the `When` marks where the stop begins. The pipeline shows the recording overlay right after it starts the recorder; recorded late, that signal counts as after the stop, and the `Then` steps that check the overlay signals after the stop fail because speech recognition is not the first. The step also waits for the recording overlay.
  - `listening_on_given` ("listening is on", `voice_control.rs`) waits for `ListeningStarted`. The pipeline plays the listening-on sound right after; played late, the sound lands after the `When`'s mark, and "no sound plays" fails in "Stop dictation without a dictation is ignored". The step also waits for the listening-on sound.
- **Test-first.** For each step, a reproduction comes first: a double records the later event a moment late (the visual indicator double records `Hidden` some tens of milliseconds after it is asked to hide; the same for the recording overlay and the listening-on sound), so the step as it is today fails on every run; then the fix makes it pass. The reproduction stays, so a step that reads the log too early fails every time instead of now and then. design.md sets the delays.
- **Nothing else moves:** no app code, no `.feature` text, no bound (`REACTION`, `SETTLE`), no harness or recipe.

## Capabilities

### New Capabilities

None. A test fix with no app behavior change: `skip_specs: true`.

### Modified Capabilities

None.

## Impact

- `app/tests/steps/text_insertion.rs`: `overlay_hides` and the visual indicator double `RecordingIndicator`.
- `app/tests/steps/processing_stages.rs`: `start_recording` and the visual indicator double `NotingOverlay`.
- `app/tests/steps/voice_control.rs`: `listening_on_given`; `app/tests/steps/voice_control/doubles.rs`: the sound double `RecordingSounds`.
- `app/` is a build input: `just check` after the Green task group (the Red group fails by design), `just full-check` before the merge. No manual run: hotkey, recording, overlay, paste and permissions behave as before.
- In-flight changes in the same files: `show-spotting-away` reworks the outage steps and the overlay double of the component voice-control scenarios, not these steps; `fail-hung-tests-fast` changes the harness entry point `app/tests/cucumber_integration.rs`, not the steps.

## Scope

- **Lifecycle stage:** current PoC, target PoC. A test fix; the product stage does not move.
- **Scope:** the three steps above and the reproduction doubles they need.
- **Acceptance bar:**
  - With the reproduction doubles, each step as it is today fails on each of 10 runs of a scenario that uses it: "Paste once the user clicks into a text input on the web page", "A hotkey dictation shows speech recognition after the stop", "Stop dictation without a dictation is ignored". With the fixed steps, the same scenarios pass 10 of 10.
  - No bar is loosened: `REACTION`, `SETTLE` and every other bound keep their values; no `.feature` text changes; no scenario is dropped, tagged `@wip` or retried.
  - `just test` and `just check` pass after the Green task group (the Red group fails by design), and `just full-check` on the final commit.
- **Out of scope:**
  - App code: the pipeline's order (paste, echo to stderr, hide) is right and stays.
  - Steps checked and found safe: the processing-stages `Then` steps, which wait for `Hidden`, the last event after the stop, before they read; the voice-control steps that compare positions, which read only after waiting for the latest event they compare, and "the listening-on sound played when the service came back", which reads after a wait for a later event of the same pipeline thread; "the text is pasted at once", since `WaitingShown` would come before the paste.
  - Checks for what must not happen, which watch for `SETTLE` by design (`nothing_pasted`, `never_waited`, `no_recording`, `never_since_mark`).
  - The wait in `service_went_away_and_is_back`, which the listening-on sound from the app's start already meets: the pipeline handles its inputs in order, so nothing races, and `show-spotting-away` rewrites these outage steps.
  - Other flakes (`http_failure_it` `should_tell_a_busy_answer_as_busy`, seen once), time limits for tests (`fail-hung-tests-fast`), CPU contention between parallel runs (`test-without-machine-lock` design decision 2), E2E and adapter integration tests.

## Release

No release: no app behavior changes; `version` in `app/Cargo.toml` keeps its value.
