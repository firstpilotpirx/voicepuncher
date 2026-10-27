## Why

The user wants statistics on how voice commands are recognized: how well the small keyword spotting model (sherpa-onnx, always listening) recognizes commands on its own, and how often the speech recognition service for commands (Whisper small) that confirms each utterance corrects or rejects it. Today that cannot be measured from what is kept:

- The dictation history (`~/.voicepuncher/history/<time>/` with `audio.wav` and `entry.json`) keeps only dictations, 289 entries on 2026-10-27; no command keeps its audio.
- The app log keeps only the outcome of a command and its timing (`voice start: total=… hear=… confirm=…`), not what each model heard.
- The command spotting service log keeps Whisper's text per utterance (`kind=segment … answered in 187ms: " Good luck."`), not what the keyword spotter heard, and the user's service log on port 8082 has no entries after 2026-10-20, so today's commands are not there at all.

The question behind it: is the second model worth its cost (about 0.15–0.2 s per command, measured in today's log) — how many false starts and wrong commands it filters out.

## What Changes

- A separate command history, beside the dictation history: for every utterance the keyword spotter reacts to, the system saves the audio of that utterance, what the keyword spotter heard (the command or keyword and its score), what the speech recognition service for commands transcribed, and the verdict (the command carried out, the command it was corrected to, or rejected as no command).
- Every entry is written whether the command was carried out or not, so false starts the confirmation filtered out are counted too.
- The history stays local, on the user's machine, like the dictation history.

Current stage: PoC. Target stage: PoC.
Scope: recording every keyword spotting reaction with its audio, both models' results and the verdict.
Acceptance bar: after saying a few commands and some ordinary speech that the keyword spotter reacts to, the command history holds one entry per reaction, each with playable audio, the keyword spotter's result, Whisper's transcript and the verdict, and the counts of agreed, corrected and rejected utterances can be read from it.
Out of scope: changing how commands are recognized or confirmed; a report or statistics tool over the history (a later change); sending the history anywhere.

## Capabilities

### Modified Capabilities

- `voice-control`: every keyword spotting reaction is kept in a command history with its audio, both models' results and the verdict.

## Impact

- The command spotting service (`command-spotting/`) and the app's voice control pipeline (`app/`): where the reaction, transcript and verdict are known and where the entry is written.
- `docs/ubiquitous-language.md`: the new term for the command history.

## Open points

- Where the history lives and who writes it: the command spotting service (which holds the audio and both results) or the app (which writes the dictation history today). Proposed: the service sends the audio and both results with its spotting event, and the app writes it next to the dictation history, e.g. `~/.voicepuncher/command-history/<time>/`.
- Whether utterances the keyword spotter did not react to but Whisper heard as a command are recorded too (missed commands). Proposed: only reactions of the keyword spotter, as the user asked; misses later.
- How long the history is kept and how much disk it may take. Proposed: the same limit as the dictation history.
- Whether recording is on by default or a config switch. Proposed: on by default, a switch in `config.toml` to turn it off.
- Why the service log on port 8082 stopped on 2026-10-20: check while implementing, as the service log is the current fallback.

## Release

SemVer: MINOR — a new capability for the user (a command history).
Target version: 0.39.0 (proposed: the next minor above 0.38.7 on `main`; take the next free minor at implementation time).
