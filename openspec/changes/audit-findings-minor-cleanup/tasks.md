## 1. Replay start on one clock

- [ ] 1.1 Red: unit test in `websocket/heard.rs` — a capture moment after lost audio maps to its stream date (`dated_at`); run it, see it fail
- [ ] 1.2 Green: add `HeardAudio::dated_at`; `just test`
- [ ] 1.3 Red: unit test in `websocket/mod.rs` (or `heard.rs`) — the replay start after lost audio begins before the loss in stream position; run it, see it fail
- [ ] 1.4 Green: date the loss by stream position in `reach_for_service` before `replay_start`; `just test`; commit

## 2. Small code cleanups

- [ ] 2.1 Remove the duplicated doc line in `visualfeedback/macos/glass.rs`
- [ ] 2.2 Drop the unused `_mtm` parameter of `status_line` in `menubar/macos/status_item.rs` and its callers
- [ ] 2.3 In `settings/macos/build.rs`, pass the placeholder instead of building a widget to match it, and turn the text view `expect` into an error the caller handles
- [ ] 2.4 `just analyzers-baseline` when a ratcheted finding is gone; `just test`; commit

## 3. Addresses through `url`

- [ ] 3.1 Red: unit test in `copy-pool/src/address.rs` — an `https` address without a port takes port 443; run it, see it fail
- [ ] 3.2 Green: parse with `url` in `ServiceAddress::parse`, keeping the path as written; add `url` to `copy-pool/Cargo.toml`; `just test`; commit

## 4. Pinning and identities

- [ ] 4.1 Pin every dependency of `app`, `e2e-test` (and any other manifest) `=x.y.z` at its `Cargo.lock` version; add `rust-version` to `e2e-test/Cargo.toml`
- [ ] 4.2 State the pinning rule in `AGENTS.md`
- [ ] 4.3 Add `.mailmap` mapping both author identities to firstpilotpirx <firstpilotpirx@gmail.com>
- [ ] 4.4 `just test`; commit

## 5. Release

- [ ] 5.1 Bump `version` in `app/Cargo.toml` to 0.37.5; `just check`; commit
- [ ] 5.2 Manual checks: none (no hotkey, recording, overlay behavior, paste or permissions change)
