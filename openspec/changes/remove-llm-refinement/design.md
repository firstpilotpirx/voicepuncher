## Context

Text refinement runs between speech recognition and the paste: `refine_or_raw` in `app/src/core/application/text_refiner.rs` strips filler sounds, sends the rest to a `TextRefiner` (the OpenAI-compatible `llama-server` adapter, or `PassthroughTextRefiner` when refinement is off, the default), checks the answer with `check_normalization` (with `respelling`) and falls back to the raw transcript. Around it: the `[refinement]` configuration section and `services.refinement_copies`, the `--llm`/`--no-llm` flags, the Refinement settings tab, the overlay's "Refining text" stage, the `PastedUnrefined` failure notice and the REFINE part, the archive entry's refinement fields, the `Refinement` local service and model of the catalog, the `llama-server` part of the service scripts (`dev-tools/src/services/`), the `llama` engine of the bundle (`app/packaging/engines.lock`, the `github-release` source of `dev-tools/src/engines/`), and the tests and stages that exercise all of it (`text_refinement.feature`, the refinement evaluation, the 20-run series, `e2e-smoke-refinement`, the refinement service stub, `bench.py`).

## Goals / Non-Goals

**Goals:**
- A dictation pastes its transcript without filler sounds, exactly what refinement-off pastes today; nothing else touches the text.
- No code, test, stage, recipe, engine, setting, port or document of text refinement is left (archived OpenSpec changes excepted).
- An old configuration file with refinement settings keeps loading.

**Non-Goals:**
- The speech recognition services, the command spotting service, recognition accuracy.
- Deleting the user's downloaded Qwen model file.

## Decisions

1. **The paste text.** The port `TextRefiner`, `refine_or_raw`, `RefinedText` and both adapters go. A domain function `text_to_paste(raw: &RawTranscript) -> Option<CleanedText>` (in `filler_sounds.rs`) returns the transcript without filler sounds, or the transcript as returned when nothing but filler sounds was said — the behaviour of refinement-off today. `CleanedText` stays as the name of the text to paste (it is cleaned of filler sounds); `normalization_check.rs` and `respelling.rs` go, used by refinement only.
2. **Old configuration files.** The loader drops the top-level `refinement` and `llm` tables and `services.refinement_copies` from the parsed TOML before deserializing, logging one warning naming them as retired and ignored; `deny_unknown_fields` stays for everything else. The settings window lists them nowhere; its in-place save leaves them in the file untouched (the "retired ones it only tolerates" clause of "Every option has a control"). The default configuration file no longer writes them.
3. **Command line.** `--llm` and `--no-llm` go; clap refuses them as unknown options with exit code 2.
4. **Overlay and notices.** `VisualIndicator::show_refining_text`, the sparkles stage, `cfg.shows_refinement`, `FailureNotice::PastedUnrefined` and `FailingPart::TextRefinement` (REFINE) go. The `stop→paste` log line loses its `refine=` stage.
5. **Archive.** The entry drops its refinement outcome and refinement model fields and its format version moves up by one; readers of older entries are not in this repository beyond the archive tests.
6. **Services.** `LocalService::Refinement`, `ModelUse::Refinement` and `ModelNeeds::refinement` go; `ModelNeeds` keeps `voice_control`. The service scripts (`dev-tools/src/services/`) drop `llama-server`, its health check and its copies; ports 8080, 28080 and the 27080 copy range are free. `VOICEPUNCHER_LLM_MODEL` and the Qwen download leave the `justfile` and `just models`.
7. **Engines.** The `[llama]` section of `engines.lock`, the `github-release` source kind and its download/verify code in `dev-tools/src/engines/`, and the llama.cpp license go; the "release archive with another checksum" check is replaced by the Homebrew-file checksum check that the staging already performs.
8. **Checks.** The refinement evaluation, the `e2e-smoke-refinement` stage, the 20-run output-limit series, their `stage_inputs` lines (`.config/stages.toml`), nextest limits, analyzer baseline entries and size-baseline entries go; `just analyzers-baseline` and `just size-baseline` drop the entries of deleted code.
9. **Specs.** `text-refinement` loses every requirement; after archive its folder is deleted from `openspec/specs/`, and the Purpose lines of `processing-stages` and `local-services` stop naming refinement.

## Test levels

Most scenarios keep their meaning and their current test; the table names where each is proved after the change. Removed requirements lose their tests with them (tasks 1.6, 2.3, 4.1).

| Spec scenario | Level |
|---|---|
| Each engine runs on its own from the bundle (app-distribution) | unchanged, E2E `app_distribution.feature` |
| The engine folders hold what the services need (app-distribution) | unchanged, E2E `app_distribution.feature` |
| The bundle holds the licenses (app-distribution) | unchanged, E2E `app_distribution.feature` |
| A different Homebrew version is refused (app-distribution) | unchanged, E2E `app_distribution.feature` |
| A Homebrew file with another checksum is refused (app-distribution) | unit `dev-tools/src/engines/tests.rs` (the Homebrew checksum check replaces the release-archive one) |
| The pinned engines are staged (app-distribution) | unchanged, E2E `app_distribution.feature` |
| A bundle copy serves dictation on Metal (app-distribution) | E2E `app_distribution.feature` (`@bundle_engines`, the former dictation-and-refinement scenario without refinement) |
| A user without Homebrew installs the disk image (app-distribution) | unchanged, E2E `app_distribution.feature` |
| A pasted dictation is archived with its audio and texts (dictation-archive) | unchanged, component `dictation_archive.feature` |
| A long dictation keeps the text of every chunk (dictation-archive) | unchanged, component `dictation_archive.feature` |
| Two dictations get two entries (dictation-archive) | unchanged, component `dictation_archive.feature` |
| A voice-started dictation is marked as such (dictation-archive) | unchanged, component `dictation_archive.feature` |
| An entry has no refinement fields (dictation-archive) | unit `archiving/filesystem/entry_file.rs` (Red/Green in 1.5) |
| The notice names the failing service and its remedy (failure-notices) | unchanged, component `failure_notices.feature` |
| Silence is processed without a startup failure (file-transcription) | unchanged, E2E `file_transcription.feature` |
| Benchmark corpus is transcribed within the accuracy bar (file-transcription) | unchanged, E2E `file_transcription.feature` |
| The help lists the developer options (file-transcription) | integration `app/tests/it/cli.rs`, unchanged but for the removed flags |
| A text refinement option is refused (file-transcription) | integration `app/tests/it/cli.rs` (Red/Green in 2.1–2.2) |
| Only the speech recognition service is started (local-services) | unchanged, E2E `local_services.feature` |
| The scripts know no text refinement service (local-services) | unit `dev-tools/src/services/tests.rs` (the scripts list three services) |
| The command spotting service hangs (local-services) | unchanged, E2E `local_services.feature` |
| A busy service is not restarted (local-services) | unchanged, E2E `local_services.feature` |
| A cold launch starts the needed services (local-services) | component `app_runtime.feature` and unit `localservices/catalog/tests.rs` (no refinement service) |
| Voice control needs the command services (local-services) | unchanged, the unit or integration test that covers it today |
| A real launch leaves no service behind at quit (local-services) | unchanged, E2E `local_services.feature` |
| Services kept warm by the service scripts are adopted (local-services) | component `app_runtime.feature`, unit `core/application/local_services/tests.rs` |
| An owned service exits (local-services) | unit `core/application/local_services/supervision_tests.rs` (command spotting instead of refinement) |
| The pause grows while a service keeps exiting (local-services) | unchanged, the unit or integration test that covers it today |
| An owned service that stops answering is restarted (local-services) | unit `core/application/local_services/supervision_tests.rs` |
| An owned speech recognition copy that stops answering is restarted (local-services) | unchanged, E2E `local_services.feature` |
| A service fails five starts (local-services) | unit `core/application/local_services/supervision_tests.rs` |
| Retry after a failure (local-services) | unit `core/application/local_services/supervision_tests.rs` |
| Quit with owned and adopted services (local-services) | unit `core/application/local_services/tests.rs` |
| A service that ignores the request to end is killed (local-services) | unchanged, the unit or integration test that covers it today |
| A quit from the system stops the owned services (local-services) | unchanged, E2E `local_services.feature` |
| The service scripts stop an adopted service (local-services) | unit `core/application/local_services/tests.rs` |
| A started service writes its own log (local-services) | integration `app/tests/it/localservices_process.rs` |
| A bundled engine is logged with its version (local-services) | unchanged, the unit or integration test that covers it today |
| A Homebrew engine is logged with its version (local-services) | unit `localservices/catalog/tests.rs` (whisper-server instead of llama-server) |
| The app and the scripts start the speech recognition service with the same arguments (local-services) | unchanged, the unit or integration test that covers it today |
| A changed argument in only one place is caught (local-services) | unchanged, the unit or integration test that covers it today |
| Of two keyword spotting models the newest is used (local-services) | unchanged, the unit or integration test that covers it today |
| The command model is not on disk (local-services) | unit `localservices/catalog/tests.rs` (the former refinement-model case) |
| Voice control starts two copies of the command spotting service (local-services) | unit `localservices/catalog/tests.rs`, unchanged |
| One configured copy runs on the service's own port (local-services) | unit `localservices/catalog/tests.rs` (command spotting instead of refinement) |
| The copy ports of the services never overlap (local-services) | unchanged, the unit or integration test that covers it today |
| A command spotting service the scripts run is adopted (local-services) | unit `core/application/local_services/tests.rs` (the former refinement adoption case) |
| A hotkey dictation shows speech recognition after the stop (processing-stages) | unchanged, component `processing_stages.feature` |
| A dictation ended by voice shows speech recognition after the stop (processing-stages) | unchanged, component `processing_stages.feature` |
| A dictation transcribed while the user speaks shows speech recognition for its last part (processing-stages) | unchanged, component `processing_stages.feature` |
| A dictation without speech hides the overlay after speech recognition (processing-stages) | unchanged, component `processing_stages.feature` |
| Speech recognition is followed by the paste (processing-stages) | component `processing_stages.feature` (the former refinement-disabled scenario) |
| Text held back for focus shows the wait after speech recognition (processing-stages) | component `processing_stages.feature` |
| Speech recognition shows an ear left of its label and spinner (processing-stages) | unchanged, the unit or integration test that covers it today |
| The stage icon reads in light and dark appearance (processing-stages) | unchanged, the unit or integration test that covers it today |
| A long speech recognition never looks nearly done (processing-stages) | unchanged, the unit or integration test that covers it today |
| Speech recognition shows no transcript (processing-stages) | unchanged, the unit or integration test that covers it today |
| Settings opens the window (settings-window) | unchanged, the unit or integration test that covers it today |
| Settings chosen again reuses the window (settings-window) | unchanged, the unit or integration test that covers it today |
| The window has no Refinement tab (settings-window) | component `settings_window.feature` |
| A new hotkey and sound apply after Save (settings-window) | unchanged, component `settings_window.feature` |
| A setting the running app cannot use is reported (settings-window) | unchanged, component `settings_window.feature` |
| A missing configuration file is created on Save (settings-window) | unchanged, component `settings_window.feature` |
| Comments and order survive a Save (settings-window) | unchanged, component `settings_window.feature` |
| Retired text refinement settings survive a Save (settings-window) | component `settings_window.feature` (the former refinement-examples scenario) |
| A hotkey overridden on the command line (settings-window) | unchanged, component `settings_window.feature` |
| Recording is transcribed by the service (speech-recognition) | unchanged, the unit or integration test that covers it today |
| App starts without a local model file (speech-recognition) | unchanged, E2E `speech_recognition.feature` |
| Silence produces a subtitle credit (speech-recognition) | unchanged, the unit or integration test that covers it today |
| Silence produces a lone "you" (speech-recognition) | unchanged, E2E `speech_recognition.feature` |
| Speech containing "you" is kept (speech-recognition) | unchanged, the unit or integration test that covers it today |
| Hallucination after speech is removed and the speech is kept (speech-recognition) | unchanged, the unit or integration test that covers it today |
| Marker inside speech is removed and the speech is kept (speech-recognition) | unchanged, the unit or integration test that covers it today |
| Service is briefly busy (speech-recognition) | unchanged, the unit or integration test that covers it today |
| Drop reported as an invalid argument (speech-recognition) | unchanged, the unit or integration test that covers it today |
| Service keeps dropping the connection (speech-recognition) | unchanged, the unit or integration test that covers it today |
| Service answers with an error (speech-recognition) | unchanged, the unit or integration test that covers it today |
| A chunk whose copy is killed is transcribed by another copy (speech-recognition) | unchanged, the unit or integration test that covers it today |
| Service hears no speech (speech-recognition) | unchanged, the unit or integration test that covers it today |
| Service closed the idle connection during a long pause (speech-recognition) | unchanged, the unit or integration test that covers it today |
| Filler sounds are dropped from the pasted text (text-insertion) | unit `core/domain/filler_sounds.rs` `text_to_paste` (Red/Green in 1.1–1.2), component `processing_stages.feature` via the pipeline |
| The transcript reaches the paste unchanged otherwise (text-insertion) | unit `core/domain/filler_sounds.rs` `text_to_paste` |
| A lost dictation shows the amber notice with its message (visual-feedback) | unchanged, the unit or integration test that covers it today |
| A failure notice over a white window stays legible (visual-feedback) | unchanged, the unit or integration test that covers it today |
| The text recognized so far shows below the pill (visual-feedback) | unchanged, the unit or integration test that covers it today |
| Ending the recording takes the text window away (visual-feedback) | unchanged, the unit or integration test that covers it today |
| The pill does not change size from the recording to Pasted (visual-feedback) | unchanged, the unit or integration test that covers it today |
| The processing stage reads in English (visual-feedback) | unchanged, the unit or integration test that covers it today |
| The recording shows no label (visual-feedback) | unchanged, the unit or integration test that covers it today |
| A pasted dictation shows the Pasted sign (visual-feedback) | unchanged, component `visual_feedback.feature` |
| Text pasted after a wait for focus shows the Pasted sign (visual-feedback) | unchanged, component `visual_feedback.feature` |
| Text left in the clipboard shows no Pasted sign (visual-feedback) | unchanged, component `visual_feedback.feature` |
| A command right after the paste replaces the Pasted sign (visual-feedback) | unchanged, component `visual_feedback.feature` |

## Risks / Trade-offs

- [The user's configuration has `[refinement] enabled = true`] → it loads, logs one warning, and dictation pastes without refinement — the intended outcome.
- [A script or alias still passes `--no-llm`] → exit 2 naming the option; the README and recipes are updated, and the release notes say so.
- [Accuracy gate measured "after refinement"] → it now measures the transcript; the 30 % bar was already met without refinement in the refinement-off runs.

## Migration Plan

Ship as 0.38.0. Nothing to migrate on disk: old configuration files load, old archive entries stay as they are, the Qwen model file stays in `~/.voicepuncher/models/` until the user removes it.
