## Context

The app has heard commands only with the streaming recognizer since 0.42.0, but the tests did not follow: `dev-tools` starts the command spotting service with the keyword spotter unless `VOICEPUNCHER_COMMAND_RECOGNIZER=streaming` is set (nobody sets it), the binary defaults to `--recognizer keyword-spotting`, and the E2E harness writes `recognizer = "keyword_spotting"` into the app's configuration in three places (`voice_control/mod.rs`, `local_services/managed.rs`, `app_distribution/bundle_launch/mod.rs`). The audit of 2026-11-02 (`test-audit-2026-11-02.md` in the session notes) sorted every voice test into: keyword spotter only (delete), duplicates (delete), and behavior the product has but tested on the keyword spotter (move).

A trial run on 2026-11-02 with `VOICEPUNCHER_COMMAND_RECOGNIZER=streaming` passed 85 of 92 smoke scenarios and all 34 regression scenarios. The streaming path failed on: a lone synthesized "paste" (the English model hears nothing, the Russian one "эйст"), "start", a 0.5 s pause and a sentence, a sentence, a short breath and "stop stop", a hands-free session whose pasted text kept "Stop saying", two pause rows of the user's quiet recorded "restart", one recorded "press enter", and four app integration tests of keyword spotter events.

## Goals / Non-Goals

**Goals:**
- Every test of behavior the product has runs on the streaming recognizer, and the gaps that move uncovers are fixed here.
- No code, recipe, model, setting, doc or test of the keyword spotter, its Whisper confirmation or the speech recognition service for commands remains.

**Non-Goals:**
- Retuning the streaming models or the voice gate beyond what a moved test needs.
- The copy pool crate and the dictation service (the app's dictation copies keep the pool).

## Decisions

**D1. Move first, then delete.** Group 1 switches `dev-tools` and the harness to the streaming recognizer and fixes what fails; only then do the later groups delete the keyword spotter. Deleting first would leave the user's path with almost no E2E test for a while; moving first shows each gap against a test that used to pass. Alternative — delete and rewrite tests from scratch: loses the coverage the existing scenarios encode.

**D2. The streaming gaps are fixed here, test-first.** Each failing moved scenario is the product's own behavior failing, so it is this change's to fix (Red: the moved scenario or a smaller `command-spotting/tests/it/streaming` test; Green: the smallest fix). Already found: "paste" said alone is heard by the Russian model as "эйст"/"ейст"; it is respelled as "paste" only with its "й", so the Russian word "ест" stays. The start cue runs 0.7–0.8 s for a doubled start and over 5 s for a start followed by speech: the English model decodes on one thread behind real time, and an utterance's close waits for it, holding back the next utterance; the user chose on 2026-11-02 to fix this lag here rather than relax the bounds (`command-spotting/tests/it/streaming` latency tests are its Red). Three bounds stay over after that because the models emit words late, not because of the code: "fine start start" (513 ms), "start" in room noise (855 ms) and "enter" with every core busy (1010 ms); the user chose on 2026-11-02 to relax them to 0.6 s, 0.9 s and 1.2 s rather than retune the models. A gap that needs model retuning rather than a code fix is a `scope` question to the user, never a frozen or `@wip` test.

Fixed along the way, each with its Red test: a recognizer that heard only the end of a quiet "restart" ("start", timed late) no longer reports it after the other heard "restart" (`doubles` ignores the gap for the end of a longer word, and the recognizer that heard the whole word speaks first); the next utterance's replay starts after this one's last speech, so a sentence's last words ("let's start") never reach it; 0.3 s of silence is fed before the replay, so an utterance's first sound is never its stream's first sound (a quiet restart 0.3–0.4 s after a sentence was missed); a one-word command settles only at the pause when the gate heard speech go on after it ("stop the car"); a pause of 0.6 s between words the gate missed in room noise splits the utterance, so "press enter … press enter" is two commands; the Russian model's "хейст" (a 48 kHz microphone) is respelled as "paste"; and a selection naming a phrase that is no command is refused.

**D3. One recognizer, no choice.** `command-spotting` loses `--recognizer`, `--model-dir`, `--transcription-url`, `--max-command-secs`, `--confirmation-deadline-secs` and `--keywords-file`; `--en-model-dir` and `--ru-model-dir` become required. The `Service` enum of `listening` keeps only the streaming engine (or becomes the engine itself). `dev-tools` starts only the streaming line; `VOICEPUNCHER_COMMAND_RECOGNIZER`, `VOICEPUNCHER_KWS_MODEL_DIR`, `VOICEPUNCHER_COMMAND_WHISPER_MODEL`, `VOICEPUNCHER_COMMAND_STT_*` and `VOICEPUNCHER_COMMAND_TRANSCRIPTION_URL` go from `dev-tools`, the `justfile`, `.config/stages.toml` and the docs.

**D4. The protocol drops the keyword spotter's events and moves to version 4.** `ServerEvent::{Phrase, Heard, ConfirmationFailed, Stalled}`, `Timing`, `Verdict` and `PhraseSelection.confirm_commands` go; `PROTOCOL_VERSION` 3 → 4, so an old service is replaced by the existing protocol-version handling, never misread. The app's websocket client, `VoiceCommandRecognizer` (the pause rule over `PhraseSpotted` and segments) and `SpottingEvent::{PhraseSpotted, ConfirmationFailed}` go with them; the app acts on `Command` events as the streaming path already does.

**D5. Retired configuration keys load and are ignored.** The configuration denies unknown keys, so an old file naming `[voice_control] recognizer` or `confirm_commands`, or `[services] command_copies` or `commands_url`, would fail to load. Each stays as a field that accepts any value, is never written back and is read by nothing; every other unknown key still fails. Alternative — drop `deny_unknown_fields`: loses the typo check for every key.

**D6. The service for commands leaves the app.** `ServiceKind::Commands`, its catalog branch, `ggml-small.bin` and the keyword spotting model in the model catalog, the settings switch and its hint, the `confirm=` stage of the voice start timing and the `Stalled` handling in the supervisor go. Scenarios that used the service for commands as their example (adopted, hung, quit, menu) use the dictation service or the command spotting service instead.

**D7. Tests are deleted by the audit's lists.** Group 1 (keyword spotter, confirmation, service for commands, their copies) and group 3 (duplicates) of the audit are deleted; the nextest group `command-spotting-engine`, the filters `engine::`/`placed_commands::`, the test needs `needs_command_stt` and `needs_kws_models` and the `e2e-test` helpers `stalled_copies`, `spotting_service/` and `tests/it/confirmation_time.rs` go with them. The `@cues` checks lose their warm-copies and cut-short reporting, which only the confirmation needed.

## Test levels

| Capability | Scenario | Level |
|---|---|---|
| local-services | The app and the scripts start the speech recognition service with the same arguments | integration (`dev-tools/src/services/tests.rs`, app catalog tests) |
| local-services | The app and the scripts start the streaming recognizer with the same arguments | integration (`dev-tools/src/services/tests.rs`, app catalog tests) |
| local-services | A changed argument in only one place is caught | integration (`dev-tools/src/services/tests.rs`) |
| local-services | The dictation model is not on disk | integration (`dev-tools/src/services/tests.rs`) |
| local-services | Only the speech recognition service is started | E2E |
| local-services | The scripts know no text refinement service | integration (`dev-tools/src/services/tests.rs`) |
| local-services | The command spotting service hangs | E2E |
| local-services | A busy service is not restarted | E2E |
| local-services | A cold launch starts the needed services | unit (`app/src/core/application/local_services/`) |
| local-services | Voice control needs the command services | unit (`app/src/core/application/local_services/`) |
| local-services | A real launch leaves no service behind at quit | E2E |
| local-services | Quit with owned and adopted services | component |
| local-services | A service that ignores the request to end is killed | component |
| local-services | A quit from the system stops the owned services | E2E |
| local-services | The service scripts report a copy ready after its first transcription | E2E |
| local-services | The app counts a started copy ready after its first transcription | unit (`app/src/core/application/local_services/supervision_tests.rs`) |
| local-services | A copy that cannot transcribe in time is not ready | unit (`app/src/core/application/local_services/supervision_tests.rs`) |
| local-services | The check of a ready copy sends it no speech | unit (`app/src/core/application/local_services/supervision_tests.rs`) |
| local-services | A hung adopted copy is killed and replaced | component |
| local-services | Another program on the port is not killed | component |
| menu-bar-presence | The services are listed with their states | component |
| menu-bar-presence | Restart from the menu | component |
| model-download | The streaming recognizer needs the streaming models | component |
| model-download | A file of the wrong size is missing | component |
| model-download | A model named by its variable counts as present | component |
| model-download | Every missing file is downloaded and placed | component |
| model-download | The keyword spotting archive is extracted | integration (app model download tests, streaming archive) |
| speech-recognition | Dictation of a chunk without speech | E2E (`speech_recognition.feature`) |
| speech-recognition | Dictation is recognized with the large-v3-turbo model by default | E2E |
| speech-recognition | Dictation is recognized with the model the user names | E2E |
| voice-control | A pause of the app's own does not lose the service | integration (`app/tests/it/commandspotting_client.rs`) |
| voice-control | A burst of audio after long listening does not lose the service | E2E (`voice_control.feature`, "A command said after a long time of listening is recognized") |
| voice-control | A voice start logs its timing | component |
| voice-control | The timing stays right after microphone audio was lost | component |
| voice-control | A hotkey start logs no voice start timing | component |
| voice-control | A command said right before a sync is reported before the sync is answered | integration (`command-spotting/tests/it/streaming/`) |
| voice-control | Start start followed by the dictation at once gives its cue at once | E2E |
| voice-control | Start followed by the dictation at once gives its cue at once | E2E |
| voice-control | Start said on its own gives its cue at once | E2E |
| voice-control | Restart restart followed by the new dictation at once gives its cue at once | E2E |
| voice-control | Restart said on its own gives its cue at once | E2E |
| voice-control | The user's recorded starts give their cue at once | E2E |
| voice-control | The overlay shows with the start cue | component |
| voice-control | A faster start keeps the dictation where the command's words end | E2E (`voice_control.feature`, `@cues`) |
| voice-control | A faster start adds no false start | E2E (`voice_control.feature`, `@cues`) |
| voice-control | A brief dip inside the command words starts nothing early | E2E (`voice_control.feature`, `@cues`) |
| voice-control | The start cue is timed once the other checks' builds have ended | E2E (`voice_control.feature`, `@cues`) |
| voice-control | A late start cue names what ran beside it | E2E (`voice_control.feature`, `@cues`) |
| voice-control | A command said alone is reported before its pause ends | E2E |
| voice-control | Silence wakes no recognizer | E2E |
| voice-control | Speech without a command is no command | E2E |
| voice-control | A slow recognizer does not hold back the other | unit (`command-spotting/src/streaming/session/tests.rs`) |
| voice-control | A quiet command opens the gate | integration (`command-spotting/tests/it/streaming/`) |

## Risks / Trade-offs

- [The streaming recognizer is weaker on some recorded forms than the keyword spotter with Whisper] → D2: fixed test-first; the user's recordings stay in the smoke set.
- [A large deletion breaks something only a removed test exercised] → delete group by group, `just test` after each; `just full-check` at the end runs every remaining stage.
- [An old service on port 8082 speaks protocol 3] → the app replaces a copy of another protocol version (existing requirement), D4.
- [The user's models folder keeps `ggml-small.bin` and the keyword spotting folder] → harmless; the app neither needs nor deletes them.

## Migration Plan

The release notes say the keyword spotter and the speech recognition service for commands are gone; nothing to do for the user. Rollback: install 0.45.0.
