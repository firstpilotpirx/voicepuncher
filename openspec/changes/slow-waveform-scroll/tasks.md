## 1. A slower waveform

- [ ] 1.1 Red: in `level_history.rs`, `should_add_nine_columns_for_one_second_of_speech` (one second of speech leaves nine tall columns) and `should_still_show_speech_from_less_than_eighteen_seconds_ago` (speech in the first second still shows at 17.5 s); both fail at 100 ms
- [ ] 1.2 Green: `COLUMN_PERIOD` 110 ms, the comment of `FULL_CAPACITY` (the last 17.6 s)
- [ ] 1.3 `should_forget_speech_older_than_eighteen_seconds`: 20 s of speech, then 19 s of silence, leaves only dots
- [ ] 1.4 Version: bump `app/Cargo.toml` to 0.38.6 (PATCH)
- [ ] 1.5 `just test`; commit
