# Baseline and results

## Stage medians before (stage journal, 14–20 Oct, passed runs only)

| Recipe | Stage | Runs | Median |
|---|---|---|---|
| test | test | 88 | 101.1 s |
| check | test | 34 | 70.0 s |
| check | e2e-smoke | 19 | 365.0 s |
| check | e2e-quick | 19 | 41.6 s |
| check | refine-eval | 19 | 61.1 s |
| full-check | e2e-smoke | 25 | 366.7 s |
| full-check | e2e-seams | 17 | 352.8 s |
| full-check | e2e-resilience | 17 | 195.5 s |
| full-check | e2e-bench | 16 | 209.8 s |
| full-check | refine-eval | 17 | 58.8 s |

## Slowest tests of `just test` before

| Test | Time |
|---|---|
| `commandspotting_reconnect_it` long outage | 61–65 s (nextest.toml note) |
| `cucumber_integration` scenarios | 59.4 s |
| `transcription_openai_it` give-up within half a minute | 22.9 s |
| `placed_commands_it` same chain at any confirmation speed | 21.6 s |
| `placed_commands_it` restart once while confirmations are slow | 17.3 s |
| `server_it` keep answering while a confirmation hangs | 15.8 s |

## Removed tests and the tests that keep their behavior

| Removed | Kept, with the same behavior |
|---|---|
| `commandspotting_websocket_it::should_ignore_a_command_phrase_said_inside_a_sentence_after_long_listening` (30 s of listening) | `voice_control.feature` "A command phrase inside a sentence is ignored while listening" (the same sentence); listening long before a command: "A command said after a long time of listening is recognized" and `should_recognize_commands_after_long_listening_with_the_phrase_set_switched_between_them` |
| `placed_commands_it::should_hear_no_command_in_a_sentence_that_only_holds_a_command_word` | `voice_control.feature` "A command word inside a sentence is still no command" and "Enter followed by speech is no command" (the same two sentences, both `@smoke`) |
| `textrefinement_openai_it::should_refine_sentence_through_real_service` | `text_refinement.feature` "Technical term is written in its usual spelling" and the other `@refinement` live scenarios, through the same adapter |
| `speech_recognition.feature` "Long speech under difficult recording conditions", rows Russian and English "spoken at a normal pace" (`@seams`) | `speech_recognition.feature` "Recording file longer than the chunk threshold", Russian and English (`@smoke`, also run by `full-check`): the same speech and bars, plus that it is chunked |
| `file_transcription.feature` "Silence is processed without a startup failure", rows 1, 10 and 60 s without refinement | `speech_recognition.feature` "Silence produces a lone "you"" (1, 10, 60 s without refinement), whose step also fails on a startup failure |

Compared and kept, as no duplicate (design decision 8):

- `placed_commands_it` stop after the last word, enter after other speech, doubled stop, chain and short breath: they check the placement the service reports (tail, head, anywhere) and the utterance split, details of the service's protocol the scenarios do not see.
- `voice_samples_it` and `voice_forms_it`: they run every recorded command and every labelled form of all the user's recordings; the `@voice_samples` scenarios pick a few recordings each.
- `chunked_transcription_it`: the seam sweep moves the seams with three lead-ins under five conditions and bounds what the merge loses; no scenario does that.

## E2E scenario concurrency (`just e2e-smoke`, three runs each under one lock hold, 20 Oct)

| Scenarios at once | Runs | Result |
|---|---|---|
| 4 | 440 s, 415 s, 409 s | all green |
| 6 | 407 s, 406 s, 409 s | all green |
| 8 | 438 s, 403 s, 403 s | first run red: hands-free session event order, and "no archive entry is written" (the app exited 1) |

The default is 6: as fast as 8 and green in every run. The Gherkin harness took 234–243 s of each run; the service binaries the rest (`commandspotting_websocket_it` 94 s, `voice_forms_it` 25 s, `placed_commands_it` 19 s, `voice_samples_it` 12 s).

## After (20 Oct, branch with `main` merged)

| Recipe | Stage | Before (median) | After |
|---|---|---|---|
| test | test (nextest summary) | 101.1 s | 46–47 s (stage 59–70 s with the build) |
| full-check | test | 106.7 s (7-day median) | 49.2 s |
| full-check | e2e-smoke | 366.7 s | 411.3 s (with service start and stop) |
| full-check | e2e-regression | — | 21.5 s (new: the voice control rows moved out of smoke) |
| full-check | e2e-seams | 352.8 s | 416.3 s |
| full-check | e2e-resilience | 195.5 s | 135.9 s |
| full-check | e2e-bench | 209.8 s | 227.9 s |
| full-check | refine-eval | 58.8 s | 26.9 s |

`just test` is about halved and `refine-eval` and `e2e-resilience` are faster; `e2e-smoke` is not. Its Gherkin scenarios stream audio in real time and take 234–243 s alone, and the service binaries (`commandspotting_websocket_it` 94 s and the rest) run one after another before them, so the scenarios set its floor. The first targets (`just test` < 20 s, smoke halved) are left to a later change (proposal § Acceptance bar). `e2e-seams` and `e2e-bench` were not changed; their times vary with the machine's load.
