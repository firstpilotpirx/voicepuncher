# System Overview

voicepuncher is an offline push-to-talk voice-to-text CLI for macOS Apple Silicon. The user holds a configurable hotkey, speaks, releases — the spoken text is transcribed by a local speech recognition service (whisper.cpp `whisper-server`, reached over loopback HTTP), cleaned by a local Qwen2.5 LLM via llama.cpp, and pasted at the cursor. Nothing leaves the machine: the app only opens loopback connections. All inference runs on the Apple Silicon GPU via Metal.

## Hexagonal Architecture

```mermaid
flowchart TB
    subgraph external["External actors"]
        USER["User\n(keyboard + voice)"]
        MACOS["macOS\n(TCC, CoreAudio,\nCGEventPost)"]
        FS["File system\n(model files, WAV,\nconfig)"]
    end

    subgraph primary["Primary adapters (driving)"]
        RDEV["RdevHotkeyListener\n(rdev crate)"]
        WIZARD["SetupWizard\n(first-run config)"]
        CLI["Cli\n(clap)"]
        PERMWIN["PermissionsWindow\n(AppKit)"]
    end

    subgraph runtime["runtime (composition)"]
        RUNTIME["Runtime\n(start · reload · stop)"]
        ROUTER["SignalRouter"]
        WIRING["WiringAssembler"]
    end

    subgraph app["core/application"]
        PIPELINE["Pipeline\n(use case)"]
        GATE["PermissionsGate\n(use case)"]
        PORTS["Ports\n(traits: AudioRecorder,\nTranscriber, TextRefiner,\nTextPaster, HotkeyListener,\nSoundPlayer, PermissionProbe)"]
    end

    subgraph domain["core/domain"]
        VO["Value Objects\nAudioBuffer16k · RawTranscript\nCleanedText · HotKey\nPttSignal · RecordingMode"]
    end

    subgraph secondary["Secondary adapters (driven)"]
        CPAL["CpalAudioRecorder\n(cpal)"]
        WHISPER["OpenAiTranscriber\n(HTTP via ureq, loopback)"]
        LLAMA["LlamaCppTextRefiner\n(llama-cpp-2 FFI)"]
        PASTER["MacOsTextPaster\n(arboard + CGEventPost)"]
        SOUND["MacosSoundPlayer\n(macOS audio)"]
        PROBE["MacOsPermissionProbe\n(AVFoundation + AX + IOHID)"]
        FSREC["FileSystemAudioRecorder\n(hound, WAV I/O)"]
        STDIO["StdioTextPaster\n(stdout)"]
    end

    USER -->|hotkey press/release| RDEV
    USER -->|voice| MACOS
    MACOS -->|audio stream| CPAL
    MACOS <-->|TCC probe| PROBE
    MACOS <--|paste text| PASTER
    WHISPER -->|WAV over loopback HTTP, one request per copy| STT[("whisper-server copies\n(separate processes)")]
    FS -->|model binary| LLAMA
    FS -->|WAV file| FSREC

    RDEV -->|PttSignal| ROUTER
    ROUTER -->|PttSignal| PIPELINE
    CLI --> RUNTIME
    MACOS -->|SIGHUP| RUNTIME
    RUNTIME -->|assembles| WIRING
    WIRING -->|builds| PIPELINE
    RUNTIME -->|Shutdown| PIPELINE
    RUNTIME -->|asks| GATE
    PERMWIN -->|request · settings| GATE
    RUNTIME -->|state| PERMWIN
    PIPELINE --> PORTS
    GATE --> PORTS
    PORTS -.implements.-> CPAL
    PORTS -.implements.-> WHISPER
    PORTS -.implements.-> LLAMA
    PORTS -.implements.-> PASTER
    PORTS -.implements.-> SOUND
    PORTS -.implements.-> PROBE
    PORTS -.implements.-> FSREC
    PORTS -.implements.-> STDIO
    PIPELINE --> domain
    GATE --> domain
```

The domain ring contains only pure Rust value objects with zero external dependencies. The application ring holds the two use cases (`Pipeline`, `PermissionsGate`) and the port traits. All technology — audio, inference, clipboard, hotkey — lives exclusively in adapters and never crosses into `core/`. The `runtime` module beside them composes the adapters into a pipeline: it starts one at launch, replaces it on a reload (the hang-up signal) once the dictation in progress ends, and ends it on a stop; the hotkey listener and the overlay live once per process, and the signal router points the listener at the pipeline that runs. The runtime asks the permissions gate before it starts a pipeline: with a permission missing it waits in the state Waiting for permissions (the Permissions window opens on it) and starts the pipeline once all are granted; while Ready it asks again every 5 s and stops the pipeline when one is revoked. In hotkey mode with `[services] manage` on (the default) the runtime also starts the local services the configuration needs (`ManagedServices`: the catalog adapter builds their commands, the process adapter starts them, the `ServiceRuntime` use case supervises them), adopts the ones that already answer, waits in the state Starting services until all are ready, starts the ones a reload newly needs, and stops the ones it started when it stops; a service that cannot start fails the runtime without ending the app, so the menu can retry it.

## Pipeline State Machine

```mermaid
stateDiagram-v2
    [*] --> Idle
    Idle --> Recording : PttSignal::Pressed
    Recording --> Transcribing : PttSignal::Released\n(press ≥ min_press_ms)
    Recording --> Idle : PttSignal::Released\n(press < min_press_ms, drop)
    Transcribing --> Refining : RawTranscript produced
    Refining --> Pasting : CleanedText produced
    Pasting --> Idle : text pasted
    Transcribing --> Idle : error (logged)
    Refining --> Idle : error (logged)
    Pasting --> Idle : error (logged)
```

Signals received while the pipeline is in any non-Idle state (except `Recording` waiting for `Released`) are silently discarded. This prevents double-triggers from key bounce or slow inference.

## Technology Choices

| Concern | Library / API | Reason |
|---|---|---|
| Hotkey capture | `rdev` | Global key listener without a visible window; works in terminal |
| Audio recording | `cpal` | Cross-platform abstraction; picks CoreAudio on macOS |
| Speech-to-text | local `whisper-server` (whisper.cpp) over the OpenAI Audio Transcriptions API, via `ureq` | The engine runs outside the app (Metal acceleration, any `ggml-*.bin` model); the app stays engine-agnostic and builds without C++. The service runs as copies on the ports 10000 above its own; the crate `copy-pool` finds them from the address and gives each request a free copy |
| Text refinement | `llama-cpp-2` (llama.cpp FFI) | Runs Qwen2.5-3B-Instruct locally; Metal acceleration |
| Clipboard paste | `arboard` + `CGEventPost` | `arboard` writes to NSPasteboard; `CGEventPost` sends Cmd+V |
| Audio playback | macOS `NSSound` / `AVAudioPlayer` | Start/stop beeps via macOS native sound API |
| Permission probe | AVFoundation authorization status and request (`objc2`, `block2`), `AXIsProcessTrustedWithOptions`, `IOHIDCheckAccess` / `IOHIDRequestAccess` | Three-valued status (granted, not granted, not asked yet) without opening a device; the system prompt on request; System Settings deep links per pane |
| Config | `serde` + `toml`; `toml_edit` for writes | TOML config file at `~/.voicepuncher/config.toml`; the settings window edits it through `toml_edit`, which keeps the user's comments and order |
| Logging | `log` + `env_logger` with `Target::Pipe` into an own size-rotating file writer (10 MB × 5) teed to a terminal | One logger and its line format for terminal and file; the rotation is ~60 lines, smaller than adopting `tracing`, `log4rs` or `flexi_logger` |
| CLI args | `clap` | Derive-based arg parser; overlays on top of TOML config |
| GUI toolkit, menu bar item | AppKit through `objc2-app-kit` (`NSStatusItem`, `NSMenu`, SF Symbols) | Native look in the light and the dark menu bar, no runtime beyond the system, the same bindings the overlay uses; rejected: Tauri (a web view and a second runtime), egui and iced (no native menu bar item), SwiftUI (a second language and build) |
| Packaging | `just` recipes on macOS tools (`plutil`, `iconutil`, `codesign`, `hdiutil`) + `create-dmg` 1.3.0 for the disk image window | A bundle is three files in a fixed tree; `create-dmg` scripts the Finder layout. No packaging crate: `cargo-packager` always signs with the hardened runtime, which blocks the microphone and the key listener without entitlements |
| Engine distribution | Prebuilt engines in a folder each under `Contents/MacOS/engines/`, their libraries beside them, relocated with `install_name_tool` (`@rpath` + `@loader_path`), pinned by version and sha256 in `app/packaging/engines.lock`, signed inside-out with the app; whisper.cpp from Homebrew, llama.cpp from its release archive, the command spotting service from this workspace | Users install nothing: no Homebrew, nothing on `$PATH`. Nothing is built from source, so a bundle is reproducible for a lock; one folder per engine keeps the two ggml builds apart; ggml's compiled-in backend folder is blanked so its Metal and CPU backends load from the engine's folder |
| Signing | self-signed code-signing certificate (`VOICEPUNCHER_SIGNING_IDENTITY`), no Developer ID, no notarization | The designated requirement names the certificate, so updates keep the TCC permissions; ad-hoc signatures change identity every build. Gatekeeper needs one "Open Anyway" per version |
| Login item | `SMAppService.mainAppService` through `objc2-service-management` | The macOS 13+ API for an app's own login item, listed in System Settings → Login Items where the user can switch it off; no helper app and no launch agent plist |
| Settings window | AppKit forms in code: `NSTabViewController` (toolbar tabs), `NSGridView`, standard controls | The macOS settings look, light and dark, keyboard and accessibility from the system; no nib files, no SwiftUI |

## Quality Goals

| Goal | Target | Notes |
|---|---|---|
| Fully offline | Zero network I/O at runtime | Both models run locally via Metal; no telemetry |
| Latency | < 3 s end-to-end (press → paste) | On Apple M-series with `large-v3-turbo-q5` + `Qwen2.5-3B-Q4` |
| Multilingual | English + Russian validated | FLEURS en/ru corpus in the benchmark suite |
| Accuracy (WER) | < 35% WER on mixed corpus | Measured by `just bench` over 100+ samples |
| macOS-only | Apple Silicon + Metal required | No Intel support; no Linux/Windows port planned at PoC stage |
