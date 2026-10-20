# Tasks

## 1. The copy-pool crate

- [ ] 1.1 Add the workspace member `copy-pool/` (package `voicepuncher-copy-pool`, dependency `log`), `just test` builds it
- [ ] 1.2 Red: unit tests in `copy-pool/src/lib.rs` with a fake probe — copies found among the base port and base + 10001…10008; a copy down at its first refused connection; a copy back, or a new one, joins on the next probe; run them and see them fail
- [ ] 1.3 Green: `CopyPool::discover` with the probe thread (1 s, 200 ms connect timeout) and `mark_down`; 1.2 passes
- [ ] 1.4 Red: unit tests — a lease takes a free copy; with every copy busy a lease waits, first come first served, and gives up at its wait with `NoFreeCopy` without holding a copy; never two leases on one copy; run them and see them fail
- [ ] 1.5 Green: `lease`, `Lease::finished`, `Lease::url`; 1.4 passes
- [ ] 1.6 Red: unit tests — a lease still held by an abandoned request keeps its copy busy until released; a copy held past twice its time limit is reported hung and gets no lease until a probe sees it again; the search starts at a per-process offset; `CopyPool::shared` returns one pool per address; run them and see them fail
- [ ] 1.7 Green: `report_hung`, the start offset, the registry behind `shared`, `snapshot`; 1.6 passes
- [ ] 1.8 Add Copy, Copy pool, Lease and Draining copy to `docs/ubiquitous-language.md` and `copy-pool/` to `docs/project-map.md`
- [ ] 1.9 `just test` and commit

## 2. The app's transcription adapter on the pool

- [ ] 2.1 Red: in `app/tests/transcription_openai_it.rs`, with fake copies on the convention ports of a free base — requests from two clients at once go to the two copies; two more wait their turn; a copy closed before a request is skipped and the request answered by the other; every copy refusing fails naming the copies; a copy that never answers the first attempt gets no second, the other copy answers it; run them and see them fail
- [ ] 2.2 Green: `OpenAiTranscriber` takes `CopyPool::shared(base_url)` in `connect` (fails as today when no copy answers), each attempt of `post_with_retries` takes a new lease, a refused connection marks the copy down, a timed-out attempt keeps its copy until the request thread finished reading, a copy past twice the deadline is reported hung, a debug log names the copy of each request; the existing tests of the file and of `http_failure_it.rs` stay green; 2.1 passes
- [ ] 2.3 Add `voicepuncher_copy_pool` to the crates `src/core` may not use in `app/tests/architecture.rs`
- [ ] 2.4 `just test` and commit

## 3. Confirmations of the command spotting service on the pool

- [ ] 3.1 Red: unit tests in `command-spotting/src/confirm.rs` with fake copies — every copy busy past the deadline gives the confirmation up without sending it; a copy that frees before the deadline takes it; run them and see them fail
- [ ] 3.2 Green: `UtteranceTranscriber` takes the shared pool of `--transcription-url`, a lease waits at most the deadline, the request's own timeout is what remains of it, a given-up confirmation is logged; the engine's handling of a timed-out confirmation is kept; 3.1 passes
- [ ] 3.3 `just test` and commit

## 4. Service scripts without HAProxy

- [ ] 4.1 Red: in `e2e-test/tests/features/local_services.feature` rename the rule to "The speech recognition service runs as several copies found from its address", add its scenario "Requests wait their turn when every copy is busy" and the rule "Copies of a speech recognition service are found from its address" with "Three copies are found from the service address" and "A single copy on the service's own port is found", remove the rule "Short speech is transcribed ahead of long speech", rewrite "An abandoned request is not transcribed" and add "The request after an abandoned one goes to another copy"; the steps in `e2e-test/tests/steps/local_services/` stop looking for the load balancer; run `just e2e-smoke` and see the new scenarios fail
- [ ] 4.2 Green: `justfile` `start_whisper` starts the copies on the convention ports with nothing on the base port, one copy on the base port; the "already running" check looks at the copy ports; `stop-model-servers` stops the copies from the list; `haproxy_config`, `require_binary haproxy` and `whisper_binary` go; `start-model-servers` prints the copies and their ports; `copy-pool/` joins `recognition_paths`; 4.1 passes
- [ ] 4.3 README (prerequisites and § Local services without HAProxy: copies, their ports, one copy), `docs/architecture/overview.md`, the `justfile` comment on the port convention
- [ ] 4.4 `just test` and commit

## 5. Release

- [ ] 5.1 Bump `version` in `app/Cargo.toml` to 0.29.0 (next minor above `main`, `kit version-check`)
- [ ] 5.2 `just check` once and commit
- [ ] 5.3 Manual run by the user: `just stop-model-servers && just start-model-servers`, then `cargo run --release -p voicepuncher` with voice control on — a 2-minute dictation pastes every chunk; a voice command during a long chunk is carried out; killing one dictation copy's process during a dictation loses no chunk; `pgrep haproxy` finds nothing; paste the log

## Workflow follow-up

- `just full-check` with WER on 100 recordings (the recognition path changed), seams and resilience suites green, before the merge.
- The agent-kit tech matrix PR (section "macOS Desktop": service copies routed by an in-process pool crate; HAProxy removed).
