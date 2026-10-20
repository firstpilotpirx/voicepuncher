## Context

See proposal.md § Why. A session of the command spotting service holds one phrase matcher, replaced whenever the app selects the phrases of its new state. Each confirmation asked is kept as a job until its answer arrives, and the answer is read against the session's matcher of that moment, not the one the job was asked under. One utterance is confirmed more than once (an early check when "start" is spotted, the whole utterance when it ends, tails, re-hearings), so a late answer about words heard in standby can be read with the dictating phrases.

## Goals / Non-Goals

- Goal: every answer is read against the phrases listened for when its audio was heard.
- Non-goal: dropping or timing out late answers; judging in the service whether a command still fits the app's state.

## Decisions

| Decision | Chosen | Why | Rejected |
|---|---|---|---|
| Where the phrases of a job live | Each job keeps a shared handle to the matcher it was asked under: the session's matcher becomes reference-counted, a new selection installs a new one, jobs in flight keep the old | No copy per job, no generations to track; the old matcher lives as long as a job needs it | Cloning the phrase list into every job (needless copies); a generation number per selection (more state for the same effect) |
| Which decisions use the job's matcher | Everything done with an answer: finding the phrase in the heard text, placing a chain, near misses, the trigger heard inside a segment. A re-hearing inherits the matcher of the job it hears again | The words were said under those phrases | Reading answers with the current matcher (the bug) |
| Which decisions keep the current matcher | Decisions made at spotting time, before a job is asked: whether "start dictation" is listened for before the early head check, and a trigger spotted now re-hearing a misheard segment | Those words are heard now, under the current phrases | Freezing the matcher per utterance ("stop enter start" switches state twice in one utterance) |
| A late command the app's state no longer accepts | Delivered as said; the app ignores it as it ignores any command said in the wrong state, and the service's record of what each utterance already reported drops a repeat of a command it already sent | No new rule, no timing heuristic that could eat a real command | A time debounce in the service (would not catch a repeat read as another command, and would drop a real "restart" said soon after "start") |

## Test levels

| Scenario | Level |
|---|---|
| A command is carried out once while confirmations come back slowly | adapter integration (`command-spotting/tests/placed_commands_it.rs`, exists) |
| A chain is carried out the same at any confirmation speed | adapter integration (same file, exists) |
| A start confirmed after the dictation began stays a start | adapter integration (same file, new: the scripted hearing takes 2 s and the test switches the session to the dictating phrases on the first start it reports, as the app does) |

The app's rule that an unaccepted command is ignored keeps its existing unit tests.

## Risks / Trade-offs

- [A job asked just before a switch reads its answer with the old phrases, though the app already moved on] → intended: the words were said in the old state; the app ignores what its state does not accept.
- [The test's switch on the first start needs the early check answered before the utterance ends] → the scripted sentence carries several seconds of dictated speech after "start dictation", so the 2 s answer arrives while it is still spoken.
