## Context

The hotkey listener (`app/src/adapters/primary/ptt/rdev/mod.rs`) runs `rdev::listen` on its own thread. rdev 0.5.3 turns every key event into a string through `TSMGetInputSourceProperty`, which macOS 27 allows only on the main queue: the first key press after the tap starts can stop the app with `EXC_BREAKPOINT` (proposal § Why, bug 1). The hotkeys need only key codes and modifier flags.

The recording buffer (`recording/cpal/buffer.rs`) keeps one `limit`, lowered by `cut_at`; `stop()` in `recording/cpal/mod.rs` warns when `is_full()`, which compares the length with that lowered limit, so a cut recording counts as full (bug 2).

## Goals / Non-Goals

**Goals:** a hotkey listener that never asks the input-source service; the limit warning only at the real limit; rdev gone from the app.

**Non-Goals:** new hotkeys; the permissions gate; the overlay.

## Decisions

### Our own listen-only CGEventTap

The module `app/src/adapters/primary/ptt/eventtap/` replaces `ptt/rdev/`: `EventTapHotkeyListener` spawns the thread `hotkey-tap`, installs `CGEventTap::with_enabled` (core-graphics 0.25, already a dependency) at the HID location, listen-only, for `KeyDown`, `KeyUp` and `FlagsChanged`, and runs the thread's run loop. The callback reads only the key code field and the flags, turns them into a `KeyEvent::{Down, Up}(code)` (`eventtap/keys.rs`) and hands it to the classifier, which keeps the old chord logic on key codes. A modifier's `FlagsChanged` is a press when its device bit (left/right Option, Shift, Control, Command) or, for Fn and Caps Lock, its flag is set, else a release; an event without device bits (a synthetic one) falls back to the generic flag. The `HotKey` → key-code table holds the old rdev mapping (Right Option 61, F13 105, …).

Alternative rejected: an rdev fork without the string lookup — a fork to maintain for a few lines of tap code.

The test steps and the real-keyboard test, which used `rdev::simulate` and `rdev::listen`, post `CGEvent` keyboard events and watch through a `CGEventTap` themselves; rdev leaves `app/Cargo.toml` entirely.

### The buffer keeps its recording limit apart from the cut

`RecordingBuffer` keeps `max_samples` beside the cut `limit`; `is_full()` compares the length with `max_samples`, so `cut_at` never makes a buffer full.

## Test mapping

| Scenario | Test |
|---|---|
| The listener reads key codes only (push-to-talk) | integration `app/tests/it/ptt_eventtap.rs` `should_use_no_library_or_call_that_turns_keys_into_text` (no `rdev` in `app/Cargo.toml`, no `TSM` call in `app/src`) |
| A modifier hotkey is pressed and released (push-to-talk) | unit `eventtap/keys.rs` `should_signal_press_and_release_of_right_option_from_its_flags` |
| The other side's modifier is not the hotkey (push-to-talk) | unit `eventtap/keys.rs` `should_ignore_left_option_when_right_option_is_the_hotkey` |
| A short live recording logs no limit warning (live-transcript) | unit `buffer.rs` `should_not_count_a_cut_buffer_as_full` |
| A recording that reached the limit still warns (live-transcript) | unit `buffer.rs` `should_append_only_up_to_the_limit` |

The existing component scenario "Hotkey press after a paste is registered" (`push_to_talk.feature`, `@real_keyboard`) runs the new listener on the real event tap; only `just test-real-keyboard` in the manual run runs it, as it posts keys into the focused app.

## Risks / Trade-offs

- A tap disabled by the system on a timeout stops the hotkey → a listen-only tap is not timed out by slow callbacks; the callback only sends on a channel.
- The manual run on macOS 27 happens after the merge (preparation decision) → the user files a new change on failure.
