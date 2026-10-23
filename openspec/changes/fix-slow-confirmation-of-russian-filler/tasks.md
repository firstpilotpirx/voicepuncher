## 1. A confirmation cut short sets no pace

- [ ] 1.1 Red: `confirm/tests/under_load.rs` — after two quick confirmations and one cut short at 1 s, a copy decoding the next for 1.5 s is cut short after about 1 s as well (`just test` fails: it is answered at 1.5 s)
- [ ] 1.2 Green: `settle` in `confirm.rs` stops noting the budget of a confirmation cut short; drop `should_raise_the_budget_after_a_confirmation_cut_short_at_the_floor` from `confirm/pace.rs` and fix the docs of `Pace::note` and D1 of the pace
- [ ] 1.3 Commit: `fix: a confirmation cut short sets no pace`

## 2. An answer stuck repeating sets no pace

- [ ] 2.1 Red: `confirm/tests/under_load.rs` — a scripted copy answers two confirmations after 950 ms, one after 1.6 s with a text of speech stuck repeating, then decodes the next for 2.5 s: that one is cut short after about 1.9 s, not answered at 2.5 s
- [ ] 2.2 Green: `settle` notes an answer only when its text is at most 120 characters (`STUCK_REPEATING_CHARS`)
- [ ] 2.3 Commit: `fix: an answer stuck repeating sets no pace`

## 3. The scenario measures its margin, and the release

- [ ] 3.1 The step "every confirmation of that speech is answered in under 2 seconds" prints the slowest time, the number cut short and every time when it passes (`e2e-test/tests/steps/local_services/confirmations.rs`)
- [ ] 3.2 Run `just e2e-resilience` beside a heavy-stage-like load (the large model decoding on Metal in two loops, 8 busy CPU loops) and fill in design.md § Results with the times printed
- [ ] 3.3 Bump `version` in `app/Cargo.toml` to the next PATCH above `main`
- [ ] 3.4 Commit: `test: measure the Russian-speech confirmations; bump version`
