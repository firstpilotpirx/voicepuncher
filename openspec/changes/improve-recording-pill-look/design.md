## Context

The recording pill (`app/src/adapters/secondary/visualfeedback/macos/`) draws the level waveform from `LevelHistory` (`app/src/core/domain/level_history.rs`): one column per 200 ms holding the peak of the smoothed level, the level mapped linearly from -50 to -10 dBFS, 30 columns 6 pt apart (the last 6 s, 176.5 pt). The pill is `NSGlassEffectView` glass with continuous corners of radius `PILL_HEIGHT / 2` (a capsule), a 1 pt white rim at 60 % and a soft shadow (drop 8, blur 24, opacity 0.2).

The user's recordings (`~/.voicepuncher/voice-samples/`, 39 759 frames of 10 ms, the capture's own RMS frame) put the room at -80…-65 dBFS (5th–50th percentile -79…-67) and the speech at -55…-25 dBFS (70th -52, 90th -39, 99th -25): with the -50…-10 range, most speech fills a third of the bar and nothing reaches the top.

The preparation settled the open points: the range and noise floor from these measurements; a 1 pt white hairline at about 15 % over a soft shadow, like the system HUDs; a column period of 100 ms; no mockup and no stop, the user checks the look after the merge. `widen-recording-pill` is absorbed: its length and outline land here.

**Goals:** silence as dots, the user's speech as columns of clearly different heights; the pill as long as ChatGPT's voice bar; notification corners; a faint hairline.

**Non-goals:** the pill's height, colours, position, its other states' content; the text window.

## Decisions

1. **Mapping:** `QUIET_DBFS` -62 (above the room's median of -67, so its noise stays a dot), `LOUD_DBFS` -28 (the user's 98th percentile), linear in dB between them. The user's 70th percentile (-52) maps to 0.29, the 90th (-39) to 0.68.
2. **Smoothing:** `RISE_RATE` 0.6 and `FALL_RATE` 0.3 per level update (about 20 ms): a syllable reaches its height within one column, and a pause falls back within two, so speech shows as spikes instead of a plateau.
3. **Period and density:** `COLUMN_PERIOD` 100 ms; `COLUMN_PITCH_PT` 4 (was 6) and the column 2 pt wide (was 2.5), so the strip slides 40 pt/s, near the 30 pt/s it had; `FULL_CAPACITY` 90 columns, the last 9 s, 358 pt — about twice the old 176.5 pt, the ChatGPT bar's length beside the record sign; `CAPACITY_BESIDE_SIGN` 78, the sign keeping its 48 pt.
4. **Outline:** `PILL_CORNER_RADIUS` 18 (about a third of the 56 pt height, the look of a macOS notification) with the continuous corner curve the glass already uses (`kCACornerCurveContinuous`), instead of `PILL_HEIGHT / 2`. The shadow mask keeps its circular rounded rectangle of the same radius: the continuous curve lies inside it, so no shadow falls on the glass.
5. **Border:** `RIM_WHITE_ALPHA` 0.15 (was 0.6), the same 1 pt width all around; the shadow stays as it is.

## Risks / Trade-offs

- Tuned on one user's microphone and room: another microphone may sit louder or quieter. Mitigation: the range is wide (34 dB) and a constant pair; a fix is a one-line change.
- A 9 s strip slides at 40 pt/s, faster than before: within the "slow scrolling" of the requirement, checked by the user in the app.

## Test mapping

| Scenario | Test |
|---|---|
| The overlay over a dark window shows light content (visual-feedback) | manual run after the merge (the user checks the look) |
| The overlay over a light window shows dark content (visual-feedback) | manual run after the merge (the user checks the look) |
| The glass overlay takes no focus (visual-feedback) | component `visual_feedback.feature` `The glass overlay takes no focus and has no bright top line` |
| The window under the overlay shows through the glass (visual-feedback) | manual run after the merge (the user checks the look) |
| The glass has no bright line along its top edge (visual-feedback) | component `visual_feedback.feature` `The glass overlay takes no focus and has no bright top line` |
| The pill has notification corners, not round ends (visual-feedback) | unit `window_set.rs` `should_round_the_pill_like_a_notification_not_a_capsule` |
| Speech leaves tall columns that slide to the left (visual-feedback) | component `visual_feedback.feature` `Speech leaves tall columns` |
| Silence leaves a dotted line (visual-feedback) | unit `level_history.rs` `should_draw_room_noise_as_dots` |
| Quiet and loud speech leave columns of different heights (visual-feedback) | unit `level_history.rs` `should_draw_quiet_and_loud_speech_at_distinct_heights`, `should_map_the_noise_floor_and_below_to_no_loudness`, `should_map_loud_speech_and_above_to_full_loudness` |
| A new recording starts with a dotted waveform (visual-feedback) | component `visual_feedback.feature` `A new recording starts with a dotted waveform beside the record sign` |
| The waveform shows together with the record sign (visual-feedback) | component `visual_feedback.feature` `A new recording starts with a dotted waveform beside the record sign` |
| The waveform keeps the last six seconds (visual-feedback) | unit `level_history.rs` `should_forget_speech_older_than_nine_seconds` |
| The waveform moves at half its former pace (visual-feedback) | unit `level_history.rs` `should_add_ten_columns_for_one_second_of_speech` |
| The record dot pulses in silence too (visual-feedback) | component `visual_feedback.feature` `The record dot pulses only without Reduce Motion, and the waveform still moves` |

The scenarios "The waveform keeps the last six seconds" and "The waveform moves at half its former pace" keep their titles (a MODIFIED requirement keeps every scenario title); their bodies now state nine seconds and ten columns a second.
