## Context

`sherpa-onnx-sys` links ONNX Runtime 1.28.2 statically by default. Its published shared library of the same version runs the int8 nemotron encoder about 4× faster per thread on the M1 Max (1 thread: 0.44 against 1.83 s per second of audio), and stays at 0.31 on 2 threads with ten busy-looping processes beside it, where the static build on 4 threads slows from 0.58 to 0.90. Measurements: the streaming bench over the user's 2482 utterances (proposal).

## Decisions

### Link ONNX Runtime as the shared library

The `sherpa-onnx` dependency of `command-spotting` takes its `shared` feature. Its build script copies `libsherpa-onnx-c-api.dylib` and `libonnxruntime.dylib` into Cargo's profile folder; a build script of `command-spotting` and of `e2e-test` adds the run paths `@loader_path` (programs) and `@loader_path/..` (test binaries in `deps/`), so every binary finds them without environment variables. The bundle stages both libraries beside `voicepuncher-command-spotting`; the bundle check already requires every `@rpath` reference to be in the engine's folder and runs the program with a clean environment.

Rejected: CoreML — slower than the CPU for both models, and nemotron has no fp32 export; a lighter English model — misses "enter" 2.5× as often; building ONNX Runtime ourselves — a toolchain to maintain for what the published library already gives.

### One user-interactive thread per recognizer

Each recognizer already decodes on a thread of its own (`streaming/session/decoder.rs`); that thread sets its service class to `QOS_CLASS_USER_INTERACTIVE` when it starts (a failure is logged and decoding goes on). macOS gives new threads the default class whatever their creator's, so ONNX Runtime's own pool threads stay at the default class: with ten busy loops beside it, 2 English threads reported "press enter" 1.9 s late at the 90th percentile, 1 thread 0.74 s (idle: 0.73 s). So the streaming default of `--threads` becomes 1 for both recognizers; the shared library makes 1 thread enough on an idle machine (0.44 s per second of audio). The keyword spotter keeps 2.

Rejected: a real-time thread class — it would starve the audio and the rest of the app when a decode runs long.

### No "please" form; the keyword spotter's start trigger at 0.10

The keyword spotter (the recognizer the scripts start unless told otherwise) runs on the same ONNX Runtime, whose shared library computes slightly differently: a "start" followed by speech no longer reached the start trigger's threshold of 0.15, so its start cue waited for the next Whisper check (721–758 ms instead of about 510). At 0.10 the cue is back (515 ms), but a "start" spotted that early is confirmed before a following "please" is heard. The user never says "please" after a command, so the "please" form goes from the product and the tests (keyword lines, `TRAILING_WORDS`, the respelling of «сто плиз», the cut of a trailing "please"), and the trigger fires at 0.10. A trigger only asks the speech recognition service for commands to check the utterance at once, so a false one costs a request, never a command.

The start cue requirement is removed and added again under a new name ("… within 0.7 s of its words"): a MODIFIED block cannot drop its two "start please" scenarios.

### The load in the test

The E2E step keeps every core busy with spinning threads of the test process (one per available core) while it streams in real time, The scenario sits with the other wall-clock scenarios under `@cues`, which run with the machine otherwise idle.

## Test levels

| Scenario | Level |
|---|---|
| Press enter after a sentence is reported in time with every core busy | E2E `.feature` (`voice_control.feature`, `@cues`) |
| Start start followed by the dictation at once gives its cue at once | E2E `.feature` (existing) |
| Start followed by the dictation at once gives its cue at once | E2E `.feature` (existing) |
| Start said on its own gives its cue at once | E2E `.feature` (existing) |
| Restart restart followed by the new dictation at once gives its cue at once | E2E `.feature` (existing) |
| Restart said on its own gives its cue at once | E2E `.feature` (existing) |
| The user's recorded starts give their cue at once | E2E `.feature` (existing; "start please" recordings skipped, so 6 remain and the bar is 5 of 6: 9 of 10 of six would allow no miss, stricter than before) |
| The overlay shows with the start cue | component `.feature` (existing) |
| A faster start keeps the dictation where the command's words end | E2E (existing) |
| A faster start adds no false start | E2E (existing); "start stop" at service integration (existing) |
| A brief dip inside the command words starts nothing early | service integration (existing) |
| The start cue is timed once the other checks' builds have ended | E2E harness (existing) |
| A late start cue names what ran beside it | E2E harness (existing) |
| The start cue is timed against warm copies | E2E harness (existing) |
| A missing start cue names the confirmations cut short | E2E harness (existing) |
| Saying clear text selects all and deletes | unit (matcher); E2E `.feature` (existing, without "please") |
| A hands-free session in a synthesized voice | E2E `.feature` (existing, without "please") |
| A hands-free session with the user's recorded commands | E2E `.feature` (existing) |
| A hands-free session with long pauses between its steps | E2E `.feature` (existing) |
| A short form is recognized as its command | E2E `.feature` (existing, "please" rows removed) |
| Cancel alone discards a dictation | E2E `.feature` short-form table (existing) |
| Saying restart drops the dictation and starts a new one | component `.feature` (existing) |
| Only the speech after a restart is pasted | component `.feature` (existing) |
| Restart restart in the middle of speech | E2E `.feature` (existing) |
| Stop right after the last dictated word | component `.feature` (existing) |
| Enter right after other speech | unit (matcher); E2E `.feature` (existing) |
| A command word inside a sentence is still no command | E2E `.feature` (existing) |
| A command whose first hearing is empty under load is reported before the sync is answered | E2E `.feature` (existing) |
| A command whose half-pause hearing is misheard is reported before the sync is answered | E2E `.feature` (existing) |
| A Russian-accented stop is heard as stop | E2E `.feature` (existing) |
| A Russian word that is no command stays speech | E2E `.feature` (existing) |

The thread defaults are unit tests of `command-spotting/src/main.rs`; the bundle's libraries are covered by the existing `e2e-bundle` stage, which starts the bundled service with nothing but the system folders.

## Risks

- A future `sherpa-onnx` release may change the shared archive's layout; the bundle check fails loudly then.
- The measured gain is on the M1 Max; other Apple Silicon chips were not measured.
