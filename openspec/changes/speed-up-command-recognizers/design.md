## Context

`sherpa-onnx-sys` links ONNX Runtime 1.28.2 statically by default. Its published shared library of the same version runs the int8 nemotron encoder about 4× faster per thread on the M1 Max (1 thread: 0.44 against 1.83 s per second of audio), and stays at 0.31 on 2 threads with ten busy-looping processes beside it, where the static build on 4 threads slows from 0.58 to 0.90. Measurements: the streaming bench over the user's 2482 utterances (proposal).

## Decisions

### Link ONNX Runtime as the shared library

The `sherpa-onnx` dependency of `command-spotting` takes its `shared` feature. Its build script copies `libsherpa-onnx-c-api.dylib` and `libonnxruntime.dylib` into Cargo's profile folder; a build script of `command-spotting` and of `e2e-test` adds the run paths `@loader_path` (programs) and `@loader_path/..` (test binaries in `deps/`), so every binary finds them without environment variables. The bundle stages both libraries beside `voicepuncher-command-spotting`; the bundle check already requires every `@rpath` reference to be in the engine's folder and runs the program with a clean environment.

Rejected: CoreML — slower than the CPU for both models, and nemotron has no fp32 export; a lighter English model — misses "enter" 2.5× as often; building ONNX Runtime ourselves — a toolchain to maintain for what the published library already gives.

### One user-interactive thread per recognizer

Each recognizer already decodes on a thread of its own (`streaming/session/decoder.rs`); that thread sets its service class to `QOS_CLASS_USER_INTERACTIVE` when it starts (a failure is logged and decoding goes on). macOS gives new threads the default class whatever their creator's, so ONNX Runtime's own pool threads stay at the default class: with ten busy loops beside it, 2 English threads reported "press enter" 1.9 s late at the 90th percentile, 1 thread 0.74 s (idle: 0.73 s). So the streaming default of `--threads` becomes 1 for both recognizers; the shared library makes 1 thread enough on an idle machine (0.44 s per second of audio). The keyword spotter keeps 2.

Rejected: a real-time thread class — it would starve the audio and the rest of the app when a decode runs long.

### The load in the test

The E2E step keeps every core busy with spinning threads of the test process (one per available core) while it streams in real time, The scenario sits with the other wall-clock scenarios under `@cues`, which run with the machine otherwise idle.

## Test levels

| Scenario | Level |
|---|---|
| Press enter after a sentence is reported in time with every core busy | E2E `.feature` (`voice_control.feature`, `@cues`) |

The thread defaults are unit tests of `command-spotting/src/main.rs`; the bundle's libraries are covered by the existing `e2e-bundle` stage, which starts the bundled service with nothing but the system folders.

## Risks

- A future `sherpa-onnx` release may change the shared archive's layout; the bundle check fails loudly then.
- The measured gain is on the M1 Max; other Apple Silicon chips were not measured.
