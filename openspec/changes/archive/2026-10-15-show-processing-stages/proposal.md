# Proposal

## Why

From the moment a recording stops until the text is pasted, the overlay shows one spinner. Behind it two different steps run one after the other: speech recognition finishes the transcript, then text refinement corrects it. The user cannot tell which step is running, so a long wait looks the same whichever step is slow, and the user cannot tell whether the speech was recognized at all or the text is only being corrected.

## What Changes

- Recording looks as today.
- After the recording stops, the overlay shows the speech recognition stage, with its own icon, until the transcript is ready.
- When text refinement starts, the overlay switches to the text refinement stage, with a different icon, until the corrected text is ready.
- The two stages are told apart at a glance by their icons, without reading any text.
- With text refinement disabled, the text refinement stage never appears.
- What follows is as today: the overlay shows the wait for focus when the text is held back, and hides once the text is pasted or the dictation is dropped.
- The same for a dictation started by the hotkey and one started by voice.

## Capabilities

### New Capabilities

- `processing-stages`: the overlay shows which processing stage runs between the end of a recording and the paste.

### Modified Capabilities

None. The specs that say the overlay shows processing (`voice-control`, `text-insertion`) stay true: processing is now shown as its stages.

## Scope

- Current stage: PoC. Target stage: PoC.
- In scope: a separate overlay stage, each with its own icon, for speech recognition and for text refinement, in push-to-talk and voice control, with text refinement enabled and disabled.
- Acceptance bar: in every dictation that ends with a paste, the overlay shows the speech recognition stage from the stop until the transcript is ready, then the text refinement stage until the corrected text is ready, then the wait for focus or nothing, as today; with text refinement disabled only the speech recognition stage appears; the time from stop to paste is as today.

## Out of scope

- Progress within a stage (percent, time left).
- A stage sign while the user speaks; recording keeps today's sign.
- Showing the recognized or corrected text (see `show-live-transcript`).
- A sound per stage.
- Stage signs for file transcription.

## Release

MINOR (new capability): target version 0.14.0, the next minor version after the current one in `app/Cargo.toml`.

## Impact

- The visual indicator's single processing signal splits into a speech recognition signal and a text refinement signal; the pipeline sends each when its step starts, both when chunks are transcribed during the recording and when the whole recording is transcribed after the stop.
- The overlay adapter draws an icon per stage instead of the single spinner. When chunks are transcribed during the recording, the speech recognition stage after the stop covers only the last chunk and may be very short; design decides whether it is held on screen for a moment.
- `docs/ubiquitous-language.md`: the `VisualIndicator` definition lists its signals; a term for a processing stage.
- The overlay is also changed by `show-live-transcript` and `confirm-voice-commands`; whichever lands later builds on the other.
- Manual run by the user (overlay).
