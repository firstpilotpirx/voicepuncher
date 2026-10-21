# Tasks

## 1. Catalog and download use case

- [x] 1.1 Red: unit tests in `app/src/core/domain/model_catalog.rs` — six files with name, pinned URL, sha256, size, purpose, kind; the URLs and the three sha256 values the `justfile`'s `models` recipe holds; `needed(needs)` picks the files of D8; see them fail
- [x] 1.2 Green: `ModelFile`, `ModelKind { File, TarBz2 { folder } }`, `CATALOG`, `ModelNeeds`, `needed`, `total_bytes`
- [x] 1.3 Red: unit tests in `app/src/core/application/model_download.rs` with fake `ModelStore`, `ModelFetcher`, `DiskSpace`, `Clock` — check report (present, missing, bytes missing, free), sequential download and placing, resume from a part, part larger than the file deleted, 200-on-resume restart, mismatch re-download then Failed, 3 retries with 2/4/8 s then no internet connection, not enough space refuses, cancel keeps the part, archive extracted, events in order; see them fail
- [x] 1.4 Green: the ports and `ModelDownload` (`check`, `download_missing`, `DownloadEvent`, `DownloadObserver`)
- [x] 1.5 `docs/ubiquitous-language.md`: Model catalog, Model download, Models folder; `just test` and commit

## 2. Adapters

- [x] 2.1 Red: `app/tests/modelfetch_http_it.rs` against a local HTTP server thread — full download, resume with Range (206), 200 on resume restarts, a redirect keeps the Range header, connection refused is a connection failure; see it fail
- [x] 2.2 Green: `adapters/secondary/modelfetch/http` — `ureq` with `Range: bytes=N-`, 8 KB reads, 30 s read timeout, `VOICEPUNCHER_MODEL_MIRROR` (D12); `sha2 = "=0.11.1"` in `app/Cargo.toml` for the sink's hash
- [x] 2.3 Red: `app/tests/modelfetch_fs_it.rs` — presence by size and by variable and in the fallback folders (D9), part length, append, place by rename, extract a tiny tar.bz2 made in the test, free space; see it fail
- [x] 2.4 Green: `adapters/secondary/modelfetch/fs` — the store and `statvfs` disk space; `architecture.rs` stays green; `just test` and commit

## 3. Runtime gate and headless download

- [x] 3.1 Red: unit tests in `runtime/mod.rs` — Waiting for models at start with no service required, the services required and Ready once the files are present, a reload to missing models ends the pipeline and waits; see them fail
- [x] 3.2 Green: `RuntimeState::WaitingForModels { folder, files }`, `ModelsGate` port, `Runtime::with_models`, the gate after permissions and before services; `runtime/models.rs` gate over the fs store
- [x] 3.3 Red: `app/tests/features/component/model_download.feature` (`@model_download @component`) with the spec scenarios that need no window, steps in `app/tests/steps/model_download.rs` over fake ports and the runtime with mocks; see them fail
- [x] 3.4 Green: steps and any missing behavior
- [x] 3.5 Red → Green: `--download-models`, `--models-dir` in `Cli`; headless run with a terminal observer, exit 0/1; `app/tests/cli_it.rs` cases (every file present → 0; one file on a refused mirror → 1 naming it)
- [x] 3.6 `justfile`: `models` runs `cargo build --release -p voicepuncher` and `--download-models` (with `--no-llm` for `refinement=no`); the curl loop deleted; `just test` and commit

## 4. Window, menu and Services tab

- [x] 4.1 Red → Green: `adapters/primary/modeldownload/macos/mapping.rs` — row status texts, human sizes, rate and time left, total line, free-space line and its red form, button states — unit tests
- [x] 4.2 The Download window (`modeldownload/macos/mod.rs`): rows, total bar, free space, Open Folder, Quit, Download/Cancel, Retry, Done; events dispatched to the main thread; opens on Waiting for models
- [x] 4.3 Red → Green: menu — status line "Waiting for models — open Download models…" and the waiting icon; `Download models…` entry under Permissions… when the app manages services — unit tests in the menu mapping
- [x] 4.4 Red → Green: Services tab — `Tab::Services` (`server.rack`), the `[services]` fields moved there, the Models row (count, Download models…, Show in Finder); the settings field tests updated
- [x] 4.5 `main.rs` wiring: the models gate, the window, the menu action, the Services tab button; `just test` and commit

## 5. Docs and version

- [x] 5.1 `README.md` (§ First launch, § Models, § Privacy), `docs/project-map.md`, `docs/architecture/overview.md`, `docs/architecture/context.md`
- [x] 5.2 Bump `version` in `app/Cargo.toml` to 0.34.0; `just test` and commit
