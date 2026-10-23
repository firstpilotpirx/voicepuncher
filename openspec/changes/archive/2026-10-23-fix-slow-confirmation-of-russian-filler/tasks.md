## 1. A confirmation cut short sets no pace

- [x] 1.1 Red: `confirm/tests/under_load.rs` — after two quick confirmations and one cut short at 1 s, a copy decoding the next ones for 2.5 s cuts each of them short after about 1.5 s (`just test` fails: the old pace doubles the budget at every cut)
- [x] 1.2 Green: `Pace::note_cut_short` (`confirm/pace.rs`) gives a budget of 1.5 s however many confirmations were cut short, and `settle` in `confirm.rs` calls it instead of noting the budget it cut at; `should_raise_the_budget_after_a_confirmation_cut_short_at_the_floor` gives way to a test of the fixed budget
- [x] 1.3 Commit: `fix: a confirmation cut short sets no pace`

## 2. An answer stuck repeating sets no pace

- [x] 2.1 Red: `confirm/tests/under_load.rs` — a scripted copy answers two confirmations after 950 ms, one after 1.6 s with a text of speech stuck repeating, then decodes the next for 2.5 s: that one is cut short after about 1.9 s, not answered at 2.5 s
- [x] 2.2 Green: `Pace::note_answer` notes an answer only when its text is at most 120 characters (`STUCK_REPEATING_CHARS` in `confirm/pace.rs`), and `settle` calls it
- [x] 2.3 Commit: `fix: an answer stuck repeating sets no pace`

## 3. The scenario measures its margin, and the release

- [x] 3.1 The step "every confirmation of that speech is answered in under 2 seconds" prints the slowest time, the number cut short and every time when it passes (`e2e-test/tests/steps/local_services/confirmations.rs`)
- [x] 3.2 Fill in design.md § Results with the times the scenario prints in this branch's `full-check` (its `e2e-resilience` stage beside other sessions' heavy stages)
- [x] 3.3 Bump `version` in `app/Cargo.toml` to the next PATCH above `main`
- [x] 3.4 Commit: `test: measure the Russian-speech confirmations; bump version`
