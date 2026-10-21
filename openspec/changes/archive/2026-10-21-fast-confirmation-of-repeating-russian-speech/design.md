## Context

Measured on 2026-10-21 against one `whisper-server` 1.9.5 copy with the small model and the flags of `start-model-servers`, with Russian `say` speech forced to English: an ordinary confirmation is answered in 50–270 ms with 1–12 tokens; a clip that gets stuck repeating ("E-e-e-e-…") decodes 219 tokens, the most whisper decodes in one window, and takes 1.69–1.82 s. That is the slow tail of the failing scenario (1.66–2.00 s): neither a busy copy nor a retry, but the full run of a repetition loop. Which clips get stuck depends on the exact cut of the audio, so the failure comes and goes.

`whisper-server` 1.9.5 takes no per-request token limit (`max_tokens` is not read from a request), so the loop cannot be stopped by a field. It does stop decoding when its client closes the connection ("client disconnected, aborted processing"): a request cut at 0.5 or 0.8 s freed the copy at once, and the next request was answered in 0.17–0.23 s.

## Decisions

### Cut a confirmation short after 1 second of decoding

The client of the speech recognition service for commands (`UtteranceTranscriber` in `command-spotting/src/confirm.rs`) waits at most 1 second for a copy's answer once the request is sent (the decoding budget); the wait for a free copy stays inside the existing deadline. When the budget runs out first, it closes the connection, so the copy stops decoding and is free at once, logs `confirmation cut short after <ms>ms on copy <port>`, and returns an empty hearing: the utterance is no command and, being answered, is not tried again. When the deadline runs out first, nothing changes: the confirmation is given up as before.

1 second is four times the slowest ordinary answer measured (270 ms) and well under the 1.7 s of a repetition loop, so a confirmation ends in at most about 1 second and the 2 s bar keeps a margin of half its size. A command phrase is a few tokens; whisper never needs a second to decode it on an idle copy, and a repetition loop is never a command.

Alternatives rejected:
- A per-request token limit: not supported by `whisper-server` 1.9.5; a patched build adds a binary to maintain.
- `audio_ctx` sized to the clip: answers twice as fast on average, but made a clip that was fine get stuck and did not stop a loop.
- Language detection instead of forcing English: Russian would not loop, but an accented English command could be heard as another language.
- Temperature fallback: retries a loop and takes longer, which is why it was turned off.

### The E2E scenario keeps its bar

A cut-short confirmation is logged as answered (in about 1 s) by the worker, so `every confirmation of that speech is answered in under 2 seconds` reads it as any other answer; its steps stay as they are, and the scenario must pass 5 `just e2e-resilience` runs in a row.

## Test levels

| Scenario | Level |
|---|---|
| Russian speech that makes speech recognition repeat itself | E2E `.feature` (`local_services.feature`, existing) |
| A command right after such speech is carried out | E2E `.feature` (`local_services.feature`, existing) |
| A confirmation still decoding after 1 second is cut short | unit (`command-spotting/src/confirm.rs`, fake copy that answers after 1.5 s) |

## Risks

- A copy slowed by a heavy dictation beside it may need over 1 second for a real command and cut it short; measured answers stay under 0.3 s idle, and a cut-short command is logged, so a lost one shows in the log.
