# voicepuncher

A menu bar app for macOS on Apple Silicon that turns speech into text: press a hotkey, speak, and the text is pasted at the cursor. Everything runs on your Mac: speech recognition (whisper.cpp) and voice commands run as local services the app starts itself, on the Metal GPU. Nothing you say leaves the machine; the only network use is the one-time download of the models (see [Privacy](#privacy)). Optional [voice control](#voice-control) works hands-free: spoken commands start and stop a dictation and press Enter.

## Requirements

- Apple Silicon Mac (Metal required — no Intel support), macOS 15 or later
- 16 GB RAM minimum (all models load resident, ≈ 5 GB working set combined: six whisper-server copies ≈ 4.3 GB)
- ≈ 2.1 GB of free disk space for the models

## Install

1. Download `voicepuncher-vX.Y.Z.dmg` from the [latest release](https://github.com/firstpilotpirx/voicepuncher/releases/latest) (check it against the `.sha256` beside it: `shasum -a 256 -c voicepuncher-vX.Y.Z.dmg.sha256`).
2. Open it and drag **voicepuncher** onto **Applications**.
3. Open voicepuncher from Applications. macOS shows a dialog saying it cannot verify that voicepuncher is free of malware, with the buttons **Move to Trash** and **Done**: voicepuncher is signed with the project's own certificate, not an Apple Developer ID, and is not notarized. Click **Done**, open **System Settings → Privacy & Security**, scroll to the message about voicepuncher and click **Open Anyway**, then confirm with your password. Needed once per version; macOS 15 and later no longer offer right-click → Open for this.

The app carries its own engines — whisper.cpp 1.9.5 for speech recognition and the command spotting service — so nothing else is installed: no Homebrew, no terminal.

## First launch

Nothing is asked in a terminal. The first launch goes through these steps, each in a window of its own, and remembers them:

1. **Settings file.** The app writes `~/.voicepuncher/config.toml` with every option at its default and a comment explaining it (hotkey Right Option, recording mode Toggle, language detected automatically). Change the options in the [Settings](#settings) window; delete the file to get the defaults back at the next launch.
2. **Permissions.** The **Permissions** window lists the three permissions the app needs, each with its status (Granted, Not granted, Not asked yet) and its remedy — **Request…** shows the system prompt, **Open System Settings** opens the exact pane. The app checks again by itself and goes on as soon as the last one is granted, without a restart; a permission revoked while it runs stops dictation within 5 s and reopens the window. **Permissions…** in the menu opens the window at any time. The panes list the app as **voicepuncher** with its icon; an update signed with the same certificate keeps them.

   | Permission | Why | Location |
   |---|---|---|
   | Microphone | to record your speech | System Settings → Privacy & Security → Microphone |
   | Accessibility | to paste the text into the focused app (it presses ⌘V) | System Settings → Privacy & Security → Accessibility (macOS 27 and later: Device Control and Data Access) |
   | Input Monitoring | to notice the hotkey in every app | System Settings → Privacy & Security → Input Monitoring |

3. **Models.** With a [model](#models) missing, the **Download speech models** window opens and the menu shows "Waiting for models — open Download models…". It lists each needed file with its size and status, the free space and the folder. **Download** fetches the missing files one after another (≈ 2.1 GB), with each file's progress, the total, the rate and the time left; **Cancel** keeps what came, and the next **Download** goes on from there; **Retry** follows a failure. Once every file is present, **Done** closes the window and the services start. **Download models…** in the menu and in **Settings → Services** opens the window again.
4. **Services.** The app starts its local services (the menu shows "Starting services… (N of M ready)"); when all answer, the menu bar icon becomes a microphone and the menu says "Ready — hold Right Option to dictate".
5. **Login item.** Once it is ready the first time, voicepuncher asks once whether to start at login: **Start at Login** adds it to **System Settings → General → Login Items**, **Not Now** adds nothing. Either answer is remembered (`[app] launch_at_login_asked` in the configuration file, kept by the app). Change it later with the switch in **Settings → General** or in Login Items. A launch at login logs `launched at login`.

## Using

**Menu bar.** The app lives in the menu bar, without a Dock icon. The icon shows what it does: an hourglass while it starts, a microphone when ready, an ear when voice control listens, a microphone with a red dot while recording, a moving waveform while it recognizes the text, a keyboard while the text waits for a text field, a warning triangle with a failure. Click it for the menu:

- the version and the state in words ("Ready — hold Right Option to dictate");
- `Settings…` (⌘,) opens the [settings window](#settings);
- `Open Logs Folder` shows `~/.voicepuncher/logs/` in Finder;
- `Quit voicepuncher` (⌘Q) cancels a recording in progress and quits.

**Hotkey** (`[hotkey] key`, default `RightOption`):

- `recording_mode = "Toggle"`: press to start a dictation, press again to stop — the text is pasted at the cursor.
- `recording_mode = "Hold"`: hold while speaking, release to paste.
- `Escape` during a dictation cancels it; presses shorter than `min_press_ms` (200 ms) are ignored.

**Voice** (Settings → General → Voice control). The app listens all the time; the hotkey keeps working.

| Say | When | Does |
|---|---|---|
| `start` / `start dictation` | no dictation | starts a dictation |
| `stop` / `stop dictation` | dictating | stops and pastes |
| `cancel` / `cancel dictation` | dictating | drops the dictation |
| `restart` / `restart dictation` | dictating | drops it and starts again |
| `enter` / `press enter` | any time | presses Return; while dictating: stops, pastes, then Return |
| `clear` / `clear text` / `clear all text` | no dictation | Cmd+A, Delete |
| `paste` / `paste text` | no dictation | Cmd+V: pastes what the clipboard holds |

A command works when it is:

- said on its own, with a short pause (0.3 s) around it;
- the last words of a phrase: "…and that is all, stop";
- `start` at the beginning: "start, the meeting moved to Friday";
- doubled anywhere, no pause needed: "stop stop", "enter enter";
- a chain of 2–3 commands: "stop enter", "stop clear enter", "stop enter start", "paste enter".

Any command may end with "please". The same words inside a sentence ("we should stop the server") do nothing. Details: [Voice control](#voice-control).

**Example.** Press `RightOption`, say "hello world, I think the API is fine", press it again; the transcript is pasted at the cursor as soon as speech recognition returns it.

### Voice control

Hands-free dictation, off by default. Turn it on with **Voice control** in Settings → General, or in the configuration file:

```toml
[voice_control]
enabled = true
```

Push-to-talk keeps working as without voice control: the hotkey starts and stops a dictation, Escape cancels it, the overlay and sounds are the same. **Listening** starts with the app (sound `Tink`) and stays on until the app quits; the microphone stays open and seven English commands take effect:

| Say | While | Effect |
|---|---|---|
| `start dictation` or `start` | listening, no dictation | starts a dictation (start sound, overlay) |
| `stop dictation` or `stop` | dictating | ends it; the text is pasted without the command |
| `cancel dictation` or `cancel` | dictating | discards it; nothing is pasted |
| `restart dictation` or `restart` | dictating | discards it and starts a new dictation at once |
| `press enter` or `enter` | listening | presses Return in the focused app; during a dictation it first ends it and pastes the text, like "stop enter" |
| `clear all text`, `clear text` or `clear` | listening, no dictation | presses Cmd+A, then Delete, in the focused app |
| `paste text` or `paste` | listening, no dictation | presses Cmd+V in the focused app, which pastes what the clipboard holds; the clipboard stays as it is |

Every command may end with "please" ("press enter please"). A pause of 0.3 s ends an utterance, also after a quiet word and when more speech follows the pause at once ("restart", a short breath, the next sentence). A command counts:

- **on its own**, with a pause before and after it;
- **at the end of an utterance**, right after other speech: "…and that is all, stop" ends the dictation and pastes the speech before it; the same words inside a sentence ("we should stop the server") are ignored;
- **"start" at the start of an utterance**: "start, the meeting moved to Friday" starts a dictation that begins with "the meeting…";
- **doubled anywhere**, with no pause around it: "start start", "stop stop", "cancel cancel", "restart restart", "enter enter", "clear clear", "paste paste"; after "stop stop" the words said after it are dropped;
- **as a chain** of two or three commands in a row, anywhere: "stop enter" pastes and sends, "stop clear", "cancel clear", "paste enter" pastes the clipboard and sends, "stop enter start" pastes, sends and starts the next dictation; "start" can only come last.

A command repeated within one utterance ("press enter, press enter") counts once.

A dictation started by voice begins where the command's words end, not when the command is recognized a moment later. What you say right after "start", "start start", a chain ending in "start" or "restart" is kept from its first word: the app takes it from the sound it hears all the time while listening.

A command said in the other state is ignored. `press enter` said right after `stop dictation` waits until the text is pasted. The hotkey and the voice commands drive the same dictation: "stop dictation" ends one the hotkey started, and the hotkey ends one "start dictation" started. `press enter`, `clear all text`, `paste text` and `restart dictation` each play a sound of their own (`Morse`, `Purr`, `Frog`, `Hero`) and show their sign on the overlay for about a second (`⏎ ENTER`, `⌫ CLEAR`, `⎘ PASTE`, `↻ RESTART`); a restart plays only its own sound, not the cancel and start sounds. The sounds are `[sounds]` settings (`enter_sound`, `clear_sound`, `paste_sound`, `restart_sound`; `command_sound` still sets the enter sound).

**Confirm commands with Whisper** (Settings → General, `[voice_control] confirm_commands`, on by default): every command waits until the speech recognition service for commands heard it too. Off, a command said on its own or doubled is carried out as soon as the keyword spotter hears it, faster but with the spotter alone deciding; a command inside a sentence still waits. The speech recognition service still hears each such command afterwards: the log (`whisper on unconfirmed ["…"]: agreed`, `other` or `none`) and the command history record whether it agreed, and the start line shows `confirm=unconfirmed`.

**Careful with `clear all text`:** it selects everything in the focused app before deleting — meant for a chat input, it erases a whole document in an editor.

### Languages

**Speech language:** keep `[transcription].language = "auto"` (the default) when you dictate in more than one language. A fixed language makes the speech recognition service write speech in another language as text in the fixed one: with `language = "ru"`, 29 of 40 English corpus samples came out as Russian, mostly garbled.

**Your languages:** `[transcription].languages` (default `["ru", "en"]`) lists the languages you speak. With `language = "auto"` the app asks the service which language it heard; when that is none of yours — whisper sometimes takes a short Russian phrase for Korean — it sends the recording again with the first of your languages fixed, uses that transcript and logs `transcript detected as …, not one of … — requesting again in …`. Accepted codes: `ru`, `en`, `uk`, `be`, `kk`, `de`, `fr`, `es`, `it`, `pt`, `nl`, `pl`, `cs`, `tr`, `zh`, `ja`, `ko`; any other code stops the app at startup.

### Dictation archive

Every dictation that reached speech recognition is kept in `~/.voicepuncher/history/`, one folder per dictation, named after its start time in UTC (`2026-10-19T08-15-02.431Z/`). Each folder holds:

- `audio.wav` — the whole recording, 16 kHz mono 16-bit, so you can replay it or transcribe it again.
- `entry.json` — what happened to it:

| Field | Meaning |
|---|---|
| `format` | Layout version of `entry.json` (now `2`; format `1` also had `refinement` and `refinement_model`) |
| `started_at` | Start of the recording, UTC |
| `start` | How it started: `hotkey_held`, `hotkey_toggled`, `voice` or `recording_file` |
| `audio` | The audio file and its length in ms |
| `chunks` | Text of each chunk as speech recognition returned it |
| `raw_transcript` | The joined transcript |
| `pasted_text` | The text that was pasted (or printed, in file mode) |
| `outcome` | `pasted`, `left_in_clipboard`, `paste_failed`, `speech_recognition_failed`, `no_text` or `only_command_words` |
| `lost_chunks` | Chunks speech recognition could not transcribe |
| `timings_ms` | Time of each stage after the recording stopped |
| `dictation_model`, `app_version` | What produced the text |

Settings (Settings → Archive, or `~/.voicepuncher/config.toml`):

```toml
[archive]
enabled = true                     # false: keep nothing
dir = "~/.voicepuncher/history"    # where the entries go
max_size_mb = 2048                 # beyond this, the oldest entries are removed first
```

`--archive-dir <DIR>` and `--no-archive` override them for one run. File mode (`--input-file`) archives only with `--archive-dir`, so tests and benchmarks never write into your archive.

**Privacy:** the archive holds the audio and text of everything you dictated. It stays on this machine: it is never sent anywhere and never committed. Delete the folder, or set `enabled = false`, to get rid of it.

## Settings

`Settings…` in the menu (⌘,) opens the settings window: a toolbar tab per section and a system control for every option of `~/.voicepuncher/config.toml` (or the file `--config` names). Rarely changed options sit under `Show advanced`.

- **Save** checks every value first: a value out of range or a service address that is not on this Mac is refused, outlined in red and named in the status line, and the file stays unchanged. Otherwise only the changed keys are written; comments, order and the examples you did not touch stay as they are. A missing file is created.
- The saved settings apply without a relaunch: the status line says `Saved and applied.`, or that they apply after the current dictation, or why the running app could not use them (the file keeps the values).
- **Revert** drops the unsaved edits; closing the window with unsaved edits asks Save, Don't Save or Cancel.
- An option this run's command line overrides (`--hotkey`, `--input-device`, …) shows the file's value with a note under it naming the value the run uses.
- **Start at login** (General) is not a configuration key: it adds or removes the app in **System Settings → General → Login Items** at once, without Save, and shows the system's state each time the window comes to the front. A login item waiting for approval shows a note and a button to that pane; outside an installed app (run from a build folder) the switch is disabled.

| Tab | Option | Control | Key |
|---|---|---|---|
| General | Hotkey | pop-up | `hotkey.key` |
| General | Recording mode | two radio buttons | `hotkey.recording_mode` |
| General | Minimum press | number, ms | `hotkey.min_press_ms` |
| General | Microphone | pop-up: System default and the Mac's input devices | `audio.input_device` |
| General | Longest recording | number, s | `audio.max_recording_seconds` |
| General | Voice control | switch | `voice_control.enabled` |
| General | Confirm commands with Whisper | switch | `voice_control.confirm_commands` |
| General | Start at login | switch, acts at once | none: the system's login items |
| General (advanced) | Voice control service | text | `voice_control.url` |
| Dictation | Language | pop-up: Detect automatically and the speech recognition service's languages | `transcription.language` |
| Dictation | Languages to expect | text, comma-separated codes | `transcription.languages` |
| Dictation | Prompt | text view | `transcription.prompt` |
| Dictation | Temperature | slider | `transcription.temperature` |
| Dictation | Timeout | number, s | `transcription.timeout_secs` |
| Dictation (advanced) | Chunk threshold, Chunk overlap, VAD lookback | numbers, s | `transcription.chunk_threshold_secs, chunk_overlap_secs, vad_lookback_secs` |
| Dictation (advanced) | Model name | text | `transcription.model` |
| Dictation (advanced) | Service address | text | `transcription.base_url` |
| Sounds | Start, Stop, Cancel, Listening on, Listening off, Enter, Clear, Paste, Restart | pop-up of the 14 system sounds and None, ▶ plays it | `sounds.*_sound` |
| Paste | Clipboard restore delay | number, ms | `paste.clipboard_restore_delay_ms` |
| Paste | Wait for a text field | number, s | `paste.focus_wait_seconds` |
| Archive | Keep every dictation | switch | `archive.enabled` |
| Archive | Folder | path, Choose… and Show in Finder | `archive.dir` |
| Archive | Size limit | number, MB | `archive.max_size_mb` |
| Logging | Level | pop-up; Open Logs Folder beside it | `logging.level` |

The configuration file `~/.voicepuncher/config.toml` holds the same options with a comment on each; the window keeps your own comments when it saves. An edit made in a text editor applies after **Save** in the window, a relaunch, or the hang-up signal ([Developer](#reload-the-configuration-without-a-restart)).

## Logs

In hotkey mode the app writes its log to `~/.voicepuncher/logs/voicepuncher.log`, beside the services' logs, whether it was started from a terminal or not. The first line of every launch names the version, process id, config file, macOS version and architecture (`voicepuncher 0.27.0 starting (pid 1234, config /Users/me/.voicepuncher/config.toml, macOS 27.0.1, aarch64)`); a panic on any thread is logged with its message, place and thread before the app ends. Started from a terminal (developers), the app also prints every line there; started without one (from the Finder, by launchd, or with the error output redirected), the file is the only output. `[logging] level` filters both.

The last dictation, with the launch header above it:

```bash
tail -n 200 ~/.voicepuncher/logs/voicepuncher.log
```

When the file would pass 10 MB, it is renamed `voicepuncher.log.1` (older ones move up to `.5`, the oldest is deleted) and a new file starts; no line is split between two files. File mode (`--input-file`) logs to the error output only and never touches the logs folder.

**Open Logs Folder** in the menu (and beside the level in Settings → Logging) shows the folder in Finder. Each local service writes its own log beside it (`whisper-server-<port>.log`, `voicepuncher-command-spotting-8082.log`); **Services → Open Log** in the menu opens one.

**In a bug report**, paste the last 200 lines of `voicepuncher.log` (above) — they start with the launch header naming the version and macOS — and say what you did and what you expected. Add the log of a service when the menu shows it Failed.

## Uninstall

1. Quit voicepuncher (menu → **Quit voicepuncher**); it stops the services it started.
2. Remove it from **System Settings → General → Login Items** if you let it start at login.
3. Drag `/Applications/voicepuncher.app` to the Trash.
4. To remove the models, settings, archive and logs too, delete the folder `~/.voicepuncher` (`rm -rf ~/.voicepuncher`).
5. Optionally remove voicepuncher from the three Privacy & Security panes of [First launch](#first-launch).

## Privacy

The model download is the app's only network use, and only when the user clicks **Download** (or runs `--download-models` / `just models`). It contacts `huggingface.co` and `github.com` (and the CDNs they redirect to) over HTTPS, at pinned addresses, and sends nothing but the requests for the files. `VOICEPUNCHER_MODEL_MIRROR` points every address at another host. Dictation and voice control never leave `127.0.0.1`.

Your dictations stay on this machine: the [dictation archive](#dictation-archive) keeps their audio and text in `~/.voicepuncher/history/`, never sent anywhere.

## Licenses

The bundle carries the license of every engine and library it bundles in `Contents/Resources/licenses/`:

| Component | License | File |
|---|---|---|
| whisper.cpp | MIT | `whisper.cpp-LICENSE` |
| ggml | MIT | `ggml-LICENSE` |
| LLVM OpenMP runtime (`libomp`) | Apache-2.0 with LLVM exceptions | `libomp-LICENSE` |
| sherpa-onnx | Apache-2.0 | `sherpa-onnx-LICENSE` |
| ONNX Runtime (linked into the command spotting service) | MIT | `onnxruntime-LICENSE` |

## Developer

Everything below is for working on voicepuncher from a checkout. The terminal is for development and [file mode](#file-mode-single-wav--stdout); the app itself needs none.

### From source

Prerequisites: Rust ≥ 1.88, [`just`](https://github.com/casey/just), Homebrew `whisper.cpp` (`just start-model-servers` offers to install it). Outside a bundle the app finds the engine on `$PATH` and logs its Homebrew version, e.g. `whisper-server 1.9.5 from $PATH`.

For the tests, also [cargo-nextest](https://nexte.st) 0.9.146: `cargo +stable install --locked cargo-nextest@0.9.146`. For the bundle and the disk image (`just bundle`, `just dmg`, the `bundle` stage of `full-check`), also `brew install create-dmg`.

```bash
git clone https://github.com/pirx/voicepuncher
cd voicepuncher
cargo build --release
```

Binary: `target/release/voicepuncher`

`just bundle` builds the application bundle `dist/voicepuncher.app` from it, with the engines `just engines` stages (below) (try it as installed: `open dist/voicepuncher.app`), and `just dmg` the disk image `dist/voicepuncher-vX.Y.Z.dmg` with its `.sha256`. Both sign with the certificate named by `VOICEPUNCHER_SIGNING_IDENTITY` (see [Release checklist](#release-checklist)); with the variable unset they sign ad-hoc and say so: such a bundle is a new application to macOS on every build, so it asks for the permissions again.

Run from source (`cargo run`, `just run`), macOS attributes the permissions to the terminal app you launch it from (e.g. iTerm2, Terminal, Warp): grant them to that app, which the Permissions window names.

```bash
just start-model-servers                 # optional: the services by hand, with [services] manage = false
cargo run --release -p voicepuncher      # the app, with its menu bar icon and windows
```

### Run everything in one command

`just run` does everything in one command: downloads missing models, restarts the local services (the command spotting service is rebuilt from the checkout; each service is started again if it crashes while the app runs), builds the app and starts it with voice control; the hotkey records as push-to-talk too. Extra arguments go to the app. Restarting reloads the models, so it takes a few seconds more than `just start-model-servers` followed by `cargo run --release -p voicepuncher`. It is also how an existing setup picks up a new model: after an update that changed one, `just run` downloads it and restarts the services on it.

### Engines

The bundle carries the engines `app/packaging/engines.lock` pins, by version and by the sha256 of every file; none is built from source. `just engines` stages them into `app/packaging/engines/` (gitignored; `just bundle` runs it):

| Engine | Source | Folder in the bundle |
|---|---|---|
| speech recognition (`whisper-server`) | Homebrew `whisper.cpp` 1.9.5 with `ggml` 0.27.0 and `libomp` 23.1.3 | `Contents/MacOS/engines/whisper/` |
| command spotting (`voicepuncher-command-spotting`) | this workspace, `cargo build --release` | `Contents/MacOS/engines/command-spotting/` |

Each folder holds the program, every library it loads and a `VERSION` file. `just engines` refuses a Homebrew version or a checksum other than the lock's, naming what it found, and writes nothing then; keep the formulae with `brew pin whisper.cpp ggml libomp`. It relocates every file (each Homebrew path becomes `@rpath`, `@loader_path` the only search path, ggml's compiled-in backend folder blanked so the Metal and CPU backends load from the engine's own folder), signs it ad-hoc, and runs each engine with nothing but `/usr/bin:/bin` on `$PATH`; `just bundle` signs every file again with the signing identity, before the app. Whisper is staged again only when the lock changed.

**Bumping an engine:** install the new version (`brew upgrade whisper.cpp`), write its version, sources and sha256 values into `engines.lock` (`shasum -a 256` of each file; the reason in the commit), run `just engines`, then `just full-check` — its `e2e-bundle` stage runs the bundled engines on the models and fails when one no longer uses the Metal GPU.

### Local services

voicepuncher runs no model itself. It sends each recording to the **speech recognition service for dictation** (OpenAI Audio Transcriptions API) on this machine and pastes the transcript it returns. The supported service is whisper.cpp `whisper-server`. With voice control on, the [command spotting service](#command-spotting-voice-control) confirms utterances with a **speech recognition service for commands** of its own, so the larger dictation model never slows down a voice command; the app never talks to that one.

| Service | Script name | Port | Model (variable) | Copies (variable) |
|---|---|---|---|---|
| speech recognition for dictation | `whisper-server` | 8081 (`VOICEPUNCHER_STT_PORT`) | `ggml-large-v3-turbo-q5_0.bin` (`VOICEPUNCHER_WHISPER_MODEL`) | 3 (`VOICEPUNCHER_STT_INSTANCES`) |
| speech recognition for commands | `whisper-server-commands` | 8085 (`VOICEPUNCHER_COMMAND_STT_PORT`) | `ggml-small.bin` (`VOICEPUNCHER_COMMAND_WHISPER_MODEL`) | 3 (`VOICEPUNCHER_COMMAND_STT_INSTANCES`) |
| command spotting | `voicepuncher-command-spotting` | 8082 (`VOICEPUNCHER_KWS_PORT`) | see [Command spotting](#command-spotting-voice-control) | 1 (the app: 2) |

**The app starts them itself.** In hotkey mode the app starts the services its configuration needs: the speech recognition service for dictation and, with voice control, the speech recognition service for commands and the command spotting service. A service that already answers on its port — started by `just start-model-servers`, say — is adopted: the app uses it and leaves it running at quit. A speech recognition copy the app started counts ready only once it has transcribed a short warm-up utterance (its first decode builds its Metal pipelines, up to seconds on a busy machine), and its log line names how long that took. The menu bar shows "Starting services… (N of M ready)" until all answer; the Services submenu lists each service with its state, with Restart for one the app started and Open Log. A service the app started is started again when it exits (after 1 s, doubling up to 30 s) or stops answering its health check three times; after five starts that never answered it is Failed, naming its log. Every service runs as copies the app starts and watches one by one, so one copy that dies or hangs is restarted while the others answer: the command spotting service as two copies (8082 → 49083–49084). A chunk whose speech recognition copies are all down waits up to 30 s for one to come back instead of being lost; the app's listening moves to another command spotting copy at once, and a confirmation refused by its copy goes to another copy for commands. At quit the app asks each service it started to end and kills one still running 5 s later. The app finds `whisper-server` in its bundle (`engines/`), through `VOICEPUNCHER_WHISPER_BINARY`, or on `$PATH`; the command spotting service next to the app or through `VOICEPUNCHER_KWS_BINARY`; the models through the variables of the table or in `~/.voicepuncher/models/`. A missing one fails that service with the file it looked for. The `[services]` section sets it:

```toml
[services]
manage = true            # false: the app starts nothing and expects the scripts below
dictation_copies = 3      # 1 to 4
command_copies = 3        # 1 to 4
command_spotting_copies = 2  # 1 to 4
models_dir = "~/.voicepuncher/models"
commands_url = "http://127.0.0.1:8085/v1"  # the speech recognition service for commands
```

The scripts below start the same services by hand, with the same flags:

```bash
just start-model-servers   # starts all three, in the order of the table (the service for commands before command spotting)
just stop-model-servers    # stops them again (or some: just stop-model-servers whisper-server-commands)
```

`start-model-servers` checks that `whisper-server` is installed (offers `brew install whisper-cpp`), skips a service if something already answers on its port, and waits until each model is loaded. A speech recognition copy counts ready only once it has transcribed a short warm-up utterance (`app/assets/warm-up.wav`), so no client's first request waits for its first decode; its ready line names that time (`, first transcription N ms`). Each Whisper model is found by its file name in `~/.voicepuncher/` or `~/.voicepuncher/models/`, where `just models` puts it; set the variable (environment or `.env`) to another file to use another model — naming one for dictation leaves the one for commands alone. A default model that is not on disk fails the start with "Run 'just models' or set it in .env". whisper-server also needs the Silero voice activity detection model `ggml-silero-*.bin` (`just models` downloads it; override with `VOICEPUNCHER_VAD_MODEL`): it removes silences longer than 2 s, so a silent recording yields no text, while the pauses between words stay. Logs: for a speech recognition service `~/.voicepuncher/logs/whisper-server-<port>.log` with one copy (see below for several). Set the port variables of the table to use other ports; PID and log files are per port, so the two speech recognition services never share one. `start-model-servers` also takes the services to start (`just start-model-servers whisper-server`). Each service it launches runs under a supervisor: when the service exits (a crash), it is started again after 1 s (doubling up to 30 s while it keeps crashing), and its log keeps the crash output followed by a `… exited with code N — restarting` note; a service that exits before it was ever ready fails the start at once, with its log. whisper-server runs without language probabilities (`-nlp`): with them, whisper.cpp 1.9.5 crashes on a command confirmation of an utterance without speech. Each speech recognition service runs as three `whisper-server` copies (`VOICEPUNCHER_STT_INSTANCES`, `VOICEPUNCHER_COMMAND_STT_INSTANCES`, at most 4; 1 runs a single whisper-server on the service's own port) copy *n* on the service's port + 10000 + *n* (8081 → 18082–18084, 8085 → 18086–18088; 10000 below when that would pass port 65535), each under its supervisor, and nothing listens on the service's own port: whisper-server transcribes one request at a time, and two to four copies on one GPU are about twice as fast on long speech (five and more are slower). The app and the command spotting service find the copies from the service's address alone — its own port when that answers, else whichever of the four ports above it plus 10000 answer; a speech recognition port counts as a copy only once it answers `GET /` with a page naming whisper, so another program on the port (logged once with its name and pid) gets no request, and a dictation lost while it holds the service's own port names it in the failure notice — and send each request to a free copy, one request per copy, or wait their turn; a copy that refuses a connection gets no requests until it answers again, a request whose copy died is sent again to another copy, and a copy that holds a request past twice its time limit is logged as hung. The CPU threads are split between the copies of a service (cores ÷ copies each); logs `~/.voicepuncher/logs/whisper-server-<copy port>.log`. The service for commands uses the same flags as the one for dictation, with its own model; three copies of small answer the bursts of confirmations of several connections at once (≈ 0.7 GB each; `VOICEPUNCHER_COMMAND_STT_INSTANCES=1` saves ≈ 1.4 GB, unmeasured). `stop-model-servers` stops only servers that `start-model-servers` launched, together with their supervisor, so they stay stopped. A service that is already running keeps the flags it was started with: after an update that changes them, restart it (`just stop-model-servers whisper-server whisper-server-commands && just start-model-servers`).

To run the services by hand instead:

```bash
whisper-server -m ~/.voicepuncher/models/ggml-large-v3-turbo-q5_0.bin \
  --host 127.0.0.1 --port 8081 --inference-path /v1/audio/transcriptions \
  -l auto -bo 1 -nf -sns -mc 0 \
  --vad -vm ~/.voicepuncher/models/ggml-silero-v5.1.2.bin -vsd 2000 -vp 500
# the same with -m ~/.voicepuncher/models/ggml-small.bin --port 8085 for the service for commands
```

whisper-server 1.9.5 accepts `-nf` (no temperature fallback) but ignores it: every request still decodes again at temperatures 0.2, 0.4, … 1.0 when the first decoding fails. `-nf` stays on the command line for a whisper-server that honours it. Dictation keeps the server's fallback; a command confirmation turns the retries off per request (`temperature_inc` 0), so on an idle service one confirmation takes at most about 1.5 s.

`-mc 0` gives a request without a prompt no text context: text decoded in one 30 s window of a request does not steer the next. Requests never share text in any case — whisper-server clears it before every request. With no text context whisper.cpp also drops the request's prompt, so a dictation request with a `[transcription] prompt` sets its own context (`max_context` 224) and keeps the prompt first in every window (`carry_initial_prompt`), followed by the text decoded earlier in the same request; whisper.cpp still decodes the last piece of a request shorter than 5 s without either. Command confirmations send no prompt: with the command phrases as the prompt, Russian speech that is no command was heard as a command.

**Writing `[transcription] prompt` for large-v3-turbo:** write it as a sentence that uses the terms, not as a list of them. With the list "Kubernetes, Grafana, Terraform, Kafka." turbo wrote "Kubernetes" in 0 of 36 tries, whatever the server settings; a sentence such as "Мы развернули сервис в Kubernetes, смотрим метрики в Grafana." steers it with every setting tried. The small model follows both forms.

**English terms in Russian dictation:** without a prompt, "oh-my-zsh, zsh и Powerlevel10k" spoken in Russian came out as "а май ZSH, ZSH и по-ирлевелтинке"; with the prompt "В терминале у меня zsh с oh-my-zsh и темой Powerlevel10k, пакеты ставлю через Homebrew и npm." the terms came out as written. There is no default prompt: a Russian prompt also pushes English dictation toward Russian (a short English phrase after a voice command came out as "Кофе машине на 3rd floor", and a long English recording got invented Russian text). Set such a prompt only if you dictate mostly in Russian, and name the terms you use.

With `[services] manage = false` the app checks the speech recognition service for dictation at startup and exits with code 2 if it is not reachable. Only addresses on this machine are accepted (`localhost`, `127.0.0.0/8`, `[::1]`). A configuration file from an older version that still holds `[refinement]`, `[llm]` or `[services] refinement_copies` loads: the app logs one warning naming them and ignores them.

### Command spotting (voice control)

With [voice control](#voice-control) on, the app also needs the **command spotting service** `voicepuncher-command-spotting`, built from this repository (`command-spotting/`): sherpa-onnx keyword spotting with the English `zipformer-gigaspeech-3.3M` model and a Silero voice activity detector. The app streams the microphone to it over a loopback WebSocket while listening; it reports the spotted phrases and where speech starts and ends. Each short utterance (up to 3 s) is also sent to the speech recognition service for commands (`--transcription-url http://127.0.0.1:8085/v1`, the Whisper small model: it answers a short utterance in about 0.25 s, where turbo takes about 1 s and lost commands), and the service reports a command when the whole utterance is a command phrase — this is what recognizes commands in a non-native accent reliably; the keyword spotter alone missed most of them. `just models` downloads both models into `~/.voicepuncher/models/` (override with `VOICEPUNCHER_KWS_MODEL_DIR`, `VOICEPUNCHER_KWS_VAD_MODEL`); `just start-model-servers` builds the service and starts it on `127.0.0.1:8082` (`VOICEPUNCHER_KWS_PORT`), log `~/.voicepuncher/logs/voicepuncher-command-spotting-8082.log`. With voice control on and `[services] manage = false`, the app checks it at startup and exits with code 2 when it does not answer; otherwise the app starts it.

**When a service stalls or goes away.** The command spotting service answers the app's pings at once, whatever its confirmations do: each confirmation runs on a thread of its own and is given up after 3 s (`--confirmation-deadline-secs`), so a slow speech recognition service for commands never makes voice control stop. When the command spotting service does go away (a crash, a restart, a hang), the app plays the listening-off sound, keeps a running dictation recording and tries the service again every few seconds; once it answers, the listening-on sound plays and the speech of the last seconds is sent to it again, so a "stop dictation" said meanwhile still counts. A speech recognition request that times out or gets an error answer is sent again up to twice; the wait for one request grows with its audio (5 s plus half its length). The scripts also restart a command spotting service that stops answering its `/health` check three times in a row (`… stopped answering its health check — restarting` in its log).

**Tracing a session.** Every log line names its thread (`[time LEVEL thread target]`). In the app, `command spotting: session=<n>` follows one connection to the command spotting service (connect, loss with its cause, each reconnect attempt, back with the replayed seconds), `request=<n>` one speech recognition request with each failed attempt, and `voice control: listening → away` / `away → listening again` the state changes. In the command spotting service log, `conn=<n>` follows one app connection over its threads `conn-<n>-io`, `conn-<n>-proc` and `conn-<n>-confirm`, `job=<conn>.<n>` one confirmation (asked, answered in N ms or given up, what it decided), `lag=` audio that waited for processing, and a `stats conn=<n>` line every 30 s. `RUST_LOG=debug` adds every confirmation asked and every skipped check.

### Models

The app downloads every model its configuration needs into the models folder (`[services] models_dir`, default `~/.voicepuncher/models/`) at its first launch, in the Download window (see [First launch](#first-launch)); a file already there, in `~/.voicepuncher/` or named by its variable (`VOICEPUNCHER_WHISPER_MODEL` and the others) counts as present. Without a window, `voicepuncher --download-models [--models-dir DIR]` does the same in a terminal, one line per file, and exits 0 when every file is present, else 1. `just models` builds the app and runs it so (≈ 2.1 GB the first time):

| Model | Used by | File | Source |
|---|---|---|---|
| Whisper large-v3-turbo q5_0 (≈ 574 MB) | speech recognition service for dictation | `ggml-large-v3-turbo-q5_0.bin` | https://huggingface.co/ggerganov/whisper.cpp, pinned revision, sha256 checked |
| Whisper small (≈ 488 MB) | speech recognition service for commands | `ggml-small.bin` | https://huggingface.co/ggerganov/whisper.cpp, pinned revision, sha256 checked |
| Silero VAD | both speech recognition services | `ggml-silero-v5.1.2.bin` | https://huggingface.co/ggml-org/whisper-vad |
| keyword spotting + Silero VAD | command spotting service | `sherpa-onnx-kws-*`, `silero_vad.onnx` | https://github.com/k2-fsa/sherpa-onnx |

Each file is downloaded into `NAME.part` beside it and resumed from there after a cancel, a cut connection or a quit; a part whose sha256 does not match is downloaded again once, and then the file fails, so a cut or altered file never lands. A lost connection is retried after 2, 4 and 8 s; a download that does not fit in the free space does not start. The turbo q5_0 file is the measured one: WER 0.077 against small's 0.116 on the fixed 100 corpus recordings, and every chunk-seam case under difficult recording conditions within the bar.

### Command line

Every flag is for development, tests and file mode; each one that changes a setting overrides the configuration for this run only. `voicepuncher --help` lists them.

```
--config <FILE>           The configuration file [default: ~/.voicepuncher/config.toml; written with the defaults when missing]
--hotkey <KEY>            The recording hotkey
--transcription-url <URL> Speech recognition service address (loopback only)
--archive-dir <DIR>       Keep each dictation in this folder (also archives a file run with --input-file)
--no-archive              Keep no dictation archive (wins over --archive-dir)
--input-file <FILE>       File mode: transcribe a WAV file and print the text to stdout
--voice-control           Listen for spoken commands from the start, beside the hotkey
--download-models         Download the missing model files without a window and exit
--models-dir <DIR>        The models folder of --download-models
-h, --help                Print help
-V, --version             Print version
```

`--llm`, `--no-llm` and `--refinement-url` are gone with text refinement: a command line that still passes one exits with code 2 naming it. The terminal setup wizard and `--setup` are gone: to start over with the defaults, delete the configuration file; the next launch writes it again.

### Exit codes

| Code | When |
|---|---|
| 0 | Quit from the menu, SIGTERM or SIGINT; `--download-models` with every file present |
| 1 | File mode: the recording cannot be read or transcribed; `--download-models` with a file missing |
| 2 | A command line the app does not accept; a configuration file it cannot read; file mode: a service that does not answer; with `[services] manage = false`, a service that does not answer at startup |

With the services the app manages (the default), a missing permission, model or service never ends the process: the Permissions window, the Download window and the menu show it.

### File mode (single WAV → stdout)

Feed a pre-recorded WAV instead of using the hotkey. Output goes to stdout — useful for scripting and E2E tests.

```bash
just start-model-servers
./target/release/voicepuncher --input-file recording.wav
```

WAV must be 16 kHz, mono, 16-bit PCM.

A file run is not archived unless you add `--archive-dir <DIR>`: then that folder gets one entry for the run, marked `recording_file`.

### Reload the configuration without a restart

Edit `~/.voicepuncher/config.toml` (or the file `--config` names), then send the running app the hang-up signal:

```bash
kill -HUP "$(pgrep -x voicepuncher)"
```

The app reads the file again, applies the command line options over it, and restarts its pipeline inside the same process: the log shows `runtime: reloading configuration`, `runtime: restarting` and `runtime: ready (hotkey …)`. A dictation in progress finishes and pastes first. A file that does not load or validate is rejected: the log names the reason (`runtime: configuration rejected: …`) and the app keeps its settings.

### Upgrading from the in-process Whisper engine

The `[whisper]` config section and `--whisper-model` are gone. A config that still has `[whisper]` makes the app exit with a message naming each key's replacement. Rename the section and move the decoder settings to the service:

```toml
# before                                   # after
[whisper]                                  [transcription]
model_path = "~/.voicepuncher/ggml.bin"    base_url = "http://127.0.0.1:8081/v1"   # model: whisper-server -m
language = "ru"                            language = "auto"   # "ru" turns English speech into Russian text
initial_prompt = "Docker, API"             prompt = "Docker, API"
threads = 4                                # whisper-server -t 4
chunk_threshold_secs = 15                  chunk_threshold_secs = 15
```

`beam_size`, `patience`, `entropy_thold`, `logprob_thold`, `no_speech_thold` and `flash_attn` are now `whisper-server` launch flags; `temperature_inc` has no flag (whisper-server 1.9.5 ignores `-nf` and keeps its own temperature fallback). New keys: `model` (sent as-is), `temperature`, `timeout_secs` (per request, default 60). Then run `just start-model-servers`.

### Tests

All checks run through [`just`](https://github.com/casey/just) (`brew install just`). `just` with no arguments lists every recipe.

| Command | What it runs | Time | When |
|---|---|---|---|
| `just test-one <name>...` | only the named tests or slice/component scenarios, in the `test` stage | seconds after the build | TDD Red and Green |
| `just test` | unit + architecture + integration tests under cargo-nextest, each within its time limit | ~1 min | once per task group |
| `just check` | fmt, clippy, `test`, release build, E2E smoke, WER accuracy on 10 samples | ~8.5 min (machine lock ≈ 7.3 min) | often while developing, before every commit you push |
| `just full-check` | fmt, clippy, `test`, release build, the bundle (`bundle`: the disk image built and mounted, the bundle's Info.plist and signature checked, then the bundled engines checked, then the `app_distribution.feature` scenarios with the signing guards; no machine lock), E2E smoke, chunk seams, outages (`e2e-resilience`), the bundled engines (`e2e-bundle`: a copy of the bundle starts every service from its own engines on the Metal GPU), WER accuracy on a fixed sample of 100 recordings (or 10, below) | ~15.5–17.5 min (machine lock ≈ 14.5 min with 10 recordings, ≈ 16.3 min with 100) | before merging a branch and before every release |

Each step also has its own recipe: `fmt`, `lint`, `build`, `e2e-smoke` (three stages, one per capability), `e2e-quick` (10 samples), `e2e-seams` (the seam sweep, plus Russian and English speech under 11 recording conditions, transcribed in chunks), `e2e-bench` (100 samples, two at a time), `e2e-resilience` (the `@resilience` scenarios: a speech recognition service paused while voice control runs, a command spotting service killed or paused and started again by the scripts, a request abandoned in the queue — each on service ports of its own, so the shared services are never touched), `bundle-smoke` (the `bundle` stage, then `e2e-bundle`: a copy of the bundle launched outside `dist/` with nothing but `/usr/bin:/bin` on `$PATH` transcribes a recording and answers the command spotting health check, and the engine logs show Metal), `e2e` (smoke + seams + start cues + 100 samples). The E2E recipes start every local service they test from the checkout's own code and models, on test ports of their own (28081 dictation, 28085 commands, 28082 command spotting; `VOICEPUNCHER_TEST_STT_PORT` and siblings change them), and stop them afterwards, also when a test fails; your everyday services on the default ports are never reused, stopped or restarted. A service a killed run left on a test port is restarted; any other process holding one stops the run before it starts, naming that process. `check` and `full-check` start the services once for all their E2E stages and end with each stage's time. Speech synthesized with `say` is cached in `~/.cache/voicepuncher/speech/`; delete it to synthesize everything again.

**Time limits.** Every Rust test runs under [cargo-nextest](https://nexte.st) 0.9.146 with a time limit, set in `.config/nextest.toml`: 2 min a test by default, and each E2E suite its own (one Gherkin harness run counts as one test, e.g. 7 min for the `@smoke` scenarios, 3 min for `e2e-quick`). A test still running at its limit is stopped and reported `TIMEOUT`, and the recipe fails at once instead of hanging. Without cargo-nextest 0.9.146 every test recipe stops with the install command above. `VOICEPUNCHER_HANG_PROBE=1 just test` and `VOICEPUNCHER_HANG_PROBE=1 just e2e-quick` arm two probes that hang on purpose, to see the limits work: they end with `TIMEOUT` after 2 and 3 min. Measured on Apple Silicon (10 cores): `just test` 67 s, `just check` ≈ 8.5 min.

**Needs and skipped tests.** A test that needs a running service, a model, your recordings, the corpus or the desktop states it in its name (`needs_stt`, `needs_desktop`, …). A recipe leaves out the tests whose need is missing and prints one line per need, `● skipped N test(s): <need> — <reason>`, instead of letting them pass without running; such a test run by hand without its need fails naming the variable to set. `full-check` runs the desktop tests (real microphone, Accessibility, windows on screen) in its `it-real` stage; when the terminal lacks Microphone or Accessibility the stage shows `–` skipped and names the permission: grant it, or run `just it-real` yourself. The tests that press keys in the focused app run only in `just test-real-keyboard`.

**Models and corpus** are found automatically: each Whisper model by its file name (`ggml-large-v3-turbo-q5_0.bin`, `ggml-small.bin`) in `~/.voicepuncher/` or `~/.voicepuncher/models/`, and the corpus in `e2e-test/corpus/`. Override with `VOICEPUNCHER_WHISPER_MODEL`, `VOICEPUNCHER_COMMAND_WHISPER_MODEL`, `VOICEPUNCHER_BENCH_CORPUS` (env or `.env`). If they are missing, `check`, `full-check` and the E2E recipes stop with an error instead of skipping. First-time setup: `just models`, `just corpus`.

**The machine-wide test lock.** Checks from several checkouts on one machine take turns through agent-kit's `with-machine-lock`. `check` and `full-check` run fmt, clippy, `test`, the release build and the E2E test builds right away, without the lock (every build at a low CPU priority, the test binaries at the normal one), and wait for the lock only for their stages on the local services (E2E, accuracy), which start and stop the services inside it. `just test` and `just release` never wait for it. A clone without the lock tool gets the command that installs it from the first check it runs (`just check` fails before its first stage); `VOICEPUNCHER_NO_MACHINE_LOCK=1` runs without it, as the hosted CI does. `full-check` builds incrementally like `check` and runs no `cargo clean`.

**The stage journal.** Every run of `test`, `check`, `full-check` and the E2E recipes ends with a table of its stages and adds one JSON line to `~/.cache/voicepuncher/stage-times.jsonl` (`VOICEPUNCHER_STAGE_JOURNAL`), shared by every checkout and kept when a worktree is removed: when, worktree, branch, commit, recipe, result (passed, failed or interrupted), each stage with its time and result (passed, failed, or skipped after a failure), the lock wait and hold, the accuracy sample, the mean WER an accuracy stage measured and the failed tests. A run killed with `kill -9` leaves no line. A stage that already passed on the same inputs, in any checkout of this machine, is not run again: the table shows it `↺ reused` with the pass it came from, and the journal records it `reused` (`VOICEPUNCHER_REUSE_STAGES=0` runs every stage; AGENTS.md § How to verify a change says what counts as an input). `just stage-times` prints the runs of the last 30 days (`just stage-times 7d`, `just stage-times 2026-10-01`): per recipe and stage the runs, median, total and failures, and the reused runs, the lock wait, the failed tests, the runs per branch and the mean WER of the last accuracy runs per sample size.

**Rerunning failed tests.** After a failed run, `just rerun-failed` reruns only the tests and Gherkin scenarios that run failed, inside the stage they failed in (same services, test ports, machine lock and time limits), from the worktree's last line of the stage journal; `just rerun-failed '<package>::<binary> <test>' 'scenario: <name>'` names them by hand. It fails when a named test never ran. Once it passes, run the stage's recipe or the check whole once: a rerun never counts as a passing check.

**Frozen tests.** A test that flaked in a check outside the change that ran it is frozen, not fixed there: `just freeze-test '<name>' <recipe> <stage> '<what ran beside it>'` lists it in `.config/frozen-tests.toml` on the branch and on `main`, and no recipe runs it until a fix removes the entry. `just frozen-tests` lists the frozen tests with when, where and beside what they failed; `just frozen-tests run` runs only them, each in its stage, to see whether a fix holds.

**The accuracy sample of `full-check`:** a fixed sample of 100 recordings when the branch changes the recognition path (`recognition_paths` in `.config/stages.toml`), on `main` itself and when the changes cannot be read; else 10, like `check`. It says which and why before its first stage. `VOICEPUNCHER_ACCURACY_SAMPLE=100 just full-check` forces 100, e.g. after a new model.

**The accuracy ratchet.** The accuracy stages fail when the mean WER is worse than the accepted baseline in `e2e-test/accuracy_baseline.json` by more than the margin (0.01, or 1.5 standard deviations of the last 10 journaled rates once there are 10). A clearly better run passes and says so; `just accuracy-baseline` then writes this checkout's last rate of each sample size, with its commit, for the branch to commit.

**Your own voice.** Voice control is tested on recordings of your own commands when they exist in `~/.voicepuncher/voice-samples/` (never committed): `<session>.wav` (16 kHz mono, e.g. `rec -r 16000 -c 1 -b 16 session-1.wav` from Homebrew `sox`) with `<session>/manifest.jsonl`, one line per command said (`file`, `start`, `end`, `said`, `command`, `note`, optional `unclear`). `just e2e-smoke` then checks that every clear recording is recognized as its command, with no extra command, and runs whole hands-free sessions with your recorded commands.

### Release checklist

1. `just full-check` passes (on `origin/main` itself it runs the 100 recordings).
1. The hosted `ci` check (`.github/workflows/ci.yml`) is green on the release commit. Owner setup, once: in the repository's Settings → Branches, the protection rule of `main` requires the status check `ci`.
2. Manual run (required before merging a change to hotkey, recording, overlay, paste or permissions) — `cargo run --release -p voicepuncher`, then check:
   - [ ] Short phrase: press, speak, release → text is pasted.
   - [ ] Local services: `just stop-model-servers`, launch → the menu shows "Starting services… (N of M ready)", then Ready; Services lists each as Ready with Restart; quit → `pgrep -fl whisper-server` finds none. After `just start-model-servers`, launch → Services lists them as Adopted, and after quit they still run. Kill an owned `whisper-server` copy while the app runs → it is Restarting, then Ready, and the next dictation is pasted.
   - [ ] Permissions: remove each permission for the terminal in System Settings (`tccutil reset Microphone`, `Accessibility`, `ListenEvent` for its bundle id), launch → the Permissions window names the terminal; Request… shows the system prompt and the row shows Checking…; Open System Settings opens the right pane (Accessibility before macOS 27, Device Control and Data Access from macOS 27; for Input Monitoring it also asks again and shows the app in Finder) and the row names that pane; a switch that does not take shows the hint to remove the app with − and add it with +; while the app waits, the menu bar shows the missing-permission icon, the status line reads `Waiting for Accessibility…` and opens the Permissions window, and Settings note the wait under Voice control; the log shows `permissions: …` at each change; dictation starts once the last one is granted. Revoke Accessibility while it runs → within 5 s the window reopens with `Accessibility was revoked — dictation stopped`; grant it again → dictation works. Compare the window in light and dark with `openspec/changes/archive/*-permissions-gate-window/mockups/states/`.
   - [ ] Long dictation (> 30 s, crosses a chunk boundary) → no repeated or missing words at the seam.
   - [ ] Mixed Russian + English ("задеплой в Docker") → each language kept as spoken, terms in Latin script.
   - [ ] The transcript is pasted right after speech recognition; the overlay shows no "Refining text".
   - [ ] `just stop-model-servers whisper-server`, then with `[services] manage = false` start the app → it exits naming the speech recognition service address and `just start-model-servers`.
   - [ ] Stop the speech recognition service while the app runs and record → nothing is pasted, the failure is logged; after `just start-model-servers` the next recording works.
   - [ ] Overlay shows while recording and while processing, also on a second display if present.
   - [ ] A tap shorter than the minimum press time is ignored.
   - [ ] `just test-real-keyboard` passes (hotkey still registered after a paste, key presses into the focused app).
   - [ ] `just it-real` passes when `full-check` showed it `–` skipped (desktop tests: microphone, Accessibility, windows).
   - [ ] Voice control (`cargo run --release -p voicepuncher -- --voice-control`): the app plays the listening-on sound at start; in a chat input "start dictation", a sentence, "stop dictation" → the sentence is pasted without the stop phrase; "press enter" → the message is sent; "start dictation", speech, "cancel dictation" → nothing is pasted; type text, "clear all text" → it is erased with the clear sound and the `⌫ CLEAR` sign; "enter" → the `⏎ ENTER` sign and the enter sound, different from the clear sound; "start", a sentence, "restart" → only the restart sound, the `↻ RESTART` sign, then the recording again; with `clear_sound = "Funk"` in `[sounds]`, "clear all text" plays Funk; copy a sentence, "paste" → it is pasted with the `⎘ PASTE` sign and the Frog sound, and the clipboard still holds it; "paste enter" → pasted and sent; "I will press enter later" said without pausing → nothing happens; "start", a sentence, "stop" → the sentence is pasted; "start start" and a sentence at once, then "stop" → the whole sentence is pasted from its first word; "start", a short pause, a sentence, "stop" → the same; the hotkey starts a dictation with the overlay and start sound, and "stop dictation" ends it; "start dictation", then the hotkey → pasted, listening stays on. The log shows each `voice control: "…"` line, and every voice start a `voice start: total=…ms` line with `total` under 700.
   - [ ] `just stop-model-servers voicepuncher-command-spotting`, then with `[services] manage = false` start with `--voice-control` → it exits naming the command spotting service and `just start-model-servers`.
3. Bundle, on the release machine (required before merging a change to `app/packaging/`, and before the first release from a machine):
   - [ ] The signing certificate exists, once per machine: `just signing-identity` creates the self-signed Code Signing certificate `voicepuncher`, valid 10 years (a new certificate means one more round of permissions for every user), and macOS asks for your password to trust it for code signing. `security find-identity -v -p codesigning` lists `"voicepuncher"`. Add `VOICEPUNCHER_SIGNING_IDENTITY=voicepuncher` to `.env`. Without it `just release` stops and prints these steps.
   - [ ] `just dmg`, open the disk image: the app on the left, Applications on the right, the background says to drag one onto the other, the volume is named `voicepuncher X.Y.Z`.
   - [ ] Drag it to Applications, open it: the Gatekeeper dialog, then Open Anyway in Privacy & Security starts it; no Dock icon appears and the menu bar keeps the frontmost app's menu. Grant the three permissions: each pane lists **voicepuncher** with its icon. Dictate once.
   - [ ] The identity survives an update: change anything, `just dmg` again, replace the app in Applications, open it → no permission prompt, dictation works. `codesign -d -r- /Applications/voicepuncher.app` names the certificate (`certificate leaf = H"…"`), the same for both builds.
   - [ ] After a release: download the DMG from the GitHub release (`xattr -p com.apple.quarantine` on it shows the quarantine flag), install it as above.
   - [ ] Without Homebrew: quit every service (`just stop-model-servers`), launch the installed app from a shell with Homebrew's folders hidden — `env -i HOME="$HOME" PATH=/usr/bin:/bin /Applications/voicepuncher.app/Contents/MacOS/voicepuncher` — and dictate: the text is pasted, the log names each engine `from the bundle`, and `grep -l ggml_metal ~/.voicepuncher/logs/whisper-server-*.log` finds the engine on Metal. A downloaded copy runs nothing, not even an engine, before Open Anyway: macOS kills every program of a quarantined bundle that is not notarized.
4. After the merge, on a clean checkout of `origin/main` with a bumped `version` in `app/Cargo.toml` and the signing identity set: `just release` — cleans `target/` and `dist/`, runs the tests and builds from scratch without the machine-wide lock, builds the signed bundle into `dist/voicepuncher-vX.Y.Z.dmg`, then tags `vX.Y.Z` and publishes a GitHub Release with the disk image and its `.sha256`, nothing else. In the main checkout, `target/release/voicepuncher` is gone until it finishes.

### Model comparison (not a release gate)

Run only when changing a model, decoding params or the prompt. Missing models and corpus are downloaded automatically. For each Whisper model the script starts its own `whisper-server` on port 8091, with the `start-model-servers` flags.

| Command | Samples | Time |
|---|---|---|
| `just bench` | full corpus | long |
| `just bench-smoke` | 10 | ~2 min |

Example output (the layout; the numbers are illustrative):

```
+--------------------+--------+--------+----------+
| Whisper            |    WER |    CER |  Samples |
+--------------------+--------+--------+----------+
| large-v3-turbo-q5  |   7.7% |   4.1% | 100/100  |
| small              |  11.6% |   6.9% | 100/100  |
+--------------------+--------+--------+----------+
```

To add a model to the matrix, add an entry to `WHISPER_MODELS` in `e2e-test/scripts/bench.py`.
