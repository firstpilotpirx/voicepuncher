## Context

See proposal.md § Why. A session holds one phrase matcher, replaced on every selection the app sends; each confirmation is a job whose answer is read with the matcher of the moment it arrives. When the state changes while an answer is on its way (a dictation the hotkey starts, a stop confirmed first), "start dictation" said in standby is read with the dictating phrases as "restart dictation".

## Goals / Non-Goals

- Goal: an answer never turns the command said into another one because the state changed while it was on its way.
- Non-goal: dropping or timing out late answers; judging in the service whether a command still fits the app's state.

## Decisions

| Decision | Chosen | Why | Rejected |
|---|---|---|---|
| Which phrases read an answer | The phrases selected when the job was asked together with those selected when it is answered; the closest phrase wins, the job's phrases first on a tie. Without a selection in between, the current matcher as today | A start matches "start dictation" exactly in either case: said in standby before a switch to dictating, or said while a stop's confirmation was on its way | The job's phrases alone (a "start" said right after a slowly confirmed "stop" would read as "restart", ignored in standby); the current phrases alone (the bug) |
| Where a job's phrases live | Each job keeps a shared handle to the matcher it was asked under | No copy per job | Cloning the phrases into every job; a generation number per selection |
| What reads with the combined phrases | Everything done with an answer, including the checks it asks (a re-hearing, a tail) | They hear the same words | Combining only for the whole-utterance phrase |
| Decisions at spotting time | Keep the current matcher: whether "start dictation" is listened for before the early head check, a trigger spotted now re-hearing a misheard segment | Those words are heard now, under the current phrases | — |
| A command the app's state no longer accepts | Reported as said; the app ignores it | No timing rule that could eat a real command | A time debounce (misses a repeat read as another command, drops a real "restart" said soon after "start") |

## Test levels

| Scenario | Level |
|---|---|
| A command is carried out once while confirmations come back slowly | adapter integration (`command-spotting/tests/placed_commands_it.rs`, exists) |
| A chain is carried out the same at any confirmation speed | adapter integration (same file, exists) |
| A start confirmed after the dictation began stays a start | adapter integration (same file, new: the session moves to the dictating phrases in the pause after "start dictation") |
| A start said right after a slowly confirmed stop stays a start | adapter integration (same file, new: the session moves to standby when the stop is reported) |

The combined matcher gets unit tests in `command-spotting/src/matcher.rs`. A scripted hearing completes a word a clip cuts off, as whisper does, so the early checks hear what the real service hears.

## Risks / Trade-offs

- [Two phrase sets read one answer, so a near phrase of the other state may win] → only for an answer asked before a switch and answered after it; the closest phrase wins, and the app ignores what its state does not accept.
