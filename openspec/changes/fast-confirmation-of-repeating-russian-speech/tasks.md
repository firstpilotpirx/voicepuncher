## 1. Cut a stuck confirmation short

- [ ] 1.1 Red: add `should_cut_a_confirmation_short_when_its_copy_decodes_past_the_budget` to `command-spotting/src/confirm.rs` (fake copy answering after 1.5 s; ends in about 1 s with nothing heard, logs "cut short", the next confirmation reaches the copy at once); run it, see it fail; commit as intentionally red
- [ ] 1.2 Green: give `UtteranceTranscriber` a 1 s decoding budget after the request is sent; on its expiry close the request, log `confirmation cut short`, return an empty hearing (`command-spotting/src/confirm.rs`)
- [ ] 1.3 `just test` and commit

## 2. Prove the scenario stays fast

- [ ] 2.1 Run `just e2e-resilience` 5 times in a row; every confirmation of the Russian speech under 2 s in each run; commit

## 3. Version

- [ ] 3.1 Bump `version` in `app/Cargo.toml` to 0.30.1; `just test`, `just check` and commit
- [ ] 3.2 Manual checks: none
