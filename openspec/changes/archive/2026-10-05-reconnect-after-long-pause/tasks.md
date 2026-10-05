# Tasks

## 1. Open a new connection for every request

- [x] 1.1 Red: add `libc = "=0.2.186"` to `[dev-dependencies]` of `app/Cargo.toml`; in `app/tests/transcription_openai_it.rs` add a stub that answers on a kept-open connection and resets it on demand, and the test `should_transcribe_on_first_attempt_after_service_reset_idle_connection` (transcribe, reset, transcribe again; both return the transcript and no dropped attempt is logged); in `app/tests/textrefinement_openai_it.rs` the same stub and `should_refine_after_service_reset_idle_connection`. Run `just test`, see both fail; commit on its own as `test: reproduce request on a reset idle connection (intentionally red)`.
- [x] 1.2 Green: build the request agents of `OpenAiTranscriber` and `OpenAiTextRefiner` with `max_idle_connections(0)` and `max_idle_connections_per_host(0)` (design § Decisions 1); `just test` passes.
- [x] 1.3 `just check`; commit `fix(services): open a new connection for every request`.

## 2. Release

- [x] 2.1 Bump `version` in `app/Cargo.toml` to 0.5.3; `just check`; commit `chore: release 0.5.3`.
- [x] 2.2 Manual checks by the user: none (hotkey, recording, overlay, paste and permissions are unchanged).
