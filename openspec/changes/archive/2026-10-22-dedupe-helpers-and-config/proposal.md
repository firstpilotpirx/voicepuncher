## Why

The audit of 2026-10-22 found helpers pasted between adapters instead of shared: `request_agent` is verbatim in `app/src/adapters/secondary/textrefinement/openai/mod.rs:131–137` and `transcription/openai/mod.rs:313–319`; `build_window`/`label`/`wrapping`/`push_button` are copied between `modeldownload/macos/mod.rs:285–300, 415–443` and `permissions/macos/mod.rs:255–271, 432–463`; `localservices/catalog/mod.rs:476–551` restates the `justfile`'s service argument lists, so a flag changes in two places; `status_item.rs:36–41` hard-codes menu indices tied to the order in `mapping.rs`; `catalog/mod.rs:449–473` builds a dummy `ServiceSpec` to probe ports; the KWS model is picked lexicographically (`:415–421`) while `newest_vad` picks by mtime; `spec()` carries `#[allow(clippy::too_many_arguments)]`; `probe.rs:22–25` builds a ureq agent per probe; the WebSocket handshake uses only the first resolved address (`client.rs:294–306`); `settings/macos/target.rs:180–228` has 15 one-line selector functions and `tag()` returns `-1` for non-controls; `config.rs:407–417` parses the TOML twice and `:141–220` carries 80 lines of serde default functions that `#[serde(default)]` on the struct would replace; the retired `debug_save_wav` key is still read (`:88–90`); `chunking.rs:1` has a no-op `use log;`.

## What Changes

- One HTTP agent builder in `adapters/secondary/http` used by transcription, refinement, the probes and the model fetcher.
- One AppKit window-building helper module used by the permissions and download windows (and the settings window where it fits).
- The service command lines held equal: the catalog and the `justfile` keep their lists, and a test fails on a drift between them (design).
- Menu items found by identifier, not index; `ServiceSpec` ports probed without a dummy spec; model files picked by one rule; `spec()` takes a value object.
- Config parsed once; defaults by `#[serde(default)]` with a `Default` impl per section; the retired key dropped with its migration message.
- The handshake tries every resolved address; the one-line selectors become a table.

Current stage: PoC. Target stage: PoC.
Scope: the files named above and their integration tests; no behavior change except the handshake and the dropped key.
Acceptance bar: the duplication detector of `adopt-static-analyzers` reports none of the pairs above; the service command drift test exists; every `_it` passes.
Out of scope: the overlay (`harden-overlay-thread-model`), the pipeline (`slim-pipeline-module`).

## Capabilities

### Modified Capabilities

- `local-services`: the app and the service scripts start each service with the same command line, held equal by a test.

## Impact

- `app/src/adapters/secondary/{http,transcription,textrefinement,localservices,modelfetch,commandspotting}/**`, `app/src/adapters/primary/{menubar,permissions,modeldownload,settings}/macos/**`, `app/src/config.rs`, `justfile` (service recipes), the matching `_it.rs` files.

## Open points

- Source of the service command lines: decided in design — a drift test, not the `justfile` calling the app binary, which would need a release build of the app before every service start.

## Release

SemVer: no release — refactor; the handshake and the dropped key change no documented behavior.
Target version: unchanged.
