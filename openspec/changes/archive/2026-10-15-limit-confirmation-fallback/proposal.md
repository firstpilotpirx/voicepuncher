# Proposal

## Why

A 2.5 s confirmation that the command spotting service sends to the speech recognition service usually takes 0.1–0.5 s, but twice on 2026-10-14 it took more than 10 s while the user spoke Russian. Release 0.13.0 made voice control survive such a stall; this change removes the stall itself.

The cause was found and measured (see design.md):

- whisper-server 1.9.5 accepts `-nf` (no temperature fallback) but never applies it. Every request still runs the fallback: when the first decoding gets stuck repeating itself or scores low, the same audio is decoded again at temperatures 0.2, 0.4, … 1.0, up to six decodings of up to 220 tokens each.
- Confirmations force English, so Russian speech often gets stuck repeating ("yes, yes, yes…", "1, 2, 3, 4…") and burns those retries. On 165 Russian 2.5 s clips, the worst took 5.5 s on an idle service and 5.7 s with three copies and the text refinement service busy. With the VAD time and heavier load, that reaches the >10 s seen in the logs.
- Retrying with randomness never turns Russian speech into an English command, so most of those seconds are spent for nothing, while the next command waits behind them.

## What Changes

- **Confirmations are decoded once.** The command spotting service asks the speech recognition service for no retry of a confirmation (temperature step 0), instead of the five retries up to 1.0 that whisper-server runs now. Measured on the same clips: worst case 1.4 s idle. A single retry at 0.55 was planned first, but on the E2E scenario both its attempts got stuck repeating and took 2.9–3.0 s, at the 3 s deadline; with no retry the same speech took 1.4–1.6 s.
- **Correct descriptions of `-nf`.** The justfile comment on the whisper-server command line, the README start command and the hint for the outdated `temperature_inc` setting stop claiming that `-nf` turns the fallback off. `-nf` stays on the command line, with a note that whisper-server 1.9.5 ignores it.
- **Tests:** a unit test that the confirmation request asks for no retry; an E2E scenario where Russian speech that makes speech recognition repeat itself is answered within the confirmation deadline, and a command said right after it is carried out.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `local-services`: a confirmation of speech that is no command is answered well within its deadline, so it never delays the commands after it.

## Scope

- Current stage: PoC. Target stage: PoC, with no confirmation stalls caused by the speech recognition service's retries.
- In scope: the confirmation request of the command spotting service; the descriptions of `-nf` and `temperature_inc`; the tests above.
- Acceptance bar:
  - On an idle speech recognition service, every confirmation of the Russian test phrases is answered in under 2 s (now up to 5.5 s).
  - A command said right after such a phrase is reported.
  - `just full-check` passes, including the E2E voice control scenarios and the user's voice samples.

## Out of scope

- Dictation requests from the app: they keep whisper-server's current retries. Whether dictation gains or loses from them is a separate measurement.
- `-mc 0` silently turning off the `prompt` of every request (whisper.cpp skips the initial prompt when the text context is 0); a separate change, since it changes recognition.
- The cost of one decoding that gets stuck repeating up to the 220-token limit (about 1.3 s idle); whisper-server has no per-request token limit.
- The time Silero VAD takes inside whisper-server (up to 0.6 s per confirmation).
- Reporting the `-nf` bug to whisper.cpp upstream.

## Release

PATCH, 0.14.0 → 0.14.1: a bug fix; confirmations behave as specified, only faster.

## Impact

- `command-spotting/src/confirm.rs`: the confirmation request gets the retry step field (0: no retry); unit test of the request body.
- `justfile`: the comment on `whisper_args`.
- `README.md`: the whisper-server start command notes.
- `app/src/config/transcription.rs`: the hint for the outdated `temperature_inc` setting.
- `e2e-test/tests/features/local_services.feature` and its steps: the regression scenario.
- `app/Cargo.toml`: version 0.14.1.
- No manual run: hotkey, recording, overlay, paste and permissions do not change.
