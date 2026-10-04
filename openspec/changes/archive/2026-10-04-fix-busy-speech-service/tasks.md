# Tasks

## 1. Reproduce the lost recording

- [x] 1.1 Red: in `app/tests/transcription_openai_it.rs`, add a stub that resets the first N connections (closes them without reading the body) and counts attempts; add `should_retry_when_service_drops_connection` (N = 2, transcript returned, 3 attempts). Run `just test`, see it fail with `Connection reset by peer`; commit on its own as `test: reproduce recording lost on dropped connection (intentionally red)`.

## 2. Retry dropped connections

- [x] 2.1 Green: in `app/src/adapters/secondary/transcription/openai/mod.rs`, retry the request up to 3 times after 200/400/800 ms when the error is a dropped connection (design § Decisions 2), logging a `warn` per retry; `should_retry_when_service_drops_connection` passes with `just test`.
- [x] 2.2 Red → Green: add `should_give_up_after_four_attempts_when_service_keeps_dropping` (stub resets every connection: error, 4 attempts) and `should_not_retry_error_answer` (HTTP 500: error, 1 attempt); see the first fail before 2.1 is complete or by temporarily reverting the retry count, then pass with `just test`.
- [x] 2.3 Unit tests for the drop classification (`ConnectionReset`, `BrokenPipe`, `UnexpectedEof` retried; `TimedOut` and status errors not); `just test` passes.
- [x] 2.4 `just check`; commit `fix(transcription): retry when the speech recognition service drops the connection`.

## 3. Keep E2E from saturating the shared service

- [x] 3.1 Set `max_concurrent_scenarios(4)` in `e2e-test/tests/cucumber_e2e.rs`; `just check` passes; commit `test(e2e): run at most 4 scenarios at a time`.

## 4. Release

- [x] 4.1 Bump `version` in `app/Cargo.toml` to 0.3.2; `just check`; commit `chore: release 0.3.2`.
- [x] 4.2 Manual checks by the user: none (hotkey, recording, overlay, paste and permissions are unchanged).
