## Why

The user's voice commands, mostly "enter enter" and "stop" at the end of a dictation, are sometimes lost or come late. Half of the misses since 2026-10-29 came from the command spotting service falling behind real time under CPU load: its log reports "dropped N samples it could not keep up with" 77 times. The English streaming recognizer (nemotron 0.6b int8) takes 0.58 s per second of speech on 4 threads on an idle machine and 0.90 s with every core busy, so it has no headroom.

Measured on the user's 2482 recorded utterances (920 with a command, labelled by two Whisper large-v3-turbo hearings that agree):

- The same model through the shared ONNX Runtime library that sherpa-onnx publishes decodes 4× faster per thread than through its static library: 0.31 s per second of speech on 2 threads, 0.31 with every core busy too; recall and false alarms are unchanged (96.6 %, 19 of 1465).
- Lighter English models are cheaper but miss "enter" 2.5× as often (kroko: 92.7 %, fast-conformer: 91.3 %).
- CoreML (Neural Engine or GPU) is slower than the CPU for both models (no fp32 nemotron export exists; the int8 one falls back).
- Truly saturated cores (ten busy loops at the default service class) still slow it: "press enter" came 3.7 s late at the 90th percentile. Its decoding thread at the user-interactive service class, alone (ONNX Runtime's extra threads keep the default class), brings that to 0.74 s, the idle machine's 0.73 s.

## What Changes

- The command spotting service links ONNX Runtime as the shared library and ships it beside its program, in development builds and in the app bundle.
- Each streaming recognizer decodes on 1 thread (was 4), its own decoding thread, at the user-interactive service class.
- With every core busy, the service reports "press enter" within 1 s of its words (it took 3–4.7 s; 0.6 s on an idle machine).
- Voice commands lose their "please" form (the user never says it): its keyword lines, respellings and the cut of a trailing "please" from the dictation go, with every test and scenario of it.
- The keyword spotter's start trigger fires at 0.10 instead of 0.15, so its start cue keeps its bound on the shared ONNX Runtime.
- The streaming bench reports the time it waited for the service per second of audio.

Current stage: PoC. Target stage: PoC.
Scope: speed of the two streaming command recognizers of the command spotting service.
Acceptance bar: with every core busy, the service reports "press enter" within 1 s; recall on the user's recordings stays at least as before.
Out of scope: the priority of builds and tests, mishearing aliases, and how the app handles a lagging service; other models or CoreML.

## Capabilities

### Modified Capabilities

- `voice-control`: streaming commands keep their report bounds while every core is busy; no command has a "please" form any more.

## Impact

- `command-spotting/` (link mode, threads, bench), `e2e-test/` (link path, scenario), `dev-tools/src/engines/` (bundle copies the libraries).

## Release

SemVer: MINOR — the "please" form of the commands is removed (pre-1.0, a breaking change is MINOR); commands are lost less under load.
Target version: 0.44.0.
