# Overlay redesign — design spec

The look the user validated for `redesign-overlay-liquid-glass`. Read it with the pictures in this folder:
the implementation is done when a screenshot of each state in the manual run matches the picture of that
state here (same parts, same proportions, same colours on the same kind of background).

| File | What it is |
|---|---|
| `overlay-states-overview.png` | every state on one page, with the user-facing notes (Russian) under each |
| `states/<NN>-<state>--blue.png` | one state over a saturated blue window (the user's IDE) — the main reference |
| `states/<NN>-<state>--light.png` | the same state over a light window — glass turns light, text turns dark |
| `states/<NN>-<state>--dark.png` | the same state over a dark window |
| `overlay-states.html` | the mockup source; open it in a browser to see the animations. `?only=<state id>`, `?bg=light\|dark`, `?still=1` pick one state, one background, no motion |
| `icons/sf-symbols-preview.png` | the SF Symbols the app uses, in their colours |
| `icons/tabler/*.svg` | the exact outline icons the mockups draw (Tabler Icons, MIT, `icons/tabler/LICENSE`) — fallback only |

All pictures are rendered at 3× (1 pt = 3 px). Sizes below are in points.

## 1. Principles

- **Regular frosted glass** (the user compared six glass styles on seven backgrounds and picked this one):
  the background behind the pill is strongly blurred and slightly saturated, the pill has a light rim and a soft
  shadow. It adapts to what is under it: over dark or saturated windows the glass is light-on-dark with white
  content; over light windows it turns into milky glass with dark content (`--light` pictures).
- **Use the system material, not a hand-drawn blur.** macOS 26: `NSGlassEffectView` (regular style,
  `cornerRadius` as below, `tintColor` for the tinted states). Older macOS: `NSVisualEffectView`
  (`.hudWindow` material, `.behindWindow` blending, `state = .active`, layer `cornerRadius`, `masksToBounds`).
  The rgba values in § 3 are what the mockup used to imitate that material in a browser; match the look, not
  the numbers, when the system material already gives it.
- **Content colours come from the system** (`labelColor`, `secondaryLabelColor`, `systemRed`, …) so the glass
  flips light/dark on its own. Only the tints and the record red are fixed.
- **Icons are SF Symbols** (§ 4), drawn with `NSImage(systemSymbolName:)` — nothing to ship. The Tabler SVGs are
  only for a platform without SF Symbols.
- **Progress is honest.** Speech recognition and text refinement report no progress: their spinner is the
  indeterminate macOS spoke spinner (`NSProgressIndicator`, `.spinning`, indeterminate), never a ring that fills.
- **All user-facing labels in English.**
- **Reduce Motion** (`NSWorkspace.shared.accessibilityDisplayShouldReduceMotion`): no breathing, no dot pulse, no
  rise-in; spinners still move and the waveform still scrolls. **Reduce Transparency**: the material turns opaque by itself;
  keep the rim and the tints.

## 2. Geometry

### Pill

| Property | Value | Today |
|---|---|---|
| Height | 56 | — |
| Corner radius | 28 (fully round ends) | 20 |
| Width | hugs its content, at least 200; padding 22 left and right | fixed 174 |
| Content | one row, vertically centred, gap 14 between items (10 in the command sign with two items) | |
| Rim | 1 pt inner stroke, white 60 % (light content: white 90 %) | none |
| Top highlight | 1 pt inner line along the top edge, white 85 % | none |
| Shadow | y 8, blur 24, black 20 % (light content: 12 %) | |
| Position | where the pill is today (top centre of each screen) | |

### Text window (under the pill)

Used by the live transcript, the refinement preview, the waiting explanation, the pasted text and the failure
message.

| Property | Value | Today |
|---|---|---|
| Gap below the pill | 8 | 6 |
| Width | 480 (as today; the mockup scales it to 84 % of a narrower scene) | 480 |
| Corner radius | 16 | 12 |
| Padding | 9 top/bottom, 14 left/right | 10 |
| Text | system font 13–14 regular, line height 1.4, at most 2 lines, start-aligned | 14, 2 lines |
| Material, rim, highlight | as the pill, shadow y 6 blur 18 black 15 % | |
| Appearance | fades in and rises 6 pt in 0.5 s ease-out | |

### Items inside the pill

| Item | Size |
|---|---|
| Record sign | ring 26 diameter, 2 pt stroke `systemRed`; dot 18 diameter inside, `systemRed` |
| Waveform | a scrolling strip of level columns (§ 6): 30 columns (22 when the commands-unavailable sign shares the pill), each 2.5 wide with fully round ends, 3.5 apart (pitch 6), so 176.5 wide (128.5 with 22); 28 tall area, vertically centred; a column is never shorter than 2; the left 30 pt fade out (alpha mask 0 → 1) |
| State icon | SF Symbol, point size 20–22, weight medium (it fills a 24 pt box) |
| Label | system font 13, weight medium, tracking +0.5 (≈ 0.04 em) |
| Spoke spinner | 20 × 20, `NSProgressIndicator` `.spinning`, `controlSize = .small`, indeterminate |
| Separator | 1 × 26, content colour at 30 % |

### Pointer badge

| Property | Value |
|---|---|
| Shape | circle, 38 diameter |
| Material | dark glass: black-ish 45 % over a 14 pt blur (`NSVisualEffectView` `.hudWindow`, `appearance = darkAqua`, or glass with a dark tint) |
| Rim | 1 pt, white 55 %; shadow y 4, blur 10, black 20 % |
| Icon | SF Symbol, point size 18–20, colour per state (§ 5.13) |
| Activity ring | while processing: 2 pt white 90 % arc of 270°, 2 pt inside the badge edge, one turn per 0.9 s, linear |
| Position | next to the pointer, as the pointer sign is today; its distance is the business of `bring-dictation-cursor-mic-closer` |
| System arrow | untouched — the app cannot replace another app's cursor; only the badge window changes |

## 3. Colours

| Token | Over dark / saturated content | Over light content |
|---|---|---|
| Glass fill (imitation) | white 26 %, blur 24, saturation 180 % | white 50 %, same blur |
| Content (icons, label, bars) | white (`labelColor` in vibrant dark) | `#1D1D1F` (`labelColor` in light) |
| Text window text | white | `#1D1D1F` |
| Record red | `systemRed` (`#FF3B30`) | same |
| Waiting tint | `systemOrange` at 42 %, rim `#FFD696` 80 % | same tint, rim 80 % |
| Failure tint | `systemOrange` (`#FF9F0A`) at 25–28 %, rim `#FFD696` 70 % | same |
| Failure icon | `systemYellow` (`#FFD60A`) | dark amber `#C27A00` (a yellow triangle disappears on white) |
| Pasted check | `systemGreen` (`#30D158`) | same |
| Commands-unavailable | content at 80 % (`secondaryLabelColor`) | same |
| Refinement preview text | text window text at 75 % | same |

## 4. Icons

| Use | SF Symbol | Tabler fallback | Colour |
|---|---|---|---|
| Record sign (pill) | `record.circle` (or two layers: ring + dot, § 2) | — | `systemRed` |
| Pointer badge: recording | `mic.fill` | `microphone.svg` | `systemRed` |
| Recognizing | `ear` | `ear.svg` | content |
| Refining text | `sparkles` | `sparkles.svg` | content |
| Waiting for focus | `keyboard` | `keyboard.svg` | content (pill), `systemOrange` (badge) |
| Pasted | `checkmark` | `check.svg` | `systemGreen` |
| Command: Enter | `return` | `arrow-back.svg` | content |
| Command: Clear | `delete.left` | `backspace.svg` | content |
| Command: Restart | `arrow.clockwise` | `refresh.svg` | content |
| Failure | `exclamationmark.triangle.fill` (palette: black mark, yellow triangle) | `alert-triangle.svg` | § 3 |
| Commands unavailable | `mic.slash` | `microphone-off.svg` | content 80 % |

Configure each with `NSImage.SymbolConfiguration(pointSize:weight: .medium)` and a palette colour.
`icons/sf-symbols-preview.png` shows them at 72 pt.

## 5. States

Each state lists its picture, its parts left to right, its text window and its motion. "Content" is the colour
of § 3.

### 5.1 Recording — `states/01-recording--*.png`
- Pill: record sign, waveform. No label.
- Text window: hidden until the first chunk has text (then § 5.2).
- Motion: the dot pulses all the time — scale 1 → 0.8 → 1 and opacity 1 → 0.75 → 1, 1.6 s, ease-in-out,
  forever; the ring stays still. The waveform scrolls right to left and shows the microphone level of the last 3 s (§ 6).
- Replaces: `◉ REC` and the current equalizer (its bars jump in place; the waveform scrolls instead).

### 5.2 Recording with the live transcript — `states/02-recording-live-transcript--*.png`
- Pill: as § 5.1.
- Text window: the live transcript, the latest words that fit 2 lines, `…` before cut-off earlier words (as today).
- Motion: the window rises in once; later updates change the text in place.

### 5.3 Speech recognition — `states/03-recognizing-speech--*.png`
- Pill: `ear` icon, label **Recognizing**, spoke spinner.
- Text window: hidden (or the live transcript stays, as today).
- Motion: the icon breathes — opacity 1 → 0.4 → 1, 1.4 s, ease-in-out, forever (`NSImageView.addSymbolEffect(.breathe)`
  on macOS 15+, else a layer opacity animation); the spinner turns.
- Replaces: `🎧` + spinner.

### 5.4 Text refinement — `states/04-refining-text--*.png`
- Pill: `sparkles` icon, label **Refining text**, spoke spinner.
- Text window: the raw transcript at 75 % opacity (it is being corrected).
- Motion: the icon breathes in the same rhythm as § 5.3; the spinner turns.
- Replaces: `✨` + spinner.

### 5.5 Waiting for focus — `states/05-waiting-for-focus--*.png`
- Pill: tinted orange glass (§ 3), `keyboard` icon, label **Click a text field**.
- Text window: **Text is ready and pastes once a text field has focus**.
- Motion: the icon breathes (§ 5.3). The pill no longer pulses as a whole.
- Replaces: the pulsing orange `⌨ FOCUS`.

### 5.6 Pasted (new) — `states/06-pasted--*.png`
- Pill: `checkmark` in `systemGreen`, label **Pasted**.
- Text window: the final pasted text, 2 lines at most.
- Timing: shown right after a successful paste for 0.5 s, then the overlay hides (fade out 0.2 s). Any later
  signal replaces it at once, as for the command sign.

### 5.7 Command sign: Enter — `states/07-command-enter--*.png`
- Pill: `return` icon, label **Enter**.
- Timing: about 1 s, then what showed before (as today). The text window keeps what it showed.
- Replaces: `⏎ ENTER`.

### 5.8 Command signs: Clear, Restart — `states/08-command-clear-restart--*.png`
- Pill: `delete.left` + **Clear**, or `arrow.clockwise` + **Restart** — one at a time; the picture puts both side by
  side, split by the separator, only to show both.
- Replaces: `⌫ CLEAR`, `↻ RESTART`.

### 5.9 Failure notice — `states/09-failure-notice-stt--*.png`
- Pill: failure-tinted glass, `exclamationmark.triangle.fill`, label = the failing part's short name
  (**STT**, **REFINE**, **VOICE**, **MIC**, **ACCESS**, **FAILED**).
- Text window: the notice message (`FailureNotice::message`, at most 2 lines), e.g.
  "Dictation lost: speech recognition failed — start the local services".
- Timing: 4 s, then hidden (as today).
- Replaces: yellow `⚠ STT`.

### 5.10 Failure notice over a light window — `states/10-failure-notice-access-light-window--*.png`
- As § 5.9; over light content the triangle is dark amber and the text dark, so the notice stays legible.

### 5.11 Commands unavailable, standby — `states/11-commands-unavailable-standby--*.png`
- Pill: a compact pill (padding 16, no minimum width): `mic.slash` + **CMD**, content at 80 %.
- Shown alone while nothing else shows; goes when the command spotting service is back (as today).
- Replaces: grey `⊘ CMD`.

### 5.12 Recording while commands are unavailable — `states/12-recording-commands-unavailable--*.png`
- Pill: record sign, waveform (22 columns), separator, `mic.slash` at 80 % (no label).
- Beside the processing and waiting states the same separator + `mic.slash` follow their spinner or label.

### 5.13 Pointer badge — `states/13-pointer-badge-recording--*.png`, `states/14-pointer-badges-all-states--*.png`
The badge mirrors the overlay at every stage:

| Overlay state | Badge icon | Colour | Motion |
|---|---|---|---|
| Recording | `mic.fill` | `systemRed` | none |
| Recognizing | `ear` | white | breathes; activity ring turns |
| Refining text | `sparkles` | white | breathes; activity ring turns |
| Waiting for focus | `keyboard` | `systemOrange` | breathes |
| Pasted | `checkmark` | `systemGreen` | none; hides with the overlay |
| Failure notice | `exclamationmark.triangle.fill` | `systemYellow` | none; hides with the notice |
| Hidden, command sign, commands unavailable alone | — (badge hidden; a command sign leaves the badge as it was) | | |

Replaces: the 🎙️ emoji that follows the pointer only while recording.

## 6. Waveform behaviour

The waveform is a strip of level history, like the voice input of chat apps: every new column enters at the right
edge and the whole strip slides slowly to the left, so the user sees how the dictation went over the last seconds.
Reference: `waveform-reference.png`, the scrolling waveform of the ChatGPT voice input (grey rounded columns, dots
in pauses).

- Input: the microphone level of the running recording, about 30–60 updates per second, as RMS of the latest
  capture buffer converted to dBFS and mapped linearly from −50 dB (0) to −10 dB (1), clamped.
- Smoothing: rise fast, fall slower — each update `level += (target − level) × 0.35` when rising and `× 0.12` when
  falling (at 60 fps ≈ 50 ms attack, 130 ms release).
- Columns: one column per 100 ms (10 per second). Its value is the peak of the smoothed level inside that 100 ms
  window; its height is `max(2, 28 × value)`, centred vertically. Columns are never recomputed once added.
- Scrolling: the strip moves left continuously at pitch / 100 ms = 60 pt/s, sub-pixel, on the display link —
  not in 6 pt jumps. When the offset reaches one pitch, the leftmost column is dropped and the new one appended on
  the right. With 30 columns the strip shows the last 3 s (2.2 s with 22).
- Silence: a column whose height is 2 is a dot drawn at 45 % of the content alpha; a pause reads as a dotted line
  that slides away, like the reference. When recording starts the strip is all dots.
- Left edge: an alpha mask fades the leftmost 30 pt from 0 to 1, so old columns dissolve instead of being cut.
- Colour: content colour of § 3 (white over saturated and dark content, `#1D1D1F` over light content).
- Reduce Motion: the strip still updates (it carries information) but steps one column per 100 ms instead of
  sliding.
- Draw it in one custom `NSView`/`CALayer` per pill (a ring buffer of column values, rounded rects), redrawn on a
  display link; no allocation per frame.
- Pictures: `states/01-*`, `02-*`, `12-*` show a still frame — a pause on the left (dots fading in), speech, a short
  pause, speech; `overlay-states.html` without `still=1` shows the motion.

## 7. Timing summary

| What | Duration | Curve |
|---|---|---|
| Record dot pulse | 1.6 s, forever | ease-in-out |
| Waveform column / scroll | 100 ms per column, 60 pt/s | linear |
| Icon breathing | 1.4 s, forever | ease-in-out |
| Spoke spinner | system | — |
| Pointer activity ring | 0.9 s per turn | linear |
| Text window rise-in | 0.5 s | ease-out |
| Command sign | ≈ 1 s | — |
| Pasted | 0.5 s + 0.2 s fade | ease-out |
| Failure notice | 4 s | — |

## 8. Checklist for the manual run

- [ ] Every state of § 5 appears and matches its `--blue` picture over a saturated window and its `--light` picture
      over a light window.
- [ ] The labels read Recognizing, Refining text, Click a text field, Pasted, Enter, Clear, Restart.
- [ ] The waveform slides right to left without jerks; speech leaves tall columns, silence a dotted line; the dot keeps pulsing either way.
- [ ] No spinner fills up; both processing states use the spoke spinner.
- [ ] The failure notice is legible over a white window.
- [ ] The pointer badge follows the pointer in every state of § 5.13 and the system arrow is unchanged.
- [ ] With Reduce Motion on, nothing breathes or pulses.
