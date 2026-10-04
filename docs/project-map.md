# Project Map

## Root folders

| Folder | Purpose |
|---|---|
| `app/` | Rust CLI application — hexagonal architecture (Ports & Adapters + DDD) |
| `e2e-test/` | Rust benchmark and E2E tests against the compiled binary |
| `docs/` | Documentation (ubiquitous language, architecture, E2E benchmark design) |
| `Cargo.toml` | Cargo workspace (members: `app`, `e2e-test`) |
| `justfile` | Task runner — all checks (`check`, `e2e`, `release-check`), setup (`models`, `corpus`), benchmarks (`bench`) |

## App structure (`app/`)

| Path | Layer | Purpose |
|---|---|---|
| `app/src/core/domain/` | Domain | Value objects: `AudioBuffer16k`, `RawTranscript`, `CleanedText`, `HotKey`, `PttSignal`, `RecordingMode` |
| `app/src/core/application/pipeline.rs` | Application | `Pipeline` use case — orchestrates one full PTT cycle, holds the state machine |
| `app/src/core/application/preflight.rs` | Application | `Preflight` use case — startup permission gate |
| `app/src/core/application/audio_recorder.rs` | Port | `AudioRecorder` trait |
| `app/src/core/application/hotkey_listener.rs` | Port | `HotkeyListener` trait |
| `app/src/core/application/transcriber.rs` | Port | `Transcriber` trait |
| `app/src/core/application/text_refiner.rs` | Port | `TextRefiner` trait |
| `app/src/core/application/text_paster.rs` | Port | `TextPaster` trait |
| `app/src/core/application/sound_player.rs` | Port | `SoundPlayer` trait |
| `app/src/core/application/permission_probe.rs` | Port | `PermissionProbe` trait |
| `app/src/adapters/primary/cli/` | Primary adapter | `Cli` — `clap`-based argument parser |
| `app/src/adapters/primary/ptt/rdev/` | Primary adapter | `RdevHotkeyListener` — captures hotkey events via `rdev` |
| `app/src/adapters/primary/setup/cli/` | Primary adapter | `SetupWizard` — interactive first-run configuration wizard + `ModelDownloader` |
| `app/src/adapters/secondary/recording/cpal/` | Secondary adapter | `CpalAudioRecorder` — records microphone via `cpal` |
| `app/src/adapters/secondary/recording/filesystem/` | Secondary adapter | `FileSystemAudioRecorder` — reads WAV file (file mode + E2E) |
| `app/src/adapters/secondary/transcription/openai/` | Secondary adapter | `OpenAiTranscriber` — sends audio to the local speech recognition service (OpenAI Audio Transcriptions API, loopback only) |
| `app/src/adapters/secondary/textrefinement/llamacpp/` | Secondary adapter | `LlamaCppTextRefiner` — runs llama.cpp via `llama-cpp-2` FFI |
| `app/src/adapters/secondary/textpasting/macos/` | Secondary adapter | `MacOsTextPaster` — pastes text via `arboard` + `CGEventPost` |
| `app/src/adapters/secondary/textpasting/stdio/` | Secondary adapter | `StdioTextPaster` — writes text to stdout (file mode / E2E) |
| `app/src/adapters/secondary/soundplaying/macos/` | Secondary adapter | `MacosSoundPlayer` — plays start/stop sounds via macOS `afplay` |
| `app/src/adapters/secondary/permissions/macos/` | Secondary adapter | `MacOsPermissionProbe` — probes macOS TCC permissions |
| `app/src/config.rs`, `app/src/config/` | Config | TOML config loading with `serde`; `Config` is the root struct; `[transcription]` section, loopback check and `[whisper]` migration error in `config/transcription.rs` |
| `app/src/main.rs` | Composition root | Wires adapters + use cases into a runnable; no business logic |
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
