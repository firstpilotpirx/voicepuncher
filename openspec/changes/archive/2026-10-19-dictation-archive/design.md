## Decisions

**D1 — A new driven port, `DictationArchive`.** `core/application/dictation_archive.rs` holds the trait (`keep(&mut self, entry: &ArchiveEntry) -> Result<()>`) and the plain data the pipeline hands it: `ArchiveEntry` (recording start as wall-clock time, `DictationStart` — hotkey held, hotkey toggled, voice, recording file — the audio, chunk texts, raw transcript, pasted text, `RefinementOutcome` — refined, off, fell back — `DictationOutcome` — pasted, left in clipboard, paste failed, speech recognition failed, no text, only command words — and named stage timings). The pipeline gets it through `with_archive(...)`; without it nothing is archived and no existing test changes.

**D2 — The pipeline traces a dictation and closes it at its end.** `start_recording_from` opens a trace (start time, how it started); the stop path fills chunk texts, raw transcript, refinement outcome, timings; every end of a dictation that reached speech recognition closes it with its outcome and hands it to the archive after the paste and after the `stop→paste` log line, so the archive adds nothing to stop→paste time. Cancel, "restart dictation" and too-short presses drop the trace. A failure of `keep` is a `log::warn!`, never a failure notice. File mode (`run_once`) builds the same entry with start "recording file".

**D3 — Audio of a chunked dictation comes from the recorder after the paste.** In live-chunk mode `stop()` returns an empty buffer and chunks overlap, so the whole audio cannot be rebuilt from them. `AudioRecorder` gets `fn recording(&mut self) -> Option<AudioBuffer16k>` (default `None`): the cpal recorder keeps the native-rate samples of the last recording (lead-in and cut included) and resamples them to 16 kHz only when asked. The pipeline asks only when an archive is set, after the paste; non-chunked dictations reuse the buffer `stop()` returned. Native-rate audio is not kept: the 16 kHz audio is what speech recognition heard.

**D4 — Filesystem adapter, one folder per entry.** `adapters/secondary/archiving/filesystem/` writes `<archive>/<UTC start, e.g. 2026-10-19T10-42-05.123Z>/` with `audio.wav` and `entry.json` (`format: 1`, every field of `ArchiveEntry` plus the static facts the adapter was built with: dictation model, refinement model or none, app version). A name already taken gets a `-2` suffix. The entry is written into a `.partial` folder and renamed, so a reader never sees half an entry. UTC is formatted by hand (no new dependency); names sort by time.

**D5 — Pruning by size, oldest first.** After each write the adapter sums the size of entry folders and removes the oldest by name until the total fits `max_size_mb` (default 2048), never the entry just written. One walk of a few thousand small folders costs milliseconds.

**D6 — Configuration.** New `[archive]` section: `enabled` (default true), `dir` (default `~/.voicepuncher/history`), `max_size_mb` (default 2048). CLI: `--archive-dir <DIR>` (names the folder and turns the archive on) and `--no-archive`. File mode archives only with `--archive-dir`, so E2E and benchmark runs never fill or prune the user's archive. The setup wizard writes the section's defaults.

**D7 — `debug_save_wav` is retired.** The archive replaces it: the `--debug-save-wav` flag and the recorder's WAV writer go; the `[audio] debug_save_wav` key is still accepted (configs use `deny_unknown_fields`) and, when true, logs once that the archive replaced it (a startup line in `main.rs`, wiring only, not tested).

## Risks

- Resampling a long chunked dictation after the paste takes up to a few hundred ms, during which the next hotkey press waits; acceptable for a PoC, logged as `archive: … in Nms`.
- The archive holds everything the user dictated in plain files; it stays under the user's home folder and is never sent anywhere.

## Test levels

| Scenario | Level |
|---|---|
| A pasted dictation is archived with its audio and texts | component `dictation_archive.feature` |
| A long dictation keeps the text of every chunk | component |
| Two dictations get two entries | component |
| Refinement turned off | component |
| Refinement fails | component |
| Speech recognition fails | component |
| Only command words were transcribed | component |
| A cancelled dictation leaves no entry | component |
| A dictation dropped by restarting leaves no entry | component |
| A press shorter than the minimum press time leaves no entry | component |
| A voice-started dictation is marked as such | component |
| Archive turned off | E2E (recording file with an archive folder named and the archive turned off) |
| A recording file is archived into a named folder | E2E `dictation_archive.feature` |
| A recording file without a named folder is not archived | E2E |
| The oldest entries go first | adapter integration `archiving_filesystem_it.rs` |
| The archive folder cannot be written | component |

Unit tests: UTC folder name formatting, entry JSON shape, config defaults and CLI overlay, cpal `recording()` resampling.
