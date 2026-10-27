## Why

The user wants statistics on how voice commands are recognized: how well the small keyword spotting model (sherpa-onnx, always listening) recognizes commands on its own, and how often the speech recognition service for commands (Whisper small) that confirms each utterance corrects or rejects it. Today that cannot be measured from what is kept:

- The dictation history (`~/.voicepuncher/history/<time>/` with `audio.wav` and `entry.json`) keeps only dictations, 289 entries on 2026-10-27; no command keeps its audio.
- The app log keeps only the outcome of a command and its timing (`voice start: total=… hear=… confirm=…`), not what each model heard.
- The command spotting service log keeps Whisper's text per utterance (`kind=segment … answered in 187ms: " Good luck."`), not what the keyword spotter heard, and the user's service log on port 8082 has no entries after 2026-10-20, so today's commands are not there at all.

The question behind it: is the second model worth its cost (about 0.15–0.2 s per command, measured in today's log) — how many false starts and wrong commands it filters out.

## What Changes

- A command history, beside the dictation history: one entry for every utterance in which either model heard a command. Each entry holds the audio the speech recognition service for commands heard, the keywords the keyword spotter reported in it (or none; the spotter gives a keyword from its list, not a transcript), the text Whisper transcribed, and the outcome: agreed (both heard the same command), corrected (Whisper heard another command), rejected (the spotter fired, Whisper heard no command) or missed (only Whisper heard the command).
- Ordinary speech in which neither model heard a command leaves no entry.
- The command spotting service writes the entries, into a folder the app names when it starts the service; on by default, a config switch turns it off; the same size limit as the dictation history.
- The history stays on the user's machine.

Current stage: PoC. Target stage: PoC.
Scope: one entry per utterance with a command heard by either model, with its audio, both models' results and the outcome.
Acceptance bar: after saying a few commands and some ordinary speech, the command history holds one entry per utterance with a command, each with playable audio, the spotter's keywords, Whisper's text and the outcome, and the counts of agreed, corrected, rejected and missed utterances can be read from it.
Out of scope: changing how commands are recognized or confirmed; a report or statistics tool over the history (a later change); sending the history anywhere; services started by the service scripts or adopted by the app (they write no history).

## Capabilities

### Modified Capabilities

- `voice-control`: every utterance with a command heard by either model is kept in a command history with its audio, both models' results and the outcome.

## Impact

- The command spotting service (`command-spotting/`): collects each utterance's results and writes its entry.
- The app (`app/`): the config switch and the folder it passes when it starts the service.
- `docs/ubiquitous-language.md`: command history, command history entry, the four outcomes.

The service log on port 8082 stopped on 2026-10-20 because the app now streams to the copies of the service it starts on other ports, which log under their own ports: no fix needed.

## Release

SemVer: MINOR — a new capability for the user (a command history).
Target version: 0.39.0 (proposed: the next minor above 0.38.7 on `main`; take the next free minor at implementation time).
