# Proposal

## Why

Since 0.14.0 the overlay shows 👂 (ear) for the speech recognition stage and ✏️ (pencil) for the text refinement stage. Shown alternatives, the user picked 🎧 (headphones), which reads more as listening, and ✨ (sparkles), the usual sign of AI at work, which fits text refinement by a language model better than a pencil. The icons exist so the user knows at a glance which stage runs, so they should be the ones the user reads that way.

## What Changes

- The speech recognition stage shows 🎧 (headphones) instead of 👂 (ear).
- The text refinement stage shows ✨ (sparkles) instead of ✏️ (pencil).
- The layout is kept and becomes part of the spec: the stage icon on the left of the overlay, the spinner turning to its right. The overlay already draws it that way, so nothing moves.
- Recording (`◉ REC`) and the wait for focus (`⌨ FOCUS`) look as today; when each stage is shown, and for how long, is unchanged.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `processing-stages`: the requirement "Each processing stage has its own icon" names the two icons — headphones for speech recognition, sparkles for text refinement — and their layout: the icon on the left, the spinner turning to its right.

## Scope

- Current stage: PoC. Target stage: PoC.
- In scope: the two stage icons on the macOS overlay, the spec naming them and their place beside the spinner, the unit test that pins them.
- Acceptance bar: in the user's manual run with text refinement on, after the stop the overlay shows 🎧 with the spinner turning to its right until the transcript is ready, then ✨ with the spinner to its right until the corrected text is ready, then hides at the paste; with `--no-llm` only 🎧 appears; both icons show in color, whole and legible in light and dark appearance; `◉ REC`, `⌨ FOCUS` and the time from stop to paste are as before.

## Out of scope

- The recording and wait-for-focus signs; the overlay's size, position and look; the spinner itself.
- When the stages are shown, including the short speech recognition stage after live chunks (`show-processing-stages`, design D4).
- Icons drawn as images (SF Symbols) or animated; text on the overlay.
- Overlay states proposed by other changes (`show-spotting-away`, `show-live-transcript`, `confirm-voice-commands`, `notify-service-failures`).

## Release

PATCH, 0.14.1 → 0.14.2 (or the next patch above `main` if another change bumps it first). No new capability and nothing breaks — no config, CLI or port change; the change corrects how an existing requirement is drawn. Earlier MINOR bumps of existing capabilities each added something the app could not do before (new phrases, a new duty, a new overlay state); this one adds nothing.

## Impact

- `app/src/adapters/secondary/visualfeedback/macos/mod.rs`: `stage_icon` returns 🎧 and ✨; its unit test pins the two icons (Red first); the comment on `apply_processing_state` says the spinner turns to the icon's right.
- `window_set.rs` beside it: no change. Checked: the icon sits centered in the left label (x 6–70 of the 174 pt overlay), the 32 pt spinner right of it (x 71–103).
- No change to the `VisualIndicator` port, the pipeline, `processing_stages.feature` (its scenarios check which stage is shown, not its icon), `docs/ubiquitous-language.md` ("Processing stage" names no icon) or the README.
- Risk: on macOS 27, Apple Color Emoji draws 🎧 as silver headphones: clear on the overlay's dark appearance, paler (though outlined) on the light one; the manual run checks both appearances, the light one most closely.
- `app/` is a build input: `just check` after each task group, `just full-check` before the merge.
- Manual run by the user (overlay), with and without `--no-llm`.
- `show-spotting-away`, `show-live-transcript`, `confirm-voice-commands` and `notify-service-failures` also edit the macOS overlay adapter, not the stage icons; whichever lands later merges on the other.
