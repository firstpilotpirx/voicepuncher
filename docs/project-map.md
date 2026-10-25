# Project Map

## Root folders

| Folder | Purpose |
|---|---|
| `app/` | Rust menu bar application (with a developer command line and file mode) — hexagonal architecture (Ports & Adapters + DDD) |
| `command-spotting/` | Rust binary: the command spotting service (voice control) |
| `copy-pool/` | Rust library: the *copy pool* — routes requests to the copies of a speech recognition service, used by `app/` and `command-spotting/` |
| `log-line/` | Rust library: the form of one log line (`[time LEVEL thread target] message`), used by `app/` and `command-spotting/` |
| `e2e-test/` | Rust benchmark and E2E tests against the compiled binary |
| `docs/` | Documentation (ubiquitous language, architecture, E2E benchmark design) |
| `dist/` | Gitignored output of `just bundle` and `just dmg`: `voicepuncher.app`, `voicepuncher-vX.Y.Z.dmg` and its `.sha256` |
| `Cargo.toml` | Cargo workspace (members: `app`, `command-spotting`, `copy-pool`, `e2e-test`) |
| `justfile` | Task runner — the checks (`just check`, `just full-check`, `just test`, the E2E recipes), setup (`just models`, `just corpus`, `just engines`), packaging and release (`just bundle`, `just dmg`, `just release`), benchmarks (`just bench`) |

## App structure (`app/`)

| Path | Layer | Purpose |
|---|---|---|
| `app/src/core/domain/` | Domain | Value objects: `AudioBuffer16k`, `RawTranscript`, `CleanedText`, `HotKey`, `PttSignal`, `RecordingMode` |
| `app/src/core/application/pipeline/mod.rs` | Application | `Pipeline` use case — orchestrates one full PTT cycle, holds the state machine |
| `app/src/core/domain/model_catalog.rs` | Domain | `CATALOG` — the model files with pinned address, sha256, size and purpose; `needed` picks a configuration's |
| `app/src/core/application/model_download/mod.rs` | Application | `ModelDownload` use case — check the models folder, download the missing files with resume, retries and checksums; its ports `ModelStore`, `ModelFetcher`, `DiskSpace` |
| `app/src/core/application/permissions_gate.rs` | Application | `PermissionsGate` use case and `PermissionReport` — the three permissions, their statuses, requests and System Settings links |
| `app/src/core/application/audio_recorder.rs` | Port | `AudioRecorder` trait |
| `app/src/core/application/hotkey_listener.rs` | Port | `HotkeyListener` trait |
| `app/src/core/application/transcriber.rs` | Port | `Transcriber` trait |
| `app/src/core/application/text_paster.rs` | Port | `TextPaster` trait |
| `app/src/core/application/sound_player.rs` | Port | `SoundPlayer` trait |
| `app/src/core/application/permission_probe.rs` | Port | `PermissionProbe` trait |
| `app/src/core/application/local_services/` | Application | `ServiceRuntime` and `Services` — start, adopt, check, restart and stop the local services on a 100 ms tick (`Supervision` per service); ports `ServiceLauncher`, `ServiceProcess`, `ServiceProbe`, `Clock` |
| `app/src/core/domain/local_service.rs` | Domain | `ServiceKind`, `ServiceSpec`, `ServiceState`, `ServiceStatus`, `ServicesProgress` |
| `app/src/runtime/` | Composition | `Runtime` — starts, reloads and stops the pipeline in place; `SignalRouter`, `SharedVisualIndicator`, `RuntimeState` and observers, the activity register, `PipelineAssembler` with the real `WiringAssembler`, the hang-up signal thread; `ManagedServices` behind the `LocalServices` port, which the runtime waits for before it starts a pipeline; `ModelsGate` and `CatalogModels` (`models.rs`), which it waits for before it starts the services, and the headless `--download-models`; the termination signal thread that stops them |
| `app/src/core/application/pipeline_activity.rs` | Port | `PipelineActivitySink` trait — the pipeline publishes `Idle` / `Recording` / `Processing` |
| `app/src/adapters/primary/cli/` | Primary adapter | `Cli` — `clap`-based parser of the developer flags and file mode, each an override of the configuration for one run |
| `app/src/adapters/primary/ptt/eventtap/` | Primary adapter | `EventTapHotkeyListener` — captures hotkey events on a listen-only `CGEventTap` (key codes and modifier flags only) |
| `app/src/adapters/primary/permissions/macos/` | Primary adapter | `PermissionsWindow` — the Permissions window (rows, badges, Request…, Open System Settings, Quit, Continue); `mapping.rs` holds what it shows |
| `app/src/adapters/primary/modeldownload/macos/` | Primary adapter | `DownloadWindow` — the Download window (file rows, total bar, free space, Open Folder, Quit, Download/Cancel/Retry, Done), opened on Waiting for models; `mapping.rs` holds what it shows |
| `app/src/adapters/primary/menubar/macos/` | Primary adapter | `MacosStatusItem` — the menu bar icon and its menu (Settings, Open Logs Folder, Services with Restart and Open Log, Quit); `StatusItemCell` fed by the overlay decorator and the runtime observer; `MacosMenuActions`; `RuntimeApplier` applies saved settings through the runtime's `Reloader`; `LoginOfferObserver` asks once, at the first Ready, whether to start at login |
| `app/src/adapters/primary/settings/macos/` | Primary adapter | The settings window: the field table (`table.rs`, a tab, label and control per configuration key), `validate`, value conversions, override notes, the GUI-free `SettingsSession` (edit, save, revert, apply through the `ConfigApplier` port) and the AppKit window (`window.rs`, `form.rs`, `layout.rs`, `build.rs`); `login.rs` and `login_row.rs`: the login switch and the one-time offer |
| `app/src/adapters/main_queue.rs` | Adapter helper | Work dispatched to the main queue, shared by the overlay and the status item |
| `app/src/adapters/secondary/recording/cpal/` | Secondary adapter | `CpalAudioRecorder` — records microphone via `cpal` |
| `app/src/adapters/secondary/recording/filesystem/` | Secondary adapter | `FileSystemAudioRecorder` — reads WAV file (file mode + E2E) |
| `app/src/adapters/secondary/transcription/openai/` | Secondary adapter | `OpenAiTranscriber` — sends audio to the local speech recognition service (OpenAI Audio Transcriptions API, loopback only) |
| `app/src/adapters/secondary/textpasting/macos/` | Secondary adapter | `MacOsTextPaster` — pastes text via `arboard` + `CGEventPost` |
| `app/src/adapters/secondary/textpasting/stdio/` | Secondary adapter | `StdioTextPaster` — writes text to stdout (file mode / E2E) |
| `app/src/adapters/secondary/soundplaying/macos/` | Secondary adapter | `MacosSoundPlayer` — plays start/stop sounds via macOS `afplay` |
| `app/src/adapters/secondary/commandspotting/websocket/` | Secondary adapter | `WebSocketCommandSpotter` — streams the microphone to the command spotting service over its loopback WebSocket and feeds its spotting events into the pipeline; reconnects when the service is away |
| `app/src/adapters/secondary/keypressing/macos/` | Secondary adapter | `MacOsKeyPresser` — the key presses of the voice commands (Enter, Cmd+A, Cmd+V) via `CGEventPost` |
| `app/src/adapters/secondary/focus/macos/` | Secondary adapter | `MacOsFocusProbe` — whether a text input has the focus, through the accessibility API |
| `app/src/adapters/secondary/visualfeedback/macos/` | Secondary adapter | `MacosVisualIndicator` — the overlay: the signs (dictation, command, pasted, failure notices), the level waveform and the live transcript near the pointer |
| `app/src/adapters/secondary/archiving/filesystem/` | Secondary adapter | `FileSystemDictationArchive` — keeps each dictation as a folder (`audio.wav`, `entry.json`) under the archive folder |
| `app/src/adapters/secondary/http/` | Adapter helper | What the clients of the local HTTP services share: the failure classes and the copy pool client |
| `app/src/adapters/secondary/logging/file/` | Secondary adapter | `RotatingFileWriter`, `TeeWriter` — the app's log file `~/.voicepuncher/logs/voicepuncher.log`, size-rotated and teed to a terminal; the launch header and the panic hook |
| `app/src/adapters/secondary/localservices/process/` | Secondary adapter | `ProcessLauncher` — starts a service as a child process in its own process group, its output in a rotating log file; `HttpProbe` — asks a service's port and `/health` whether it is ready |
| `app/src/adapters/secondary/localservices/catalog/` | Secondary adapter | `needed` — the services a configuration needs, with the `justfile`'s commands, copy ports, thread split, engine and model lookup |
| `app/src/adapters/secondary/loginitem/macos/` | Secondary adapter | `MacOsLoginItem` — the app's login item through `SMAppService`, unavailable outside an installed bundle; `launch` logs a launch by the login item |
| `app/src/adapters/secondary/modelfetch/http/` | Secondary adapter | `HttpModelFetcher` — fetches a model file over HTTPS with `ureq`, resuming a part with a Range request |
| `app/src/adapters/secondary/modelfetch/fs/` | Secondary adapter | `FsModelStore`, `FsDiskSpace` — the models folder: presence, `NAME.part` files and their sha256, placing, archive extraction, free space |
| `app/src/adapters/secondary/permissions/macos/` | Secondary adapter | `MacOsPermissionProbe` — macOS TCC status, requests, deep links and the responsible app |
| `app/src/config/` | Config | TOML config loading with `serde`; `Config` is the root struct; `[transcription]` and `[whisper]` migration error in `config/transcription/`, retired `[refinement]` and `[llm]` sections ignored with one warning in `config/mod.rs`, loopback check in `config/loopback.rs`; `config/document/` — `ConfigDocument`, the file as `toml_edit` keeps it, so a Save changes only the edited keys |
| `app/src/main.rs` | Composition root | Wires adapters + use cases into a runnable; no business logic |
| `app/packaging/` | Packaging | Sources of the application bundle: `Info.plist` template (version filled in by `just bundle`), `icons/` (`icon-1024.png` and `voicepuncher.iconset/`, turned into `.icns` by `iconutil`), `dmg-background.png`; `engines.lock` (the pinned engines: source, version and sha256 of each file, the license sources), `licenses/` (licenses no engine source carries: ONNX Runtime's) and the gitignored `engines/` that `just engines` stages (one folder per engine, `licenses/`); checked by `app/tests/it/packaging.rs` |
| `app/tests/it/architecture/` | Architecture test | Import-graph assertions (layer dependency rule, naming rules, structure rules); `test_layout/` beside it keeps every integration test inside `tests/it/` |
| `app/tests/it/{feature}_{tech}.rs` | Integration tests | One module per adapter, all built into the one `it` binary that `app/tests/it/main.rs` roots (one link per member instead of one per file); the macOS tests that need the main thread live in `app/tests/main_thread/` |
| `app/tests/features/{slice,component}/<capability>.feature` | Slice / component tests | Gherkin scenarios written from `openspec/specs/<capability>/spec.md`; run by `app/tests/cucumber_integration.rs`, steps in `app/tests/steps/<capability>.rs` |

## E2E test structure (`e2e-test/`)

| Path | Purpose |
|---|---|
| `e2e-test/src/corpus.rs` | `Corpus` loader — reads `manifest.jsonl`, pairs WAV files with `GroundTruth` |
| `e2e-test/src/wer.rs` | WER / CER metric computation |
| `e2e-test/tests/features/<capability>.feature` | E2E scenarios in Gherkin, written from `openspec/specs/<capability>/spec.md`; `@smoke` = binary loads and file mode runs, `@accuracy` = WER over the corpus |
| `e2e-test/tests/steps/<capability>.rs` | Step definitions, one module per capability |
| `e2e-test/tests/cucumber_e2e.rs` | cucumber-rs harness: `E2eWorld` + `main()` |
| `e2e-test/corpus/` | Benchmark audio files (FLEURS en/ru, LibriSpeech, Podlodka) + `manifest.jsonl` |
| `e2e-test/scripts/bench.py` | Multi-model benchmark matrix runner |
| `e2e-test/scripts/download_corpus.py` | Corpus downloader |

## Documentation structure (`docs/`)

| File | Purpose |
|---|---|
| `docs/ubiquitous-language.md` | Domain glossary — single source of truth for all domain terms |
| `docs/project-map.md` | This file |
| `docs/architecture/overview.md` | arc42 overview — system description, hexagonal diagram, tech choices, quality goals |
| `docs/architecture/context.md` | arc42 context — external actors, system boundary, permission flows |
| `docs/benchmark-datasets.md` | Benchmark corpus description and WER targets |
| `docs/e2e-benchmark-design.md` | E2E benchmark design and test strategy |
