## Why

The user's voice commands, mostly "enter enter" and "stop" at the end of a dictation, are sometimes lost or come late. Half of the misses since 2026-10-29 came from the command spotting service falling behind real time under CPU load: its log reports "dropped N samples it could not keep up with" 77 times. The English streaming recognizer (nemotron 0.6b int8) takes 0.58 s per second of speech on 4 threads on an idle machine and 0.90 s with every core busy, so it has no headroom.

Measured on the user's 2482 recorded utterances (920 with a command, labelled by two Whisper large-v3-turbo hearings that agree):

- The same model through the shared ONNX Runtime library that sherpa-onnx publishes decodes 4× faster per thread than through its static library: 0.31 s per second of speech on 2 threads, 0.31 with every core busy too; recall and false alarms are unchanged (96.6 %, 19 of 1465).
- Lighter English models are cheaper but miss "enter" 2.5× as often (kroko: 92.7 %, fast-conformer: 91.3 %).
- CoreML (Neural Engine or GPU) is slower than the CPU for both models (no fp32 nemotron export exists; the int8 one falls back).

## What Changes

- The command spotting service links ONNX Runtime as the shared library and ships it beside its program, in development builds and in the app bundle.
- The English streaming recognizer decodes on 2 threads, the Russian one on 1 (was 4 each).
- With every core busy, the service keeps the bounds it has on an idle machine: it spends at most half a second on each second of streamed speech and reports "press enter" within 0.6 s of its words.
- The streaming bench takes the recognizers' threads and reports each recognizer's decode time per second of audio.

Current stage: PoC. Target stage: PoC.
Scope: speed of the two streaming command recognizers of the command spotting service.
Acceptance bar: with every core busy, the service processes speech in at most 0.5 s per second of it and reports "press enter" within 0.6 s; recall on the user's recordings stays at least as before.
Out of scope: thread priority and how the app handles a lagging service; mishearing aliases; other models or CoreML.

## Capabilities

### Modified Capabilities

- `voice-control`: streaming commands keep their report bounds while every core is busy.

## Impact

- `command-spotting/` (link mode, threads, bench), `e2e-test/` (link path, scenario), `dev-tools/src/engines/` (bundle copies the libraries).

## Release

SemVer: PATCH — commands are lost less under load; no new behavior or setting.
Target version: 0.43.1.
