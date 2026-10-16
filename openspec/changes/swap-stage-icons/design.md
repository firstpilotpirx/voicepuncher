# Design

## Context

- `stage_icon` in `app/src/adapters/secondary/visualfeedback/macos/mod.rs` maps `OverlayState::RecognizingSpeech` to 👂 and `OverlayState::RefiningText` to ✏️. `apply_processing_state` is its only reader: it writes the glyph into the overlay's left label and starts the spinner.
- `window_set.rs` builds the 174 × 55 pt overlay: a label in the 28 pt bold system font at x 6–70 that centers its text, and a 32 pt spinner at x 71–103, centered in the window. So the icon already sits left of the spinner.
- Tests today:
  - The unit test `should_give_each_processing_stage_an_icon_of_its_own` checks that there are two icons, that they differ, and that neither matches ◉ REC or ⌨ FOCUS.
  - The component test `processing_stages.feature` drives the real `Pipeline` with a recording `VisualIndicator` double. It checks which stage is signalled and when. It never sees an icon.
  - The tests in `visualfeedback_macos_it.rs` need the main thread. The test harness doesn't provide one, so they return before asserting (archived `show-processing-stages`, design § Test levels).
- Motivation: proposal.md § Why.

## Goals / Non-Goals

**Goals:**

- Swap the two glyphs, pinned first by unit tests that fail on today's glyphs.

**Non-Goals:**

- Any edit to `window_set.rs`, the `VisualIndicator` port, the pipeline or `processing_stages.feature`.
- A screenshot or pixel test of the overlay.

## Decisions

### D1. Two literals in `stage_icon`

`stage_icon` returns 🎧 (U+1F3A7) for `RecognizingSpeech` and ✨ (U+2728) for `RefiningText`. The doc comment on `apply_processing_state` changes from "beside it" to "to its right". Nothing else changes.

- Both glyphs are single code points that show as emoji by default, so neither needs U+FE0F (✏️ did). Checked on macOS 27.0.1 with the label's 28 pt bold system font: both AppKit's font substitution (`NSTextStorage::fixAttributes`) and CoreText draw each glyph from Apple Color Emoji. They came out in color even with the default black text color.
- Alternative considered: named constants like `RECORDING_SIGN`. Rejected because a test that compares `stage_icon` with the constant pins nothing. The glyph should appear once in `stage_icon` and once, spelled out, in its test.

### D2. Unit tests pin the glyphs; no `.feature`

Two tests go in `mod tests` of `mod.rs`, Red first:

- `should_show_headphones_for_the_speech_recognition_stage` asserts that `stage_icon(OverlayState::RecognizingSpeech) == Some("\u{1F3A7}")`.
- `should_show_sparkles_for_the_text_refinement_stage` asserts that `stage_icon(OverlayState::RefiningText) == Some("\u{2728}")`.

Both fail today on 👂 and ✏️. The expected values are written as `\u{…}` escapes, so a stray U+FE0F on either side fails the test. `should_give_each_processing_stage_an_icon_of_its_own` stays as it is and stays green.

There is no Gherkin test for the icon:

- The glyph is picked inside the macOS adapter, behind the `VisualIndicator` port. The component harness only sees the signals its double records, and the existing scenarios already check those. A component step about the icon could not fail on a wrong glyph.
- The adapter can't run under cucumber because there is no main thread. E2E runs the binary without the overlay.
- This follows `gherkin.md`: a test of one adapter's technical contract is a plain test. It also follows the precedent of `show-processing-stages`. `processing_stages.feature` keeps no `Rule:` for this requirement, as it hasn't since 0.14.0.

### D3. The layout keeps its code; the manual run proves it

The icon-left, spinner-right placement comes from the frames in `window_set.rs` (label x 6–70, spinner x 71–103), and this change does not touch them.

There is no automated frame test, for three reasons:

- The frames are computed inline in `WindowSet::build`, which needs the main thread.
- A pure test would first have to move that geometry out of `build`, refactoring a file this change keeps.
- That test would pass on its first run, so it would prove nothing new (AGENTS.md § TDD).

The manual run checks the layout, as the acceptance bar requires.

### D4. Light and dark appearance: the user's manual run only

The user runs `just start-model-servers`, then `cargo run --release -p voicepuncher`, once in light appearance and once in dark. In each run they end one dictation with speech and text refinement on, and check:

- 🎧 shows with the spinner turning to its right, then ✨ the same way, then the overlay hides at the paste.
- Both icons are in color, whole and legible.

A third run with `--no-llm` shows only 🎧. ◉ REC, ⌨ FOCUS and the `stop→paste` time stay as before. The user pastes the log.

No automated test can prove this scenario:

- Whether a glyph shows in color, whole and legible is settled only when it is drawn. That depends on the system emoji font and AppKit on a live window server, against the overlay's `Menu` vibrancy material. The material blends with whatever lies behind the overlay and changes with the appearance.
- `cargo test` has no main thread to build the overlay. Capturing it would need the Screen Recording permission, which the agent sandbox lacks.
- "Legible" is a human judgement.
- An offline render like the D1 check shows the glyph, not the overlay: it misses the material and whatever is behind the window.

### D5. Version

PATCH, 0.14.1 → 0.14.2 in `app/Cargo.toml` (also 0.14.1 on `origin/main` today). The last task group bumps it. If another change bumps first, take the next patch above `main` (proposal § Release).

## Test levels

| Scenario | Level |
|---|---|
| Speech recognition shows headphones left of the spinner | Unit: `should_show_headphones_for_the_speech_recognition_stage` pins the icon. Component (unchanged): `processing_stages.feature` "A hotkey dictation shows speech recognition after the stop" shows the stage appears at the stop. Manual run: the icon sits left of the turning spinner. |
| Text refinement shows sparkles left of the spinner | Unit: `should_show_sparkles_for_the_text_refinement_stage` pins the icon. Component (unchanged): "Text refinement follows speech recognition" shows the switch when refinement starts. Manual run: the icon sits left of the turning spinner. |
| The two stages look different | Unit (unchanged): `should_give_each_processing_stage_an_icon_of_its_own`. Manual run. |
| The stage icons read in light and dark appearance | Manual run only, in both appearances (D4). |

Checks:

- `just test` after every edit.
- `just check` after the task group, because `app/` is a build input.
- `just full-check` before the merge.

## Risks / Trade-offs

- [🎧 contrast] 🎧 may read poorly against the overlay's material in one of the two appearances. The D1 check rendered it at 28 pt on solid stand-ins for the material (#EDEDED, #2B2B2B, #1A1A1A). On macOS 27, Apple Color Emoji draws 🎧 as silver headphones. They are clear on both dark backdrops; on the light one they are paler than ✨ but outlined. → The manual run judges both appearances (D4), with the light one as the one to watch. If either fails, there are two fixes, and neither is built ahead:
  - A backing behind the icon, inside the adapter.
  - Another glyph picked by the user, which means a spec change.
- [The emoji font is the platform's] A later macOS may redraw both glyphs. → Accepted. This change's manual run checks how they look today.
- [Parallel overlay changes] `show-spotting-away`, `show-live-transcript`, `confirm-voice-commands` and `notify-service-failures` edit the same adapter but not `stage_icon`. → Whichever lands later merges on top of the other (proposal § Impact).

## Migration Plan

None. No config, CLI, port or stored data changes. Rollback is a revert.
