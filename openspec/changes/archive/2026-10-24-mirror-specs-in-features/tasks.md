## 1. The mirror check

- [x] 1.1 Red: `app/tests/it/spec_mirror/` `should_mirror_every_requirement_in_a_rule` without the list; it fails naming every requirement without a rule
- [x] 1.2 Rename the reworded rules to their requirement's heading; the shrinking list of the rest (design decision 2)
- [x] 1.3 `just test`; commit

## 2. Push-to-talk

- [x] 2.1 Red: scenarios of the spec delta, of "Nothing is asked in a terminal" and of "The hotkey listener survives every key press" on the fake hotkey listener; Green: their steps; they leave the list
- [x] 2.2 `just test`; commit

## 3. Visual feedback, processing stages, live transcript, dictation pointer

- [x] 3.1 Rules and scenarios for every listed requirement of these capabilities, on the recording fake overlay and the overlay's pure layout
- [x] 3.2 `just test`; commit

## 4. Menu bar, permissions, settings, model download

- [x] 4.1 Rules and scenarios for menu-bar-presence, permissions-gate, settings-window and model-download
- [x] 4.2 `just test`; commit

## 5. Runtime, logging, archive, text insertion, file transcription

- [x] 5.1 Rules and scenarios for app-runtime, app-logging, dictation-archive, text-insertion and file-transcription
- [x] 5.2 `just test`; commit

## 6. Speech recognition and voice control

- [x] 6.1 Rules and scenarios for speech-recognition and voice-control
- [x] 6.2 `just test`; commit

## 7. Local services and app distribution

- [x] 7.1 Rules and scenarios for local-services on the fake service binaries, and for app-distribution
- [x] 7.2 `just test`; commit

## 8. Change workflow

- [x] 8.1 The `dev-tools` cucumber harness and `change_workflow.feature` with a rule per requirement
- [x] 8.2 `just test`; commit

## 9. Close

- [x] 9.1 The list is empty and removed: the check passes for all 21 capabilities
- [x] 9.2 `AGENTS.md` § Gherkin names the mirror check and the `dev-tools` harness
- [x] 9.3 No version bump: tests and spec text only, no behavior change (proposal § Release)
- [x] 9.4 `just test`; commit
