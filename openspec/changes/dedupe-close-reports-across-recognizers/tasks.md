# Tasks

## 1. One report per command across the recognizers

- [x] 1.1 Red: session test `should_report_a_command_the_recognizers_time_a_second_apart_once` in `command-spotting/src/streaming/session/tests/doubled_reports.rs` (scripted "энтер", then "enter" stamped about a second later); `just test-one` on it fails with two reports
- [x] 1.2 Commit the red test alone (`test: reproduce …`, intentionally red)
- [x] 1.3 Green: hearings per recognizer and reports per command set in `Reports`, the repeat check in `Utterance::report` beside `doubles` (`command-spotting/src/streaming/session/utterance.rs`); `just test-one` on 1.1 passes; `just static`
- [x] 1.4 Guard: session test `should_report_a_command_said_twice_with_a_pause_between_twice` in `doubled_reports.rs` (both recognizers hear both, the English one a second late); prove it fails with the count replaced by "any earlier report of the command", then restore; `just test-one` on it passes
- [x] 1.5 `just test`; commit (`fix: dedupe-close-reports-across-recognizers`)

## 2. The user's voice and the version

- [x] 2.1 `just streaming-bench --recorded` on sessions 1–6: one report per label, no clip lost against the numbers of `fix-missed-dictation-start-stop-commands` task 4.2 (47/53 on its branch); note the result in this file
  Result (2026-11-05, `just streaming-bench --recorded ~/.voicepuncher/voice-samples --verbose`): all right 47/53 (88.7 %), false alarms 4/9, delay median 418 ms p90 911 ms — the same 47/53 as before the change, no clip lost; the 6 wrong clips are the known ones (chain tails, a `cancel` heard before `restart`), none a doubled report.
- [ ] 2.2 Bump `version` in `app/Cargo.toml` to the version `kit version-check dedupe-close-reports-across-recognizers` names (0.46.3 unless the base moved); `just test`; `just check` once; commit (`chore: release 0.46.3`)
- [x] 2.3 Manual checks by the user: none (the decision path of the command spotting service; no hotkey, recording, overlay, paste or permission changed)
