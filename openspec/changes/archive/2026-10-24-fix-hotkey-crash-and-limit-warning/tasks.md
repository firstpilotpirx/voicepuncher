## 1. Recording-limit warning

- [x] 1.1 Red: `buffer.rs` `should_not_count_a_cut_buffer_as_full`; commit it red (`test: reproduce the false recording-limit warning`)
- [x] 1.2 Green: `RecordingBuffer` keeps `max_samples`; `is_full` compares with it
- [x] 1.3 `just test`; commit

## 2. Hotkey listener on our own event tap

- [x] 2.1 Red: `app/tests/it/ptt_eventtap.rs` `should_use_no_library_or_call_that_turns_keys_into_text`; unit tests in `eventtap/keys.rs` for the modifier flags and the old classifier cases
- [x] 2.2 Green: `eventtap/keys.rs` (key events from code and flags, the hotkey table, the classifier) and `eventtap/mod.rs` (`EventTapHotkeyListener` on a listen-only `CGEventTap`); `menubar_app.rs` uses it; `ptt/rdev/` removed
- [x] 2.3 The slice steps and `keypressing_macos.rs` post and watch `CGEvent`s; rdev removed from `app/Cargo.toml`; `ptt_rdev.rs` becomes `ptt_eventtap.rs`
- [x] 2.4 `just test`; commit

## 3. Release

- [x] 3.1 Bump `version` in `app/Cargo.toml` to 0.38.2
- [x] 3.2 `just test`; commit
