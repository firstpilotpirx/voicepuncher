## 1. Read each late answer with the phrases of its state and of now

- [x] 1.1 Red: add to `command-spotting/tests/placed_commands_it.rs` a test for "A start confirmed after the dictation began stays a start" (standby phrases, hearing delayed 2 s and completing cut-off words in `command-spotting/tests/scripted_hearing/mod.rs`, the session moved to the dictating phrases in the pause after "start dictation"); see it fail with "restart dictation"
- [x] 1.2 `just test`; commit the red test alone (`test: reproduce a late start confirmation read as a restart`, intentionally red)
- [ ] 1.3 Red: unit tests in `command-spotting/src/matcher.rs` for a matcher combining two phrase sets (exact phrase of either set wins, the first set's phrase on a tie); see them fail
- [ ] 1.4 Green: combine the matchers in `command-spotting/src/matcher.rs`; keep the matcher each job was asked under and read every answer with the combined one in `command-spotting/src/engine.rs` and `command-spotting/src/engine/confirming.rs`; the tests of 1.1 and 1.3 pass
- [ ] 1.5 Red: add to `placed_commands_it.rs` a test for "A start said right after a slowly confirmed stop stays a start" (dictating phrases, hearing delayed 2.5 s, standby once the stop is reported); see it fail with the job's phrases alone, then pass with the combined ones
- [ ] 1.6 Refactor: `just test`; commit (`fix: read a late confirmation with the phrases of its state`)

## 2. Version

- [ ] 2.1 Bump `version` in `app/Cargo.toml` to 0.24.3; `just test`; `just check` once; commit (`chore: bump version to 0.24.3`)
- [ ] 2.2 Manual checks by the user: none (no hotkey, recording, overlay, paste or permission change)
