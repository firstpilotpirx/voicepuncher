# Proposal

## Why

During a dictation the overlay shows only that a recording runs; the user learns what was recognized only after the text is pasted. A long recording is already transcribed in chunks that are stitched at their seams, so the text recognized so far could be shown while the user speaks. Seeing it lets the user catch a misrecognition early and re-say or cancel instead of fixing the pasted text.

## What Changes

- During a dictation the overlay shows the text recognized so far, next to the recording sign, and updates it as each chunk is transcribed and stitched to the text before it.
- The shown text is the raw transcript; the pasted text may still differ after refinement.
- A long text shows its latest part, so the overlay keeps a bounded size.
- The same for a dictation started by the hotkey and one started by voice.

## Capabilities

### New Capabilities

- `live-transcript`: showing the text recognized so far while the user dictates.

### Modified Capabilities

None.

## Scope

- Current stage: PoC. Target stage: PoC.
- In scope: showing the stitched transcript of the chunks transcribed so far in the overlay during a dictation, for push-to-talk and voice control.
- Acceptance bar: in a dictation longer than one chunk, the overlay shows the stitched text of every chunk transcribed so far shortly after each chunk is transcribed; the pasted text and the time from stop to paste are as today.

## Out of scope

- Editing the shown text in the overlay.
- Showing refined text while the user speaks.
- Updates finer than a chunk (word-by-word streaming).
- A history of earlier dictations.

## Release

MINOR (new capability), next minor version after the current one in `app/Cargo.toml` (0.15.3 → 0.16.0 at the time of writing).

## Impact

- The visual indicator gains a "text so far" signal; the overlay adapter draws text (wrapping, size, latest part only).
- The chunked transcription step passes the stitched text so far to the visual indicator.
- `docs/ubiquitous-language.md`: a term for the text shown while dictating.
- Manual run by the user (overlay, recording).
