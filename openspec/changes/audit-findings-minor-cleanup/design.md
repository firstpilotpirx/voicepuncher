## Context

The audit of 2026-10-22 named minor sites; this change fixes those still in the code. Two named sites are gone on `main` (the dead `channels` field, the change-folder citations in `confirm.rs`), one is not a defect (the focus wait checks the focus once at once and sleeps only between re-checks), and the two-clock comparison now lives in the command spotting adapter.

## Decisions

### The replay start is dated by stream position

The adapter dates every report by stream position (audio start plus samples streamed); `HeardAudio` maps such a date to its capture time and back only where a report time leaves the adapter. The replay start after an outage compared the wall-clock moment of the loss with the stream-dated end of the last command and read the result as a stream date. Once audio was lost earlier, stream dates fall behind the clock, so the replay started that far after the intended point and could miss the speech said before the loss. The loss moment is now mapped to its stream date (`HeardAudio::dated_at`, the inverse of `captured_at`) before the comparison. Chosen over mapping command ends to capture time, because `replay_from` and `heard_since` read stream dates.

### `url` parses service addresses

`ServiceAddress::parse` takes scheme, host and port from `url::Url`; the port falls back to the scheme's known default, then 80. The path stays as written: `url` writes `/` for an address with no path, which would double the slash where callers append a path, so a root path counts only when the address ends with `/`. The public API of `copy-pool` is unchanged.

### Exact pins everywhere

Every dependency in every manifest is pinned `=x.y.z` at the version `Cargo.lock` holds now, so `cargo update` alone never moves one; `AGENTS.md` states the rule. `e2e-test` gets the workspace's `rust-version`.

## Test levels

| Scenario | Level |
|---|---|
| "stop dictation" said during a short outage ends the dictation | E2E `.feature` (existing, `@resilience`) |
| Speech sent again after an outage begins before the loss when microphone audio was lost earlier | unit (`heard.rs`, `link.rs` replay start) |

The address parsing keeps its unit tests in `address.rs`; the settings, overlay and menu bar edits change no behavior and are covered by `just test` and the build.
