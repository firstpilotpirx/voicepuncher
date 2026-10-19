## Why

Today voicepuncher keeps nothing of a dictation once it is pasted: no audio, no raw speech-to-text transcript, no text after LLM refinement. The console log has only stage timings (`chunk 1 | 3.2s audio | transcribed in 0.41s`, `stop→paste: …`); the text itself appears only for dropped whisper hallucinations. `~/.voicepuncher/logs/` holds only the services' own logs. `debug_save_wav` writes `./debug.wav`, overwritten on every dictation, with no text beside it.

The user wants to evaluate the product on real usage later, with an agent: how well speech-to-text recognizes their speech, how much LLM refinement helps or hurts, and whether the LLM is worth using at all. That needs, for every dictation, the original raw audio, the raw speech-to-text transcript and the text after LLM refinement, kept together.

## What Changes

- Every dictation is archived in its own entry of a dictation archive (for example `~/.voicepuncher/history/<timestamp>/`): the original raw audio recording, the raw speech-to-text transcript (per chunk and stitched, as pasted without refinement), and the text after LLM refinement — or a note that refinement was off (`--no-llm`) or fell back to the raw text.
- Each entry also records metadata needed to compare runs: stage timings, the dictation and refinement models, the recording mode (push-to-talk, toggle, voice start), and the app version.
- The archive is on by default (decided by the user), since the goal is data from real usage; a config switch turns it off.
- The archive format is stable and machine-readable, so a future evaluation agent can read every entry without the app.

Current stage: PoC. Target stage: PoC.
Scope: writing one archive entry per dictation, in live and file mode, with audio, raw transcript, refined text and metadata.
Acceptance bar: after a dictation, its entry holds an audio file that plays back the dictated speech, the raw transcript equal to what speech-to-text returned, and the refined text equal to what was pasted; covered by tests at the right levels.
Out of scope: the evaluation agent itself, any UI to browse the archive, uploading the archive anywhere (the app stays offline).

## Capabilities

### New Capabilities

- `dictation-archive`: keeping the audio, raw transcript, refined text and metadata of every dictation for later evaluation.

## Impact

- `app/src/core/application/pipeline.rs` and the recording adapter (audio of the whole dictation), a new secondary adapter writing the archive to the filesystem, `config.rs` (archive settings), `docs/ubiquitous-language.md` (dictation archive, archive entry), README.

## Open points

- Retention and size: audio grows fast; proposed a size or age limit with the oldest entries removed first, values chosen in design.
- Audio format: proposed the 16 kHz mono WAV sent to speech-to-text; whether to also keep the native-rate recording is for design.
- Voice-control commands and dropped dictations (only command words, hallucination dropped, lost chunks): proposed archived too, marked as such, since they show recognition errors.
- Whether `debug_save_wav` is replaced by the archive: proposed yes.
- Privacy: the archive holds everything the user dictated; proposed kept local only, under `~/.voicepuncher/`, never in the repository.

## Release

SemVer: MINOR — new capability.
Target version: 0.22.0.
