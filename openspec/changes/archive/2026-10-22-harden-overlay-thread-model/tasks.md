## 1. Main-thread handle

- [x] 1.1 Red: unit tests in `main_queue.rs` — the main-thread store lends a value until it is removed and never as another type, and `MainOnly<T>` is `Send + Sync` while `assert_not_send!` accepts a `!Send` type (and, checked by hand, rejects a `Send` one at compile time)
- [x] 1.2 Green: `MainOnly<T>`, `on_main`, `on_main_after` and the `assert_not_send!` macro in `main_queue.rs`
- [x] 1.3 `just test`; commit

## 2. Overlay and sign without unsafe impls

- [x] 2.1 Red: static assertions (`const _: () = assert_not_send!(T)`) that `WindowSet`, the overlay's `MainState` and `SignWindow` are not `Send`; `unsafe_impls` `ALLOWED` emptied (fails while the impls exist)
- [x] 2.2 Green: the overlay keeps its windows and topology in `MainOnly<MainState>` and its screen watcher in a `MainOnly` of its own; `Arc<WindowSet>` becomes `Rc`; timed drawings take the windows from the handle; the screen watcher moves to `adapters/screen_watcher.rs`; the four `unsafe impl` go
- [x] 2.3 Green: the dictation sign keeps `MainOnly<SignWindow>`; the transcript window's rise captures `MainOnly<Retained<NSWindow>>`; their `unsafe impl` go
- [x] 2.4 Every overlay and sign lock tolerates poisoning (`adapters/locking.rs`); `visualfeedback_macos_it` passes unchanged
- [x] 2.5 `just test`; commit

## 3. Owned delayed work

- [x] 3.1 Red: `textpasting_macos_it` — dropping the paster right after a paste with a long restore delay restores the clipboard at once; a second paste within the delay still leaves the user's clipboard
- [x] 3.2 Green: `MacOsTextPaster` owns the pending restore, finished at the next paste and on drop
- [x] 3.3 Red: unit tests — dropping the dictation sign's self-hide timer cancels a pending self-hide and ends its thread, and a later badge replaces a pending one; the existing self-hide tests stay green
- [x] 3.4 Green: one owned timer thread in `PointerSignIndicator`, poison-tolerant generation lock
- [x] 3.5 Red: unit test of the macOS sound player (all but one of its integration tests move beside it) — after a sound ends no `afplay` zombie of this process remains
- [x] 3.6 Green: a reaper waits for each `afplay` child
- [x] 3.7 Red: unit tests in `localservices/process/mod.rs` — `kill` returns only after the output copy threads ended, and stops waiting for a copy that never ends
- [x] 3.8 Green: `ChildProcess` keeps and joins its copy threads, at most 1 s each
- [x] 3.9 `just test`; commit

## 4. Release

- [x] 4.1 Bump `version` in `app/Cargo.toml` to 0.36.3; `just size-baseline` drops entries that shrank; `just test` and `just lint` pass; commit
