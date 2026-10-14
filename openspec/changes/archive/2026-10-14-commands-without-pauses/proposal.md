# Proposal

## Why

A command counts only when it is a whole utterance, with a pause of about half a second before and after it. When the user dictates and says "stop" right after the last word, the voice activity detector joins both into one long utterance: the pause rule rejects the command and the speech recognition service never checks the utterance (only utterances up to 3 s go to it). The user wants commands to work without that pause.

## What Changes

- **Command at the end of an utterance.** A command phrase that ends an utterance counts even with speech right before it: "…and that is all, stop" ends the dictation, "…okay, enter" presses Return. The pasted text keeps the speech before the command and drops the command words.
- **"start" at the start of an utterance.** "start dictation" (or "start") followed by speech without a pause starts a dictation, and the speech after the command is dictated: "start, meeting moved to Friday" pastes "meeting moved to Friday" when the dictation ends. Other commands at the start of an utterance followed by speech are no command.
- **Doubled short form anywhere.** "start start", "stop stop", "cancel cancel", "enter enter" and "clear clear" count wherever they are said, with no pause around them, and take effect at once. Text dictated before a doubled "stop stop" is pasted; words after it in the same utterance are dropped.
- **Chains of commands.** Two or three different commands said one right after the other — short or full — are carried out in order when each is accepted after the one before: "stop enter" pastes and sends; "stop clear" pastes and clears; "cancel clear" discards and clears; "stop enter start" pastes, sends and starts the next dictation. "start" can only come last ("start stop" would end an empty dictation). Like a doubled word, a chain counts anywhere, with no pause around it.
- **"cancel" alone.** Asked for by the user: "cancel" is the short form of "cancel dictation", like the other short forms.
- **"enter" during a dictation.** Asked for by the user: during a dictation "enter" (any form) is "stop enter": the text is pasted, then Return is pressed.
- **Restart.** A new command, asked for by the user: "restart dictation" or "restart" during a dictation drops what was dictated and starts a new dictation at once; "restart restart" works anywhere, like the other doubled forms.
- **Shorter pause.** The silence that ends an utterance goes from 0.4 s to 0.3 s.
- Every new form works in the state that accepts its command, as today.

## Capabilities

### Modified Capabilities

- `voice-control`: commands at the end of an utterance, "start" at its start, doubled short forms and chains of two or three commands anywhere, a 0.3 s pause.

## Impact

- `command-spotting/`: the matcher finds a command at the end, at the start, doubled anywhere, or a chain of two or three commands in a transcript; the service asks the speech recognition service about the last seconds of every closed utterance, about the first seconds of an utterance still running, and about a sliding window of a running utterance; a new event tells the app a command needs no pause rule; `--min-silence-secs` defaults to 0.3.
- `app/`: the recognizer carries out such a command at once; a dictation started by a "start" at the head of an utterance begins with the audio heard after the command; the dictated text loses command words at its head and tail and everything from a doubled stop on.
- Tests: matcher unit tests; service integration with `say`; recognizer and pipeline unit tests; component and E2E `.feature` scenarios; a second recording session of the user's voice with these forms.
- Load: while the user speaks, the service sends a short window to the speech recognition service about once a second; dictation chunks wait a few hundred ms longer when they collide.

## Scope

- **Lifecycle stage:** PoC, stays PoC.
- **Acceptance bar:** the new forms are recognized in `say` E2E scenarios, and the forms the user recorded (`session-2`, `session-3`) are recognized; no command on the benchmark corpus sample, the short Russian phrases or sentences that only contain a command word once inside them; `voice_samples_it` on `session-1` still passes; `just check` passes and `just full-check` bar the known chunk-seam failure; the user's manual run confirms.
- **Out of scope:** new commands other than restart; Russian commands; a wake word.

## Release

MINOR — new ways to say the commands; target version **0.10.0**.
