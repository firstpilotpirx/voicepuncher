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
- **Synonyms to AVOID:** "shortcut" (implies a combo), "binding" (the holder the listener reads is the Hotkey binding)

### AudioBuffer16k
- **Type:** Value Object
- **Definition:** A captured audio segment, mono, signed 16-bit PCM at 16 kHz — the canonical input format for Whisper-family speech recognition; sent to the speech recognition service as a WAV file.
- **Code:** `app/src/core/domain/audio_buffer.rs`
- **Invariants:** sample rate is 16 kHz (constructor convention), values are `i16`, single channel.
- **Synonyms to AVOID:** "recording", "snippet", "utterance", "clip"

### RawTranscript
- **Type:** Value Object
- **Definition:** The speech recognition service's output, before its filler sounds are removed. May contain filler words, missing punctuation, casing inconsistencies. Constructor rejects empty/whitespace.
- **Code:** `app/src/core/domain/raw_transcript.rs`
- **Synonyms to AVOID:** "transcription", "STT output", "text"

### CleanedText
- **Type:** Value Object
- **Definition:** Final text ready to paste — the `RawTranscript` without its filler sounds (`text_to_paste`), or the raw transcript as returned when it holds nothing else. Constructor rejects empty/whitespace.
- **Code:** `app/src/core/domain/cleaned_text.rs`
- **Synonyms to AVOID:** "refined transcript", "polished text", "corrected text"

### Pipeline
- **Type:** Use Case (Application Service)
- **Definition:** The orchestrator that consumes `PttSignal`s and drives one full PTT cycle: `Pressed → start record → Released → stop record → transcribe → paste → Idle`. Holds a state machine; ignores signals while non-Idle (except in Recording).
- **Code:** `app/src/core/application/pipeline.rs`
- **States:** `Idle`, `Recording`, `Transcribing`, `Pasting`
- **Synonyms to AVOID:** "controller", "engine", "manager"

### Permissions gate

- **Definition:** Lets dictation by hotkey run only while the three permissions the app needs — Microphone, Accessibility, Input Monitoring — are granted. The runtime asks it before it starts the pipeline and every few seconds after; while one is missing the runtime waits for permissions and the Permissions window shows which. Replaces the retired *Preflight*, which exited the app on the first missing permission. File mode never asks it.
- **Code:** `app/src/core/application/permissions_gate.rs` (`PermissionsGate`, `PermissionReport`)
- **Synonyms to AVOID:** "preflight", "permission check", "bootstrap"

### Permission status

- **Definition:** What the system says about one permission: Granted; Not granted (denied, revoked or restricted: the user grants it in System Settings); Not asked yet (a request shows the system prompt). Read without opening a device.
- **Code:** `app/src/core/domain/permission.rs` (`Permission`, `PermissionStatus`)
- **Synonyms to AVOID:** "authorization", "TCC state"

### Permissions window

- **Definition:** The window that shows one row per permission with its status and its remedy — Request… or Open System Settings on the exact page — and names the application to grant to when the app runs as a bare binary from a terminal. It opens by itself while the runtime waits for permissions and from the menu at any time.
- **Code:** `app/src/adapters/primary/permissions/macos/`
- **Synonyms to AVOID:** "setup dialog", "onboarding"

### PermissionProbe
- **Type:** Driven Port (trait in `core/application/`)
- **Definition:** The port the *permissions gate* reads and requests the permissions through: the status of each, the system request, the System Settings page, and the application macOS attributes the permissions to when the app is not bundled. The macOS adapter uses status calls that open no device (AVFoundation, `AXIsProcessTrustedWithOptions`, `IOHIDCheckAccess`).
- **Code:** `app/src/core/application/permission_probe.rs`
- **Synonyms to AVOID:** "PermissionChecker", "PermissionService"

### Runtime
- **Type:** Composition root (outside `core/`)
- **Definition:** Owns the life of the application between launch and quit: starts the `Pipeline` from the configuration, restarts it with a new configuration inside the same process (after the dictation in progress ends), and stops it. Keeps one hotkey listener and one overlay for the whole process.
- **Code:** `app/src/runtime/mod.rs`
- **Synonyms to AVOID:** "app", "supervisor", "daemon"

### Runtime state
- **Type:** Value (enum, outside `core/`)
- **Definition:** What the `Runtime` reports to its observers: `Starting`, `Ready` (with the hotkey and whether voice control is on), `Restarting`, `Starting services`, `Waiting for permissions`, `Waiting for models` (with the models folder and the needed files), `Stopped`, `Failed` (with the reason). A new observer is told the current state at once.
- **Code:** `app/src/runtime/observer.rs`
- **Synonyms to AVOID:** "status", "health"

### Hotkey binding
- **Type:** Shared value (domain)
- **Definition:** The `HotKey` the hotkey listener matches every key event against. The `Runtime` rebinds it on a reload; the listener sees the new key at its next event, and a chord half pressed at that moment is forgotten.
- **Code:** `app/src/core/domain/hotkey.rs` (`HotkeyBinding`)
- **Synonyms to AVOID:** "shortcut", "key mapping"

### Signal router
- **Type:** Runtime part (outside `core/`)
- **Definition:** Forwards `PttSignal`s from the hotkey listener to the `Pipeline` that runs now; while no pipeline runs (during a restart), signals are dropped.
- **Code:** `app/src/runtime/router.rs`
- **Synonyms to AVOID:** "dispatcher", "bus"

### Pipeline activity
- **Type:** Value + driven port (`core/application/`)
- **Definition:** What the `Pipeline` is doing as far as a restart cares: `Idle`, `Recording` or `Processing` (recognition, the focus wait, the paste). The pipeline publishes it on every transition; a restart waits for `Idle`.
- **Code:** `app/src/core/application/pipeline_activity.rs`
- **Synonyms to AVOID:** "busy flag", "pipeline state" (that is the state machine of `Pipeline`)

### Shutdown
- **Type:** Pipeline input
- **Definition:** The input that ends a `Pipeline`: it cancels a recording in progress, stops voice control listening (closing the microphone) and returns from its run loop. The `Runtime` sends it on a restart, once the pipeline is `Idle`, and on a stop, at once.
- **Code:** `app/src/core/application/pipeline_input.rs` (`PipelineInput::Shutdown`)
- **Synonyms to AVOID:** "quit", "kill", "close"

### SoundPlayer
- **Type:** Driven Port (trait in `core/application/`)
- **Definition:** Plays an auditory cue at the start, stop and cancel of a recording cycle, when voice control listening turns on or off, and a sound of its own for each voice command with no visible effect of its own ("press enter", "clear all text", "paste text", "restart dictation" — the last instead of the cancel and start cues). Abstracts macOS `afplay` so `Pipeline` stays platform-agnostic. Implementations may be no-ops (e.g. in tests or file mode).
- **Code:** `app/src/core/application/sound_player.rs`
- **Synonyms to AVOID:** "AudioPlayer", "BeepService"

### RecordingMode
- **Type:** Value Object (enum)
- **Definition:** Controls how the user triggers a recording cycle. `Toggle` — one press starts, a second press stops. `Hold` — hold the key while speaking, release to stop.
- **Code:** `app/src/core/domain/recording_mode.rs`
- **Variants:** `Toggle` (default), `Hold`
- **Synonyms to AVOID:** "TriggerMode", "CaptureMode"

### Default configuration file
- **Type:** Configuration (embedded text)
- **Definition:** The configuration file a *First launch* writes when none exists: every option the settings window shows, at its default, with a comment above it. It parses to the same configuration as an empty file; retired settings of an old file are ignored. Replaces the retired *SetupWizard*, which asked for the options in a terminal.
- **Code:** `app/src/config/default.toml`, `config::ensure_exists`
- **Synonyms to AVOID:** "config template", "setup wizard", "example config"

### First launch
- **Definition:** The app's first start for a user: it writes the *Default configuration file*, the *Permissions window* asks for the permissions, the *Download window* fetches the models, the local services start, and the menu bar shows Ready. Nothing is asked in a terminal.
- **Synonyms to AVOID:** "setup", "onboarding", "installation"

### VisualIndicator
- **Type:** Driven Port (trait in `core/application/`)
- **Definition:** Surfaces the current `Pipeline` state to the user as
  an on-screen widget — nine discrete signals: recording in progress,
  each `Processing stage` (speech recognition), waiting
  for `Text input focus`, the `Pasted sign`, hidden, the `Command sign`, the `Failure notice`, and the
  `Commands-unavailable sign` (shown and hidden on its own, beside whatever else shows). Abstracts the platform-specific
  always-on-top overlay so `Pipeline` stays platform-agnostic.
- **Code:** `app/src/core/application/visual_indicator.rs`
- **Synonyms to AVOID:** "Overlay", "HUD", "StatusBadge", "Toast", "Indicator" (alone — too generic)

### Live transcript
- **Type:** Domain concept (overlay content)
- **Definition:** The raw transcript of the chunks transcribed so far, stitched at their seams as the pasted transcript is, shown next to the recording sign while the user dictates and updated after every chunk with text. It is gone once the recording ends; the pasted text may still differ where filler sounds are removed. A dictation shorter than the chunk threshold has none.
- **Code:** `LiveTranscriptView` (driven port in `app/src/core/application/live_transcript_view.rs`), handed out by `VisualIndicator::live_transcript_view`; `app/src/core/application/pipeline/live_transcript.rs`
- **Synonyms to AVOID:** "partial transcript", "preview", "text so far" (in code)

### Command sign
- **Type:** Domain concept (overlay signal)
- **Definition:** The overlay naming a voice command with no visible effect of its own — "⏎ ENTER", "⌫ CLEAR", "↻ RESTART" — for about a second after it was carried out, then showing again what it showed before: hidden in standby, the recording during a dictation. Any later signal (the recording a chain starts, the next command's sign) replaces it at once. "start", "stop" and "cancel dictation" have none: the overlay already shows them.
- **Code:** `VisualIndicator::show_command_sign`
- **Synonyms to AVOID:** "toast", "notification", "badge", "confirmation" (alone — also names the second hearing of a command)

### Commands-unavailable sign
- **Type:** Domain concept (overlay signal)
- **Definition:** The overlay showing that voice commands do not work now, because the *command spotting service* has been *away* for 5 s while listening. It shows alone in standby and beside the recording or processing sign during a dictation, takes no focus, outlasts a dictation that ends meanwhile, and goes once the service is back; the overlay then shows only what the current state shows. A shorter outage shows nothing, and no outage plays a sound.
- **Code:** `PipelineInput::SpottingStillAway`; `VisualIndicator::show_commands_unavailable`, `VisualIndicator::hide_commands_unavailable`
- **Synonyms to AVOID:** "failure notice" (that one goes on its own), "offline badge", "warning"

### Dictation sign
- **Type:** Adapter concept (visual feedback next to the mouse pointer)
- **Definition:** A small round dark-glass badge shown just below and to the right of the mouse pointer while a dictation runs, over any app and on every screen, so the user sees the dictation where they already look. It mirrors the overlay's stage: a red microphone while recording, an ear with a turning activity ring during speech recognition, an orange keyboard while the text waits for focus, a green check after the paste and a yellow warning triangle with a failure notice. The check and the triangle go away by themselves with the Pasted sign and the failure notice; hiding the overlay hides it; a command sign or the commands-unavailable sign leaves it as it is, so it never shows for them alone. It follows the pointer and takes no clicks or focus. The pointer itself never changes: macOS lets an app set the pointer only over its own windows.
- **Code:** `app/src/adapters/secondary/visualfeedback/pointer_sign.rs` (`PointerSignIndicator`, `PointerSignView`, `DictationSignState`, `SignLifetimes`), `visualfeedback/macos/pointer_sign.rs` (`MacosPointerSign`), `visualfeedback/macos/sign_badge.rs` (the drawing)
- **Synonyms to AVOID:** "cursor" (the text caret), "custom pointer", "pointer badge"

### Failure notice
- **Type:** Value Object (enum `FailureNotice`, with the failing part `FailingPart`)
- **Definition:** The overlay telling the user that an action they started failed: what was lost (the dictation, part of it, the paste, the recording, a voice command, voice commands at all) and the failing part — speech recognition, command spotting, the microphone or the Accessibility permission — with the remedy when one is known ("start the local services", "check the Accessibility permission"). The pill shows a warning sign and the part's short name, the text window under it a sentence of at most two lines; it hides by itself after a few seconds, and any later signal replaces it. The log keeps the technical detail. `AccessibilityMissing` is the domain error a paste or key press fails with when the process is not trusted for accessibility.
- **Code:** `app/src/core/domain/failure_notice.rs`
- **Synonyms to AVOID:** "error popup", "alert", "toast", "notification" (macOS notifications are not used)

### Pasted sign
- **Type:** Domain concept (overlay signal)
- **Definition:** The overlay confirming that a dictation's text was pasted: a green check, "Pasted" and the pasted text in the text window for about half a second, then hidden. It follows a paste at once or after the focus wait; any later signal (a command sign, the next recording) replaces it at once. A dictation that pastes nothing — no speech, cancelled, failed, or left in the clipboard — ends without it, and a pending `Failure notice` shows instead of it.
- **Code:** `VisualIndicator::show_pasted`
- **Synonyms to AVOID:** "success toast", "done sign", "confirmation" (alone — also names the second hearing of a command)

### Status item
- **Type:** Adapter concept (primary adapter, the macOS menu bar)
- **Definition:** The app's icon in the menu bar, from launch to quit: its only presence once it has no Dock icon. Its picture is the current `Icon state`; clicking it opens the menu: the app's name and version, the `Status line`, Settings…, Open Logs Folder and Quit voicepuncher. Quit stops the `Runtime` (a recording in progress is cancelled with the cancel sound) and ends the process with exit code 0. It learns the dictation from the overlay signals it decorates and the runtime from the runtime states it observes.
- **Code:** `app/src/adapters/primary/menubar/macos/` (`MacosStatusItem`, `StatusItemCell`, `StatusItemIndicator`, `StatusItemObserver`, `MacosMenuActions`)
- **Synonyms to AVOID:** "tray icon", "menu bar extra", "status bar" (the whole bar), "overlay" (the pill)

### Icon state
- **Type:** Value (enum `IconState`, adapter)
- **Definition:** What the `Status item` shows, picked from the dictation stage, the `Runtime state` and the voice control signs in one place: waiting (starting or restarting), the `Missing-permission icon` (waiting for permissions), ready, listening (voice control on), commands unavailable, recording, processing, waiting for focus, warning (while a `Failure notice` shows) and failed. A dictation outranks the commands-unavailable sign, which outranks listening, which outranks ready; a runtime state shows only while no dictation runs; a `Command sign` changes nothing. Each is an SF Symbol: a template image in the menu bar's ink, the recording one a filled microphone with a red dot, the processing one a waveform stepping every 400 ms (still under Reduce Motion).
- **Code:** `app/src/adapters/primary/menubar/macos/mapping.rs`
- **Synonyms to AVOID:** "status", "badge", "icon" (alone)

### Missing-permission icon
- **Type:** Value (`IconState::MissingPermission`, adapter)
- **Definition:** The `Icon state` while the `Runtime` waits for a permission the user has to grant: a shield with a lock, apart from the hourglass of the other waits, so the menu bar says the app waits for the user, not for itself.
- **Code:** `app/src/adapters/primary/menubar/macos/mapping.rs`
- **Synonyms to AVOID:** "lock icon", "permission badge"

### Status line
- **Type:** Adapter concept (the menu's second line)
- **Definition:** The `Icon state` in plain words, set when the menu opens: "Ready — hold Right Option to dictate" (or "press" in Toggle mode), "Recording — release Right Option to stop", "Recognizing speech…", "Waiting for a text field…", "Voice control: listening", "Starting…", "Restarting…", "Waiting for Accessibility and Input Monitoring…" (the missing permissions, named; choosing the line then opens the `Permissions window`), "Failed: " with the reason. Hotkeys are named as a person reads them ("Right Option", "Cmd+E").
- **Code:** `StatusItemState::status_line`, `hotkey_name` in `app/src/adapters/primary/menubar/macos/mapping.rs`
- **Synonyms to AVOID:** "tooltip", "subtitle", "status" (alone)

### Config document
- **Type:** Adapter-facing value (configuration layer)
- **Definition:** The configuration file as the user wrote it — comments, order and the format of every value kept — that the `Settings window` edits: only the values the user changed are rewritten, a missing section is added at the end, a missing file is created on the first save, and a save replaces the file in one step. Its values are checked exactly as the loader checks a file before anything is written.
- **Code:** `ConfigDocument` in `app/src/config/document.rs`
- **Synonyms to AVOID:** "config struct", "settings file" (alone), "serialised config"

### Settings window
- **Type:** Primary adapter concept (macOS)
- **Definition:** The window Settings in the menu opens, one at a time: a tab per area (General, Dictation, Sounds, Paste, Archive, Logging, Services) with one control per option of the configuration file. Save checks every edited value (`Field validation`), writes the edits into the `Config document` and applies the saved configuration, with this run's command line over it, to the running app without a relaunch; its status line says whether the settings wait for the current dictation, were applied, or were not applied and why. Revert drops unsaved edits; closing with unsaved edits asks first. An option the command line overrides shows the file's value with a note naming the value of this run.
- **Code:** `SettingsSession` (without its controls) in `app/src/adapters/primary/settings/macos/session.rs`; the field table in `table.rs`; `RuntimeApplier` in `app/src/adapters/primary/menubar/macos/applier.rs`
- **Synonyms to AVOID:** "preferences pane", "config editor", "options dialog"

### Login item
- **Type:** Port (`LoginItem`) and secondary adapter (macOS)
- **Definition:** The entry in System Settings → General → Login Items that starts the installed app when the user logs in. Its status is the system's: not registered, enabled, waiting for the user's approval, or unavailable (the app does not run from an installed bundle). The `Settings window` switch registers or unregisters it at once, without Save; the app offers it once, after its first Ready, and records that it asked in `[app] launch_at_login_asked`, a key no control edits.
- **Code:** `LoginItem` in `app/src/core/application/login_item.rs`; `MacOsLoginItem` and the launch origin in `app/src/adapters/secondary/loginitem/macos/`; `LoginSwitch` and `LoginOffer` in `app/src/adapters/primary/settings/macos/login.rs`
- **Synonyms to AVOID:** "autostart", "launch agent", "startup item"

### Field validation
- **Type:** Primary adapter rule (settings window)
- **Definition:** The check a value entered in the `Settings window` must pass before Save writes anything: a number or slider within its range, a service address on this Mac (loopback, the right scheme), a folder that exists or can be created, known language codes. A refused value marks its field, the window names the reason (the allowed range, "must be on this Mac"), and neither the file nor the running app changes.
- **Code:** `validate` in `app/src/adapters/primary/settings/macos/validation.rs`
- **Synonyms to AVOID:** "form validation", "sanity check"

### Processing stage
- **Type:** Domain concept (overlay state)
- **Definition:** The step a dictation is in between the end of its recording and the paste, as the overlay shows it with an icon and an English label of its own, and an indeterminate spinner beside the label: **speech recognition** — a breathing ear, "Recognizing", no text window — from the stop until the transcript is ready (with live chunks, the transcription still running after the stop). The transcript is pasted right after it; no other stage is shown.
- **Code:** `VisualIndicator::show_recognizing_speech`
- **Synonyms to AVOID:** "phase", "step" (alone), "processing" (alone — names both stages)

### Model catalog
- **Type:** Value (domain data)
- **Definition:** The five model files the local services load — the dictation and commands Whisper models, the Whisper voice activity model, the voice control voice activity model and the keyword spotting archive — each with its pinned download address, sha256, size, purpose and the part of the app that needs it (dictation always, voice control). The one source of the files `just models` and the *Download window* fetch.
- **Code:** `app/src/core/domain/model_catalog.rs` (`CATALOG`, `ModelFile`, `ModelNeeds`)
- **Synonyms to AVOID:** "model list", "manifest"

### Models folder
- **Type:** Domain concept
- **Definition:** Where the model files live: `[services] models_dir`, `~/.voicepuncher/models` by default. A catalog file is *present* there (or in `~/.voicepuncher/models`, `~/.voicepuncher`) with its catalog size, or when its variable names an existing path; downloads write `NAME.part` files there.
- **Code:** `ServicesCfg::models_dir`; `app/src/adapters/secondary/modelfetch/fs/`
- **Synonyms to AVOID:** "model cache", "model dir"

### Model download
- **Type:** Use Case (Application Service)
- **Definition:** Checks which needed catalog files are present and fetches the missing ones one at a time: resumed from their part, hashed while they stream, placed only when the sha256 matches (downloaded once more on a mismatch), the archive extracted; connection failures are retried after 2, 4 and 8 s, and nothing starts when the free space is short. The *Download window* and `--download-models` run it; the runtime waits for models (state *Waiting for models*) before it starts any service. The app's only use of the internet.
- **Code:** `app/src/core/application/model_download.rs` (`ModelDownload`); ports `ModelStore`, `PartSink`, `ModelFetcher`, `DiskSpace`, `Clock`
- **Synonyms to AVOID:** "installer", "updater", "sync"

### Download window
- **Type:** Primary adapter (window)
- **Definition:** The window that lists the needed model files with their purpose, size and status, the total progress and the free space, with Download (Cancel while it runs), Retry, Open Folder, Quit and Done. It opens when the runtime waits for models, and from the menu's Download models…
- **Code:** `app/src/adapters/primary/modeldownload/macos/`
- **Synonyms to AVOID:** "installer", "setup window"

### Service runtime
- **Type:** Use Case (Application Service)
- **Definition:** The part of the app that starts, *adopts*, watches, restarts and stops the local services a configuration needs — the *speech recognition services* (as their *copies*) and the *command spotting service* — when the app manages them (`[services] manage`, on by default). It counts the needed services ready, which the runtime waits for before it starts the pipeline, and lists each with its *service state* in the menu. File mode and `manage = false` start nothing.
- **Code:** `app/src/core/application/local_services/` (`ServiceRuntime`, `Services`, `Supervision`); ports `ServiceLauncher`, `ServiceProcess`, `ServiceProbe`, `Clock`
- **Synonyms to AVOID:** "service manager", "daemon", "supervisor" (the runtime thread's name), "launcher" (the port that starts one process)

### Owned service
- **Type:** Domain concept
- **Definition:** A local service the *service runtime* started: restarted when it exits or stops answering, Failed after five starts in a row that never answered, stopped at quit.
- **Code:** `ServiceStatus::owned`
- **Synonyms to AVOID:** "managed service", "child"

### Adopted service
- **Type:** Domain concept
- **Definition:** A local service that already answered on its port when the *service runtime* looked (kept warm by `just start-model-servers`, or another app instance): counted ready and watched, never started or stopped by the app; when it stops answering it is *stopped outside the app* and waited for.
- **Code:** `ServiceState::Adopted`, `ServiceState::StoppedOutside`
- **Synonyms to AVOID:** "external service", "foreign service"

### Service state
- **Type:** Value Object (enum)
- **Definition:** What a supervised local service is doing: Pending, Starting, Ready, Restarting (with the try), Failed (with the reason and the log file), Adopted, Stopped outside the app, Stopped. Ready and Adopted count ready for dictation.
- **Code:** `app/src/core/domain/local_service.rs` (`ServiceState`, `ServiceStatus`, `ServicesProgress`, `ServiceSpec`, `ServiceKind`)
- **Synonyms to AVOID:** "health", "status code"

### Speech recognition service
- **Type:** External system (outside the app process)
- **Definition:** A process on the same machine that turns audio into a `RawTranscript` over the OpenAI Audio Transcriptions API — whisper.cpp `whisper-server` is the supported one. The service scripts run two: the *speech recognition service for dictation* and the *speech recognition service for commands*. The *service runtime* starts them in hotkey mode, or the user with `just start-model-servers`; the app connects to the one for dictation, checks it is reachable at startup, and rejects any address that is not loopback.
- **Code:** `app/src/adapters/secondary/transcription/openai/` — `OpenAiTranscriber`; config section `[transcription]` (`app/src/config/transcription.rs`)
- **Synonyms to AVOID:** "STT server", "Whisper server" (names one implementation), "backend", "engine"

### Speech recognition service for dictation
- **Type:** External system (a *speech recognition service*)
- **Definition:** The *speech recognition service* the app sends dictations and recording files to (`[transcription].base_url`, port 8081 by default). The service scripts start it as `whisper-server` (three *copies* found from its address) with the Whisper large-v3-turbo model, 5-bit quantized (`ggml-large-v3-turbo-q5_0.bin`), unless the user names another model in `VOICEPUNCHER_WHISPER_MODEL`.
- **Synonyms to AVOID:** "turbo server", "main whisper", "dictation whisper"

### Speech recognition service for commands
- **Type:** External system (a *speech recognition service*)
- **Definition:** The *speech recognition service* the *command spotting service* confirms utterances with (port 8085 by default, `VOICEPUNCHER_COMMAND_STT_PORT`). The service scripts start it as `whisper-server-commands` (three *copies* found from its address) with the Whisper small model (`ggml-small.bin`), unless the user names another model in `VOICEPUNCHER_COMMAND_WHISPER_MODEL`. The app never talks to it.
- **Synonyms to AVOID:** "command whisper", "confirmation server", "second whisper", "small server"

### Copy
- **Type:** External system (one process of a local service)
- **Definition:** One process of a local service — a `whisper-server` of a *speech recognition service* or a `voicepuncher-command-spotting` of the *command spotting service*; a speech recognition copy answers one request at a time, a command spotting copy serves one connection's listening. With several copies, copy *n* listens on the service's port + its copy offset + *n* — 10000 for a speech recognition service, 41000 for command spotting (8082 → 49083–49086); − the offset when that would pass the last port; at most 4 copies — and nothing the app starts listens on the service's own port; a single copy listens on the service's own port. A client finds the copies from the service's address alone: the own port when it answers, else the copy ports.
- **Code:** `copy-pool/src/address.rs` — `ServiceAddress::copy_ports`
- **Synonyms to AVOID:** "instance", "backend", "replica", "worker"

### Copy pool
- **Type:** Shared library (`copy-pool/`, used by the app's transcription adapter, its command spotting client and the command spotting service)
- **Definition:** The *copies* of one local service as one client process sees them: which accept connections (probed every second), which are held, and the requests waiting for one, first come first served. A request whose copy refuses the connection or drops it goes to another copy; when no copy accepts connections, the client waits for one to come back (30 s) before it fails. One pool per process per service address and copy offset.
- **Code:** `copy-pool/src/pool.rs` — `CopyPool`
- **Synonyms to AVOID:** "load balancer", "proxy", "router"

### Lease
- **Type:** Value (held by one request)
- **Definition:** One *copy* held by one request: no other request goes to that copy until the lease ends — when the copy answered, or refused the connection (the copy is then down until it accepts connections again).
- **Code:** `copy-pool/src/pool/lease.rs` — `Lease`
- **Synonyms to AVOID:** "slot", "connection", "ticket"

### Draining copy
- **Type:** State of a *copy*
- **Definition:** A *copy* still transcribing a request its client gave up on: its *lease* lasts until that answer arrives, so the client's next request goes to another copy or waits. A copy draining past twice the request's wait is logged as hung.
- **Synonyms to AVOID:** "stuck copy", "zombie request"

### Filler sound
- **Type:** Domain policy (closed word list)
- **Definition:** A hesitation sound the user never means to write — `э`, `э-э`, `эм`, `ммм`, `um`, `uh`, `hmm`, … Removed with the comma after it from the `RawTranscript` before it is pasted; a transcript of nothing but filler sounds is pasted as it was returned. Words like "ну" or "типа" are not filler sounds.
- **Code:** `app/src/core/domain/filler_sounds.rs` — `is_filler_sound`, `without_filler_sounds`, `text_to_paste`
- **Synonyms to AVOID:** "filler words", "disfluencies", "noise"

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

### Dictation archive
- **Type:** Driven Port (trait `DictationArchive` in `core/application/`) and the folder it writes
- **Definition:** The local folder (`~/.voicepuncher/history/` by default, `[archive]`) where every dictation that reached speech recognition is kept as an *archive entry*, so its recognition can be evaluated later. On by default for dictations, opt-in (`--archive-dir`) for recording files; kept within a size limit by removing the oldest entries; never sent anywhere.
- **Code:** `app/src/core/application/dictation_archive.rs`; `app/src/adapters/secondary/archiving/filesystem/`
- **Synonyms to AVOID:** "history" (alone — names the folder only), "log", "recordings", "dump"

### Archive entry
- **Type:** Value Object (`ArchiveEntry`)
- **Definition:** What the *dictation archive* keeps of one dictation, in a folder of its own named by the recording's start: the 16 kHz audio speech recognition heard, the text of each chunk, the stitched `RawTranscript`, the pasted text, how the dictation ended (pasted, left in the clipboard, paste failed, speech recognition failed, no text, only command words), how it started, stage timings, the models and the app version.
- **Code:** `app/src/core/application/dictation_archive.rs`
- **Synonyms to AVOID:** "record", "history item", "log entry"

### Log file
- **Type:** Adapter output (no port: logging goes through the `log` facade)
- **Definition:** `~/.voicepuncher/logs/voicepuncher.log`, where the app in hotkey mode writes every log line of a session, starting with a launch header (version, process id, config path, macOS version, architecture); a terminal, when there is one, gets the same lines. File mode never writes it.
- **Code:** `app/src/adapters/secondary/logging/file/`
- **Synonyms to AVOID:** "dictation archive" (that keeps dictations, not log lines), "history"

### Log rotation
- **Type:** Behavior of the *log file* writer
- **Definition:** Before a write would take the *log file* past 10 MB, it is renamed `.1`, earlier backups move one number up to `.5`, the oldest is deleted and a new file starts; a line is never split between files.
- **Code:** `app/src/adapters/secondary/logging/file/rotating.rs` — `RotatingFileWriter`
- **Synonyms to AVOID:** "log cleanup", "truncation"

### HotkeyListener / AudioRecorder / Transcriber / TextPaster / FocusProbe
- **Type:** Driving / Driven Ports (traits in `core/application/`)
- **Code:** `app/src/core/application/{hotkey_listener,audio_recorder,transcriber,text_paster,focus_probe}.rs`

### Bundle
- **Type:** Distribution artifact (not a domain object)
- **Definition:** The macOS application `voicepuncher.app`: the release binary, its engine folders, the licenses of its engines, its `Info.plist` (bundle identifier `io.github.firstpilotpirx.voicepuncher`, version from `app/Cargo.toml`, no Dock icon, microphone usage description) and the icon, signed as one unit. macOS grants Microphone, Accessibility and Input Monitoring to the bundle, by its signing identity, not to the terminal that started it.
- **Code:** `app/packaging/` (sources), `just bundle` (builds `dist/voicepuncher.app`)
- **Synonyms to AVOID:** "app package", "binary" (the binary is only one file of the bundle), "installer"

### Engine
- **Type:** Distribution artifact (not a domain object)
- **Definition:** The program a local service runs: `whisper-server` (whisper.cpp) for both speech recognition services, `voicepuncher-command-spotting` for the command spotting service. A service is the running process on its port; the engine is the program and the libraries it loads. The app logs, per service, the engine it runs, its version and where it came from: the bundle, a program the environment names, `$PATH` or the build folder.
- **Code:** `EngineOrigin`, `EngineSource`, `engine_origin` in `app/src/adapters/secondary/localservices/catalog/`
- **Synonyms to AVOID:** "server" for the program (the service is the process), "backend" (ggml's compute backends are libraries an engine loads), "binary" alone

### Engine folder
- **Type:** Distribution artifact (not a domain object)
- **Definition:** The folder of one engine inside the bundle — `Contents/MacOS/engines/whisper/` or `command-spotting/` — holding its program, every library it loads (relocated to `@rpath` + `@loader_path`) and a `VERSION` file; the app takes each engine from its folder before `$PATH`.
- **Code:** `ENGINES_DIR`, `engine_folder` in the catalog; `just engines` stages them into `app/packaging/engines/`
- **Synonyms to AVOID:** "Frameworks", "lib folder"

### Engine lock
- **Type:** Distribution artifact (not a domain object)
- **Definition:** `app/packaging/engines.lock`: the source, version and sha256 of every file of every bundled engine, and the source of every license. `just engines` takes the engines only as it says, never builds one, and refuses any other version or checksum.
- **Code:** `app/packaging/engines.lock`, `just engines` (`_engines`)
- **Synonyms to AVOID:** "manifest", "versions file"

### Signing identity
- **Type:** Distribution concept
- **Definition:** The self-signed code-signing certificate, named `voicepuncher` in the maintainer's keychain and in `VOICEPUNCHER_SIGNING_IDENTITY`, that signs every release *bundle*. Its designated requirement names that certificate, so every build signed with it is the same application to the permission system and an update keeps the permissions. Without it a bundle is signed ad-hoc and each build is a new application.
- **Code:** `justfile` `_sign`, `_require-signing-identity`, `_require-release-identity`
- **Synonyms to AVOID:** "certificate" (alone — the identity is the certificate with its private key), "Developer ID" (the project has none), "notarization"

### Disk image
- **Type:** Distribution artifact
- **Definition:** `voicepuncher-vX.Y.Z.dmg`, the only file a release publishes besides its `.sha256`: a volume `voicepuncher X.Y.Z` whose window shows the *bundle* on the left and a shortcut to Applications on the right, over a background that says to drag one onto the other.
- **Code:** `just dmg` (builds it with `create-dmg`), `app/packaging/dmg-background.png`
- **Synonyms to AVOID:** "DMG installer", "package", "tarball" (retired)

### Project-local tech vocabulary

| `<tech>` segment | Refers to | Used in |
|---|---|---|
| event tap | listen-only `CGEventTap` | `adapters/primary/ptt/eventtap/` — `EventTapHotkeyListener` |
| `cpal` | `cpal` crate | `adapters/secondary/recording/cpal/` — `CpalAudioRecorder` |
| `openai` | OpenAI-compatible HTTP API (wire protocol, not the vendor) via `ureq`, on loopback only | `adapters/secondary/transcription/openai/` — `OpenAiTranscriber` |
| `macos` | macOS native APIs (`CGEventPost` via `enigo`, `NSPasteboard` via `arboard`, `AXIsProcessTrusted` via `core-foundation`) | `adapters/secondary/textpasting/macos/`, `adapters/secondary/soundplaying/macos/`, `adapters/secondary/permissions/macos/` |
| `filesystem` | WAV file I/O via `hound` | `adapters/secondary/recording/filesystem/` — `FileSystemAudioRecorder` |
| `file` | a plain file on disk via `std::fs`, size-rotated | `adapters/secondary/logging/file/` — `RotatingFileWriter`, `TeeWriter` |
| `stdio` | stdout via `println!` | `adapters/secondary/textpasting/stdio/` — `StdioTextPaster` |
| `cli` | interactive terminal UI via `dialoguer` + `console` | `adapters/primary/setup/cli/` — `SetupWizard` |

### AudioLevel
- **Type:** Value Object
- **Definition:** Normalised RMS audio level of a single audio frame, in range [0.0, 1.0]. Computed by the recorder adapter on every CPAL buffer callback and consumed by the visual feedback adapter to drive the *level waveform*.
- **Code:** `app/src/core/domain/audio_level.rs`
- **Invariants:** value ∈ [0.0, 1.0]; NaN and ±Inf are clamped to 0.0 (SILENT).
- **Synonyms to AVOID:** "volume", "loudness", "amplitude" (alone — too generic), "rms_value"

### Level waveform
- **Type:** Value Object (model) drawn by the visual feedback adapter
- **Definition:** The overlay's picture of recent microphone loudness: one rounded column per 100 ms, as tall as the loudest smoothed `AudioLevel` of its 100 ms (dBFS from −50 to −10 mapped to 2–28 pt; silence is a 2 pt dot), sliding left so the strip shows the last 3 s (30 columns, 22 beside the *Commands-unavailable sign*). A new waveform is all dots.
- **Code:** `app/src/core/domain/level_history.rs` (`LevelHistory`)
- **Synonyms to AVOID:** "equalizer", "pills", "level bar", "spectrum" (it shows loudness over time, not frequencies)

### Voice control
- **Type:** Mode of the app (opt-in, `[voice_control]` config section or `--voice-control`)
- **Definition:** Hands-free operation alongside push-to-talk: *listening* stays on while the app runs, and spoken *voice commands* start, stop and cancel a dictation, press Enter, clear the focused text and paste the clipboard; the hotkey still records as push-to-talk.
- **Code:** `app/src/core/application/pipeline/voice_control.rs` (`VoiceControl`: off or listening)
- **Synonyms to AVOID:** "voice mode", "hands-free mode", "RecordingMode" (that is Toggle/Hold)

### Listening
- **Type:** State of voice control
- **Definition:** The microphone stays open and audio streams to the *command spotting service*, so *voice commands* take effect. *Standby* is listening with no dictation running; *dictating* is listening while a dictation records. Listening stays on until the app exits: while the service is *away*, the app *reconnects* and listening resumes once the service is *back*.
- **Synonyms to AVOID:** "wake", "armed", "always-on"

### Away (command spotting service)
- **Type:** State of *listening*
- **Definition:** The *command spotting service* dropped the connection or answered nothing, not even a ping, for 5 s. Listening stays on, a running dictation keeps recording, no sound plays, and the app *reconnects*; once the service has been away for 5 s, the overlay shows the *Commands-unavailable sign* until it is back. Slow spotting results alone never make the service away.
- **Code:** `PipelineInput::SpottingLost`; `lose_spotting` in `app/src/core/application/pipeline/voice_control.rs`
- **Synonyms to AVOID:** "dead", "offline", "disconnected" (alone: the connection may be up while the service is slow)

### Confirmation failed
- **Type:** Boundary signal (service event `confirmation_failed`)
- **Definition:** The *command spotting service* got no answer for a closed speech segment from any copy of the *speech recognition service for commands*, within both attempts of its *confirmation deadline*: a command said in it is lost. A copy that sends nothing within its decoding budget and does not answer its health check counts as stalled, and the confirmation goes to another copy first. The app shows the *failure notice* that voice commands are unavailable at once; the recognizer ignores the event.
- **Code:** `ServerEvent::ConfirmationFailed` in `command-spotting/src/protocol.rs`; `SpottingEvent::ConfirmationFailed` in `app/src/core/domain/voice_command_recognizer.rs`
- **Synonyms to AVOID:** "timeout" (a copy may time out and another still answer), "no command" (nothing heard is an answer)

### Stalled (command spotting service)
- **Type:** State of a connection of the command spotting service (service event `stalled`)
- **Definition:** A confirmation of the connection has run past every attempt it has (at least 10 s): its thread hangs, since every request is given up at its deadline. The service's `/health` answers 503, and it tells the app once; the app takes it as a loss and *reconnects*.
- **Code:** `ConfirmationWorker::stalled_at` in `command-spotting/src/worker.rs`; `Beat::stalled` in `health.rs`
- **Synonyms to AVOID:** "slow" (a slow copy keeps answering within its deadlines), "stuck" (alone: audio waiting with no progress)

### Dropped audio
- **Type:** Behavior of the command spotting service (service event `dropped`)
- **Definition:** Audio arriving while the oldest audio waiting for a connection's processing has waited past 10 s is dropped on arrival (a burst the service keeps up with, such as a replay, is kept), and the service tells the app how many samples; the stream's clock no longer matches the app's, so the app takes it as a loss, *reconnects* and sends the *speech captured while the service was away*.
- **Code:** `Beat::queued` in `command-spotting/src/health.rs`; `drop_audio` in `server.rs`
- **Synonyms to AVOID:** "lag" (audio that waited and was still processed)

### Protocol version (command spotting)
- **Type:** Concept
- **Definition:** The version of the messages between the app and the *command spotting service*, now 2: every connection opens with the app's `hello` naming it, and each message names its `type` (`hello`, `select`, `sync`). The service closes a connection of another version, or one without a `hello` first, naming both versions; a message it cannot read and an audio frame of partial samples close the connection with the reason.
- **Code:** `PROTOCOL_VERSION`, `AppMessage` in `command-spotting/src/protocol.rs`; `command-spotting/src/server/refusal.rs`; `SpottingClient::connect`
- **Synonyms to AVOID:** "API version"

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
- **Code:** `JobKind::Rehear`, `hear_again` in `command-spotting/src/engine/confirming/rehear.rs`

### Steady clip
- **Type:** Value of the *command spotting service*
- **Definition:** An utterance cut 0.25 s before its first sound (the first 10 ms louder than 0.4 of its loudest, padded with silence when the audio starts later) and raised toward a loudest 10 ms of 0.1 when quieter, by at most 10 times: the same words sound alike to the speech recognition service whatever sound came before them. Only the *second hearing* sends it.
- **Code:** `steady_clip` in `command-spotting/src/engine/clip.rs`

### Health check
- **Type:** Rule of the service scripts
- **Definition:** Every 5 s the supervisor of the command spotting service asks `/health`; a service that answered once and then fails 3 checks in a row is killed and so restarted, with a note in its log. The command spotting service answers 503 when a connection has audio waiting and its processing made no progress for 10 s.
- **Code:** `supervise` in `justfile`, `command-spotting/src/health.rs`
- **Synonyms to AVOID:** "heartbeat", "ping" (the app's pings check one connection)

### VoiceCommand
- **Type:** Value Object (enum)
- **Definition:** A spoken English command. `SpokenCommand` pairs it with when its words were said and its *command placement*: the whole utterance (the pause rule), the end of an utterance, "start" at its head, or a doubled form or *command chain* anywhere.
- **Code:** `app/src/core/domain/voice_command.rs`, recognition (pause rule) in `app/src/core/domain/voice_command_recognizer.rs`
- **Variants:** `StartDictation` ("start dictation"), `StopDictation` ("stop dictation"), `CancelDictation` ("cancel dictation"), `RestartDictation` ("restart dictation"), `PressEnter` ("press enter"), `ClearAllText` ("clear all text"), `PasteText` ("paste text")
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
- **Definition:** A synthetic keyboard event posted into the focused app: Return for `press enter`; Cmd+A then Delete for `clear all text`; Cmd+V for `paste text`, so the focused app pastes what the clipboard holds.
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
