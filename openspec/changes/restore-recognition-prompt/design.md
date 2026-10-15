# Design

## Context

See proposal.md § Why. How whisper.cpp v1.9.5 (`src/whisper.cpp`, `examples/server/server.cpp`) handles the prompt and the text context:

- whisper-server keeps `no_context = true` for every request (no flag or field changes it), so `whisper_full_with_state` clears the stored text (`prompt_past0`, `prompt_past1`) at the start of every request. Text never crosses requests, whatever `-mc` is.
- The token budget for the prompt and carried text is `max_prompt_ctx = min(n_max_text_ctx, n_text_ctx / 2)` (224 for the large models). `-mc N` sets `n_max_text_ctx`; the request field `max_context` overrides it per request (`-1` keeps the server value).
- Before each 30 s window the decoder prompt is built only when `n_max_text_ctx > 0` **and** the temperature is below 0.5 (`WHISPER_HISTORY_CONDITIONING_TEMP_CUTOFF`). With `-mc 0` it is never built, so the request's `prompt` is dropped.
- Without `carry_initial_prompt`, the prompt is put into the rolling text (`prompt_past1`) and is replaced by the decoded text of the first window for the next window. With `carry_initial_prompt = true` (request field `carry_initial_prompt`, or `--carry-initial-prompt`), the prompt is kept apart (`prompt_past0`, cut to `max_prompt_ctx − 1` tokens) and taken first in every window; the decoded text fills only what is left of the budget: `max_prompt_ctx − len(prompt) − 1` tokens.
- So `-mc 0` today only blocks text carried between windows of one request (a dictation chunk with a pause, which whisper resumes after the last segment, or a chunk over 30 s), and the prompt.

Earlier measurement (`limit-confirmation-fallback` investigation, 165 windows of 2.5 s synthesized Russian speech, confirmation fields, idle whisper-server, `-t 3`):

| variant | p50 | p95 | max | > 3 s |
|---|---|---|---|---|
| today (prompt dropped, fallback step 0.2) | 0.146 | 0.237 | 5.51 | 3 |
| `max_context=64` (prompt used), step 0.2 | 0.173 | 4.05 | 5.74 | 35 |
| step 0 | 0.143 | 0.221 | 1.39 | 0 |
| `max_context=64`, step 0 | 0.134 | 0.192 | 1.54 | 0 |

With the prompt in effect, many more Russian windows fail the first greedy pass and fall back; with the fallback limited they stay fast. The step 0.55 chosen by `limit-confirmation-fallback` was not measured with the prompt: its second pass runs at 0.55, above the cutoff, so it decodes *without* the prompt — which may be what escapes the loop.

## Goals / Non-Goals

**Goals:** the prompt is used in every window of every request that carries one; no decoded text steers another request or window; numbers before and after for commands, confirmation time and WER.

**Non-Goals:** new prompt text; the retry schedule (owned by `limit-confirmation-fallback`); a per-request token limit.

## Decisions

### Send the prompt budget per request, keep `-mc 0` on the server

Requests that carry a prompt send `carry_initial_prompt = true` and `max_context = B`, where B is just above the prompt's length in tokens; requests without a prompt send neither and keep the server's `-mc 0`. With `carry_initial_prompt`, the prompt takes the budget first, and decoded text gets `B − len(prompt) − 1` tokens: with B one more than the prompt, none.

The client does not know the token count. Options, to settle in the first task group by measurement:

1. **A fixed B per request kind.** The command prompt is fixed (about 25 tokens); its B is a constant next to `PROMPT`, checked by a test against the token count whisper-server logs. The dictation prompt is user text: B = a conservative estimate from its length (e.g. bytes / 2, capped at 223), accepting that a short estimate cuts the prompt's head (whisper keeps the last tokens and logs a warning) and a long one lets a few decoded tokens through.
2. **B = 224 (the cap) with `carry_initial_prompt`.** The prompt is always whole, and decoded text from earlier windows of the same chunk fills the rest. Simplest, but brings back in-chunk context, which `-mc 0` was meant to block (it feeds repetition between windows); acceptable only if the seam scenarios (`just e2e-seams`) and WER show no loss.

Preferred: option 1 for confirmations (single window, B is exact); for dictation, option 1 unless measurement shows option 2 no worse.

**Alternatives rejected:**
- *Start whisper-server with `-mc N --carry-initial-prompt`.* One value for all requests, and every request without a prompt would carry decoded text between windows. A per-request field keeps today's behavior where no prompt is set.
- *Prompt without `carry_initial_prompt`.* Only the first window sees the prompt; the second sees decoded text instead. Fails "The prompt holds in every window of a long chunk".

### Measure before and after, on the same setup

Both runs on the branch base and on the change, same models, three copies, after `limit-confirmation-fallback` has landed:

| what | how | bar |
|---|---|---|
| user's commands | `voice_samples_it` (and the `@voice_samples` E2E scenarios) | every command recognized exactly once, as now |
| E2E voice control | `just e2e-quick` voice control scenarios | all pass |
| confirmation time | replay of the 165 Russian windows (a `#[ignore]`d measurement test or an E2E `@resilience` step, not a scratch script) | worst < 2 s idle |
| dictation WER | `just full-check` accuracy on the 100 recordings, with and without a prompt | not worse than before |
| prompt effect | a recording with a domain term (below) | the term appears only with the prompt |

The numbers go into this file's § Results before archive. If the prompt makes commands, time or WER worse for a request kind, that kind keeps the prompt off (no `max_context` field), the spec requirement for it is reworded to say so, and the proposal records why.

### Test levels

| scenario | level |
|---|---|
| speech-recognition: A domain term named in the prompt is recognized | E2E `.feature` (`speech_recognition.feature`), a synthesized sentence with a term whisper misses without the prompt (`just test` runs no service, so effects on recognition are E2E); plus unit tests of the request fields (`request.rs`) |
| speech-recognition: The prompt holds in every window of a long chunk | E2E `.feature` (`speech_recognition.feature`), a > 30 s synthesized chunk with the term after 30 s |
| speech-recognition: A dictation does not echo the previous one | E2E `.feature` (`speech_recognition.feature`), two dictations in a row |
| voice-control: The prompt is in effect for a confirmation | unit test of the multipart body (`confirm.rs`: `prompt`, `carry_initial_prompt`, `max_context`), plus E2E (`voice_control.feature`): with the prompt, the confirmation text of a command clip is the command phrase (read from the command spotting service's log) |
| voice-control: Speech that is no command is not turned into a command by the prompt | E2E `.feature` (`voice_control.feature`), extends the existing short Russian phrases scenario with counting words |

## Risks / Trade-offs

- **The prompt can make whisper write the prompt text** for audio that is unclear or silent (known whisper behavior). Confirmations of noise could then read as a command phrase. Covered by the "no command" scenario and the voice samples; if it shows up, keep the prompt off for confirmations.
- **The prompt changes the text of the 0.55 retry differently** (it runs without the prompt). Confirmations may differ between the two passes; measured, not reasoned about.
- **Token estimate for user prompts** can be off; the cost is a cut prompt head or a few carried tokens, both logged by whisper-server.
- **whisper.cpp upgrades** may change the cutoff or the field names; the unit tests pin the fields we send, the integration test pins the effect.

## Results

To be filled by the implementation: before/after tables for each row of § Measure before and after.
