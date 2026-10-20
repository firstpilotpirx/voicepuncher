## Why

The services need six model files, about 3.3 GB: `ggml-large-v3-turbo-q5_0.bin` (547 MB, dictation), `ggml-small.bin` (465 MB, commands), `ggml-silero-v5.1.2.bin` (whisper's voice activity detection), `Qwen_Qwen3-4B-Instruct-2507-Q4_K_M.gguf` (2.3 GB, text refinement), `silero_vad.onnx` and the folder `sherpa-onnx-kws-zipformer-gigaspeech-3.3M-2024-01-01/` (voice control). Today `just models` downloads them with curl into `~/.voicepuncher/models/` and checks their sha256. The DMG cannot carry them (GitHub limits a release asset to 2 GB; the models change independently of the app; the download would be 3.3 GB for every version) and a user has no `just`. The user decided: the application downloads the models itself — the only time it uses the internet — into the same folder, with an explicit button, progress and checksums.

## What Changes

- Launch flow: after the permissions gate (`permissions-gate-window`) and before the services start (`local-services-runtime`), the app checks the models folder (`[services] models_dir`, default `~/.voicepuncher/models`). When any needed file is missing, the Download window opens, the runtime state is Waiting for models, the menu bar shows the waiting icon with `Waiting for models — open Download models…`. When every file is present, nothing is shown.
- The Download window (look fixed by `mockups/design-spec.md`, made with Claude Design before the proposal gate): title `Download speech models`; a line `voicepuncher works offline. It needs these files once, 3.3 GB in total; they go to ~/.voicepuncher/models.`; a table with one row per file: name, what it is for (`speech recognition — dictation`, `speech recognition — commands`, `voice activity detection — speech recognition`, `text refinement`, `voice activity detection — voice control`, `keyword spotting — voice control`), size, status (`Present ✓`, `Missing`, `Downloading 42 % — 18 MB/s — 1:20 left`, `Verifying…`, `Failed: no internet connection`, `Failed: checksum mismatch, downloading again`, `Cancelled`); a total progress bar with `1.2 GB of 3.3 GB`; the line `Free space: 120 GB` (red with `Not enough space: needs 3.3 GB` when short); buttons `Download` (all missing files, becomes `Cancel` while running), `Retry` on a failed row, `Open Folder`, `Quit`, and `Done` (enabled when every file is present; closes the window; the services then start by themselves).
- Downloads: from the same sources the `justfile` uses — the whisper models from `https://huggingface.co/ggerganov/whisper.cpp/resolve/5359861c739e955e79d9a303bcbc70fb988958b1/`, the whisper VAD model from `https://huggingface.co/ggml-org/whisper-vad/resolve/main/`, the Qwen model from `https://huggingface.co/bartowski/Qwen_Qwen3-4B-Instruct-2507-GGUF/resolve/ae44f08e1392f39c0e474af10c3ff8355c8b6688/`, `silero_vad.onnx` and the keyword spotting archive from `https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/` and `…/kws-models/` — with the sha256 values the `justfile` holds (`394221709cd5…`, `1be3a9b2…`, `2fde00ce…`; the silero and kws files get theirs computed once from the files in `~/.voicepuncher/models` and pinned). One catalog of name, URL, sha256, size and purpose lives in the app (domain data); `just models` becomes a thin recipe that runs `voicepuncher --download-models [--models-dir DIR]` — the same code, headless, with a terminal progress line — so there is one source of truth.
- Each file downloads to `NAME.part`; an existing `.part` resumes with an HTTP Range request (Hugging Face and GitHub support it); the sha256 is computed while streaming; on match the file is renamed into place, on mismatch the `.part` is deleted and downloaded once more, then reported; network errors retry 3 times with 2, 4, 8 s pauses; `Cancel` stops the stream and keeps the `.part` for a later resume. Files download one at a time (the disk and the link are the bottleneck; one progress is easier to read). The keyword spotting archive (`.tar.bz2`) is extracted with `/usr/bin/tar -xjf` into the models folder and deleted; the folder's presence counts as "present". Free space is checked before starting (`statvfs`).
- The Settings window gets the **Services** tab (`server.rack`): Models folder (`NSPathControl` + `Choose…`; changing it re-checks and offers to download into the new folder), `Models: 6 of 6 present` with `Download missing…` and `Show in Finder`; Speech recognition copies — dictation and commands (number fields 1–8, `[services] dictation_copies`, `command_copies`); Manage the services (switch, `[services] manage`, help `Off: start the services yourself with just start-model-servers`); a read-only list of the services with their state (from `local-services-runtime`'s snapshot) and `Restart` buttons. The menu gets `Download models…` under `Permissions…` when any file is missing.
- Privacy: this is the only network use of the app; README § Privacy says which hosts are contacted and when; the loopback-only validation for the services stays.
- Docs: `README.md` (§ First launch, § Models — the window and `--download-models`; § Privacy), `docs/ubiquitous-language.md` (Model catalog, Model download, Models folder), `docs/project-map.md`, `docs/architecture/overview.md` (model distribution row), `docs/architecture/context.md` (the two external hosts). Tech matrix (agent-kit PR, section "macOS Desktop"): model distribution — downloaded by the app at first launch from pinned URLs with sha256, resumable, never inside the installer (asset size limit, independent versions); download client `ureq` with the TLS feature; hashing `sha2`.

Current stage: PoC. Target stage: Prototype (a fresh Mac gets working models from inside the app).
Scope: the catalog, the download use case and its ports, the http fetcher with resume and hashing, the window, the Services tab, the menu item, the `--download-models` flag and the `just models` wrapper, the launch flow state, docs, mockups.
Acceptance bar: on a Mac with an empty models folder the first launch shows the window; `Download` fetches every file with progress, verifies them, extracts the archive, enables `Done`, and the services start; a download interrupted by `Cancel` (or a network cut) resumes from the `.part`; a corrupted file (edited `.part`) is detected and downloaded again; `just models` on a developer machine produces the same folder; the window's pictures match the approved mockups; the user pastes a healthy log of a first launch (models folder moved aside).
Out of scope: choosing other models (the catalog is fixed; `[transcription] model` names stay as they are); updating models when the catalog changes (a later change: the catalog version in the file names makes old files harmless); parallel downloads; a mirror or offline installer with models (deferred).

## Capabilities

### New Capabilities

- `model-download`: the needed model files are listed, downloaded with progress and resume, verified by checksum, extracted, into the models folder; the services start when every file is present; the same download runs headless for developers.

### Modified Capabilities

- `local-services`: the services start only when every model file is present; the Services tab shows and edits the models folder and copy counts.
- `settings-window`: the Services tab.
- `menu-bar-presence`: the `Download models…` item and the waiting state for models.
- `app-runtime`: the state Waiting for models.

## Requirements to specify

1. Missing models are listed before anything downloads: at launch with any file missing the window lists every file with its size, purpose and status, and the free space.
2. Download fetches, verifies and places each file: every missing file is downloaded to a temporary name, its checksum verified, then moved into the models folder; the archive is extracted; the window shows progress per file and in total.
3. A download resumes where it stopped: after a cancel or a network cut the next Download continues from the bytes already received.
4. A file whose checksum does not match is downloaded again, then reported if it fails twice.
5. Without internet the user is told and nothing is deleted: a connection failure after the retries shows `no internet connection` on the file and keeps what is present.
6. Not enough space is refused before starting, naming the amount needed.
7. The services start when the last file is present, without a relaunch.
8. The same download runs headless: `--download-models` downloads into the folder with a terminal progress and exits 0 when every file is present, 1 otherwise; `just models` uses it.
9. The models folder can be changed in the Services tab; the check runs again on the new folder.

## Implementation guide

1. **Mockups first.** `mockups/design-spec.md`; PNGs in `mockups/states/`; shown with the proposal at stage 3.
2. **Catalog.** `core/domain/model_catalog.rs`: `ModelFile { name, url, sha256, size_bytes, purpose, kind: File | TarBz2 { folder } }` and `const CATALOG: [ModelFile; 6]`; a unit test compares every URL and sha256 with the `justfile`'s `models` recipe text until that recipe delegates (then the test is removed in the same change — do both in one task group).
3. **Use case.** `core/application/model_download.rs`: `ModelDownload::new(store, fetcher, disk, observer)`; `check() -> Report` (present/missing per file, bytes needed, free bytes); `download_missing(cancel: &AtomicBool)`; events to the observer: `Started(file)`, `Progress { file, received, total, bytes_per_s }`, `Verifying(file)`, `Done(file)`, `Failed { file, reason }`, `AllPresent`. Ports: `ModelStore` (`present(file)`, `part_len(file)`, `place(file, part)`, `extract(file)`), `ModelFetcher` (`fetch(url, from_byte, sink) -> Result<()>` streaming chunks into a sink that hashes and writes; `supports_range`), `DiskSpace` (`free_bytes(dir)`). Retry and backoff schedule inside the use case with a `Clock` port (unit-tested with fakes).
4. **Adapters.** `adapters/secondary/modelfetch/http/mod.rs`: `ureq` (TLS feature already on) with `Range: bytes=N-`, expecting 206 for a resume (200 restarts from zero), 8 KB reads, the sink hashes with `sha2 = "=0.11.1"`; `adapters/secondary/modelfetch/fs/mod.rs`: the store (`.part` files, atomic rename, `/usr/bin/tar -xjf` with the folder check), `statvfs` through `libc` (already a dependency after `local-services-runtime`).
5. **Window.** `adapters/primary/modeldownload/macos/{mod.rs,mapping.rs}`: an `NSWindow` 520×420 pt with an `NSTableView` (columns File, Purpose, Size, Status), an `NSProgressIndicator` (bar, determinate) per row in the Status column (or a text status plus one total bar — the mockups decide), the total bar, the free-space label, the buttons; the observer events arrive on a download thread and are dispatched to the main thread; `Cancel` sets the atomic. Mapping functions (event → status text, bytes → human sizes, rate and ETA) unit-tested.
6. **Runtime.** `Runtime::start`: after the gate, `check()`; missing → state `WaitingForModels(report)` and the window; `AllPresent` → services start (`local-services-runtime` path). The menu item opens the window at any time (it shows `Present ✓` rows and `Done`).
7. **Headless.** `--download-models` in `Cli` (with `--models-dir` optional): runs `check` + `download_missing` with a stderr progress line (`\r` updates), no AppKit; exit code. `just models`: `cargo build --release -p voicepuncher` then `target/release/voicepuncher --download-models --models-dir "$VOICEPUNCHER_MODELS_DIR"`; the old curl loop is deleted; `justfile` variables keep resolving the model paths.
8. **Services tab** in `adapters/primary/settings/macos/tabs/services.rs` with the fields in What Changes, reading the service snapshot through the runtime.
9. **Docs** as listed; the agent-kit tech matrix PR.

Pitfalls: Hugging Face answers a `resolve` URL with a redirect to a CDN — ureq follows redirects by default, keep the Range header on the redirected request (ureq re-sends headers on redirect for GET); a `.part` larger than the file (changed upstream) must be deleted, not resumed (compare with the catalog size); hash while streaming, never read the 2.3 GB file again (except when a `.part` is resumed: hash the existing part first, with progress shown as `Verifying…`); extraction must land the folder name the command spotting service expects (`sherpa-onnx-kws-zipformer-gigaspeech-3.3M-2024-01-01`); check `tar`'s exit code; the window must not block the main thread — all I/O on the download thread; the GitHub URLs for sherpa-onnx assets redirect to `objects.githubusercontent.com`; keep the per-request timeout generous (no overall timeout; a read timeout of 30 s) so a slow link is not a failure.

## Impact

- `app/src/core/domain/model_catalog.rs`, `app/src/core/application/model_download.rs`, new ports in `core/application/`
- new `app/src/adapters/secondary/modelfetch/{http,fs}/mod.rs`, new `app/src/adapters/primary/modeldownload/macos/{mod.rs,mapping.rs}`, `app/src/adapters/primary/settings/macos/tabs/services.rs`, `app/src/adapters/primary/menubar/macos/` (item)
- `app/src/adapters/primary/cli/mod.rs` (`--download-models`, `--models-dir`), `app/src/main.rs`, `app/src/runtime/` (state, flow)
- `app/Cargo.toml` (`sha2`), `Cargo.lock`, `justfile` (`models` recipe delegates), `README.md`, `docs/*`; agent-kit tech matrix (separate PR); `mockups/`
- tests: unit in catalog, use case, mappings; new `app/tests/modelfetch_http_it.rs` (a local HTTP server thread serving a file with Range support: full download, resume from a part, 200 on resume restarts, checksum mismatch), `app/tests/modelstore_fs_it.rs` (part, rename, extract a tiny tar.bz2 made in the test), `app/tests/features/component/model_download.feature` + steps (mock fetcher, fake store), a CLI test for `--download-models` against the local server
- `openspec/specs/local-services`, `settings-window`, `menu-bar-presence`, `app-runtime` deltas
- Checks: `just test` after every edit; `just check` after the last group; `just full-check` before the merge; manual run by the user (first launch): move `~/.voicepuncher/models` aside, launch, download (or cancel and resume), `Done`, dictate; `just models` on the developer machine; the log pasted.

## Open points

- `just models` delegating to the binary (proposed, one source of truth) versus keeping curl in the `justfile` with a test that compares the lists (fallback if building first is a problem for a fresh checkout).
- Per-row progress bars (proposed) versus one total bar with text statuses: the mockups decide.
- Whether to verify present files' checksums at every launch (2.3 GB hashed, ~5 s): proposed no — presence and size only; a `Check files` button in the Services tab does the full check on demand.
- Sequential downloads (proposed) versus two at a time.

## Release

SemVer: MINOR — a new capability (download inside the app) and modified capabilities; pre-1.0.
Target version: 0.33.0 (corrected at merge with `kit version-check`).
