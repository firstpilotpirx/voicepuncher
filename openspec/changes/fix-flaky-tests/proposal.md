## Why

A third of check time (294 of 954 minutes, 14–18 Oct) went to failed runs, rerun under the machine-wide lock. Most traced to TDD reds, fixed defects or a stale shared service; seven causes remain, two of them app misbehaviors under load. A test of fine behavior must never fail.

## What Changes

1. A worktree without the corpus reads the main checkout's; a test missing it is gated, not failed.
2. Stub command spotting services set their read timeout after the WebSocket handshake.
3. Tests against a real command spotting service wait for the first command up to its promised bound, not 2 s of quiet, and run one at a time.
4. The app never counts its own busy moments (a long burst of held audio) as the command spotting service going away.
5. The command spotting service carries out each command exactly once however fast confirmations come back: a check due while another is pending waits on the same audio instead of being skipped; E2E audio is paced on a fixed schedule.
6. The chunk seam check counts a repeat only for a word pair in the speech.
7. Dictation never falls back to random sampling, so the same audio gives the same text — kept only if WER on 100 recordings is not worse.

No test is dropped, tagged `@wip` or given more time.

Current stage: PoC. Target stage: PoC.
Scope: the seven causes above.
Acceptance bar: on one unchanged commit, `just full-check` passes 5 times and `just test` 10 times in a row with no rerun; the PR lists each cause and its fix.
Out of scope: faster tests; refinement prompt caching (no failure traced to it).

## Capabilities

### Modified Capabilities

- `voice-control`: a busy app is not a lost service; commands are carried out once whatever the confirmation latency.
- `speech-recognition`: the same audio is transcribed to the same text.

## Impact

Tests, steps, `justfile`, nextest config, the app's command spotting client, the command spotting service, the dictation request.

## Release

SemVer: PATCH — fixes; no new behavior.
Target version: 0.18.1.
