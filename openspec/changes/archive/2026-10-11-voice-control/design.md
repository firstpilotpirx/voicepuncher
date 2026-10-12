# Design

## Context

- `Pipeline` (`app/src/core/application/pipeline.rs`) runs on its own thread and reads one `mpsc::Receiver<PttSignal>` fed by the hotkey listener. `step` is synchronous: a stop runs drain → refine → focus wait → paste before the next signal is read. The focus wait also reads this channel to catch a hotkey press.
- `CpalAudioRecorder` opens a cpal input stream only between `start` and `stop`; live chunks go to whisper while recording.
- `MacOsTextPaster` already posts keyboard events with `CGEvent` (Cmd+V), so the Accessibility permission covers synthetic key presses.
- `AGENTS.md` requires every model to run in a separate local service reached over loopback; the user confirmed this holds for the command model too. sherpa-onnx ships keyword-spotting CLI tools and ASR WebSocket servers, but no keyword-spotting server.
- The official `sherpa-onnx` crate (1.13.8, Apache-2.0, static linking by default) exposes `KeywordSpotter` (keywords from a buffer, per-stream keywords, result with start time and token timestamps) and a Silero VAD.

## Goals / Non-Goals

**Goals:**
- Spot the five phrases with a model running outside the app, and keep every rule of the spec (pause rule, per-state commands, order, stop phrase cut) in the core, where unit and component tests reach it without a model.
- Leave push-to-talk untouched when voice control is off.

**Non-Goals:**
- Sharing one microphone stream between the recorder and the command spotting capture.
- Tuning thresholds beyond the defaults needed to pass the E2E scenarios and the manual run.

## Decisions

### 1. The command spotting service is a new workspace binary

New workspace member `command-spotting/` (package and binary `voicepuncher-command-spotting`), built on `sherpa-onnx` = `1.13.8` (exact pin, default static feature). It listens on `127.0.0.1:8082` (`VOICEPUNCHER_KWS_PORT`) and speaks WebSocket over loopback (`tungstenite`, synchronous, one thread per connection — the app is thread-based and uses blocking `ureq` already). It opens no microphone: like whisper-server it only turns audio into results.

- Alternatives: in-process `KeywordSpotter` (simpler, rejected by the user to keep the separate-services rule); sherpa-onnx's ASR WebSocket server with a streaming ASR model and phrase matching in the app (not the chosen KWS model, heavier, and phrase matching on free text is what KWS avoids); HTTP requests per audio slice (connection and JSON overhead on a continuous stream).

**Protocol** (one connection per listening session):
- Client → server, text frame `{"phrases": ["start dictation", "press enter", "clear all text"]}`: selects the phrase set; may be sent again at any time. The server builds a new keyword stream for the set (`create_stream_with_keywords`) and keeps its sample clock running.
- Client → server, binary frames: 16 kHz mono `f32` little-endian samples, about 100 ms per frame.
- Server → client, text frames, times in seconds since the first sample of the connection:
  - `{"event": "speech_start", "t": 3.10}` and `{"event": "speech_end", "t": 4.62}` from the Silero VAD (minimum silence 0.4 s — this is what "a pause" means);
  - `{"event": "phrase", "phrase": "press enter", "start": 3.18, "end": 4.05}` from the keyword spotter.
- Phrases outside the known five are rejected with a close frame naming them.

**Keywords:** the gigaspeech KWS model needs keywords as BPE tokens. The crate commits a `keywords.txt` with each phrase's tokens, generated once from the model's `bpe.model`, plus a per-phrase boost and threshold and the phrase after `@` with its words joined by `_` (sherpa-onnx splits the label on spaces; the service turns `_` back into spaces). A phrase may have several lines: `press enter` runs together into "presenter", so it also lists those tokens. Thresholds live in `keywords.txt` (lower = easier to trigger; `press enter` and `clear all text` needed a lower threshold and a higher boost to be spotted in `say` speech); the service's command-line arguments are the port, the model directory, the VAD model, the thread count and the minimum silence. The spotter's token times do not count from the start of the stream, so they only give the phrase's length: the service decodes in 100 ms frames, places the phrase end at the stream position where it was spotted less a 0.25 s detection lag, and the start that length earlier, ignoring a stray first token heard in noise.

### 2. The pause rule and the state rules live in the core

- `VoiceCommandRecognizer` (domain, pure): fed `speech_start`, `speech_end` and `phrase` events in order; at each `speech_end` it emits a `VoiceCommand` when the segment held exactly one phrase and the phrase starts ≤ 0.3 s after the segment start and ends ≤ 0.5 s before the segment end (the voice activity detector holds quiet speech longer). Anything else in the segment (extra words before or after) drops the phrase. Latency is the VAD's 0.4 s silence after the phrase.
- `VoiceCommand` (domain enum): `StartDictation`, `StopDictation`, `CancelDictation`, `PressEnter`, `ClearAllText`; the recognizer emits a `SpokenCommand` = command + the `Instant` its phrase started.
- The state rules are in `Pipeline`, which already owns `Idle`/`Recording`; voice control adds a listening flag beside it (off, or listening: standby when `Idle`, dictating when `Recording`). A command not accepted in the current state is logged and dropped (spec: each state accepts only its own commands). The per-state phrase set sent to the service is an accuracy aid (`start` vs `stop` never compete); the core check is the rule.
- Alternative: run the pause rule in the service. Rejected: it is a business rule the spec states, and in the core it is unit-testable with scripted events.

### 3. One ordered input stream into the pipeline

`Pipeline::run` takes `Receiver<PipelineInput>`, `PipelineInput = Hotkey(PttSignal) | Spotting(SpottingEvent) | SpottingLost`. The hotkey listener port keeps sending `PttSignal`; `main` forwards it into the shared channel. Because `step` is synchronous, a `press enter` that arrives while a stop is still being transcribed, refined and pasted waits in the channel — the order requirement holds by construction. The focus wait reads the channel for a hotkey press; any other input it reads is pushed to a pending queue that `run` drains before reading the channel again, so no command is lost during a wait.

- Alternative: a second channel for voice commands. Rejected: two channels lose the spoken order against hotkey presses.

### 4. Command spotting port and adapter

- Port `CommandSpotter` (`app/src/core/application/command_spotter.rs`): `start_listening(commands)`, `listen_for(commands)`, `stop_listening()`, plus `probe()` for the startup check. The adapter is built with a sender into the pipeline's input channel. The pipeline drives it (listening on/off, phrase set on every state change).
- Adapter `app/src/adapters/secondary/commandspotting/websocket/`: opens its own cpal input stream while listening (CoreAudio allows several input streams on one device), resamples to 16 kHz mono, streams frames, parses server events, converts service times to `Instant` (`capture start + t`) and sends `PipelineInput::Spotting` into the channel; a closed or failed connection sends `SpottingLost` once.
- `main` calls `probe()` at start when voice control is on and exits with code 2 and `command spotting service not reachable at ws://127.0.0.1:8082 — run 'just start-model-servers'` when it fails (as it does today for the other services).
- A service that hangs without closing the socket is noticed by WebSocket pings: the client pings every 2 s, and when nothing — event or pong — arrives for 10 s it reports `SpottingLost`; sends time out after 2 s, so a service that stopped reading cannot block the worker.
- On `SpottingLost` the pipeline logs an error, ends a running dictation as a hotkey press would (stop and paste, so no speech is lost) and turns listening off with the listening-off sound.

### 5. The stop phrase is cut out of the audio

`stop dictation` carries the `Instant` its phrase started. Before the usual stop the pipeline calls `AudioRecorder::truncate_at(cut)`: the recorder stops appending and drops the samples captured from `cut` on (it maps `cut` to a sample index from its own capture start), so the following `flush_live_chunks` and `stop` deliver only the audio before the phrase, which therefore never reaches whisper. A live chunk already cut and sent inside the ~1.5 s between the phrase start and the command is not taken back; the chunk cutter prefers the pause before the phrase, so this is rare. Clock skew between the two capture streams is a few tens of milliseconds; the pause before the phrase (≥ 0.4 s by the VAD) absorbs it. `cancel dictation` discards the audio anyway; `start dictation` is spoken before the recording starts.

- Alternative: strip the phrase from the transcript. Rejected: whisper with auto language detection transliterates or translates it unpredictably inside Russian speech.

### 6. Key presses

Port `KeyPresser` (`press_enter`, `clear_all_text`) with the adapter `app/src/adapters/secondary/keypressing/macos/` posting `CGEvent`s: Return; Cmd+A, ~30 ms, Delete. No new permission: Accessibility is already required.

### 7. Configuration, sounds, hotkey

- `[voice_control]`: `enabled` (default `false`), `url` (default `ws://127.0.0.1:8082`, loopback-only check as for the other services). CLI flag `--voice-control` turns it on.
- `[sounds]` gains `listening_on_sound` (default `Tink`), `listening_off_sound` (`Bottle`) and `command_sound` (`Morse`); `SoundPlayer` gains the matching methods. Dictation start, stop and cancel keep their existing sounds.
- With voice control on, listening starts when the pipeline starts running (the user asked for no hotkey press at start). The hotkey is read as a toggle whatever `recording_mode` says: `Pressed` toggles listening (or ends a dictation and turns listening off); `Released` is ignored; `Cancel` (Escape) during a dictation cancels it as `cancel dictation` does, and listening stays on.

### 8. Models and services in the justfile

- `just models` also fetches `sherpa-onnx-kws-zipformer-gigaspeech-3.3M-2024-01-01` (sherpa-onnx `kws-models` release tarball, unpacked into `~/.voicepuncher/models/`) and sherpa-onnx's `silero_vad.onnx`.
- `start-model-servers` builds the service (`cargo build --release -p voicepuncher-command-spotting`) and starts it as a third service on its port, readiness = port open; `stop-model-servers` and the E2E service helpers know it. New env vars `VOICEPUNCHER_KWS_MODEL_DIR` and `VOICEPUNCHER_KWS_VAD_MODEL` follow the existing auto-discovery.

### 9. Test levels

| Spec scenario | Level |
|---|---|
| The hotkey records when voice control is off | component `.feature` |
| The hotkey does not record when voice control is on | component `.feature` |
| Listening is on at start | component `.feature`; unit (pipeline starts listening when it runs) |
| A hotkey press turns listening on | component `.feature` |
| A hotkey press turns listening off | component `.feature` |
| Saying start dictation starts a dictation | component `.feature`; E2E `.feature` (real service spots the phrase) |
| Saying stop dictation pastes the dictated text | component `.feature` (cut at the phrase start); E2E `.feature` (real service spots the phrase) |
| Saying cancel dictation pastes nothing | component `.feature`; E2E `.feature` (real service spots the phrase) |
| Saying press enter presses Return | component `.feature`; E2E `.feature` (real service spots the phrase) |
| Saying clear all text selects all and deletes | component `.feature`; E2E `.feature` (real service spots the phrase) |
| A command phrase spoken on its own is recognized | E2E `.feature` (each of the five phrases, real service + recognizer on `say` audio) |
| A command said after a long time of listening is recognized | unit (`phrase_span`), service integration (`engine_it`: 25 s of silence, several commands in one long stream, one large message), adapter integration (48 kHz microphone path after 30 s of listening), E2E `.feature` (each phrase after 30 s of listening) |
| A command phrase inside a sentence is ignored while listening | unit (recognizer); E2E `.feature` (real service + recognizer on `say` audio) |
| Ordinary speech triggers no command | E2E `.feature` `@accuracy` (benchmark corpus recordings through the real service, both phrase sets) |
| A stop phrase inside a dictated sentence does not end the dictation | unit (recognizer); component `.feature` |
| Press enter during a dictation is ignored | component `.feature` |
| Stop dictation without a dictation is ignored | component `.feature` |
| Press enter right after stop dictation follows the paste | component `.feature` |
| A message is dictated and sent by voice alone | adapter integration (the commands in one connection with the phrase set switched), E2E `.feature` (real pipeline and service, stand-in recorder, transcriber, paster and key presser) |
| A hotkey press during a dictation pastes and stops listening | component `.feature` |
| The app refuses to start without the command spotting service | E2E `.feature` (runs the binary, no service on the port) |
| Listening turns off when the command spotting service goes away | component `.feature` |

- Component tests (`app/tests/features/component/voice_control.feature`, steps `app/tests/steps/voice_control.rs`) drive the real `Pipeline` with a scripted `CommandSpotter` that emits speech and phrase events, and stand-ins for recorder, transcriber, paster, key presser and sound player, as `text_insertion.rs` does.
- E2E (`e2e-test/tests/features/voice_control.feature`, tagged `@smoke` so `just e2e-smoke` runs it with the services up) synthesizes each phrase with `say`, alone and inside a sentence, and streams the audio through the app's WebSocket client logic and the recognizer to the real service.
- Plain tests: adapter integration `app/tests/commandspotting_websocket_it.rs` (protocol contract against the real service, run by `e2e-smoke`), `app/tests/keypressing_macos_it.rs` (gated like `textpasting_macos_it.rs`), config loader tests, unit tests in the service crate for protocol parsing and phrase-set validation.
- Manual run: microphone, hotkey, real key presses and sounds — the chat flow of the proposal's acceptance bar.

## Risks / Trade-offs

- [False trigger from nearby speech sends a message] → pause rule (the phrase must be the whole utterance), per-state phrase sets, listening switched on by hand; the command sound makes every trigger audible.
- [Accent lowers recognition of the English phrases] → thresholds and boosts are tuned in `command-spotting/keywords.txt`; the manual run checks recognition on the user's voice.
- [Wrong BPE tokens in `keywords.txt` silently never fire] → the E2E scenarios spot every phrase with the real model.
- [sherpa-onnx static build downloads native archives at build time; `full-check` runs `cargo clean`] → build-time network only, runtime stays offline; cost is a slower clean build.
- [The command sound and the recorder's microphone overlap: the command sound is picked up while listening] → the sound plays after the phrase's segment ended; a short sound does not form a phrase.
- [The spotter places a phrase start only to about ±0.25 s, and in noise its first token may fire early] → the service attaches a phrase to the speech segment it ends in and places its end by the stream position (decoded in 100 ms frames); the app rejects speech more than 0.3 s before or 0.5 s after the phrase. Limit: one short word right before or after a command ("please press enter", "press enter now") is within that error and still counts; longer speech before ("I think we should press enter") or after ("press enter when you are ready") is rejected.
- [Clock drift between the command capture and the recorder over a long session moves the stop-phrase cut] → both clocks start when their stream starts; drift of 50–100 ppm adds 0.2–0.4 s per hour of listening. Checked by the manual run; a long soak run is not automated.
- [Commands said during a focus wait are carried out after it] → voice inputs that arrive while the app waits for a text input are queued in order and run after the paste or timeout, like a hotkey press during processing; a stale "press enter" may go to the window focused by then. Accepted for the PoC.
- [A microphone that disappears while listening (device switch, unplugged USB microphone)] → the capture error is logged; the pings keep the service alive, so listening stays on with no audio until the hotkey turns it off and on.
- [`clear all text` in an editor erases the whole document] → stated in the README; the pause rule keeps it from firing inside a sentence.

## Migration Plan

Opt-in: nothing changes until the user turns voice control on and runs `just models` and `just start-model-servers`. Rollback: turn it off.
