# Design

## Context

See proposal.md § Why. The proposal assumed the real-service tests of `placed_commands.rs` feed the speech as fast as possible; they no longer do: since `eradicate-flaky-tests` (431ee6c2) `engine()` wraps the real `UtteranceTranscriber` in `Clocked`, so every answer comes back 0.3 s later in the speech (`REAL_HEARING`) and the speech waits up to 60 s for it (`REAL_DEADLINE`). They still failed on 2026-10-21 19:35, 2026-10-22 06:57 and 19:35, and 2026-10-23 10:16 — each time the engine reported `SpeechStart`/`SpeechEnd` and no command at all (`[]`), and the failed test ran 12 s for 5 s of speech.

The cause is the decoding budget of `UtteranceTranscriber` (`confirm/pace.rs`, from `voice-commands-under-load`): a copy still decoding after the budget is cut short and, when it answers its health check, the confirmation counts as "nothing heard". The budget is twice the slowest confirmation of the last 10 s, at least 1 s. Each test builds a new transcriber, so its pace is empty and its first confirmations get the 1 s floor. Beside another check's heavy stage (whisper benchmarks on Metal) the shared speech recognition service for commands decodes a clip in more than 1 s: the confirmation is cut short, heard as nothing, and the command is dropped. `Clocked` passes 60 s as the deadline, but the budget, not the deadline, cuts the request.

The budget is right for the product (it cuts speech stuck repeating; the scripted `under_load` tests prove a loaded copy still confirms once the pace adapts). The real-service tests prove something else: that the real keyword spotter, voice activity detector and speech recognition service hear placed commands. Their result must not depend on how fast the shared service decodes.

## Goals / Non-Goals

**Goals:** the real-service tests of `placed_commands.rs` give the same result on an idle and on a loaded machine; the engine and the production transcriber keep their budgets.

**Non-Goals:** the budget, pace or deadline of the command spotting service; the scripted tests of the file; the `under_load` tests.

## Decisions

| Question | Chosen | Why | Rejected |
|---|---|---|---|
| Which seam | `UtteranceTranscriber::with_decoding_floor(base_url, floor)`: a transcriber whose budget is never below `floor` (`Pace::with_floor`); `new` keeps the 1 s `FLOOR`. The real-service `engine()` builds it with `REAL_DEADLINE`, so a confirmation is never cut short before its 60 s deadline | the budget is the only wall-clock cut left between the shared service and the speech clock; a floor at the deadline turns it off for these tests alone, with the production client, lease and copy handling unchanged | a retry in the test's `Hear` wrapper on an empty answer (hides a real empty answer, waits longer for each); posting to the service from the test (duplicates `request.rs` and the copy pool); a wait for each confirmation before feeding on (already done by `Clocked`: the cut happens inside the transcriber) |
| Speech time vs a wait per confirmation (open point 1) | keep speech time: it is already there | the remaining flake is the budget, not the feeding | — |
| Which tests (open point 2) | every test built on `engine()`: the five `should_hear_*` tests and `should_hear_a_stop_after_a_short_breath_as_an_utterance_of_its_own` | they share `engine()`; the fix lives there | — |
| How the bug is reproduced | a test that the real-service hearing (`real_hearing(url)`, used by `engine()`) hears a clip from a scripted copy decoding for 1.2 s (`under_load::scripted_copy`), as the shared service does beside another check; it fails with `UtteranceTranscriber::new` (cut short at 1 s, nothing heard) | deterministic, needs no loaded machine and no models beyond `say` | reproducing with a real GPU load (minutes, not deterministic) |

## Test levels

| Behavior | Level |
|---|---|
| a transcriber built with a decoding floor gives at least that floor | unit, `confirm/pace.rs` |
| the real-service hearing hears a clip a copy decodes for 1.2 s | integration, `command-spotting/tests/it/placed_commands/under_load/` |
| the real-service tests pass beside another check holding the machine lock | the acceptance bar (20 runs) |

## Risks / Trade-offs

- [A real hang of the shared service now waits up to 60 s per confirmation instead of failing in 1 s] → nextest's time limit of the stage still stops a hang; the tests already wait up to 60 s for the answer in the speech.
- [`fix-load-sensitive-confirm-tests` freezes three of these tests with `#[ignore]`] → whichever lands second removes the `#[ignore]` lines when it merges the base: this change unfreezes them.

## Dependencies

None.
