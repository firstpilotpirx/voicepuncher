# Tasks

## 1. Reproduce the reset reported as an invalid argument

- [x] 1.1 Red: in `app/src/adapters/secondary/transcription/openai/mod.rs`, add the unit test `should_treat_os_invalid_argument_as_dropped` (an `io::Error::from_raw_os_error(22)` is a dropped connection) and `should_not_treat_library_invalid_input_as_dropped` (an `io::Error::new(InvalidInput, …)` is not); change `is_dropped_io` to take `&io::Error` and pass the kind through unchanged, so the first test fails. Run `just test`, see it fail; commit on its own as `test: reproduce reset reported as invalid argument (intentionally red)`.

## 2. Retry it

- [x] 2.1 Green: `is_dropped_io` returns true for an OS error of kind `InvalidInput` (design § Decisions 1); `just test` passes.
- [x] 2.2 Red → Green: add `should_treat_os_invalid_argument_wrapped_by_http_client_as_dropped` (the OS `EINVAL` rewrapped in an `io::Error` without OS code, as `ureq` does); see it fail and commit it on its own (intentionally red); then `is_dropped_connection` checks every I/O error in the source chain (design § Decisions 2); `just test` passes.
- [x] 2.3 Run the `transcription_openai_it` test binary 200 times in a row; every run passes.
- [x] 2.4 `just check`; commit `fix(transcription): retry a reset reported as invalid argument`.

## 3. Release

- [x] 3.1 Bump `version` in `app/Cargo.toml` to 0.4.1; `just check`; commit `chore: release 0.4.1`.
- [x] 3.2 Manual checks by the user: none (hotkey, recording, overlay, paste and permissions are unchanged).
