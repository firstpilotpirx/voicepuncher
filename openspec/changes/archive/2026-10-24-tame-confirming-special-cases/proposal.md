## Why

`command-spotting/src/engine/confirming.rs` (1 297 lines) is where the audit of 2026-10-22 found accretion rather than an algorithm: `apply_answer` (`:515–613`, ~100 lines) juggles ~10 flags (`near_miss`, `short_segment`, `rehear`, `stale`, `heard_early`, `early_pending`, `heard_again`, `tail`, …); `close_segment` is 85 lines; `JobContext` has 8 variants; `hear_whole_instead` (`:478–512`) returns `Result<AskedJob, Vec<ServerEvent>>` where `Err` is success; `apply` swaps `self.matcher` through `mem::replace`; `place` (`:889–947`) uses `usize::MAX` for "no clip end"; `Session` (`engine.rs:197–247`) holds ~25 interleaved fields; `is_check` is defined twice (`worker.rs:33`, `confirming.rs:1094`). Mishearings are patched one by one in code — "at ten enter"→"at 10am" (`:28, :951`), "okay, paste"→"okay, based" (`:470`), "restart"→"Restocked." (`:688`), "Let's start." (`matcher.rs:30`), `ends_with_and_restart` (`placed.rs:294–300`), `CUT_WORD_SECS` for "please" (`:889`) — and the app's `STARTED_EARLY` state leaks into the engine (`engine.rs:412–479`). The command forms and the states they are accepted in are declared three times (`placed.rs:51–84`, `keywords.txt`, the app) with nothing that catches a drift. `words()` (`matcher.rs:136–143`) splits on non-ASCII, so non-Latin text becomes empty. Churn confirms the diagnosis: `engine.rs` 11 commits, `confirming.rs` 8, `matcher.rs` 8, against `pool.rs` 2.

## What Changes

- Mishearings become data: one `NEAR_MISSES` table (phrase heard → command, with the recording that produced it) consulted by the matcher; the per-case branches in `confirming.rs` and `matcher.rs` go, except the three cases that work as mechanisms ("Restocked.", "okay, based", "at 10am"), which stay as code whose comments name the case only as an example (design § Context); a test runs the table against the voice samples.
- One source for the commands, their forms and the states that accept them: a declaration (a Rust module or a data file) from which `keywords.txt` and the app's `command_words` are generated or checked by a test that fails on a drift.
- The confirmation state machine is split by `JobContext`: one handler per context with the flags it needs as typed state, `apply_answer` and `close_segment` under the function budget, `Result<AskedJob, Vec<ServerEvent>>` replaced by an enum that says what it is; `Session` grouped into the sub-states it actually holds.
- The engine takes a "search the start early" fact through the protocol (`early_start`) instead of inferring it from the selected phrases; it travels with the phrases the matcher holds (design § 7).
- `words()` handles Unicode letters; `is_check` lives once.
- Stale doc comments (`engine.rs:620–621`), constants in the middle of methods (`:614`), the double `ask_spotted_check()` per frame (`:285–302`, now one call after the answers), and the two `impl PhraseMatcher` blocks are cleaned up on the way.

Current stage: PoC. Target stage: PoC.
Scope: `command-spotting/src/engine/**`, `matcher/**`, `keywords.rs`, `keywords.txt`, `app/src/core/domain/command_words.rs`, the `engine_it`, `placed_commands_it` and voice-sample tests.
Acceptance bar: every `engine_it`, `placed_commands_it`, `voice_samples_it` and `@smoke`/`@regression` voice control scenario passes unchanged; no mishearing word is matched by a branch in code (the three mechanisms above stay, design § Context); the drift test proves the three declarations agree; `confirming.rs` under the file budget.
Out of scope: stalls, queues and the protocol version (`command-spotting-stall-handling`); new commands.

## Capabilities

### Modified Capabilities

- none (a refactor; the voice control spec stays).

## Impact

- `command-spotting/src/**`, `command-spotting/keywords.txt` (or its source), `app/src/core/domain/command_words.rs`, `command-spotting/tests/**`.

## Open points

- Whether the single source of commands is Rust (a `const` table both crates compile) or a data file both read; proposed: a Rust module in `copy-pool`'s neighbour, a small `voice-commands` crate both depend on, since `command-spotting` cannot depend on `app`.

## Release

SemVer: no release — refactor without behavior change.
Target version: unchanged.
