# Tasks

## 1. Discard a transcript made only of "you"

- [x] 1.1 Red: add the E2E outline "Silence produces a lone \"you\"" (1/10/60 s, `@smoke`) to `e2e-test/tests/features/speech_recognition.feature` under a new `Rule: Known recognition hallucinations are discarded`, with step definitions in `e2e-test/tests/steps/` (reuse the silence `Given`, add a no-refinement `When` that keeps stdout, and a `Then no text is produced` that checks stdout is empty and the exit is not a startup failure); run it against the service started by `just` and see it fail with "you" in stdout
- [x] 1.2 Red: add unit tests in `transcript_filter.rs` — `should_detect_lone_you` for "you", " You.", "you you", "YOU, you!", "You. - You." — and run `just test`, seeing them fail
- [x] 1.3 Red: add unit test `should_not_filter_speech_containing_you` for "you know", "Thank you", "Hello, how are you doing?" and confirm it passes already (guards against an over-broad fix; it is a regression guard, not the Red test of this behavior)
- [x] 1.4 Green: extend `is_whisper_hallucination` with the whole-transcript "only you" check from design.md; `just test` green and the E2E outline from 1.1 passes
- [x] 1.5 Refactor with every test green; run `just check` and commit (`fix(transcription): discard a transcript made only of "you"`)

## 2. Release

- [x] 2.1 Bump `version` in `app/Cargo.toml` to 0.4.2 and verify `cargo metadata` / `just check` reports the new version; commit (`chore(release): voicepuncher 0.4.2`)
- [x] 2.2 Manual checks by the user: none (hotkey, recording, overlay, paste and permissions are unchanged)
