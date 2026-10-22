## 1. Main-thread handle

- [ ] 1.1 Red: unit tests in `main_queue.rs` — a `MainOnly` value is lent on the main thread, is gone after its last handle drops, and `MainOnly<T>` is `Send + Sync` while a test-only `assert_not_send` helper rejects a `!Send` type at compile time
- [ ] 1.2 Green: `MainOnly<T>`, `on_main`, `on_main_after` and the `assert_not_send` test helper in `main_queue.rs`
- [ ] 1.3 `just test`; commit

## 2. Overlay and sign without unsafe impls

- [ ] 2.1 Red: static assertions that `WindowSet`, the overlay's `MainState` and `SignWindow` are not `Send`; `unsafe_impls` `ALLOWED` emptied (fails while the impls exist)
- [ ] 2.2 Green: the overlay keeps its windows, topology and screen watcher in `MainOnly<MainState>`; `Arc<WindowSet>` becomes `Rc`; timed drawings take the windows from the handle; the test accessors move to `probe.rs`; the four `unsafe impl` go
- [ ] 2.3 Green: the dictation sign keeps `MainOnly<SignWindow>`; the transcript window's rise captures `MainOnly<Retained<NSWindow>>`; their `unsafe impl` go
- [ ] 2.4 Every overlay and sign lock tolerates poisoning; `visualfeedback_macos_it` passes unchanged
- [ ] 2.5 `just test`; commit

## 3. Owned delayed work

- [ ] 3.1 Red: `textpasting_macos_it` — dropping the paster right after a paste with a long restore delay restores the clipboard at once
- [ ] 3.2 Green: `MacOsTextPaster` owns the pending restore, finished at the next paste and on drop
- [ ] 3.3 Red: unit test — dropping a `PointerSignIndicator` cancels a pending self-hide and ends its timer thread; the existing self-hide tests stay green
- [ ] 3.4 Green: one owned timer thread in `PointerSignIndicator`, poison-tolerant generation lock
- [ ] 3.5 Red: `soundplaying_macos_it` — after a sound ends no `afplay` zombie of this process remains
- [ ] 3.6 Green: a reaper waits for each `afplay` child
- [ ] 3.7 Red: `localservices_process_it` — after `kill` the output copy threads are joined (the log holds the service's last output and no copy thread remains)
- [ ] 3.8 Green: `ChildProcess` keeps and joins its copy threads, at most 1 s each
- [ ] 3.9 `just test`; commit

## 4. Release

- [ ] 4.1 Bump `version` in `app/Cargo.toml` to 0.37.1; `just size-baseline` drops entries that shrank; `just test` and `just lint` pass; commit
