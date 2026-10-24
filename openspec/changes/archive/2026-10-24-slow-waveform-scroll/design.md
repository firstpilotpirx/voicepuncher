## Context

`LevelHistory` (`app/src/core/domain/level_history.rs`) closes a column every `COLUMN_PERIOD` (100 ms) and `slide_offset` moves the strip one pitch (4 pt) over that period, so the strip moves 40 pt/s. The pill's width comes from the column count (`FULL_CAPACITY` 160, `CAPACITY_BESIDE_SIGN` 148) and the pitch, not from the period.

## Goals / Non-Goals

**Goals:** the waveform moves 10 % slower; the pill keeps its 727 pt width.

**Non-Goals:** the column count, pitch, heights, smoothing, the pill's size.

## Decisions

1. **Longer column period, same count and pitch.** `COLUMN_PERIOD` 100 → 110 ms: the strip moves 4 pt per 110 ms ≈ 36.4 pt/s (−9.1 % speed, each column 10 % longer on screen), and 160 columns now cover 17.6 s. The width is untouched because it depends only on the count and the pitch. A smaller pitch was rejected: it would shrink the pill or need more columns.
2. **Smoothing per sample stays.** The rise and fall rates act per microphone level, not per column, so the columns' heights for the same speech do not change.

## Risks / Trade-offs

- [Columns cover 110 ms of sound, so very short sounds weigh a bit less] → each column keeps the peak of its period, so a short sound still shows.

## Test mapping

| Spec scenario | Test |
|---|---|
| The waveform moves at half its former pace (visual-feedback) | unit `level_history.rs` `should_add_nine_columns_for_one_second_of_speech` |
| The waveform keeps the last six seconds (visual-feedback) | unit `level_history.rs` `should_still_show_speech_from_less_than_eighteen_seconds_ago`, `should_forget_speech_older_than_eighteen_seconds` |
| The recording pill is as long as ChatGPT's dictation bar (visual-feedback) | component `visual_feedback.feature` `The recording pill is as long as ChatGPT's dictation bar` (unchanged) |
| Speech leaves tall columns that slide to the left (visual-feedback) | component `visual_feedback.feature` `Speech leaves tall columns`; unit `level_history.rs` `should_slide_one_pitch_per_column_period` (unchanged) |
| Silence leaves a dotted line (visual-feedback) | unit `level_history.rs` `should_draw_room_noise_as_dots` (unchanged) |
| Quiet and loud speech leave columns of different heights (visual-feedback) | unit `level_history.rs` `should_draw_quiet_and_loud_speech_at_distinct_heights` (unchanged) |
| A new recording starts with a dotted waveform (visual-feedback) | component `visual_feedback.feature` `A new recording starts with a dotted waveform beside the record sign` (unchanged) |
| The waveform shows together with the record sign (visual-feedback) | component `visual_feedback.feature` `A new recording starts with a dotted waveform beside the record sign` (unchanged) |
| The record dot pulses in silence too (visual-feedback) | component `visual_feedback.feature` `The record dot pulses only without Reduce Motion, and the waveform still moves` (unchanged) |

The scenario titles "The waveform keeps the last six seconds" and "The waveform moves at half its former pace" stay as they are (the validator refuses a renamed scenario in a MODIFIED block); their steps carry the new numbers.
