## Context

The overlay (`visualfeedback/macos/`) and the dictation sign keep their `AppKit` objects in `Arc`s that cross threads through three `unsafe impl Send`/`Sync` (`MacosVisualIndicator`, `WindowSet`, `SignWindow`) and one more in `transcript_window.rs` (`WindowOnMain`). `objc2` marks `NSWindow` and the views main-thread-only, so without these impls the compiler would refuse every closure that carries them to the main queue — which is exactly the guarantee the code gives up. The pipeline drives the overlay from its own threads; every drawing goes through `dispatch_to_main`.

Around it: twelve `lock().unwrap()` in the overlay; the clipboard restore, the self-hide of the dictation sign's badge and every sound run on detached threads; service output copy threads are never joined.

## Goals / Non-Goals

**Goals:** no `unsafe impl Send`/`Sync` in `app/`; the `AppKit` objects of the overlay and the sign are reachable only with a `MainThreadMarker`; overlay locks tolerate poisoning; the clipboard restore, the badge self-hide, sounds and service output copies are owned and end with their owner.

**Non-Goals:** the overlay's look, states and timings; the menu bar (its own thread-per-event pattern in `menubar/macos/indicator.rs` stays); dropping `SharedVisualIndicator`'s `Mutex`.

## Decisions

### 1. `MainOnly<T>`: a `Send + Sync` handle to a value kept on the main thread

`main_queue.rs` gains `MainOnly<T>`: the value lives in a thread-local registry of the main thread, keyed by a number; the handle holds only that number (in an `Arc`, with `PhantomData<fn() -> T>`), so it is `Send + Sync` by construction, without `unsafe impl`. `MainOnly::new(mtm, value)` stores the value; `with(mtm, |value| …)` lends it, and only on the main thread, since a `MainThreadMarker` is required and is itself `!Send`. When the last handle drops, the value is removed on the main thread: at once when the drop runs there, otherwise through the main queue. The registry lends values through `Rc`, so a closure that builds or drops another `MainOnly` while it runs does not re-borrow the registry.

`main_queue.rs` also gains `on_main(work)` and `on_main_after(ms, work)`, which hand the main-queue work its `MainThreadMarker`: the main queue runs on the main thread, so the trampoline creates the marker with `MainThreadMarker::new_unchecked` under a `// SAFETY:` comment. The existing `dispatch_to_main` stays for the other callers.

*Alternatives:* a wrapper with `unsafe impl Send` that checks the thread at access (what the code does today, plus a runtime check) — rejected: the acceptance bar asks for no `unsafe impl`. `dispatch2`'s `MainThreadBound` — not a dependency today, and its `get` panics off main rather than not compiling.

### 2. The overlay's main-thread state

`MacosVisualIndicator` keeps only thread-safe parts (the state under its lock, atomics, the level feed, the live transcript text) plus one `MainOnly<MainState>`, where `MainState` holds the windows (`RefCell<Vec<Rc<WindowSet>>>`) and the last screen topology, and a `MainOnly<ScreenWatcher>` of its own (the watcher's handler holds the state's handle, so the state cannot hold the watcher without a cycle). `WindowSet` and the windows' `Arc` become `Rc`, `!Send`. Every timed drawing (heartbeat, waveform tick, Pasted fade, notice expiry, command sign) captures the handle and takes the windows of the moment from it on each tick, after its turn check: a rebuild always starts a new turn, so the windows a tick finds are the ones its turn drew on. The test accessors (`first_icon_symbol`, …) read through the handle with the marker of the calling thread and answer "nothing shows" off the main thread, so their signatures, and the integration tests, stay. The screen watcher knows nothing of the overlay, so it moves to `adapters/screen_watcher.rs`, beside `main_queue.rs`: the overlay's directory, listed in the size baseline, must not grow.

The dictation sign's window becomes `MainOnly<SignWindow>`; the transcript window's rise animation captures a `MainOnly<Retained<NSWindow>>` instead of `WindowOnMain`.

### 3. Compile-time proof

A test that calls an `AppKit` getter off the main thread must not compile. The workspace runs its tests under cargo-nextest, which runs no doctests, so a `compile_fail` doctest would never run; trybuild would add a dependency and a slow nested build. Instead each module with a main-thread-only type holds a static assertion, `const _: () = assert_not_send!(T);`, that the type is **not** `Send` (the ambiguous-impl trick of `static_assertions::assert_not_impl_any!`, written out as a macro in `main_queue.rs`): `WindowSet`, `SignWindow` and `MainState` turning `Send` again breaks every build, `just test` and `just lint` included. With them `!Send`, any closure that carries one to `thread::spawn` or `dispatch_to_main` fails to compile, which is the guarantee the bar asks for. The `unsafe_impls` allowlist test loses all six entries; its `ALLOWED` becomes empty, and it then fails on any new `unsafe impl Send`/`Sync`.

### 4. Poison-tolerant locks

Every overlay and dictation-sign `lock().unwrap()` becomes `lock().unwrap_or_else(PoisonError::into_inner)` through one shared helper, `adapters/locking.rs` `lock`, as in `menubar/macos/indicator.rs`: a panic in one closure no longer brings down the next signal. The state, the waveform and the topology locks are all covered.

### 5. Owned delayed work

- **Clipboard restore:** `MacOsTextPaster` keeps the pending restore: a thread that waits on a channel up to `restore_delay_ms`, then restores. Dropping the paster — the pipeline drops it when the runtime stops, which Quit and the termination signal both do before the process ends — wakes the thread at once and joins it, so the clipboard is restored before the app ends. A new paste first finishes the pending restore, so its own snapshot holds the user's clipboard rather than the previous dictation.
- **Badge self-hide:** `PointerSignIndicator` keeps one timer thread fed by a channel instead of a thread per Pasted or failure badge; dropping the indicator closes the channel and joins the thread, cancelling a pending self-hide.
- **Sounds:** each `afplay` child is waited for by a reaper thread, so no zombie stays after a sound.
- **Service output:** the two copy threads of a service are kept with its process and joined after it ended (`kill`, or an exit `exited` observed), each waited for at most 1 s, since a grandchild of the service could still hold its pipe.

## Test levels

| Scenario | Level |
|---|---|
| text-insertion: A screenshot copied during the recording survives the paste | adapter integration (`textpasting_macos_it`, unchanged) |
| text-insertion: Copied text survives the paste | adapter integration (`textpasting_macos_it`, unchanged) |
| text-insertion: An empty clipboard stays empty | adapter integration (`textpasting_macos_it`, unchanged) |
| text-insertion: Quitting right after a paste still restores the clipboard | adapter integration (`textpasting_macos_it`: the paster dropped as the stopping runtime drops it) |
| text-insertion: A copy made after the paste is not overwritten | adapter integration (`textpasting_macos_it`, unchanged) |
| text-insertion: A second dictation within the delay keeps the user's clipboard | adapter integration (`textpasting_macos_it`: two pastes inside one restore delay) |

The thread-model goals have no spec scenario: unit tests and compile-time assertions (§ 3), unit tests of the sound player (all but one of its integration tests move beside them, so `app/tests` does not grow) and of the service process.

## Risks / Trade-offs

- A dropped `MacosVisualIndicator` off the main thread frees its windows only when the main queue next runs — as today, where the last `Arc` was dropped inside main-queue work.
- A reload drops the old pipeline's paster: a restore pending at that moment runs early. Reloads come from the Settings window, not in the middle of a paste.
- Tests that read the overlay must keep running on the main thread (the harness already does).
