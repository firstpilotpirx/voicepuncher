# Project Map

## Root folders

| Folder | Purpose |
|---|---|
| `app/` | Rust CLI application — hexagonal architecture (Ports & Adapters + DDD) |
| `command-spotting/` | Rust binary: the command spotting service (voice control) |
| `copy-pool/` | Rust library: the *copy pool* — routes requests to the copies of a speech recognition service, used by `app/` and `command-spotting/` |
| `e2e-test/` | Rust benchmark and E2E tests against the compiled binary |
| `docs/` | Documentation (ubiquitous language, architecture, E2E benchmark design) |
| `dist/` | Gitignored output of `just bundle` and `just dmg`: `voicepuncher.app`, `voicepuncher-vX.Y.Z.dmg` and its `.sha256` |
| `Cargo.toml` | Cargo workspace (members: `app`, `command-spotting`, `copy-pool`, `e2e-test`) |
| `justfile` | Task runner — all checks (`check`, `e2e`, `release-check`), setup (`models`, `corpus`), benchmarks (`bench`) |

## App structure (`app/`)

| Path | Layer | Purpose |
|---|---|---|
| `app/src/core/domain/` | Domain | Value objects: `AudioBuffer16k`, `RawTranscript`, `CleanedText`, `HotKey`, `PttSignal`, `RecordingMode` |
| `app/src/core/application/pipeline.rs` | Application | `Pipeline` use case — orchestrates one full PTT cycle, holds the state machine |
| `app/src/core/application/permissions_gate.rs` | Application | `PermissionsGate` use case and `PermissionReport` — the three permissions, their statuses, requests and System Settings links |
| `app/src/core/application/audio_recorder.rs` | Port | `AudioRecorder` trait |
| `app/src/core/application/hotkey_listener.rs` | Port | `HotkeyListener` trait |
| `app/src/core/application/transcriber.rs` | Port | `Transcriber` trait |
| `app/src/core/application/text_refiner.rs` | Port | `TextRefiner` trait |
| `app/src/core/application/text_paster.rs` | Port | `TextPaster` trait |
| `app/src/core/application/sound_player.rs` | Port | `SoundPlayer` trait |
| `app/src/core/application/permission_probe.rs` | Port | `PermissionProbe` trait |
| `app/src/runtime/` | Composition | `Runtime` — starts, reloads and stops the pipeline in place; `SignalRouter`, `SharedVisualIndicator`, `RuntimeState` and observers, the activity register, `PipelineAssembler` with the real `WiringAssembler`, the hang-up signal thread |
| `app/src/core/application/pipeline_activity.rs` | Port | `PipelineActivitySink` trait — the pipeline publishes `Idle` / `Recording` / `Processing` |
| `app/src/adapters/primary/cli/` | Primary adapter | `Cli` — `clap`-based argument parser |
| `app/src/adapters/primary/ptt/rdev/` | Primary adapter | `RdevHotkeyListener` — captures hotkey events via `rdev` |
| `app/src/adapters/primary/setup/cli/` | Primary adapter | `SetupWizard` — interactive first-run configuration wizard |
| `app/src/adapters/primary/permissions/macos/` | Primary adapter | `PermissionsWindow` — the Permissions window (rows, badges, Request…, Open System Settings, Quit, Continue); `mapping.rs` holds what it shows |
| `app/src/adapters/primary/menubar/macos/` | Primary adapter | `MacosStatusItem` — the menu bar icon and its menu (Settings, Open Logs Folder, Quit); `StatusItemCell` fed by the overlay decorator and the runtime observer; `MacosMenuActions` |
| `app/src/adapters/main_queue.rs` | Adapter helper | Work dispatched to the main queue, shared by the overlay and the status item |
| `app/src/adapters/secondary/recording/cpal/` | Secondary adapter | `CpalAudioRecorder` — records microphone via `cpal` |
| `app/src/adapters/secondary/recording/filesystem/` | Secondary adapter | `FileSystemAudioRecorder` — reads WAV file (file mode + E2E) |
| `app/src/adapters/secondary/transcription/openai/` | Secondary adapter | `OpenAiTranscriber` — sends audio to the local speech recognition service (OpenAI Audio Transcriptions API, loopback only) |
| `app/src/adapters/secondary/textrefinement/openai/` | Secondary adapter | `OpenAiTextRefiner` — sends the raw transcript to the local text refinement service (OpenAI Chat Completions API, loopback only) |
| `app/src/adapters/secondary/textrefinement/passthrough/` | Secondary adapter | `PassthroughTextRefiner` — pastes the raw transcript (`--no-llm`) |
| `app/src/adapters/secondary/textpasting/macos/` | Secondary adapter | `MacOsTextPaster` — pastes text via `arboard` + `CGEventPost` |
| `app/src/adapters/secondary/textpasting/stdio/` | Secondary adapter | `StdioTextPaster` — writes text to stdout (file mode / E2E) |
| `app/src/adapters/secondary/soundplaying/macos/` | Secondary adapter | `MacosSoundPlayer` — plays start/stop sounds via macOS `afplay` |
| `app/src/adapters/secondary/logging/file/` | Secondary adapter | `RotatingFileWriter`, `TeeWriter` — the app's log file `~/.voicepuncher/logs/voicepuncher.log`, size-rotated and teed to a terminal; the launch header and the panic hook |
| `app/src/adapters/secondary/permissions/macos/` | Secondary adapter | `MacOsPermissionProbe` — macOS TCC status, requests, deep links and the responsible app |
| `app/src/config.rs`, `app/src/config/` | Config | TOML config loading with `serde`; `Config` is the root struct; `[transcription]` and `[whisper]` migration error in `config/transcription.rs`, `[refinement]` and `[llm]` migration error in `config/refinement.rs`, loopback check in `config/loopback.rs` |
| `app/src/main.rs` | Composition root | Wires adapters + use cases into a runnable; no business logic |
| `app/packaging/` | Packaging | Sources of the application bundle: `Info.plist` template (version filled in by `just bundle`), `icons/` (`icon-1024.png` and `voicepuncher.iconset/`, turned into `.icns` by `iconutil`), `dmg-background.png`; checked by `app/tests/packaging_it.rs` |
| `app/tests/architecture.rs` | Architecture test | Import-graph assertions (layer dependency rule, naming rules, structure rules) |
| `app/tests/{feature}_{tech}_it.rs` | Integration tests | One file per secondary adapter; named `<feature>_<tech>_it.rs` |
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
