# Design

## Context

The command spotting adapter (`commandspotting/websocket`) already tracks an outage (`link::Outage`, since `lost_at`) and wakes every 10 ms while the service is away. The core learns of the loss and the return through `PipelineInput::SpottingLost` / `SpottingBack` and today answers both with a listening sound. The overlay adapter keeps one `OverlayState` (recording, a processing stage, waiting for focus, hidden) and redraws the label from it.

## Goals / Non-Goals

**Goals:** silence on loss and return; a `⊘ CMD` overlay sign after 5 s of outage, until the service is back.

**Non-Goals:** notices for other failures (`notify-service-failures`), any change to reconnecting or the replay.

## Decisions

1. **The adapter measures the threshold and sends a new input, `PipelineInput::SpottingStillAway`, once per outage, when the outage reaches 5 s.** The adapter already owns the outage clock and wakes often enough; the core stays clock-free and event-driven, so component tests drive it with inputs alone. `SpottingLost` keeps its meaning and timing, because the command de-duplication (`carried_before_outage`) depends on the moment of the loss. Rejected: a timer in the core (a new clock port and tick input for one threshold); delaying `SpottingLost` itself (breaks the de-duplication). The 5 s equals `REPLAY_BEFORE_LOSS`: an outage shorter than that is fully covered by the replay, so a command said in it still works and there is nothing to tell the user.

2. **The sign is a flag on the overlay, orthogonal to its state.** `VisualIndicator` gains `show_commands_unavailable()` and `hide_commands_unavailable()`, with no-op defaults like `show_command_sign`. The macOS adapter keeps the flag beside `OverlayState` and composes the label: `◉ REC ⊘ CMD` while recording, the stage icon followed by `⊘ CMD` while processing or waiting for focus, `⊘ CMD` alone when the state is hidden; the window widens to the text as for a failure notice. `hide()`, the end of a failure notice and the end of a command sign land on a state, so the sign comes back with it. A failure notice or command sign shows alone for its time. The core never restores "what the overlay showed before". Rejected: a new `OverlayState` (it would replace the recording sign and force the core to restore the previous state).

3. **The core shows the sign only while it counts the service as away.** On `SpottingStillAway` with listening on and the service away, it calls `show_commands_unavailable()` once; on `SpottingBack` it calls `hide_commands_unavailable()` only if it showed the sign. `lose_spotting` and `regain_spotting` no longer play a sound; the listening-on sound at app start stays.

4. **E2E outage scenarios detect loss and return from the pipeline inputs, not from sounds.** The hands-free harness tees the input stream into its event log (`SpottingLost`, `SpottingBack`) and records the overlay double's calls; "listening is still on" means no `SpottingLost` was seen.

## Risks / Trade-offs

- The label gets longer while recording; `⊘ CMD` is short and the overlay already sizes to its text. The manual run confirms readability.
- `notify-service-failures` also changes the overlay. Whichever lands later builds on the flag (decision 2) rather than adding a competing state.

## Dependencies

`show-live-transcript`, `confirm-voice-commands` and `notify-service-failures` are on `main`; the sign composes with the command sign and the failure notice (decision 2); the notice covers only listening that cannot start, so the runtime outage stays this change's. Waits for `show-dictation-cursor`: its `PointerSignIndicator` decorator implements `VisualIndicator` and would swallow the new calls through their defaults, so this change forwards them in it.

## Test levels

| Scenario | Level |
|---|---|
| The app keeps reaching for the command spotting service when it goes away | component `.feature` (no sound); retry logging stays in `commandspotting_reconnect_it` |
| Commands work again after the command spotting service restarts | component and E2E `.feature` |
| Listening is not given up after many failed attempts | adapter integration (`commandspotting_reconnect_it`, unchanged) |
| A short outage shows nothing | adapter integration (no `SpottingStillAway` when back within 5 s) + component `.feature` |
| A long outage in standby shows the sign until the service is back | adapter integration (`SpottingStillAway` once after 5 s; `visualfeedback_macos_it`: the sign alone shows and hides with the flag) + component + E2E `.feature` |
| A long outage during a dictation shows the sign beside the recording sign | component `.feature`; label composition in unit tests of the overlay adapter |
| A dictation that ends during a long outage keeps the sign | component `.feature`; `visualfeedback_macos_it`: `hide()` and the end of a failure notice keep the sign |
| Overlay readable, takes no focus | manual run |
