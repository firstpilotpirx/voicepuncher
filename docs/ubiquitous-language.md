# Ubiquitous Language

> Source of truth for all domain vocabulary in voicepuncher. Every term used in code, tests, diagrams, commit messages, and PR descriptions must come from this file.

## Conventions
- Each term has its own `### <Term>` heading.
- "Synonyms to AVOID" listed = review block on use.
- Glossary changes BEFORE code changes.

## voicepuncher (single bounded context)

### PttSignal
- **Type:** Boundary signal (NOT a Domain Event — nothing in the domain has changed state yet)
- **Definition:** A momentary observation that the configured hotkey was pressed or released. Emitted by the hotkey adapter; consumed by the `Pipeline` orchestrator.
- **Code:** `app/src/core/domain/ptt_signal.rs`
- **Variants:** `Pressed`, `Released`
- **Synonyms to AVOID:** "key event", "keystroke" (too generic), "trigger" (collides with non-hotkey triggers)

### HotKey
- **Type:** Value Object (immutable enum)
- **Definition:** The keyboard key the user holds for push-to-talk. Curated subset of macOS keys (modifiers + F-row + a few others).
- **Code:** `app/src/core/domain/hotkey.rs`
- **Synonyms to AVOID:** "shortcut" (implies a combo), "binding"

### AudioBuffer16k
- **Type:** Value Object
- **Definition:** A captured audio segment, mono, signed 16-bit PCM at 16 kHz — the canonical input format for whisper.cpp.
- **Code:** `app/src/core/domain/audio_buffer.rs`
- **Invariants:** sample rate is 16 kHz (constructor convention), values are `i16`, single channel.
- **Synonyms to AVOID:** "recording", "snippet", "utterance", "clip"

### RawTranscript
- **Type:** Value Object
- **Definition:** Whisper output before LLM cleanup. May contain filler words, missing punctuation, casing inconsistencies. Constructor rejects empty/whitespace.
- **Code:** `app/src/core/domain/raw_transcript.rs`
- **Synonyms to AVOID:** "transcription", "STT output", "text"

### CleanedText
- **Type:** Value Object
- **Definition:** Final text ready to paste — punctuation fixed, filler words removed, casing normalized, semantic content unchanged. Constructor rejects empty/whitespace.
- **Code:** `app/src/core/domain/cleaned_text.rs`
- **Synonyms to AVOID:** "refined transcript", "polished text", "corrected text", "LLM output"

### Pipeline
- **Type:** Use Case (Application Service)
- **Definition:** The orchestrator that consumes `PttSignal`s and drives one full PTT cycle: `Pressed → start record → Released → stop record → transcribe → refine → paste → Idle`. Holds a state machine; ignores signals while non-Idle (except in Recording).
- **Code:** `app/src/core/application/pipeline.rs`
- **States:** `Idle`, `Recording`, `Transcribing`, `Refining`, `Pasting`
- **Synonyms to AVOID:** "controller", "engine", "manager"

### Preflight
- **Type:** Use Case (Application Service)
- **Definition:** Startup gate that verifies the host environment grants the three permissions the app needs (Microphone, Accessibility, Input Monitoring) before the `Pipeline` is constructed. Calls a `PermissionProbe` and produces a single fail-fast error with the parent process name in its message.
- **Code:** `app/src/core/application/preflight.rs`
- **Synonyms to AVOID:** "permission check", "bootstrap", "startup hook"

### PermissionProbe
- **Type:** Driven Port (trait in `core/application/ports/`)
- **Definition:** Abstracts macOS-specific permission probing (`AXIsProcessTrusted`, opening the default `cpal` input device, parent process discovery via `ps`) so `Preflight` stays platform-agnostic.
- **Code:** `app/src/core/application/ports/permission_probe.rs`
- **Synonyms to AVOID:** "PermissionChecker", "PermissionService"

### SoundPlayer
- **Type:** Driven Port (trait in `core/application/`)
- **Definition:** Plays an auditory cue at the start and stop of a recording cycle. Abstracts macOS `afplay` so `Pipeline` stays platform-agnostic. Implementations may be no-ops (e.g. in tests or file mode).
- **Code:** `app/src/core/application/sound_player.rs`
- **Synonyms to AVOID:** "AudioPlayer", "BeepService"

### RecordingMode
- **Type:** Value Object (enum)
- **Definition:** Controls how the user triggers a recording cycle. `Toggle` — one press starts, a second press stops. `Hold` — hold the key while speaking, release to stop.
- **Code:** `app/src/core/domain/recording_mode.rs`
- **Variants:** `Toggle` (default), `Hold`
- **Synonyms to AVOID:** "TriggerMode", "CaptureMode"

### SetupWizard
- **Type:** Primary Adapter (interactive CLI wizard)
- **Definition:** Runs on first launch (or when `--setup` is passed) to guide the user through selecting models, downloading them, and writing the initial config file. Lives in `adapters/primary/setup/cli/`. Not a Use Case — it contains user interaction logic, not domain logic.
- **Code:** `app/src/adapters/primary/setup/cli/mod.rs`
- **Synonyms to AVOID:** "Installer", "Bootstrapper", "Onboarding"

### HotkeyListener / AudioRecorder / Transcriber / TextRefiner / TextPaster
- **Type:** Driving / Driven Ports (traits in `core/application/`)
- **Code:** `app/src/core/application/{hotkey_listener,audio_recorder,transcriber,text_refiner,text_paster}.rs`

### Project-local tech vocabulary

| `<tech>` segment | Refers to | Used in |
|---|---|---|
| `rdev` | `rdev` crate | `adapters/primary/ptt/rdev/` — `RdevHotkeyListener` |
| `cpal` | `cpal` crate | `adapters/secondary/recording/cpal/` — `CpalAudioRecorder` |
| `whispercpp` | `whisper.cpp` C++ library (FFI'd via `whisper-rs`) | `adapters/secondary/transcription/whispercpp/` — `WhisperCppTranscriber` |
| `llamacpp` | `llama.cpp` C++ library (FFI'd via `llama-cpp-2`) | `adapters/secondary/textrefinement/llamacpp/` — `LlamaCppTextRefiner` |
| `macos` | macOS native APIs (`CGEventPost` via `enigo`, `NSPasteboard` via `arboard`, `AXIsProcessTrusted` via `core-foundation`) | `adapters/secondary/textpasting/macos/`, `adapters/secondary/soundplaying/macos/`, `adapters/secondary/permissions/macos/` |
| `filesystem` | WAV file I/O via `hound` | `adapters/secondary/recording/filesystem/` — `FileSystemAudioRecorder` |
| `stdio` | stdout via `println!` | `adapters/secondary/textpasting/stdio/` — `StdioTextPaster` |
| `cli` | interactive terminal UI via `dialoguer` + `console` | `adapters/primary/setup/cli/` — `SetupWizard` |

## E2E / Test vocabulary

> These terms describe reference data used for accuracy measurement only. They are NOT part of the production domain.

### Corpus
- **Type:** E2E/test reference dataset (not a domain type)
- **Definition:** A fixed set of audio files paired with GroundTruth transcripts, used for WER measurement in the E2E benchmark suite.
- **Code:** `e2e-test/src/corpus.rs`

### GroundTruth
- **Type:** Value within Corpus (not a domain type)
- **Definition:** The human-verified reference transcript for one corpus sample. Stored as the `reference` field in `manifest.jsonl`.
- **Code:** `e2e-test/src/corpus.rs` (field `Sample.reference`)
- **Synonyms to AVOID:** "label", "annotation", "transcript" (use `RawTranscript` for Whisper output, `GroundTruth` for reference data)
