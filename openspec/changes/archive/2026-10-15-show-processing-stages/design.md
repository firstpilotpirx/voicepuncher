## Context

- The port `VisualIndicator` (`app/src/core/application/visual_indicator.rs`) has four signals: `show_recording`, `show_processing`, `show_waiting_for_focus`, `hide`.
- `Pipeline::stop_recording` (`app/src/core/application/pipeline.rs`) calls `show_processing` right after the stop sound, before it knows how the recording is transcribed. Then one of two paths runs:
  - **Whole recording:** `run_transcribe_refine_paste` transcribes, then calls `refine_or_raw`, then `deliver`.
  - **Live chunks:** the live worker transcribed most chunks while the user spoke; `stop_recording` flushes the last chunk, waits for the worker, merges, then `run_refine_paste` calls `refine_or_raw` and `deliver`.
- A dictation ended by voice (`pipeline/voice_control.rs`) and one ended by the hotkey both go through `stop_recording`.
- `--no-llm` wires `PassthroughTextRefiner`; `refine_or_raw` still runs (it strips filler sounds), so the pipeline cannot tell from the call that refinement is off.
- `deliver` → `insert_text` calls `show_waiting_for_focus` when it holds the text back; the pipeline calls `hide` after every end.
- The macOS overlay (`adapters/secondary/visualfeedback/macos/`) has one `NSTextField` label on the left (REC, FOCUS), an equalizer label and a centered spinner. Processing hides the label and shows the spinner.
- Component tests drive the real `Pipeline` with doubles that log ordered events (`app/tests/steps/voice_control/doubles.rs`).
- `close-focus-wait-gaps` and `fix-short-pause-after-command` run in parallel and touch `pipeline.rs` and `text_insertion.rs`; expect merges.

## Goals / Non-Goals

**Goals:**

- The overlay shows speech recognition from the stop until the transcript is ready and text refinement from its start until the corrected text is ready.
- Text refinement is never shown when it is disabled.
- The two stages are told apart by their icon.
- Stop→paste time stays as today.

**Non-Goals:**

- Progress within a stage, text on the overlay, a sound per stage, stage signs for file transcription (proposal § Out of scope).

## Decisions

### D1. Two signals replace `show_processing`

`VisualIndicator` gets `show_recognizing_speech` and `show_refining_text`; `show_processing` is removed. Every implementation (macOS overlay, `NoopVisualIndicator`, test doubles) implements both.

*Alternative:* `show_processing(stage: ProcessingStage)`. One method with a domain enum is equally small, but two methods match the existing one-method-per-signal style and the mocks' expectations read more plainly.

### D2. Where the pipeline sends each signal

- `show_recognizing_speech` replaces `show_processing` in `stop_recording`, before either path. With live chunks it covers flushing and draining the last chunk; that is the transcription still running after the stop.
- `show_refining_text` is sent right before `refine_or_raw` in both `run_transcribe_refine_paste` and `run_refine_paste`, only when text refinement is enabled.
- A dictation dropped before refinement (no speech, too short, only command words, a failed transcription) hides the overlay as today and never sends `show_refining_text`.

### D3. The pipeline is told that text refinement is off

`Pipeline::without_text_refinement_stage()` (builder, like `with_focus_wait`) sets a flag; `main.rs` calls it with `--no-llm`. The refiner port stays unchanged.

*Alternative:* a `TextRefiner::refines()` method. It would add an expectation to every `MockTextRefiner` in the unit tests and puts an overlay concern on the refiner port.

### D4. No minimum time on screen

With live chunks the speech recognition stage after the stop may last tens of milliseconds. It is not held on screen: holding it would either delay the paste (against the acceptance bar) or show a stage that is no longer running. The manual run judges whether the flash is a problem; if so, a follow-up change adds a hold.

### D5. Icons on the macOS overlay

Each stage shows its icon in the left label and keeps the centered spinner turning, so the overlay still shows work under way:

- speech recognition: 👂 (the app is still hearing what was said);
- text refinement: ✏️ (the text is being corrected).

Both are color emoji with different shapes, distinct from ◉ REC (red text) and ⌨ FOCUS (orange text). The overlay state enum gets `RecognizingSpeech` and `RefiningText` instead of `Processing`, so a screen change redraws the right stage.

### D6. Glossary

`docs/ubiquitous-language.md`: a **Processing stage** term (speech recognition or text refinement, as shown on the overlay) and the `VisualIndicator` definition lists its signals.

## Test levels

| Scenario | Level |
|---|---|
| A hotkey dictation shows speech recognition after the stop | component `processing_stages.feature` |
| A dictation ended by voice shows speech recognition after the stop | component `processing_stages.feature` |
| A dictation transcribed while the user speaks shows speech recognition for its last part | component `processing_stages.feature` |
| A dictation without speech hides the overlay after speech recognition | component `processing_stages.feature` |
| Text refinement follows speech recognition | component `processing_stages.feature` |
| With text refinement disabled only speech recognition is shown | component `processing_stages.feature` |
| Text held back for focus shows the wait after text refinement | component `processing_stages.feature` |
| The two stages look different | unit (the two icons differ) + manual run |

Pipeline unit tests (`pipeline.rs`, `pipeline/voice_control.rs`) switch their expectations to the new signals. The adapter integration tests in `visualfeedback_macos_it.rs` need the main thread, which the test harness does not give them, so they return before asserting; they only follow the signal rename. What the overlay draws for each stage is proven by the unit test on the stage icons and by the manual run. The component steps record when the transcriber and the refiner start and finish, so a scenario checks the stage is shown before the step starts and switched only after it finished.

## Risks / Trade-offs

- **The speech recognition stage flashes with live chunks** → accepted (D4); the manual run decides.
- **Emoji rendering in a 28 pt bold system font** → both glyphs render via Apple Color Emoji fallback; checked in the manual run.
- **Parallel changes edit `pipeline.rs`** → merge `origin/main` before the PR and keep this change's edits to the signal calls.
