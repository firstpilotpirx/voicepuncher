# Context and Scope

## System Context

```mermaid
flowchart LR
    USER["User\n(developer / power user\non Apple Silicon Mac)"]

    subgraph vp["voicepuncher (CLI process)"]
        direction TB
        CORE["Pipeline + Preflight\n(core logic)"]
    end

    TCC["macOS TCC\n(Transparency Consent Control)"]
    CLIPBOARD["macOS Clipboard\n(NSPasteboard)"]
    CURSOR["Target Application\n(editor, browser, IDE,\nSlack, etc.)"]
    MODELFS["Local file system\n(~/.voicepuncher/models/)"]

    USER -- "holds hotkey\n(RightOption by default)" --> vp
    USER -- "speaks into microphone" --> vp
    vp -- "requests audio stream" --> TCC
    TCC -- "grants / denies\nMicrophone permission" --> vp
    TCC -- "grants / denies\nAccessibility permission" --> vp
    TCC -- "grants / denies\nInput Monitoring permission" --> vp
    vp -- "writes CleanedText\nto NSPasteboard" --> CLIPBOARD
    vp -- "sends CGEventPost(Cmd+V)" --> CURSOR
    CLIPBOARD -- "Cmd+V pastes\nclipboard content" --> CURSOR
    MODELFS -- "whisper.cpp model\n(ggml-large-v3-turbo-q5_0.bin)" --> vp
    MODELFS -- "llama.cpp model\n(Qwen2.5-3B-Instruct-Q4_K_M.gguf)" --> vp
```

voicepuncher is a single OS process with no server side. It reads audio from CoreAudio, runs inference fully in-process on Apple Silicon Metal, and delivers the result by writing to the system clipboard and synthesizing a Cmd+V keystroke. The three macOS TCC permissions must be granted to the terminal emulator that launches the process — the process inherits them from its parent.

## External Systems

| External system | Direction | What is exchanged |
|---|---|---|
| macOS TCC (Transparency Consent Control) | voicepuncher → macOS | Permission probe at startup (`AXIsProcessTrusted`, `cpal` device open, `rdev` key-event subscription); returns grant or denial per permission |
| CoreAudio (via `cpal`) | macOS → voicepuncher | Raw PCM audio frames from the default input device; streamed during the `Recording` state |
| `rdev` global key listener | macOS → voicepuncher | Raw keyboard events delivered as `PttSignal::Pressed` / `PttSignal::Released` when the configured hotkey changes state |
| Local model files (`~/.voicepuncher/models/`) | file system → voicepuncher | Whisper GGML binary and Qwen2.5 GGUF binary, loaded once at startup and kept resident in RAM |
| NSPasteboard (macOS clipboard) | voicepuncher → macOS | `CleanedText` written as a plain-text clipboard entry via `arboard` |
| CGEventPost (macOS event injection) | voicepuncher → macOS | Synthetic Cmd+V key-down/key-up pair injected into the global event stream to trigger paste in the frontmost application |
| Target application (editor, browser, etc.) | macOS → target app | Receives the synthetic Cmd+V and reads the clipboard; voicepuncher has no direct channel to the target application |

## Permission Flow at Startup

```mermaid
sequenceDiagram
    participant main as main.rs
    participant preflight as Preflight
    participant probe as MacOsPermissionProbe
    participant tcc as macOS TCC

    main->>preflight: run(&probe)
    preflight->>probe: check_microphone()
    probe->>tcc: open default cpal input device
    tcc-->>probe: granted / denied
    probe-->>preflight: Ok / Err

    preflight->>probe: check_accessibility()
    probe->>tcc: AXIsProcessTrusted()
    tcc-->>probe: true / false
    probe-->>preflight: Ok / Err

    preflight->>probe: check_input_monitoring()
    probe->>tcc: subscribe rdev key listener
    tcc-->>probe: granted / denied
    probe-->>preflight: Ok / Err

    alt all granted
        preflight-->>main: Ok(())
    else any denied
        preflight-->>main: Err("grant <permission> to <parent process name>")
        main->>main: exit(2)
    end
```

`Preflight` is the only place in the codebase that interacts with macOS TCC. It runs before any adapter is constructed, so a missing permission produces a single human-readable error message that names the parent process (e.g. `iTerm2`) rather than a cryptic OS error.
