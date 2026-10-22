## Why

The E2E scenario "Service hits its output limit" (`e2e-test/tests/features/text_refinement.feature:70`, `@smoke`, stage `e2e-smoke-refinement` of `just full-check`) failed once on 2026-10-22 19:39 on `chore/slim-pipeline-module` (commit 042189be) with "expected one request to the text refinement service, got 0": the app sent no chat request to the refinement stub (`e2e-test/tests/steps/refinement_service_stub.rs`), although the sibling scenarios of the same rule ("Service stops between recordings", the translation and the correction ones) passed in the same run. Beside it ran the `full-check` of `fix/fix-load-sensitive-confirm-tests` (commit 6ace76a0) in its light phase (voice control stages), so the machine was under load. After the step was reordered to check the app's exit status and stderr before the request count (commit 4f2fe596 on `chore/slim-pipeline-module`), `just rerun-failed` passed the scenario at 19:42 on an otherwise idle machine, so the first failure is a flake whose cause is unknown: the app either lost the transcription under load (a speech recognition timeout ends file mode before any refinement request) or the stub missed the request (its `read_request` drops a connection it cannot parse without recording it). The stage journal (`~/.cache/voicepuncher/stage-times.jsonl`) shows no earlier failure of this scenario.

## What Changes

- The scenario fails with the app's own reason when it fails: the step keeps reporting the exit status and stderr first (already on `chore/slim-pipeline-module`), and the stub records every connection it drops, so a missing request is told apart from a lost transcription.
- The cause found in the next failure's output is fixed in the app, the stub or the scenario, without widening a budget or a timeout.
- Acceptance bar: the scenario passes 20 times in a row while `just e2e-resilience` runs beside it.

Current stage: PoC. Target stage: PoC.
Scope: the output-limit scenario, the refinement service stub and, when the cause lies there, the file-mode path of the app that it exercises.
Acceptance bar: the scenario passes 20 times in a row beside `just e2e-resilience`; no bar loosened.
Out of scope: the other refinement scenarios; the speech recognition timeouts.

## Capabilities

### Modified Capabilities

- `text-refinement`: no requirement changes; its output-limit scenario becomes deterministic under load.

## Impact

- `e2e-test/tests/steps/text_refinement/mod.rs`, `e2e-test/tests/steps/refinement_service_stub.rs`; possibly `app/src/main.rs` (`run_file_mode`) or `app/src/adapters/secondary/textrefinement/openai/`.
- `just e2e-smoke`, `just full-check` (stage `e2e-smoke-refinement`).

## Open points

- Whether the cause is the stub or the app is unknown until the scenario fails again with the app's output; proposed: first make both sides report (stub drop log), then fix what the next failure names.
- Target version: proposed no release, as the fix is in tests only unless the app's file-mode path proves at fault (then PATCH).

## Release

SemVer: no release — a test fix unless the app proves at fault (then PATCH).
Target version: 0.37.3.
