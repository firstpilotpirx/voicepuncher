## Why

The overlay (the pill at the top of the screen, the text window under it and the 🎙️ sign next to the mouse pointer) still looks like a flat light-blue plate with text glyphs (`◉ REC`, `⌨ FOCUS`, `⏎ ENTER`, `⚠ STT`). macOS 26 Tahoe draws widgets and panels in Liquid Glass. The user compared six glass styles of the pill on seven backgrounds (white, grey, black, blue with text, a muddy texture, coloured dots, coloured stripes) in mockups in chat and picked the regular frosted glass. Then they validated mockups of every overlay state in that style. The user also wants the pill to show progress honestly: speech recognition and text refinement report no progress, so a spinner must not look like a filling progress ring. And the user wants the equalizer to visibly follow the microphone.

## What Changes

- The pill and the text window under it become regular frosted glass: the system material that blurs and adapts to light or dark content beneath, rounded corners (~28 pt on the pill), a light rim highlight and a soft shadow. Icons replace the text glyphs. Every user-facing overlay label is in English.
- Recording: a red record dot inside a ring, pulsing all the time, and a new equalizer: about 12–14 thin rounded vertical bars driven by the live microphone level. The bars jump with speech and collapse into a flat line in silence. It replaces the current equalizer.
- Live transcript: unchanged in behaviour (up to two lines under the pill), drawn in the glass text window.
- Speech recognition: an ear icon that breathes (fades out and back in), the label "Recognizing" and an indeterminate macOS-style spoke spinner that says nothing about when it ends.
- Text refinement: a sparkles icon breathing in the same rhythm, the label "Refining text" and the same spoke spinner. The raw transcript under the pill is dimmed while it is refined.
- Waiting for focus: orange-tinted glass, a breathing keyboard icon, the label "Click a text field" and the line "Text is ready and pastes once a text field has focus" under the pill. It replaces `⌨ FOCUS`.
- New "Pasted" state: a green check, the label "Pasted" and the final text under the pill for about 0.5 s, then the overlay hides. Today it hides at once.
- Command signs (Enter, Clear, Restart): an icon and the English name in the glass pill for about 1 s, as today, restyled.
- Failure notice: amber-tinted glass with a warning triangle and the failing part's short name (STT, REFINE, VOICE, MIC, ACCESS, FAILED), the message (two lines at most) in the glass window under it. It hides after 4 s as today and stays legible over light app windows (darker icon and text there).
- Commands-unavailable sign: a small grey glass pill with a microphone-off icon and "CMD" alone in standby. During a recording it shows to the right of the equalizer, after a thin separator.
- Pointer sign: the 🎙️ emoji next to the mouse pointer becomes a small round dark-glass badge (~38 pt) that mirrors the overlay at every stage: a red microphone while recording, the ear or the sparkles with a rotating indeterminate ring while processing, a pulsing orange keyboard while waiting for focus, a green check after the paste, an amber triangle on a failure. The system arrow cursor stays as it is: only the badge window next to it changes.

Current stage: PoC. Target stage: PoC.
Scope: the look and the states of the overlay pill, its text window and the pointer sign, including the new Pasted state and the level-driven equalizer.
Acceptance bar: every state above appears in the manual run as in the validated mockups, legible over light, dark and busy app windows; the equalizer follows speech and goes flat in silence; the spinners are indeterminate; the user pastes a healthy log of a manual run.
Out of scope: when each state shows and for how long (except the new Pasted state), the failure notice wording, the pointer sign's distance from the cursor (change bring-dictation-cursor-mic-closer), voice commands, any non-overlay UI.

## Capabilities

### Modified Capabilities

- `visual-feedback`: the overlay's look, its icons and English labels, the level-driven equalizer, the indeterminate spinners, the new Pasted state and the pointer sign's state-mirroring badge.

## Impact

- `app/src/adapters/secondary/visualfeedback/` (macOS overlay windows, pill, text window, pointer sign).
- `app/src/core/application/visual_indicator.rs` and the pipeline, for the Pasted signal and the microphone level feeding the equalizer.
- `docs/ubiquitous-language.md` (Pasted sign, pointer sign states).
- Slice/component `.feature` files of the overlay capability.

## Open points

- The capability id: proposed `visual-feedback`; the spec stage picks the existing overlay capability if it is named differently.
- Glass implementation: proposed the system material (`NSGlassEffectView` on macOS 26, falling back to `NSVisualEffectView` on older macOS) rather than hand-drawn blur.
- Icons: proposed SF Symbols (`ear`, `sparkles`, `keyboard`, `checkmark`, `return`, `delete.left`, `arrow.clockwise`, `exclamationmark.triangle`, `mic.slash`, `mic.fill`) instead of the mockup's web icons.
- Pasted state duration: proposed 0.5 s; the user said "about half a second is fine".
- Whether the pointer badge also shows command signs: proposed no, it keeps the state it shows.

## Release

SemVer: MINOR — a new visible state (Pasted) and a new pointer sign behaviour.
Target version: 0.22.0.
