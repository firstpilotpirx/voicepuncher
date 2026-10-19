# Tasks

## 1. Adapter: the outage that lasts 5 s

- [ ] 1.1 Red: unit test in `commandspotting/websocket/link.rs` — an outage is long from 5 s after `lost_at` (equal to the replay reach), not before; run it and see it fail
- [ ] 1.2 Green: `Outage` reports when it became long, once per outage
- [ ] 1.3 Red: in `app/tests/commandspotting_reconnect_it.rs` — a stub away for more than 5 s yields one `SpottingStillAway` between `SpottingLost` and `SpottingBack`; a stub back within 5 s yields none; run them and see them fail
- [ ] 1.4 Green: `PipelineInput::SpottingStillAway`; the websocket worker sends it once when the outage becomes long; 1.3 passes
- [ ] 1.5 `just test` and commit

## 2. Overlay: the "commands unavailable" sign

- [ ] 2.1 Red: unit tests in `visualfeedback/macos/mod.rs` for the label composition — recording with the flag reads `◉ REC ⊘ CMD`, a processing stage reads its icon then `⊘ CMD`, hidden with the flag reads `⊘ CMD` alone, without the flag nothing changes; run them and see them fail
- [ ] 2.2 Green: `VisualIndicator::show_commands_unavailable` and `hide_commands_unavailable`; the macOS adapter keeps the flag beside `OverlayState`, composes the label and keeps the sign shown on `hide()`; no-op defaults; the pointer sign decorator forwards both; the voice-control test double records them
- [ ] 2.3 `just test` and commit

## 3. Core: silence on loss and return, the sign on a long outage

- [ ] 3.1 Red: in `app/tests/features/component/voice_control.feature` change "The app keeps reaching for the command spotting service when it goes away" and "Commands work again after the command spotting service restarts" to expect no sound, and add "A short outage shows nothing", "A long outage in standby shows the sign until the service is back", "A long outage during a dictation shows the sign beside the recording sign" and "A dictation that ends during a long outage keeps the sign", with steps that send `SpottingStillAway` and read the overlay double; run them and see them fail
- [ ] 3.2 Green: `lose_spotting` and `regain_spotting` play no sound; `SpottingStillAway` shows the sign once while listening and away; `SpottingBack` hides it when shown; update the unit tests in `pipeline/voice_control.rs` that expected the sounds; 3.1 passes
- [ ] 3.3 Update the "Away" entry and add "Commands-unavailable sign" to `docs/ubiquitous-language.md`, with the `VisualIndicator` entry
- [ ] 3.4 `just test` and commit

## 4. E2E: outages without sounds

- [ ] 4.1 Red: in `e2e-test/tests/features/voice_control.feature` change "Commands work again after the command spotting service restarts" to expect no listening sound after start and add "A long outage in standby shows the sign until the service is back" (`@serial @resilience`); the hands-free harness records `SpottingLost`, `SpottingBack` and the overlay double's sign calls instead of the listening sounds; run them and see them fail
- [ ] 4.2 Green: the outage steps in `e2e-test/tests/steps/voice_control/outages.rs` detect loss and return from the recorded inputs; 4.1 passes against the code of groups 1–3
- [ ] 4.3 `just test` and commit

## 5. Release

- [ ] 5.1 Bump `version` in `app/Cargo.toml` to the next minor version above `main` (0.21.0 at the time of writing)
- [ ] 5.2 `just check` once and commit
- [ ] 5.3 Manual run by the user: with `just start-model-servers` running, `cargo run --release -p voicepuncher`; stop the command spotting service for 10 s in standby and during a dictation — `⊘ CMD` appears, is readable, takes no focus and goes away when the service is back; a restart within 5 s shows nothing and plays no sound
