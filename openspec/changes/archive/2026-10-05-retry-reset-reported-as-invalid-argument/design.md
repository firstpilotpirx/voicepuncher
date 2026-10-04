# Design

## Context

The stub in `transcription_openai_it.rs` resets a connection by closing it with the request body unread, so the OS sends RST. Depending on when the RST reaches the client, `ureq` 2 sees `BrokenPipe` (while writing the body), `ConnectionReset` (while reading the status line) or, on macOS, `EINVAL` (`Invalid argument`, os error 22) while preparing to read the status line — a socket call on a connection the peer already reset fails with `EINVAL` on Darwin. `is_dropped_io` knows the first two but not the third, so that attempt is not retried and the recording is lost. A busy `whisper-server` resets connections the same way, so the bug is in the app, not only in the test.

## Goals / Non-Goals

**Goals:** every reset the OS can report is retried; the integration tests are stable.

**Non-Goals:** changing the retry schedule; replacing `ureq`; changing the stub.

## Decisions

1. **Classify by OS error code, not only by `io::ErrorKind`.** `is_dropped_io` takes the `io::Error` and also returns true for an OS error whose kind is `InvalidInput` (`EINVAL`). An `InvalidInput` error built by a library has no OS code and stays not retried, so a malformed request is not sent 4 times. Alternative — adding `InvalidInput` to the kind list — would also retry library-raised invalid-input errors; rejected.
2. **Look at every I/O error in the chain.** `ureq` rewraps an error from reading the status line in a new `io::Error` with the same kind but no OS code, keeping the original as its source. `is_dropped_connection` therefore walks the whole source chain and returns true when any I/O error in it is a drop, instead of judging only the first one.
3. **No change to the stub.** The stub reproduces what a busy service does; making it gentler would hide the bug.

## Test levels

| Spec scenario | Level |
|---|---|
| Drop reported as an invalid argument | unit (`is_dropped_io` with `io::Error::from_raw_os_error(EINVAL)`; `is_dropped_connection` with that error rewrapped as `ureq` does), plus the adapter integration tests below run 200 times |
| Service is briefly busy | adapter integration (`should_retry_when_service_drops_connection`) |
| Service keeps dropping the connection | adapter integration (`should_give_up_after_four_attempts_when_service_keeps_dropping`) |
| Service answers with an error | adapter integration (`should_not_retry_error_answer`) |

The reset timing cannot be forced from the test, so the deterministic Red test is the unit test; the 200-run loop shows the integration tests are no longer flaky.

## Risks / Trade-offs

- [An `EINVAL` with another cause is retried] → at most 3 extra attempts within 1.4 s, then the same failure as before.
