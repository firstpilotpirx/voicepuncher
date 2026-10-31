# Design

## Context

Live traces (2026-10-30) find a command ~0.2 s after its words in stream time but deliver it ~1 s later: Nemotron (0.6B int8, 2 threads) decodes about as long as the audio lasts, frames up to 380 ms, and the session joins both recognizers on every frame, so the Russian one (26 ms per utterance) and the gate wait. A lone "stop" also waits `SETTLE_SECS`, which the Russian recognizer shows only in 0.64 s steps. The archived `streaming-command-recognition` research found hotwords on Nemotron useless and CoreML 2× slower.

## Goals / Non-Goals

**Goals:** the specs' wall-clock bounds, ≥ 89 % accuracy on the user's recordings (measured 91.2 %), every step measured on one bench.

**Non-Goals:** the keyword spotter, a Whisper fallback, GPU inference.

## Decisions

**A replay bench in Rust on the service's own session.** A `command-spotting` binary (`streaming-bench`, recipe `just streaming-bench`) streams voice samples, labelled history entries and negatives through the streaming session in real time and prints, per recognizer setup, median and p90 wall-clock delay from the end of the command's words to the report, accuracy and false alarms. It measures what ships, unlike the Python bench. Data stays on the machine.

**One worker thread per recognizer.** The session sends every frame to a long-lived worker per recognizer and decides on each one's words as they arrive; the first report wins, de-duplicated as now. Rejected: more threads inside the joined step — the slow recognizer still gates the fast one.

**The English recognizer is measured, then picked.** On the bench: Nemotron at 2, 4 and 6 threads with decode threads at user-interactive QoS (performance cores), its 80 ms chunk variant, and Parakeet Realtime EOU 120M (5× smaller) when sherpa-onnx 2.0.1 loads it. The fastest setup within 1 point of Nemotron's accuracy wins; a new model joins the model catalog. Moonshine streaming only with a sherpa-onnx streaming export.

**A lone command reports at the gate's pause.** It settles on the earlier of `SETTLE_SECS` of later decoded audio and the gate's 0.1 s silence after its last word; "stop the car" still waits, since the gate hears speech. The 0.5 s utterance tail shrinks to the shortest that keeps every bench command.

**Quiet speech.** Gate gain and threshold are swept on quiet clips against false openings on negatives.

**Missed utterances in the history.** Every utterance the gate opened is kept, outcome `none` without a command; the size limit bounds it.

**Mishearings.** Recurring Russian mishearings of command words from the bench and the history become respellings in `command-spotting/src/streaming/respell/`; English mishearings stay out of this change.

## Risks / Trade-offs

- [Workers add jitter] → the bench measures wall-clock time.
- [A 0.1 s breath after "stop" ends a sentence early] → "stop the car" scenario and negatives guard it.
- [A new English model is another download] → only if it wins.
- [More history entries] → existing size limit.

## Test levels

| Scenario | Level |
|---|---|
| A command said alone is reported before its pause ends; Silence wakes no recognizer; Speech without a command is no command; A reported command leaves an entry with its timeline | E2E, existing |
| A slow recognizer does not hold back the other | unit, scripted slow model |
| A quiet command opens the gate | adapter integration (`streaming`) |
| An utterance without a command is kept too | E2E (`voice_control.feature`) |
| Start said alone is reported within its bound; Stop said alone is reported at the pause after it; Press enter after a sentence is reported soon after the pause | E2E (`voice_control.feature`), real time, `@cues`: timed on an otherwise idle machine |
| Stop opening a sentence is still no command | unit (`decide`) and E2E |
| The user's recorded commands are reported within their bound; The user's recorded commands are heard | E2E, `@voice_samples` |
| A mishearing from the history counts as its command | unit (`streaming/respell/`), E2E `@voice_samples` |
