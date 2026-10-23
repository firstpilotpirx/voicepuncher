## 1. Replay start on one clock

- [x] 1.1 Red: unit test in `websocket/heard.rs` — a capture moment after lost audio maps to its stream date (`dated_at`); run it, see it fail
- [x] 1.2 Green: add `HeardAudio::dated_at`; `just test`
- [x] 1.3 Red: unit test in `websocket/mod.rs` (or `heard.rs`) — the replay start after lost audio begins before the loss in stream position; run it, see it fail
- [x] 1.4 Green: date the loss by stream position in `reach_for_service` before `replay_start`; `just test`; commit

## 2. Small code cleanups

- [x] 2.1 Remove the duplicated doc line in `visualfeedback/macos/glass.rs`
- [x] 2.2 Keep the `_mtm` of `status_line`: it is the main-thread proof, not an unused parameter
- [x] 2.3 In `settings/macos/build.rs`, pass the placeholder instead of building a widget to match it, and replace the text view `expect` with a text view of the field's own
- [x] 2.4 Drop the gone `expect_used` entry from `.config/analyzers/clippy.toml`; `just test`; commit

## 3. Addresses through `url`

- [x] 3.1 Red: unit test in `copy-pool/src/address.rs` — an `https` address without a port takes port 443; run it, see it fail
- [x] 3.2 Green: parse with `url` in `ServiceAddress::parse`, keeping the path as written; add `url` to `copy-pool/Cargo.toml`; `just test`; commit

## 4. Pinning and identities

- [x] 4.1 Pin every dependency of `app`, `e2e-test` (and any other manifest) `=x.y.z` at its `Cargo.lock` version; add `rust-version` to `e2e-test/Cargo.toml`
- [x] 4.2 State the pinning rule in `AGENTS.md`
- [x] 4.3 Add `.mailmap` mapping both author identities to firstpilotpirx <firstpilotpirx@gmail.com>
- [x] 4.4 `just test`; commit

## 5. Release

- [x] 5.1 Bump `version` in `app/Cargo.toml` to 0.37.7; `just check`; commit
- [x] 5.2 Manual checks: none (no hotkey, recording, overlay behavior, paste or permissions change)
