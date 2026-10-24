## Context

The pill's one width (`one_pill_width` in `pill_content.rs`) is the widest recording row plus padding: the record ring (26 pt), a gap (14 pt), the waveform (`waveform_width(FULL_CAPACITY)` = (columns − 1) × 4 pt + 2 pt) and 22 pt padding at each end. With 90 columns the widest row (beside the commands-unavailable sign) makes it 447 pt. The screenshot puts ChatGPT's dictation bar at 1.65 times our pill: about 728 pt.

## Goals / Non-Goals

**Goals:** the recording pill about as long as ChatGPT's bar, the waveform still spanning it.

**Non-Goals:** the column pitch, the heights, the height of the pill, a width that follows the screen.

## Decisions

1. **More columns, same pitch.** `FULL_CAPACITY` 90 → 160 (the last 16 s at 100 ms a column) gives a full row of 26 + 14 + 638 + 44 = 727 pt; the widest row stays the one beside the commands-unavailable sign (decision 2), so the one width is 727 pt (today 447 pt). Keeping the pitch keeps the waveform's look and scroll speed; only its history grows. A wider pitch was rejected: it would slow the visible scroll and thin the strip.
2. **Beside the commands-unavailable sign** the waveform gives up the same room as today (12 columns for the separator and the icon): `CAPACITY_BESIDE_SIGN` 78 → 148, so both recording rows stay within the one width.
3. The width stays derived from the column count: every state keeps the one width by the existing rule, nothing else changes.

## Risks / Trade-offs

- [A 727 pt pill covers more of a small screen] → still under half of a 13" display (1470 pt).
- [160 column layers per frame instead of 90] → the layers are reused every frame; the cost is a few more frame updates of the same kind.

## Test mapping

| Spec scenario | Test |
|---|---|
| The recording pill is as long as ChatGPT's dictation bar (visual-feedback) | component `app/tests/features/component/visual_feedback.feature` `The recording pill is as long as ChatGPT's dictation bar`, step in `app/tests/steps/visual_feedback/overlay/width.rs` |
| A new recording starts with a dotted waveform (visual-feedback) | component `visual_feedback.feature` `A new recording starts with a dotted waveform beside the record sign`; unit `level_history.rs` `should_draw_a_new_history_as_as_many_dots_as_its_capacity` |
| The waveform shows together with the record sign (visual-feedback) | component `visual_feedback.feature` `A new recording starts with a dotted waveform beside the record sign` (the first frame holds both) |
| The waveform keeps the last six seconds (visual-feedback) | unit `level_history.rs` `should_still_show_speech_from_less_than_sixteen_seconds_ago`, `should_forget_speech_older_than_sixteen_seconds` |
| The waveform moves at half its former pace (visual-feedback) | unit `level_history.rs` `should_add_ten_columns_for_one_second_of_speech` |
| The record dot pulses in silence too (visual-feedback) | component `visual_feedback.feature` `The record dot pulses only without Reduce Motion, and the waveform still moves` |
| Speech leaves tall columns that slide to the left (visual-feedback) | component `visual_feedback.feature` `Speech leaves tall columns`; unit `level_history.rs` `should_draw_speech_as_tall_columns`, `should_slide_one_pitch_per_column_period` |
| Silence leaves a dotted line (visual-feedback) | unit `level_history.rs` `should_draw_room_noise_as_dots` |
| Quiet and loud speech leave columns of different heights (visual-feedback) | unit `level_history.rs` `should_draw_quiet_and_loud_speech_at_distinct_heights` |
