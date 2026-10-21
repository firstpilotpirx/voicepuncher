## Why

The E2E scenario "Russian speech that makes speech recognition repeat itself" (`e2e-test/tests/features/local_services.feature`, `@serial @resilience`, steps in `e2e-test/tests/steps/local_services/confirmations.rs`) requires every confirmation of Russian counting and filler words to be answered in under 2 seconds. It failed twice:

- on `chore/speed-up-tests`, 2026-10-20 17:13 (`just e2e-resilience`);
- on `fix/match-late-confirmation-to-its-state`, 2026-10-21 01:30 (`just full-check`): one confirmation took 2.002 s; the others took 108 ms to 1.78 s, several near 1.7 s (1.697 s, 1.662 s, 1.779 s, 1.657 s).

Most confirmations of that speech take about 0.1–0.25 s, but a few take 1.7–2 s, right at the bar, so the scenario fails now and then and blocks merges. A confirmation that slow also delays a command said right after such speech.

## What Changes

- Every confirmation of Russian speech that makes the speech recognition service for commands repeat itself is answered well under 2 seconds, with margin, in every run.
- A confirmation the speech recognition service for commands has not answered within 1 second of being sent is cut short: the utterance counts as no command, and the copy is free for the next confirmation at once.

Current stage: PoC. Target stage: PoC.
Scope: why some confirmations of Russian counting and filler words take 1.7–2 s in the command spotting service and the speech recognition service for commands (whisper repeating itself on Russian filler, its token limit, the copies behind HAProxy), and making them fast.
Acceptance bar: the scenario passes 5 `just e2e-resilience` runs in a row with every confirmation well under 2 s; the 2 s bar stays as it is.
Out of scope: loosening the 2 s bar; the dictation path; other resilience scenarios.

## Capabilities

### Modified Capabilities

- `local-services`: confirmations of speech that makes speech recognition repeat itself stay fast.

## Impact

- `command-spotting/` (confirmation requests), the speech recognition service for commands and its start recipe in `justfile`
- `e2e-test/tests/features/local_services.feature`, `e2e-test/tests/steps/local_services/confirmations.rs`

## Release

SemVer: PATCH — bug fix in confirmation latency.
Target version: 0.30.1.
