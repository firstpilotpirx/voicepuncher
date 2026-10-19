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
- **Definition:** Plays an auditory cue at the start, stop and cancel of a recording cycle, when voice control listening turns on or off, and a sound of its own for each voice command with no visible effect of its own ("press enter", "clear all text", "restart dictation" — the last instead of the cancel and start cues). Abstracts macOS `afplay` so `Pipeline` stays platform-agnostic. Implementations may be no-ops (e.g. in tests or file mode).
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
  an on-screen widget — seven discrete signals: recording in progress,
  each `Processing stage` (speech recognition, text refinement), waiting
  for `Text input focus`, hidden, the `Command sign`, and the `Failure notice`. Abstracts the platform-specific
  always-on-top overlay so `Pipeline` stays platform-agnostic.
- **Code:** `app/src/core/application/visual_indicator.rs`
- **Synonyms to AVOID:** "Overlay", "HUD", "StatusBadge", "Toast", "Indicator" (alone — too generic)

### Live transcript
- **Type:** Domain concept (overlay content)
- **Definition:** The raw transcript of the chunks transcribed so far, stitched at their seams as the pasted transcript is, shown next to the recording sign while the user dictates and updated after every chunk with text. It is gone once the recording ends; the pasted text may still differ after text refinement. A dictation shorter than the chunk threshold has none.
- **Code:** `LiveTranscriptView` (driven port in `app/src/core/application/live_transcript_view.rs`), handed out by `VisualIndicator::live_transcript_view`; `app/src/core/application/pipeline/live_transcript.rs`
- **Synonyms to AVOID:** "partial transcript", "preview", "text so far" (in code)

### Command sign
- **Type:** Domain concept (overlay signal)
- **Definition:** The overlay naming a voice command with no visible effect of its own — "⏎ ENTER", "⌫ CLEAR", "↻ RESTART" — for about a second after it was carried out, then showing again what it showed before: hidden in standby, the recording during a dictation. Any later signal (the recording a chain starts, the next command's sign) replaces it at once. "start", "stop" and "cancel dictation" have none: the overlay already shows them.
- **Code:** `VisualIndicator::show_command_sign`
- **Synonyms to AVOID:** "toast", "notification", "badge", "confirmation" (alone — also names the second hearing of a command)

### Dictation sign
- **Type:** Adapter concept (visual feedback next to the mouse pointer)
- **Definition:** A small 🎙️ shown just below and to the right of the mouse pointer while a recording runs, over any app and on every screen, so the user sees the recording where they already look. It follows the pointer, takes no clicks or focus, and goes away with the signals that end a recording (a processing stage, the wait for focus, hidden); a command sign or a failure notice leaves it as it is. The pointer itself never changes: macOS lets an app set the pointer only over its own windows.
- **Code:** `app/src/adapters/secondary/visualfeedback/pointer_sign.rs` (`PointerSignIndicator`, `PointerSignView`), `visualfeedback/macos/pointer_sign.rs` (`MacosPointerSign`)
- **Synonyms to AVOID:** "cursor" (the text caret), "custom pointer", "pointer badge"

### Failure notice
- **Type:** Value Object (enum `FailureNotice`, with the failing part `FailingPart`)
- **Definition:** The overlay telling the user that an action they started failed: what was lost (the dictation, part of it, the refinement, the paste, the recording, a voice command, voice commands at all) and the failing part — speech recognition, text refinement, command spotting, the microphone or the Accessibility permission — with the remedy when one is known ("start the local services", "check the Accessibility permission"). The pill shows a warning sign and the part's short name, the text window under it a sentence of at most two lines; it hides by itself after a few seconds, and any later signal replaces it. The log keeps the technical detail. `AccessibilityMissing` is the domain error a paste or key press fails with when the process is not trusted for accessibility.
- **Code:** `app/src/core/domain/failure_notice.rs`
- **Synonyms to AVOID:** "error popup", "alert", "toast", "notification" (macOS notifications are not used)

### Processing stage
- **Type:** Domain concept (overlay state)
- **Definition:** The step a dictation is in between the end of its recording and the paste, as the overlay shows it with an icon of its own: **speech recognition** from the stop until the transcript is ready (with live chunks, the transcription still running after the stop), then **text refinement** from its start until the corrected text is ready. With text refinement off only speech recognition is shown.
- **Code:** `VisualIndicator::show_recognizing_speech`, `VisualIndicator::show_refining_text`; `Pipeline::without_text_refinement_stage`
- **Synonyms to AVOID:** "phase", "step" (alone), "processing" (alone — names both stages)

### Speech recognition service
- **Type:** External system (outside the app process)
- **Definition:** A process on the same machine that turns audio into a `RawTranscript` over the OpenAI Audio Transcriptions API — whisper.cpp `whisper-server` is the supported one. The service scripts run two: the *speech recognition service for dictation* and the *speech recognition service for commands*. The user starts them (normally with `just start-model-servers`); the app only connects to the one for dictation, checks it is reachable at startup, and rejects any address that is not loopback.
- **Code:** `app/src/adapters/secondary/transcription/openai/` — `OpenAiTranscriber`; config section `[transcription]` (`app/src/config/transcription.rs`)
- **Synonyms to AVOID:** "STT server", "Whisper server" (names one implementation), "backend", "engine"

### Speech recognition service for dictation
- **Type:** External system (a *speech recognition service*)
- **Definition:** The *speech recognition service* the app sends dictations and recording files to (`[transcription].base_url`, port 8081 by default). The service scripts start it as `whisper-server` (three copies behind HAProxy) with the Whisper large-v3-turbo model, 5-bit quantized (`ggml-large-v3-turbo-q5_0.bin`), unless the user names another model in `VOICEPUNCHER_WHISPER_MODEL`.
- **Synonyms to AVOID:** "turbo server", "main whisper", "dictation whisper"

### Speech recognition service for commands
- **Type:** External system (a *speech recognition service*)
- **Definition:** The *speech recognition service* the *command spotting service* confirms utterances with (port 8085 by default, `VOICEPUNCHER_COMMAND_STT_PORT`). The service scripts start it as `whisper-server-commands` (three copies behind HAProxy) with the Whisper small model (`ggml-small.bin`), unless the user names another model in `VOICEPUNCHER_COMMAND_WHISPER_MODEL`. The app never talks to it.
- **Synonyms to AVOID:** "command whisper", "confirmation server", "second whisper", "small server"

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
- **Type:** Value Object (`TextFocus`) decided from a `FocusedElement` (role, text selection, settable value, whether it lies in web content)
- **Definition:** Whether the final text can be pasted now. The app holds the text back only when it is sure no text input has keyboard focus: the focused element is a clearly non-text control (button, list, table, outline, image, menu, check box, radio button, slider, tab group, toolbar) that offers neither a text selection nor an editable value, or it lies in *web content* (a page of a browser or an Electron app, `AXWebArea`, or an element inside one) and is neither a text input (text field, text area, combo box, search field) nor offers an editable value. Any other element, no reported element or an Accessibility error means "paste now". Before reading the focus the app asks the focused app to expose its web content (`AXManualAccessibility`), so Chromium and Electron report a text input on a page as one.
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

### Voice control
- **Type:** Mode of the app (opt-in, `[voice_control]` config section or `--voice-control`)
- **Definition:** Hands-free operation alongside push-to-talk: *listening* stays on while the app runs, and spoken *voice commands* start, stop and cancel a dictation, press Enter and clear the focused text; the hotkey still records as push-to-talk.
- **Code:** `app/src/core/application/pipeline/voice_control.rs` (`VoiceControl`: off or listening)
- **Synonyms to AVOID:** "voice mode", "hands-free mode", "RecordingMode" (that is Toggle/Hold)

### Listening
- **Type:** State of voice control
- **Definition:** The microphone stays open and audio streams to the *command spotting service*, so *voice commands* take effect. *Standby* is listening with no dictation running; *dictating* is listening while a dictation records. Listening stays on until the app exits: while the service is *away*, the app *reconnects* and listening resumes once the service is *back*.
- **Synonyms to AVOID:** "wake", "armed", "always-on"

### Away (command spotting service)
- **Type:** State of *listening*
- **Definition:** The *command spotting service* dropped the connection or answered nothing, not even a ping, for 5 s. Listening stays on, a running dictation keeps recording, the listening-off sound plays, and the app *reconnects*. Slow spotting results alone never make the service away.
- **Code:** `PipelineInput::SpottingLost`; `lose_spotting` in `app/src/core/application/pipeline/voice_control.rs`
- **Synonyms to AVOID:** "dead", "offline", "disconnected" (alone: the connection may be up while the service is slow)

### Reconnect
- **Type:** Behavior of the command spotting adapter
- **Definition:** While the service is *away*, the adapter tries it again after 0.25, 0.5, 1, 2 and then every 4 s, logging the first and every tenth failed attempt, for as long as listening is on.
- **Code:** `app/src/adapters/secondary/commandspotting/websocket/{mod.rs,link.rs}`
- **Synonyms to AVOID:** "retry loop" (retries are per request), "restart" (the service scripts restart services)

### Back (command spotting service)
- **Type:** Boundary signal
- **Definition:** A *reconnect* succeeded: the new connection was told the commands of the current state and sent the *speech captured while the service was away*. The core resets the recognizer, plays the listening-on sound and listens again.
- **Code:** `PipelineInput::SpottingBack`; `regain_spotting` in `app/src/core/application/pipeline/voice_control.rs`

### Speech captured while the service was away
- **Type:** Concept
- **Definition:** The audio from 5 s before the loss (never earlier than the end of the last command reported) to the return, at most the last 10 s held by the adapter, sent to the service that is *back* so a command said in that time is carried out late rather than lost. A command that ends before the last one carried out is not carried out twice.
- **Code:** `HeardAudio::replay_from` in `app/src/adapters/secondary/commandspotting/websocket/heard.rs`, `replay_start` in `link.rs`

### Confirmation deadline
- **Type:** Rule of the *command spotting service*
- **Definition:** How long one confirmation by the *speech recognition service for commands* may take before the service gives it up (3 s, `--confirmation-deadline-secs`). The confirmation of a closed utterance is tried twice; a check of a running utterance is skipped while another one waits. Confirmations run off the audio path, so a slow one delays only its own check.
- **Code:** `command-spotting/src/worker.rs`

### Near miss
- **Type:** Rule of the *command spotting service*
- **Definition:** What the *speech recognition service* hears in place of a command's short form, listed per command: "Start." or "Let's start." (at most three words ending in "start") while "restart dictation" is selected, since the user's quiet "restart" comes out so. A near miss is no command; it only asks for a *second hearing*.
- **Code:** `NEAR_MISSES`, `PhraseMatcher::near_miss` in `command-spotting/src/matcher.rs`

### Second hearing
- **Type:** Rule of the *command spotting service*
- **Definition:** When the confirmation of a short utterance hears no command but a *near miss*, the utterance is heard once more through its *steady clip*, and counts as the near-missed command only when that hearing says it as a whole. Its events wait for the second answer.
- **Code:** `JobKind::Rehear`, `hear_again` in `command-spotting/src/engine/confirming.rs`

### Steady clip
- **Type:** Value of the *command spotting service*
- **Definition:** An utterance cut 0.25 s before its first sound (the first 10 ms louder than 0.4 of its loudest, padded with silence when the audio starts later) and raised toward a loudest 10 ms of 0.1 when quieter, by at most 10 times: the same words sound alike to the speech recognition service whatever sound came before them. Only the *second hearing* sends it.
- **Code:** `steady_clip` in `command-spotting/src/engine/clip.rs`

### Health check
- **Type:** Rule of the service scripts
- **Definition:** Every 5 s the supervisor of the command spotting and text refinement services asks `/health`; a service that answered once and then fails 3 checks in a row is killed and so restarted, with a note in its log. The command spotting service answers 503 when a connection has audio waiting and its processing made no progress for 10 s.
- **Code:** `supervise` in `justfile`, `command-spotting/src/health.rs`
- **Synonyms to AVOID:** "heartbeat", "ping" (the app's pings check one connection)

### VoiceCommand
- **Type:** Value Object (enum)
- **Definition:** A spoken English command. `SpokenCommand` pairs it with when its words were said and its *command placement*: the whole utterance (the pause rule), the end of an utterance, "start" at its head, or a doubled form or *command chain* anywhere.
- **Code:** `app/src/core/domain/voice_command.rs`, recognition (pause rule) in `app/src/core/domain/voice_command_recognizer.rs`
- **Variants:** `StartDictation` ("start dictation"), `StopDictation` ("stop dictation"), `CancelDictation` ("cancel dictation"), `RestartDictation` ("restart dictation"), `PressEnter` ("press enter"), `ClearAllText` ("clear all text")
- **Synonyms to AVOID:** "keyword", "hotword", "wake word", "voice shortcut"

### SpottingEvent
- **Type:** Boundary signal
- **Definition:** What the *command spotting service* reports about the audio it hears: speech started, speech ended, a phrase spotted with its start and end. Fed to the `VoiceCommandRecognizer`.
- **Code:** `app/src/core/domain/voice_command_recognizer.rs`
- **Variants:** `SpeechStarted`, `SpeechEnded`, `PhraseSpotted`, `CommandsSaid` (one to three commands said without a pause around them, carried out at once without the pause rule)

### Command placement
- **Type:** Value Object (enum `CommandPlacement`)
- **Definition:** Where a *voice command* sat in its utterance: `Alone` (the whole utterance), `Tail` (it ended the utterance), `Head` ("start" opened it and speech followed), `Anywhere` (a doubled short form or a *command chain*).
- **Code:** `app/src/core/domain/voice_command.rs`

### Start cue
- **Type:** Concept
- **Definition:** What tells the user a dictation started: the start sound (the restart sound for "restart dictation"), the overlay's recording sign and the recording running. A voice start gives it as soon as its *voice command* is confirmed and logs `voice start: total=… hear=… confirm=… deliver=… cue=…` (ms from the capture time of the command's end).
- **Code:** `app/src/core/application/pipeline/voice_control.rs`
- **Synonyms to AVOID:** "start signal", "beep"

### Command chain
- **Type:** Concept
- **Definition:** Two or three different *voice commands* said one right after the other ("stop enter", "stop enter start"), each accepted in the state the one before leads to; carried out in order.
- **Synonyms to AVOID:** "macro", "combo"

### Command spotting service
- **Type:** Local service (separate process, loopback WebSocket)
- **Definition:** The `voicepuncher-command-spotting` program in `command-spotting/`: sherpa-onnx keyword spotting plus a voice activity detector, with each short utterance confirmed by the *speech recognition service for commands* (the whole transcript must be a command phrase). It turns streamed microphone audio into *spotting events*; it opens no microphone itself.
- **Synonyms to AVOID:** "KWS server", "wake word engine"

### Key press
- **Type:** Effect of a *voice command*
- **Definition:** A synthetic keyboard event posted into the focused app: Return for `press enter`; Cmd+A then Delete for `clear all text`.
- **Code:** port `app/src/core/application/key_presser.rs`
- **Synonyms to AVOID:** "keystroke" (too generic), "shortcut"

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
