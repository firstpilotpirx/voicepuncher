## Why

The audit of 2026-10-22 left minor findings that fit no other change and should not be lost: `text_insertion.rs:41–83` sleeps `poll_interval` before the first focus check, adding ≈ 100 ms to every paste that could pass at once; `voice_control.rs:139–149` compares `Instant`s captured on two clocks (recorder and spotter stream); `runtime/mod.rs:253–261` exits from a non-main thread so `ServiceRuntime::drop` never runs (also covered by the pipeline fix); `settings/macos/build.rs:154` builds a widget to match a variant and `:173` `expect`s a view; `cpal/mod.rs:58` `#[allow(dead_code)] channels`; `glass.rs:373–374` duplicated doc lines; `status_item.rs:109` unused `_mtm`; `confirm.rs:14–18` cites OpenSpec design documents that move to the archive; `address.rs` parses URLs by hand instead of the `url` crate already in the tree; the `lint` stage passes clippy with no `pedantic`; dependency pinning differs by crate (`app/Cargo.toml` mixes `"1"` and `=x.y.z`; `command-spotting` and `copy-pool` pin everything; `e2e-test` has no `rust-version`); two git author identities.

## What Changes

- The first focus check runs at once, the poll interval applies between checks.
- Outage dedupe uses one clock (the capture time mapped as the spotter already maps it).
- The dead `channels` field, the duplicated doc lines, the unused parameter and the widget-to-match go; the `expect` on the view becomes a typed error.
- Comments cite the archived path or the spec, never a change folder.
- `address.rs` uses `url`.
- One pinning rule for the workspace (`exact-versions.md`), stated in `AGENTS.md`, applied to every manifest; `rust-version` in every crate.
- `.mailmap` joins the two author identities.

Current stage: PoC. Target stage: PoC.
Scope: the sites named; no other behavior.
Acceptance bar: each site changed as stated; `just test` and the `@smoke` E2E pass; the paste timing in the log shows the focus check without the leading sleep.
Out of scope: anything another audit change names.

## Capabilities

### Modified Capabilities

- `text-insertion`: a focused element is pasted into without the leading wait.

## Impact

- `app/src/core/application/text_insertion.rs`, `pipeline/voice_control.rs`, `app/src/adapters/**` sites named, `copy-pool/src/address.rs`, all `Cargo.toml`, `.mailmap`, `AGENTS.md`.

## Open points

- Pinning rule: exact (`=x.y.z`) everywhere with `Cargo.lock` as the second lock, or caret with the lockfile; proposed: exact everywhere, as two crates already do and `exact-versions.md` asks.

## Release

SemVer: PATCH — the paste starts up to 100 ms sooner.
Target version: 0.36.4 (next PATCH above `main` at merge; `kit version-check` corrects it).
