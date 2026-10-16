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

(The plan before the measurements; § Confirmations keep the prompt off and § Dictation: option 2 below settle it.)

Requests that carry a prompt send `carry_initial_prompt = true` and `max_context = B`, where B is just above the prompt's length in tokens; requests without a prompt send neither and keep the server's `-mc 0`. With `carry_initial_prompt`, the prompt takes the budget first, and decoded text gets `B − len(prompt) − 1` tokens: with B one more than the prompt, none.

The client does not know the token count. Options, to settle in the first task group by measurement:

1. **A fixed B per request kind.** The command prompt is fixed (about 25 tokens); its B is a constant next to `PROMPT`, checked by a test against the token count whisper-server logs. The dictation prompt is user text: B = a conservative estimate from its length (e.g. bytes / 2, capped at 223), accepting that a short estimate cuts the prompt's head (whisper keeps the last tokens and logs a warning) and a long one lets a few decoded tokens through.
2. **B = 224 (the cap) with `carry_initial_prompt`.** The prompt is always whole, and decoded text from earlier windows of the same chunk fills the rest. Simplest, but brings back in-chunk context, which `-mc 0` was meant to block (it feeds repetition between windows); acceptable only if the seam scenarios (`just e2e-seams`) and WER show no loss.

Preferred: option 1 for confirmations (single window, B is exact); for dictation, option 1 unless measurement shows option 2 no worse.

**Alternatives rejected:**
- *Start whisper-server with `-mc N --carry-initial-prompt`.* One value for all requests, and every request without a prompt would carry decoded text between windows. A per-request field keeps today's behavior where no prompt is set.
- *Prompt without `carry_initial_prompt`.* Only the first window sees the prompt; the second sees decoded text instead. Fails "The prompt holds in every window of a long chunk".

### Confirmations keep the prompt off

Measured (§ Results): with the command prompt in effect (`max_context` 27, `carry_initial_prompt`), command phrases are written exactly as in the prompt, but Russian speech that is no command becomes one ("четыре" → "clear all text.", "пять" → "press enter.", "очисти" → "clear all text."), the user's recorded speech gives four wrong commands instead of one, and p95 of a confirmation of Russian speech grows from 0.16 s to 1.40 s. Commands get worse, so confirmations keep the prompt off; the request no longer sends the prompt that `-mc 0` dropped anyway, and the voice-control requirement says so. The command prompt measured 26 tokens.

### Dictation: option 2

A dictation request with a prompt sends `carry_initial_prompt = true` and `max_context = 224`: the prompt first in every window, the text decoded in earlier windows of the same chunk in the rest. The measurements (§ Results) settle it by the rule above — option 2 is no worse: options 1 and 2 give the same text on all 100 recordings and on long speech, and only option 2 writes the term in a window after the first ("кубернетис" at 54 s of a 108 s chunk: "Kubernetes" with 224, "Кубернетис" with the prompt alone). Requests without a prompt send neither field and keep `-mc 0`.

whisper.cpp 1.9.5 clears the prompt and all earlier text before the last piece of a request when less than 5 s of audio is left (`seek + 500 >= seek_end` in `whisper_full_with_state`): a term in the last few seconds of a chunk is decoded without the prompt, whatever the request sends. The spec states this exception, and the long-chunk scenario says the term with speech after it.

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
| speech-recognition: The prompt holds in every window of a long chunk | E2E `.feature` (`speech_recognition.feature`), a > 30 s synthesized chunk with the term after 30 s and more speech after it |
| speech-recognition: A dictation does not echo the previous one | E2E `.feature` (`speech_recognition.feature`), two dictations in a row |
| voice-control: A confirmation carries no prompt | unit test of the multipart body (`confirm.rs`: no `prompt`, `carry_initial_prompt` or `max_context`) |
| voice-control: Speech that is no command is not turned into a command | E2E `.feature` (`voice_control.feature`), the short Russian phrases and Russian counting words |

## Risks / Trade-offs

- **The prompt can make whisper write the prompt text** for audio that is unclear or silent (known whisper behavior). Confirmations of noise could then read as a command phrase. Covered by the "no command" scenario and the voice samples; if it shows up, keep the prompt off for confirmations.
- **The prompt changes the text of the 0.55 retry differently** (it runs without the prompt). Confirmations may differ between the two passes; measured, not reasoned about.
- **A prompt longer than 223 tokens** loses its first tokens (whisper-server logs it); dictation sends `max_context` 224, so no estimate of the prompt's length is needed.
- **whisper.cpp upgrades** may change the cutoff or the field names; the unit tests pin the fields we send, the integration test pins the effect.

## Results

Measured on 2026-10-17, `ggml-small.bin`, whisper-server 1.9.5 with the justfile flags; services of the branch on ports of their own (9280–9282, three copies), so the shared services on 8080–8082 were never used.

### Before (branch base, the prompt dropped)

| what | result |
|---|---|
| user's commands | `voice_forms_it`: 81 of 82 recorded forms; session-2 at 27.23 s, "restart" while dictating, is heard as "Let's start" (red on `main` since 2026-10-16) |
| E2E voice control (`@voice_control and @smoke`) | 96 of 98 pass; the two failures are the new "prompt is in effect" scenarios (Red) |
| confirmation time, 166 windows of Russian speech (`confirmation_time`) | p50 0.143 s, p95 0.164 s, max 0.190 s, none over 2 s |
| dictation WER, 100 recordings (50 ru, 50 en), raw transcripts, no prompt | 9.47 % (ru 12.03 %, en 6.91 %) |
| the same with the prompt sent today (dropped by `-mc 0`) | identical to no prompt: the prompt has no effect |
| prompt effect, "кубернетис" in Russian speech | "Кубернетис" without the prompt and with it |

Token counts whisper-server logs for prompts (`initial prompt is too long (N tokens)`): the command prompt 26; "Docker, API" 4; "Docker, Kubernetes, GitHub, PostgreSQL." 11; "Kubernetes, Grafana, Terraform, Kafka." 12; Russian term lists of 88 and 153 bytes 23 and 46; a mixed Russian/English sentence of 157 bytes 33; a list of rare English names of 91 bytes 39. Every prompt measured has at least 2.3 bytes per token.

### Confirmations with the command prompt in effect (`max_context` 27, `carry_initial_prompt`)

| what | before | with the prompt |
|---|---|---|
| user's commands, `voice_forms_it` | 1 of 82 wrong (session-2 27.23 s "restart") | 4 of 82 wrong: session-2 27.23 s now right, but "okay and so" (session-3 8.89 s) and Russian speech about the console (session-5 52.59 s) give "cancel dictation", and two forms get an extra "restart dictation" |
| user's commands, `voice_samples_it` | (not reached: the run stopped at `voice_forms_it`) | an unexpected "start dictation" at session-1 15.61 s |
| E2E voice control, 98 scenarios | 96 pass (the 2 failures are the Red prompt scenarios) | 93 pass: short Russian phrases and Russian counting words give "clear all text", a chain gives "clear all text" twice, and the speech right after "start" loses its first words twice |
| confirmation time, 166 windows | p50 0.143, p95 0.164, max 0.190 s | p50 0.136, p95 1.397, max 1.697 s |
| raw confirmations of single words (Eddy / Milena) | "press enter" → "Reset.", "clear text" → "the next.", "cancel" → "console" | each command phrase exactly as in the prompt; "четыре" → "clear all text.", "пять" → "press enter.", "очисти" → "clear all text." |

Confirmations keep the prompt off (§ Decisions).

### Dictation: option 1 or 2

Prompt "Docker, Kubernetes, GitHub, PostgreSQL." (11 tokens), raw transcripts:

| variant | WER, 100 recordings | long speech > 30 s in one request (ru / en / ru with pauses / en with pauses) |
|---|---|---|
| no prompt | 9.47 % | 10.3 / 2.1 / 9.5 / 3.5 % |
| option 1 (`max_context` 16, `carry_initial_prompt`) | 9.74 % | 12.1 / 2.1 / 12.1 / 3.5 % |
| option 2 (`max_context` 224, `carry_initial_prompt`) | 9.74 % | 12.1 / 2.1 / 12.1 / 3.5 % |
| `max_context` 224 without `carry_initial_prompt` | 9.74 % | 12.1 / 2.1 / 12.1 / 3.5 % |

Options 1 and 2 give the same text on every recording: text carried between windows of a request changes nothing measurable here.

The term in a later window of one request (Russian speech of 50 s, "…сервис в кубернетис.", then 50 s more; prompt "Kubernetes, Grafana, Terraform, Kafka.", 12 tokens):

| `max_context`, `carry_initial_prompt` | term in the middle (54 s of 108 s) | term in the last 4 s |
|---|---|---|
| 13, yes (the prompt alone) | Кубернетис | Кубернетис |
| 20, yes | Кубернетис | Кубернетис |
| 64, yes | Kubernetes | Kubernetes |
| 224, yes (option 2) | Kubernetes | Кубернетис |
| 224, no | Kubernetes | Кубернетис |

Option 2 is chosen (§ Decisions). The last 4 s are decoded without any earlier text by whisper.cpp itself. A prompt of English terms costs Russian speech without those terms about 0.5 points (ru 12.03 → 12.53 %, two words of 116 in the long speech); the accuracy bar of `just full-check` sends no prompt and is not affected.
