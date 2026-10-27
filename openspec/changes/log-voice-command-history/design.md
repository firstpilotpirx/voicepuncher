## Context

The command spotting service is the only place that holds, at the same time, the audio of an utterance, what the keyword spotter spotted in it (a phrase from its list with its tokens; the spotter gives no score), every hearing of the speech recognition service for commands and the events it reports to the app. The speech recognition service hears every closed speech segment, and some segments several times (start, windows, tail, half pause, rehearing), so a command is often found where the spotter spotted nothing.

## Goals / Non-Goals

**Goals:** one command history entry per utterance with a command heard by either model, written off the path that reports commands.

**Non-Goals:** what the app does with a reported command (its own pause rule); a statistics tool; histories of services the app adopts or the scripts start.

## Decisions

1. **The entry is written by the service.** It alone has the audio and both results; sending them to the app would grow the protocol for data the app never uses. The app names the folder when it starts its copies (`--command-history-dir`, `--command-history-max-mb`); a service without the folder records nothing, so tests, benchmarks and the scripts never write into the user's history.
2. **An entry is one speech segment as reported to the app.** A recorder in the session sees three things: each spotted phrase (selected phrases and trigger keywords), each answered hearing (kind, clip span, text) and each event leaving the session. A segment's entry gathers the spotted phrases and reported phrases or commands whose end lies in the segment or up to 0.5 s past it, and the hearings whose clip overlaps it; it closes once its end went out and the stream ran 1 s past it, or when the session settles or ends. Rejected and refused alternative: grouping by the engine's internal utterance numbers, which several kinds of checks share differently.
3. **Outcome from two sets.** S: the commands spotted (a trigger counts as its command), R: the commands reported. S empty → missed; R empty → rejected; R within S → agreed; else corrected. Neither → no entry.
4. **Audio.** From the earliest spotted, reported or heard start minus 0.3 s to the segment end plus 0.3 s, at most the last 10 s, as the microphone gave it (16 kHz mono 16-bit WAV), taken from the session's 30 s of recent audio.
5. **Format.** `<dir>/<local start time>-<pid>-<conn>-<n>/` with `audio.wav` and `entry.json` (version 1: start time, service version, outcome, spotted phrases with times and tokens, hearings with kind, span, text and time taken, transcript — the segment's own hearing, else the last — reported commands with times, audio span). Times are seconds in the entry's audio.
6. **A writer thread per engine.** Sessions hand finished entries to one thread over a channel; it writes the entry, then removes the oldest entries until the folder fits the limit, never the new one; every failure is a warning. Commands never wait on the disk. `Engine::flush_history` lets tests wait for the writes.
7. **App config.** `[archive] keep_commands = true` (Settings → Archive, "Keep every voice command"); folder `~/.voicepuncher/command-history`; limit `[archive] max_size_mb`. Passed only to the copies the app starts.

## Risks / Trade-offs

- An adopted service started before this change records nothing until the app restarts it: stated in the proposal's out of scope.
- A segment longer than 10 s keeps only its last 10 s, which hold the command that ends it.
- The outcome describes the service, not the app: a phrase the app's pause rule ignores still counts as reported.

## Test levels

| Scenario | Level |
|---|---|
| A command both models heard is kept as agreed | E2E `.feature` (engine in process, real spotter, scripted hearing) |
| A command only the speech recognition service heard is kept as missed | E2E `.feature` |
| A spotted phrase the speech recognition service does not hear is kept as rejected | E2E `.feature` |
| A spotted phrase heard as another command is kept as corrected | E2E `.feature` |
| Ordinary speech leaves no entry | E2E `.feature` |
| The app's own services keep the command history by default | component `.feature` (service specs the app builds) |
| Command history turned off | component `.feature` |
| A service given no folder writes no entry | E2E `.feature` |
| The oldest command history entries go first | E2E `.feature`; unit for the pruning |
| The command history folder cannot be written | E2E `.feature` |
