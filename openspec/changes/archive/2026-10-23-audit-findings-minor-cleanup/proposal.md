## Why

The audit of 2026-10-22 left minor findings that fit no other change and should not be lost: `text_insertion.rs:41–83` sleeps `poll_interval` before the first focus check, adding ≈ 100 ms to every paste that could pass at once; `voice_control.rs:139–149` compares `Instant`s captured on two clocks (recorder and spotter stream); `runtime/mod.rs:253–261` exits from a non-main thread so `ServiceRuntime::drop` never runs (also covered by the pipeline fix); `settings/macos/build.rs:154` builds a widget to match a variant and `:173` `expect`s a view; `cpal/mod.rs:58` `#[allow(dead_code)] channels`; `glass.rs:373–374` duplicated doc lines; `status_item.rs:109` unused `_mtm`; `confirm.rs:14–18` cites OpenSpec design documents that move to the archive; `address.rs` parses URLs by hand instead of the `url` crate already in the tree; the `lint` stage passes clippy with no `pedantic`; dependency pinning differs by crate (`app/Cargo.toml` mixes `"1"` and `=x.y.z`; `command-spotting` and `copy-pool` pin everything; `e2e-test` has no `rust-version`); two git author identities.

## What Changes

- After an outage of the command spotting service, the replay start is computed on one clock: the moment of the loss is dated by stream position, as the ends of carried-out commands already are, so audio lost before the outage no longer shifts the replay past the speech said just before the loss. (The audit's `voice_control.rs` site moved into the spotter adapter; the audit's leading focus-check sleep in `text_insertion.rs` is not in the code — the first check already runs at once — so it is dropped.)
- The duplicated doc line in `glass.rs` and the widget built only to be matched in `settings/macos/build.rs` go; the `expect` on the text view there gives way to a text view of the field's own when AppKit holds none (an error path would raise the cyclomatic ratchet of `control`). The `_mtm` of `status_line` stays: it proves the call runs on the main thread. (The dead `channels` field and the change-folder citations in `confirm.rs` are already gone.)
- `copy-pool/src/address.rs` parses addresses with the `url` crate.
- One pinning rule for the workspace: every dependency exact (`=x.y.z`) with `Cargo.lock` as the second lock, stated in `AGENTS.md` and applied to every manifest; `rust-version` in every crate.
- `.mailmap` joins the author identities under firstpilotpirx <firstpilotpirx@gmail.com>.

Current stage: PoC. Target stage: PoC.
Scope: the sites named; no other behavior.
Acceptance bar: each site changed as stated; `just check` passes; a unit test shows the replay after lost audio starts where the speech before the loss is.
Out of scope: anything another audit change names; clippy `pedantic` in `lint` (the clippy ratchet already enables it); the non-main-thread exit in `runtime/mod.rs` (left to `slim-pipeline-module`).

## Capabilities

### Modified Capabilities

- `voice-control`: a command said just before the command spotting service went away still counts when audio was lost earlier.

## Impact

- `app/src/adapters/secondary/commandspotting/websocket/{mod.rs,heard.rs}`, `visualfeedback/macos/glass.rs`, `settings/macos/build.rs`, `copy-pool/src/address.rs`, every `Cargo.toml`, `.mailmap`, `AGENTS.md`.

## Release

SemVer: PATCH — a command said before a command spotting outage is no longer missed after lost audio.
Target version: 0.37.7 (next PATCH above `main` at merge; `kit version-check` corrects it).
