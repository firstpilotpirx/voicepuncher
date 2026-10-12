# Proposal

## Why

Today every dictation starts and ends with the hotkey, so the user's hands must go back to the keyboard twice per message. The user wants a hands-free chat flow: read a message, say "start dictation", speak, say "stop dictation", say "press enter" — without touching the keyboard. This needs the app to listen all the time for a small set of spoken commands, which the current push-to-talk design cannot do.

## What Changes

- **New voice control mode**, opt-in (config file or command-line flag). With it on, listening starts with the app; the hotkey no longer records, it turns *listening* off and on again. While listening, the microphone stays open and the app reacts to five spoken English commands:

  | Phrase | Accepted while | Effect |
  |---|---|---|
  | `start dictation` | listening, not dictating | starts a dictation, as the hotkey does today |
  | `stop dictation` | dictating | ends the dictation; the text is transcribed, refined and pasted |
  | `cancel dictation` | dictating | discards the dictation; nothing is pasted |
  | `press enter` | listening, not dictating | presses Return in the focused app |
  | `clear all text` | listening, not dictating | presses Cmd+A, then Delete, in the focused app |

- **A command counts only when spoken on its own**, with a pause before and after it. The same words inside a sentence are ignored (they are dictated text, or background speech).
- **A command spoken in the wrong state is ignored** (for example `press enter` during a dictation). Each state listens only for its own phrases.
- **The spoken `stop dictation` and `cancel dictation` never reach the pasted text.**
- **Order is kept:** `press enter` said right after `stop dictation` is carried out only after the dictated text is pasted.
- **Every recognized command plays a sound**; turning listening on and off plays its own sounds.
- **The hotkey during a dictation in voice control mode** ends the dictation as `stop dictation` does, pastes, and turns listening off.
- **New local service — the command spotting service.** A small program in this repository, built on sherpa-onnx keyword spotting with the English `zipformer-gigaspeech-3.3M` model. The app streams microphone audio to it over loopback; it reports the phrases it spots and where speech starts and ends. `just start-model-servers` starts it next to the speech recognition and text refinement services; `just models` downloads its models. Whisper keeps doing only speech-to-text, and only during a dictation.
- Push-to-talk without voice control is unchanged and stays the default.

## Capabilities

### New Capabilities

- `voice-control`: the hands-free mode — turning listening on and off with the hotkey, the five spoken commands, the pause rule, which commands each state accepts, keeping command phrases out of the pasted text, command order and sounds.

### Modified Capabilities

_None._ (`push-to-talk` keeps its requirement: without voice control the hotkey works as today. The hotkey's meaning inside voice control is specified in `voice-control`.)

## Impact

- **Code:** in `app/`, a voice control state machine and command recognizer (pause rule) in the core; a port for the command spotting service and its adapter (microphone capture + loopback WebSocket client); a port for key presses (Return, Cmd+A + Delete) and its macOS adapter; `Pipeline` accepts voice commands next to hotkey signals in one ordered stream and can end a recording at a given moment so the stop phrase is cut out; config gains a `[voice_control]` section and sounds; a `--voice-control` flag.
- **New workspace member:** the command spotting service crate (binary), depending on the `sherpa-onnx` crate; models (`zipformer-gigaspeech-3.3M` KWS, Silero VAD for sherpa-onnx) in `~/.voicepuncher/models/`.
- **justfile:** `start-model-servers` / `stop-model-servers` and the E2E service helpers handle a third service on its own port; `models` fetches the new models.
- **Docs:** `docs/ubiquitous-language.md` adds *voice control*, *listening*, *voice command*, *command spotting service*, *key press*; `AGENTS.md` and `README.md` mention the third service and the new mode.
- **Tests:** unit tests for the state machine and pause rule; a component `.feature` for `voice-control` with fake command spotting and key presses; an adapter integration test and E2E `.feature` scenarios that feed `say`-synthesized commands to the real service; manual run for the microphone, hotkey and key presses.
- **Risks:** false triggers from speech nearby while listening (a wrong `press enter` sends a message) — reduced by the pause rule, per-state phrase sets and per-phrase thresholds, and by listening being opt-in and switched on by hand; a non-native accent lowering recognition — checked in the manual run, thresholds tunable; the microphone indicator stays on while listening.

## Scope

- **Lifecycle stage:** PoC, stays PoC. The voice control flow is a new technical risk this change proves viable.
- **Acceptance bar:** unit and component tests pass in `just check`; the E2E scenarios show the real service spots each of the five phrases spoken on their own and ignores them inside a sentence; `just full-check` passes before merge; the user's manual run walks the chat flow (listening on → `start dictation` → speech → `stop dictation` → `press enter`), plus `cancel dictation` and `clear all text`, with the healthy log and no stop phrase in the pasted text.
- **Out of scope:** commands in Russian or user-defined phrases; more commands (new line, undo, switching apps); a wake word or name before commands; an overlay state for listening; echo cancellation or speaker separation; replacing the hotkey flow.

## Release

MINOR — new capability; target version **0.7.0**.
