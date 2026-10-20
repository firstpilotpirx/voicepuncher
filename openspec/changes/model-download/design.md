# Design

## Context

`just models` downloads six files with curl and checks three sha256 values; the services' catalog (`local-services-runtime`) resolves model paths in `[services] models_dir`. `ureq` 2 with TLS is a dependency; `libc` arrives with `local-services-runtime`; `sha2` 0.11.1 is the hashing crate. The app's network policy is loopback only except this download. The windows of `permissions-gate-window` and `settings-window` set the AppKit technique.

## Decisions

**D1 — The catalog is domain data, the download a use case with ports.** `ModelFile` entries (name, URL, sha256, size, purpose, kind) are plain data; `ModelDownload` orchestrates check, fetch, verify, place, extract with `ModelStore`, `ModelFetcher`, `DiskSpace`, `Clock` ports — testable with fakes, independent of ureq and AppKit.

**D2 — One implementation for the window and the terminal.** `--download-models` runs the same use case with a terminal observer, and `just models` calls it, so the catalog cannot drift from the scripts. Alternative: keep curl and compare lists by a test — the fallback named in the proposal's open points.

**D3 — Resume with Range, hash while streaming, atomic placement.** `.part` + Range + streaming sha256 give resumable, verifiable downloads of multi-GB files without reading them twice; the rename makes presence atomic. A `.part` larger than the catalog size is discarded.

**D4 — Sequential downloads, bounded retries.** One file at a time with 3 retries (2, 4, 8 s) and one automatic re-download on checksum mismatch; the schedule lives in the use case and is unit-tested with a fake clock.

**D5 — `/usr/bin/tar` for the archive.** macOS ships bsdtar with bzip2; spawning it avoids two crates for one file; the exit code and the resulting folder are checked.

**D6 — Presence by name and size at launch, full hash on demand.** Hashing 3.3 GB at every launch would cost seconds; the Services tab's `Check files` does it when asked.

**D7 — The runtime gates the services on models, as it gates on permissions.** `WaitingForModels` precedes `StartingServices`; the window is a primary adapter observing the use case and the runtime; `AllPresent` triggers the services start without a relaunch.

## Risks / Trade-offs

- [Upstream files change under the pinned URLs] → the Hugging Face URLs pin a commit; the GitHub release assets are immutable; the catalog's sha256 catches any change.
- [A user on a metered link] → the window states the total before anything downloads; Cancel keeps the `.part`.
- [Redirects dropping the Range header] → the http adapter test covers a redirect to a Range-capable server.
- [Disk fills during the download] → write errors fail the file with `disk full`; the free-space check runs before the start.

## Test levels

| Scenario | Level |
|---|---|
| Catalog matches the `justfile` (until it delegates); sizes and purposes present | unit test in `model_catalog.rs` |
| Check report, retry schedule, mismatch re-download, cancel, AllPresent | unit tests in `model_download.rs` (fake ports, fake clock) |
| Full download, resume from a part, 200-on-resume restart, checksum mismatch, redirect keeps Range | adapter integration `modelfetch_http_it.rs` (local server thread) |
| Part files, atomic rename, extraction, free space | adapter integration `modelstore_fs_it.rs` |
| Missing models listed before download; progress; resume; mismatch; no internet; not enough space; services start when the last file is present | component `model_download.feature` (mock fetcher, fake store, runtime with mocks) |
| Headless `--download-models` exit codes and output | CLI integration test against the local server |
| Models folder change in the Services tab re-checks | component `settings_window.feature` (new scenario) |
| Status texts, sizes, rates, ETA | unit tests in `modeldownload/macos/mapping.rs` |
| First launch on an empty folder; cancel and resume; `just models` | manual run by the user |
