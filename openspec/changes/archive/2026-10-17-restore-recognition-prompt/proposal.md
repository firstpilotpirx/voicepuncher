# Proposal

## Why

Both prompts that are meant to steer speech recognition have no effect today. The command spotting service sends the command phrases as a prompt with every confirmation, and the dictation setting `[transcription] prompt` is sent with every dictation chunk, but whisper-server is started with `-mc 0`, and whisper.cpp v1.9.5 drops the prompt of every request when its text context is 0 (`src/whisper.cpp` ~7285: the prompt is used only when `n_max_text_ctx > 0`). So the spec promise that the prompt biases recognition toward domain terms is broken, and voice control gets no help from the command phrases.

`-mc 0` was set to keep text from one request out of the next. whisper-server already clears that context before every request (`no_context` is always on in v1.9.5), so what `-mc 0` really blocks is text carried from one 30 s window to the next inside the same request — and, by accident, the prompt.

## What Changes

- **The prompt reaches speech recognition.** Every dictation chunk, when `[transcription] prompt` is set, makes whisper-server use it, in every 30 s window of the request. Confirmations were measured with the command prompt in effect and keep it off (see Modified Capabilities).
- **Still no carried-over text between requests.** Text decoded for one request never steers another. Within a chunk sent with a prompt, the text decoded in earlier windows follows the prompt in the next window (design.md option 2: measured no worse, and the prompt alone missed a term in a later window); without a prompt nothing changes.
- **Measured before and after.** Command recognition (the user's voice samples, the E2E voice control scenarios), the time of confirmations of Russian speech, and dictation WER (`just full-check`, 100 recordings) are measured on today's setup and with the prompt in effect. If the prompt makes command recognition or WER worse, or confirmations slower than the bar below, the change keeps the prompt off for that request kind and says so in the specs, rather than pretending it works.
- **Correct descriptions.** The justfile comment on `whisper_args`, the README start command and the doc comment of the `prompt` setting say what the prompt and `-mc` actually do.
- **Tests:** unit tests of the request fields; an integration or E2E check that a prompt changes the transcript of a recording where it should (a domain term in dictation); a unit test that a confirmation carries no prompt.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `speech-recognition`: the configured prompt steers dictation recognition; text of one request never steers another.
- `voice-control`: confirmations carry no prompt. Measured with the command phrases as the prompt, Russian counting words and short Russian phrases became commands, the user's recorded speech gave four wrong commands instead of one and the slowest confirmations took 1.7 s instead of 0.2 s, so the prompt stays off for confirmations (design.md § Results) and the dropped prompt is no longer sent.

## Scope

- Current stage: PoC. Target stage: PoC, with the prompts doing what the configuration and the code say.
- In scope: the request fields of confirmations and dictation chunks, the whisper-server command line, the descriptions above, the measurements and tests above.
- Acceptance bar:
  - A dictation with a configured prompt naming a domain term gives that term in the raw transcript where it is missed without the prompt.
  - Every recorded command of the user is still recognized exactly once (`voice_samples_it`), and the E2E voice control scenarios pass.
  - Confirmations of the Russian test phrases still take under 2 s on an idle service (the bar of `limit-confirmation-fallback`).
  - WER on the 100 recordings of `just full-check` is not worse than before the change.

## Out of scope

- The retry schedule of confirmations (`limit-confirmation-fallback`, which should land first: the prompt is dropped at retry temperatures of 0.5 and above, so the two interact and are measured together).
- A per-request token limit; the time Silero VAD takes.
- A user-editable command vocabulary; new default prompt text for dictation.
- Repairing English terms in Russian dictation (`recognize-english-terms`); this change only makes a prompt usable for that work.

## Release

PATCH (bug fix): target version 0.14.3, the next patch after 0.14.2 on `main`.

## Impact

- `justfile` (`whisper_args`), `README.md` (whisper-server start command).
- `command-spotting/src/confirm.rs`: confirmation request fields (the prompt removed).
- `app/src/adapters/secondary/transcription/openai/request.rs`, `app/src/config/transcription.rs`: dictation request fields, the `prompt` doc comment.
- Tests in `app/src/` and `command-spotting/src/` (unit), `e2e-test/tests/` (E2E and the confirmation-time measurement).
- `app/Cargo.toml`: version.
- No manual run: hotkey, recording, overlay, paste and permissions do not change.
