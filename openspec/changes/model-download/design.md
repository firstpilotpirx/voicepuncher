# Design

## Context

`just models` downloads six files with curl and checks three sha256 values; the services' catalog (`local-services-runtime`) resolves model paths in `[services] models_dir`. `ureq` 2 with TLS is a dependency; `libc` arrives with `local-services-runtime`; `sha2` 0.11.1 is the hashing crate. The app's network policy is loopback only except this download. The windows of `permissions-gate-window` and `settings-window` set the AppKit technique.

## Decisions

**D1 — The catalog is domain data, the download a use case with ports.** `ModelFile` entries (name, URL, sha256, size, purpose, kind) are plain data; `ModelDownload` orchestrates check, fetch, verify, place, extract with `ModelStore`, `ModelFetcher`, `DiskSpace`, `Clock` ports — testable with fakes, independent of ureq and AppKit.

**D2 — One implementation for the window and the terminal.** `--download-models` runs the same use case with a terminal observer, and `just models` calls it, so the catalog cannot drift from the scripts. Alternative: keep curl and compare lists by a test — the fallback named in the proposal's open points.

**D3 — Resume with Range, hash while streaming, atomic placement.** `.part` + Range + streaming sha256 give resumable, verifiable downloads of multi-GB files without reading them twice; the rename makes presence atomic. A `.part` larger than the catalog size is discarded.

**D4 — Sequential downloads, bounded retries.** One file at a time with 3 retries (2, 4, 8 s) and one automatic re-download on checksum mismatch; the schedule lives in the use case and is unit-tested with a fake clock.

**D5 — `/usr/bin/tar` for the archive.** macOS ships bsdtar with bzip2; spawning it avoids two crates for one file; the exit code and the resulting folder are checked.

**D6 — Presence by name and size at launch, full hash on demand.** Hashing 3.3 GB at every launch would cost seconds; a placed file was verified when it was downloaded.

**D7 — The runtime gates the services on models, as it gates on permissions.** `WaitingForModels` precedes `StartingServices`; the window is a primary adapter observing the use case and the runtime; `AllPresent` triggers the services start without a relaunch.

**D8 — The gate asks for the files the configuration needs.** The dictation model and the whisper voice activity model always; the commands model, `silero_vad.onnx` and the keyword spotting folder only with voice control on; the Qwen model only with refinement on. The window lists those files; `--download-models` takes every catalog file (the Qwen model left out with `--no-llm`, which `just models no` passes). A refinement-off user is never asked for 2.5 GB they do not use.

**D9 — Presence matches how the services find the models.** A file is present in the models folder, `~/.voicepuncher/models` or `~/.voicepuncher` (the folders the service catalog searches) with the catalog size, or when the variable the service catalog reads for it (`VOICEPUNCHER_WHISPER_MODEL`, `VOICEPUNCHER_VAD_MODEL`, `VOICEPUNCHER_COMMAND_WHISPER_MODEL`, `VOICEPUNCHER_LLM_MODEL`, `VOICEPUNCHER_KWS_VAD_MODEL`, `VOICEPUNCHER_KWS_MODEL_DIR`) names an existing path — so E2E runs, which name every model by variable in a home of their own, never wait for models. Downloads always go to the models folder.

**D10 — The window: text statuses and one total bar.** Rows are drawn as the Permissions window draws its rows (frames, rebuilt from a `WindowModel` on every event), the Status column is text (`Downloading 42 %`), one determinate bar shows the total. Per-row bars were the alternative; text keeps the row model a pure mapping that unit tests cover.

**D11 — The Services tab holds fields and the models row; service states stay in the menu.** The `[services]` options move from General's advanced part to the Services tab; the tab adds the Models row (`N of M present`, `Download models…`, `Show in Finder`). The per-service state list with Restart already lives in the menu's Services submenu, so the tab does not repeat it; the on-demand `Check files` hash is left out (presence by size is the check). Copies stay 1–4, the limit the configuration already validates.

**D12 — A test mirror.** `VOICEPUNCHER_MODEL_MIRROR` (empty by default, read only by `--download-models` and the window's fetcher) replaces each pinned address with `<mirror>/<file>`, so the CLI test points the download at a local port without touching the real hosts; the catalog addresses stay pinned. The retry pauses come from the use case's `Clock`, so unit tests run without waiting.

**D13 — Mockups without a gate.** The change ran without a proposal gate (the user chose none); `mockups/design-spec.md` is the reference the window and tab follow, and no PNG set is drawn.

## Decisions taken at start

Gates: none (the user's choice for the batch). Open points of the proposal settled: `just models` delegates to the binary (D2); text statuses with one total bar (D10); no full hash at launch and no `Check files` button (D6, D11); sequential downloads (D4). The agent-kit tech matrix note (model distribution) is out of this repository and left to a separate agent-kit change.

## Risks / Trade-offs

- [Upstream files change under the pinned URLs] → the Hugging Face URLs pin a commit; the GitHub release assets are immutable; the catalog's sha256 catches any change.
- [A user on a metered link] → the window states the total before anything downloads; Cancel keeps the `.part`.
- [Redirects dropping the Range header] → the http adapter test covers a redirect to a Range-capable server.
- [Disk fills during the download] → write errors fail the file with `disk full`; the free-space check runs before the start.

## Test levels

| Scenario | Level |
|---|---|
| Start with a model missing (app-runtime) | unit test in `runtime/mod.rs`; component `model_download.feature` |
| The last model arrives while waiting (app-runtime) | unit test in `runtime/mod.rs`; component `model_download.feature` |
| A reload to an empty models folder (app-runtime) | unit test in `runtime/mod.rs` |
| The pipeline starts once the services are ready (app-runtime) | existing unit tests in `runtime/mod.rs` and component `app_runtime.feature`; unchanged |
| The refinement model is not on disk (local-services) | existing unit test in `localservices/catalog/tests.rs` |
| A missing model at launch (menu-bar-presence) | unit test in `menubar/macos/mapping/tests.rs` |
| Download models while every model is present (menu-bar-presence) | unit tests in `menubar/macos/mapping/tests.rs` and `modeldownload/macos/mapping.rs`; manual run |
| A launch with the dictation model missing | component `model_download.feature`; window: manual run |
| A launch with every model present shows no window | component `model_download.feature` |
| A file of the wrong size is missing | adapter integration `modelstore_fs_it.rs` |
| A model named by its variable counts as present | adapter integration `modelstore_fs_it.rs` |
| Every missing file is downloaded and placed | component `model_download.feature`; unit tests in `model_download.rs` |
| The keyword spotting archive is extracted | adapter integration `modelstore_fs_it.rs`; unit test in `model_download.rs` |
| Cancel and download again | component `model_download.feature`; adapter integration `modelfetch_http_it.rs` (Range) |
| A server that ignores the resume | adapter integration `modelfetch_http_it.rs`; unit test in `model_download.rs` |
| A corrupted part is downloaded again | component `model_download.feature` |
| A file that never matches is reported | component `model_download.feature` |
| No connection | component `model_download.feature` (fake clock); adapter integration `modelfetch_http_it.rs` (refused port) |
| A disk that is too full | component `model_download.feature`; unit test of the red line in `modeldownload/macos/mapping.rs` |
| The last file arrives | component `model_download.feature`; Done: unit test in `modeldownload/macos/mapping.rs` |
| A headless run with every file present | CLI integration `cli_it.rs` |
| A headless download that fails | CLI integration `cli_it.rs` |
| The Services tab shows the models | unit tests in `settings/macos` (field table, models line); manual run |
| A new models folder is checked | unit test in `runtime/mod.rs` (reload to an empty folder); manual run |
| Settings opens the window | manual run |
| Settings chosen again reuses the window | manual run; existing `settings_macos_it.rs` |
| Status texts, sizes, rates, time left | unit tests in `modeldownload/macos/mapping.rs` |
| First launch on an empty folder; cancel and resume; `just models` | manual run by the user |
