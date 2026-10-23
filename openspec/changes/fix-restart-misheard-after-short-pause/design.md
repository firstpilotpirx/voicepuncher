## Context

The E2E scenario "Restart, a short pause, then speech that starts like another word" failed once (1 of 82 voice smoke runs), on commit `f0e051d1`, which lacked `voice-commands-under-load` (#288). The proposal's first hypothesis, a "Stud" mishearing, was refuted (proposal § Why): the restart segment is heard "Restart!" by the real service, idle and under CPU load. The service log of the failing run is gone, so the cause cannot be read; its symptom and load match what #288 fixed: confirmations cut short at the 1 s decoding budget of an idle machine.

## Goals / Non-Goals

**Goals:** pin the restart-before-a-short-pause placement under a slow speech recognition service for commands, so a regression of the decoding budget fails `just test` instead of an occasional E2E run.

**Non-Goals:** a mishearing entry, a clip change, a change to the E2E scenario, keeping service logs of failed runs.

## Decisions

- **A test, not a fix.** The investigation found no defect left on `main`; a mishearing entry for a word never heard would add a branch with no evidence (rejected). Prepared decision "fix the clipping if the word is cut, else add the mishearing": the segment is not cut and nothing is misheard, so neither applies.
- **Where:** `command-spotting/tests/it/placed_commands/under_load/mod.rs`, beside #288's under-load tests, with its `loaded_copy` (a scripted copy over HTTP decoding each confirmation for 1.2 s, so the real `UtteranceTranscriber` and its `Pace` decide the budget). The engine's scripted `Hear` alone would bypass the budget and could not fail.
- **Audio:** built in the module from `super`'s helpers: a pause, "Weather is sunny in Berlin today", a 1 s pause, "Restart", 0.4 s of silence, "Coffee machine on the third floor is broken", a pause — the E2E scenario's speech, word by word as the scripted hearing needs. `placed_commands.rs` is listed in the size baseline and does not grow.
- **Red:** the test passes on `main`, since #288 already fixed the cause. Its failing run is shown once, locally and uncommitted, with `Pace::budget` returning the fixed 1 s floor (the budget before #288); the result is recorded in `tasks.md`.
- **Version:** unchanged; test and spec only.

## Risks / Trade-offs

- The real cause of the one failure stays unproven (log lost) → the scenario keeps running in every `full-check`; another failure there is a new finding with its log at hand.

## Test levels

| Scenario (voice-control) | Level | Test |
|---|---|---|
| A stop right after dictated speech under load | integration (unchanged, #288) | `placed_commands::under_load::should_hear_a_stop_right_after_the_last_dictated_word_under_load` |
| A chain of commands under load | integration (unchanged, #288) | `placed_commands::under_load::should_carry_out_the_same_chain_under_load` |
| A restart before a short pause and more speech under load | integration (new) | `placed_commands::under_load::should_restart_before_a_short_pause_and_more_speech_under_load`; E2E "Restart, a short pause, then speech that starts like another word" stays the end-to-end check |
