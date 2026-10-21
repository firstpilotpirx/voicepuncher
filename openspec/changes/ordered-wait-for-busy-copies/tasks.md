## 1. A request keeps its place while it waits for a copy

- [ ] 1.1 Red: `should_transcribe_the_first_of_two_waiting_requests_first_when_its_wait_ran_out` in `app/tests/transcription_openai_it.rs`; run it, see it fail; commit it alone as intentionally red
- [ ] 1.2 Red: unit test in `copy-pool/src/pool/tests.rs` that a place kept after a wait ran out is served before a later request
- [ ] 1.3 Green: a queue place in `copy-pool/src/pool.rs` and `copy-pool/src/pool/state.rs`, kept across waits until a copy is leased or it is dropped
- [ ] 1.4 Green: the transcription client in `app/src/adapters/secondary/transcription/openai/mod.rs` holds one place across its attempts and asks again at once after a wait for a copy ran out
- [ ] 1.5 `just test`; commit

## 2. The scenario checks the start order

- [ ] 2.1 Distinct audio lengths for the two waiting requests and per-request start times from the copies' logs in `e2e-test/tests/steps/local_services/copies.rs`
- [ ] 2.2 Replace the answer-order check with the start-order check in the same steps
- [ ] 2.3 `just test`; commit

## 3. Version and manual checks

- [ ] 3.1 Bump `version` in `app/Cargo.toml` to 0.30.1
- [ ] 3.2 `just test`; `just check`; commit
- [ ] 3.3 Acceptance: the scenario passes 5 `just e2e-smoke` runs in a row
- [ ] 3.4 Manual checks: none
