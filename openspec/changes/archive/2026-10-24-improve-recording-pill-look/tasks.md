## 1. The waveform's mapping, smoothing and period

- [x] 1.1 Red: `level_history.rs` tests `should_draw_room_noise_as_dots`, `should_draw_quiet_and_loud_speech_at_distinct_heights`, `should_map_the_noise_floor_and_below_to_no_loudness`, `should_map_loud_speech_and_above_to_full_loudness`, `should_add_ten_columns_for_one_second_of_speech`, `should_forget_speech_older_than_nine_seconds` (the old bound, period and capacity tests rewritten to the new values)
- [x] 1.2 Green: `QUIET_DBFS` -62, `LOUD_DBFS` -28, `RISE_RATE` 0.6, `FALL_RATE` 0.3, `COLUMN_PERIOD` 100 ms, `COLUMN_PITCH_PT` 4, `FULL_CAPACITY` 90, `CAPACITY_BESIDE_SIGN` 78; the waveform's column 2 pt wide
- [x] 1.3 `just test`; commit

## 2. The pill's outline and border

- [x] 2.1 Red: `window_set.rs` test `should_round_the_pill_like_a_notification_not_a_capsule`
- [x] 2.2 Green: `PILL_CORNER_RADIUS` 18; `RIM_WHITE_ALPHA` 0.15
- [x] 2.3 Bump `version` in `app/Cargo.toml` to 0.38.4
- [x] 2.4 `just test`; commit
