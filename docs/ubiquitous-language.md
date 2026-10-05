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
- **Variants:** `Pressed`, `Released`, `Cancel`
- **Synonyms to AVOID:** "key event", "keystroke" (too generic), "trigger" (collides with non-hotkey triggers)

### HotKey
- **Type:** Value Object (immutable enum)
- **Definition:** The keyboard key or chord used to control recording. Curated subset of macOS keys, including CmdE and LeftOptionE (left Option + E).
- **Code:** `app/src/core/domain/hotkey.rs`
- **Synonyms to AVOID:** "shortcut" (implies a combo), "binding"

### AudioBuffer16k
- **Type:** Value Object
- **Definition:** A captured audio segment, mono, signed 16-bit PCM at 16 kHz — the canonical input format for Whisper-family speech recognition; sent to the speech recognition service as a WAV file.
- **Code:** `app/src/core/domain/audio_buffer.rs`
- **Invariants:** sample rate is 16 kHz (constructor convention), values are `i16`, single channel.
- **Synonyms to AVOID:** "recording", "snippet", "utterance", "clip"

### RawTranscript
- **Type:** Value Object
- **Definition:** The speech recognition service's output before text refinement. May contain filler words, missing punctuation, casing inconsistencies. Constructor rejects empty/whitespace.
- **Code:** `app/src/core/domain/raw_transcript.rs`
- **Synonyms to AVOID:** "transcription", "STT output", "text"

### CleanedText
- **Type:** Value Object
- **Definition:** Final text ready to paste — the text refinement service's normalization of a `RawTranscript` (punctuation, capitals, filler sounds, misrecognized words, technical terms and wrong word endings fixed; meaning and language unchanged; reasoning removed) that passed the normalization check, or the raw transcript itself when refinement is disabled, fails or its answer is rejected. Constructor rejects empty/whitespace.
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
- **Type:** Driven Port (trait in `core/application/`)
- **Definition:** Abstracts macOS-specific permission probing (`AXIsProcessTrusted`, opening the default `cpal` input device, parent process discovery via `ps`) so `Preflight` stays platform-agnostic.
- **Code:** `app/src/core/application/permission_probe.rs`
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
- **Definition:** Runs on first launch (or when `--setup` is passed) to ask for the local service addresses, language, hotkey, recording mode and sounds, and writes the initial config file. Downloads nothing. Lives in `adapters/primary/setup/cli/`. Not a Use Case — it contains user interaction logic, not domain logic.
- **Code:** `app/src/adapters/primary/setup/cli/mod.rs`
- **Synonyms to AVOID:** "Installer", "Bootstrapper", "Onboarding"

### VisualIndicator
- **Type:** Driven Port (trait in `core/application/`)
- **Definition:** Surfaces the current `Pipeline` state to the user as
  an on-screen widget — three discrete signals: recording in progress,
  processing in progress, hidden. Abstracts the platform-specific
  always-on-top overlay so `Pipeline` stays platform-agnostic.
- **Code:** `app/src/core/application/visual_indicator.rs`
- **Synonyms to AVOID:** "Overlay", "HUD", "StatusBadge", "Toast", "Indicator" (alone — too generic)

### Speech recognition service
- **Type:** External system (outside the app process)
- **Definition:** A process on the same machine that turns audio into a `RawTranscript` over the OpenAI Audio Transcriptions API — whisper.cpp `whisper-server` is the supported one. The user starts it (normally with `just start-model-servers`); the app only connects to it, checks it is reachable at startup, and rejects any address that is not loopback.
- **Code:** `app/src/adapters/secondary/transcription/openai/` — `OpenAiTranscriber`; config section `[transcription]` (`app/src/config/transcription.rs`)
- **Synonyms to AVOID:** "STT server", "Whisper server" (names one implementation), "backend", "engine"

### Text refinement service
- **Type:** External system (outside the app process)
- **Definition:** A process on the same machine that corrects a `RawTranscript` into `CleanedText` over the OpenAI Chat Completions API — llama.cpp `llama-server` is the supported one. The app sends the configured system prompt as the instruction and the transcript as delimited data. The user starts it (normally with `just start-model-servers`); the app only connects to it, checks at startup that it answers and offers the configured model (unless refinement is disabled), rejects any address that is not loopback, and pastes the raw transcript when it fails.
- **Code:** `app/src/adapters/secondary/textrefinement/openai/` — `OpenAiTextRefiner`; config section `[refinement]` (`app/src/config/refinement.rs`)
- **Synonyms to AVOID:** "LLM server", "llama server" (names one implementation), "backend", "engine"

### Text refinement
- **Type:** Pipeline step
- **Definition:** Normalizing a `RawTranscript` before it is pasted — punctuation, capitals, filler sounds removed, misrecognized words, technical terms and wrong word endings (case, gender, number, conjugation) fixed, meaning and language kept — done by the text refinement service and accepted only after the normalization check. Dictated words that sound like commands are normalized, never carried out. Disabled with `--no-llm`: the raw transcript is pasted unchanged.
- **Code:** `app/src/core/application/text_refiner.rs` — `TextRefiner` port; `refine_or_raw`
- **Synonyms to AVOID:** "LLM cleanup", "post-processing", "polishing"

### Normalization check
- **Type:** Domain policy (pure function)
- **Definition:** Decides whether a text refinement service answer is still the user's dictation: it must keep the transcript's writing system (Cyrillic share of letters moves by at most 50 points) and at least half of its Cyrillic and half of its Latin words, filler sounds aside — a word whose ending was corrected (время → времени) still counts as kept — so translating only the Russian part of a mixed dictation is caught too. A rejected answer — a translation, an answer to a dictated question, a rewrite — is logged and the raw transcript is pasted instead.
- **Code:** `app/src/core/domain/normalization_check.rs` — `check_normalization`, `NotANormalization`
- **Synonyms to AVOID:** "guard", "injection filter", "sanity check"

### Filler sound
- **Type:** Domain policy (closed word list)
- **Definition:** A hesitation sound the user never means to write — `э`, `э-э`, `эм`, `ммм`, `um`, `uh`, `hmm`, … Removed with the comma after it from the `RawTranscript` before text refinement; ignored by the normalization check. Words like "ну" or "типа" are not filler sounds.
- **Code:** `app/src/core/domain/filler_sounds.rs` — `is_filler_sound`, `without_filler_sounds`
- **Synonyms to AVOID:** "filler words", "disfluencies", "noise"

### Reasoning block
- **Type:** Part of a text refinement service answer
- **Definition:** The `<think>…</think>` text a reasoning model puts before its answer. Removed before the answer becomes `CleanedText`, so it is never pasted.
- **Code:** `app/src/adapters/secondary/textrefinement/openai/request.rs` — `without_reasoning`
- **Synonyms to AVOID:** "thoughts", "chain of thought"

### Recognition hallucination
- **Type:** Application policy (pure functions in `core/application/`)
- **Definition:** A transcript the speech recognition service invents instead of hearing speech — subtitle credits, subscription calls, blank-audio markers, or one phrase repeated in a loop. Discarded instead of pasted.
- **Code:** `app/src/core/application/transcript_filter.rs` — `is_whisper_hallucination`
- **Synonyms to AVOID:** "noise", "garbage output", "false positive"

### Chunk threshold
- **Type:** Configuration value (`[transcription].chunk_threshold_secs`)
- **Definition:** Recording length above which a dictation or a recording file is split into overlapping chunks, each transcribed through the speech recognition service, and the texts merged at the chunk seams. It also bounds every chunk, its overlap with the previous chunk included: the service hears at most 30 s at a time.
- **Code:** `app/src/core/application/chunking.rs` — `ChunkingCfg`, `next_cut`, `split_into_chunks`
- **Synonyms to AVOID:** "segment size", "window"

### Chunk seam
- **Type:** Domain concept
- **Definition:** Where two neighbouring chunks meet: the next chunk starts with the last seconds of the previous one (the overlap), so both transcripts contain those words, often worded a little differently. The merge aligns the two transcripts, keeps the overlap once and drops nothing where they share no words.
- **Code:** `app/src/core/application/chunking.rs` — `merge_chunk_texts`
- **Synonyms to AVOID:** "boundary" (the cut point is only one side of a seam), "join"

### Pause
- **Type:** Domain concept
- **Definition:** A stretch of at least 160 ms in which every 20 ms frame is quieter than the noise floor allows: a gap between words or phrases, longer than a microphone dropout. A chunk ends in the middle of the longest pause within the lookback before the chunk threshold (the latest of equally long ones); without one it ends at the threshold.
- **Code:** `app/src/core/application/chunking.rs` — `next_cut`, `MIN_PAUSE_MS`
- **Synonyms to AVOID:** "silence" (a single quiet frame inside a word is not a pause), "VAD"

### Noise floor
- **Type:** Domain concept
- **Definition:** The background loudness of a chunk, taken as the 10th percentile of its 20 ms frame loudness. A frame is quiet below twice the noise floor, but never above half the median frame; a frame below an RMS of 0.002 is always quiet. So pauses are found in noisy and in quiet recordings without a configured level.
- **Code:** `app/src/core/application/chunking.rs` — `quiet_level`
- **Synonyms to AVOID:** "threshold", "VAD level"

### Text input focus
- **Type:** Value Object (`TextFocus`) decided from a `FocusedElement` (role, text selection, settable value)
- **Definition:** Whether the final text can be pasted now. The app holds the text back only when it is sure no text input has keyboard focus: the focused element is a clearly non-text control (button, list, table, outline, image, menu, check box, radio button, slider, tab group, toolbar) that offers neither a text selection nor an editable value. Any other element, no reported element or an Accessibility error means "paste now".
- **Code:** `app/src/core/domain/text_focus.rs` — `FocusedElement`, `TextFocus`
- **Synonyms to AVOID:** "editable focus", "cursor" (the caret is not what is checked)

### Focus wait
- **Type:** Application policy (`[paste].focus_wait_seconds`, default 300; 0 turns it off)
- **Definition:** The time the app holds the final text back while no text input has focus, re-checking about every 100 ms and pasting as soon as one has. A hotkey press or the end of the wait time ends it without a paste: the text is left in the clipboard. The overlay shows the wait.
- **Code:** `app/src/core/application/text_insertion.rs` — `insert_text`, `FocusWaitCfg`
- **Synonyms to AVOID:** "retry", "paste delay"

### HotkeyListener / AudioRecorder / Transcriber / TextRefiner / TextPaster / FocusProbe
- **Type:** Driving / Driven Ports (traits in `core/application/`)
- **Code:** `app/src/core/application/{hotkey_listener,audio_recorder,transcriber,text_refiner,text_paster,focus_probe}.rs`

### Project-local tech vocabulary

| `<tech>` segment | Refers to | Used in |
|---|---|---|
| `rdev` | `rdev` crate | `adapters/primary/ptt/rdev/` — `RdevHotkeyListener` |
| `cpal` | `cpal` crate | `adapters/secondary/recording/cpal/` — `CpalAudioRecorder` |
| `openai` | OpenAI-compatible HTTP API (wire protocol, not the vendor) via `ureq`, on loopback only | `adapters/secondary/transcription/openai/` — `OpenAiTranscriber`; `adapters/secondary/textrefinement/openai/` — `OpenAiTextRefiner` |
| `passthrough` | no technology: hands the input on unchanged | `adapters/secondary/textrefinement/passthrough/` — `PassthroughTextRefiner` |
| `macos` | macOS native APIs (`CGEventPost` via `enigo`, `NSPasteboard` via `arboard`, `AXIsProcessTrusted` via `core-foundation`) | `adapters/secondary/textpasting/macos/`, `adapters/secondary/soundplaying/macos/`, `adapters/secondary/permissions/macos/` |
| `filesystem` | WAV file I/O via `hound` | `adapters/secondary/recording/filesystem/` — `FileSystemAudioRecorder` |
| `stdio` | stdout via `println!` | `adapters/secondary/textpasting/stdio/` — `StdioTextPaster` |
| `cli` | interactive terminal UI via `dialoguer` + `console` | `adapters/primary/setup/cli/` — `SetupWizard` |

### AudioLevel
- **Type:** Value Object
- **Definition:** Normalised RMS audio level of a single audio frame, in range [0.0, 1.0]. Computed by the recorder adapter on every CPAL buffer callback and consumed by the visual feedback adapter to drive the equalizer display.
- **Code:** `app/src/core/domain/audio_level.rs`
- **Invariants:** value ∈ [0.0, 1.0]; NaN and ±Inf are clamped to 0.0 (SILENT).
- **Synonyms to AVOID:** "volume", "loudness", "amplitude" (alone — too generic), "rms_value"

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
- **Synonyms to AVOID:** "label", "annotation", "transcript" (use `RawTranscript` for speech recognition output, `GroundTruth` for reference data)
