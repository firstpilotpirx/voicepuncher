# Design

## Context

See proposal.md § Why. The measurements behind it were taken outside the repo with a replay script that sends 2.5 s windows of synthesized Russian speech (`say -v Milena`, 85 s, 165 windows, step 0.5 s) to a whisper-server 1.9.5 on a free port, started with the justfile flags (`-bo 1 -nf -sns -mc 0 --vad -vsd 2000 -vp 500 -nlp`, `-t 3` like one copy), with exactly the fields of the confirmation request (`language=en`, the command prompt, `temperature=0`, `verbose_json`, word timestamps).

How whisper.cpp v1.9.5 decodes a request:

- `examples/server/server.cpp` parses `-nf` into `params.no_fallback` and never reads it; `temperature_inc` defaults to 0.20 and only the request field `temperature_inc` changes it. No command line flag does.
- `whisper_full_with_state` builds the temperature list `t, t + inc, …` while `t ≤ 1.0` (inclusive), and decodes the window again at the next temperature when the best decoder failed: it hit the token limit (220) in a repetition loop, or its average log probability is below `logprob_thold` (−1.0).
- The response does not show which temperature produced the text, so the retries are invisible to the client.

Measured worst cases (seconds), same 165 windows:

| run | now (inc 0.2) | inc 0 | inc 0.55 |
|---|---|---|---|
| idle | 5.51 | 1.39 | 1.93 |
| 3 copies, two decoding 27 s chunks | 4.84 | 1.82 | 1.47 |
| 3 copies + llama-server refining | 5.73 | 2.79 | 2.12 |
| user's 40 command clips, p95 | 0.178 | 0.138 | 0.151 |

Variants that did not help: `response_format=json`, `no_timestamps`, `language=ru` (7.8 s), language auto (7.8 s), 10 threads, a single copy. A cold start was ruled out (first 27 s chunk after start 2.4 s, after 200 s idle 2.7 s).

## Goals / Non-Goals

**Goals:** bound the time one confirmation can take, keep one retry for clips that get stuck repeating.

**Non-Goals:** dictation requests; recognition accuracy of commands beyond keeping today's results; a token limit per request.

## Decisions

### Send `temperature_inc=0.55` with every confirmation

The confirmation request gets one more multipart field, `temperature_inc` = `0.55`, next to `temperature` = `0`. The schedule is then 0 and 0.55: 0.55 + 0.55 = 1.1 is above 1.0.

- **Why 0.55, not 0.5:** the schedule includes 1.0, so a step of 0.5 gives 0, 0.5 and 1.0 — three attempts, the last one nearly random.
- **Why not 0 (no retry):** it is as fast idle (1.4 s) but slower under load (2.8 s against 2.1 s), because a clip stuck repeating then always runs to the 220-token limit and returns that loop as text. The 0.55 attempt usually escapes the loop early and returns a short phrase. The user also wants a retry kept for unclear words.
- **Why not 0.4 or 0.6:** 0.4 gives three attempts (0, 0.4, 0.8); 0.6 behaves like 0.55 with a more random second attempt.
- **Why per request, not on the server:** whisper-server 1.9.5 has no flag for it, and the dictation requests must keep today's schedule (proposal § Out of scope).

### Keep `-nf` and correct what is written about it

`-nf` stays on the whisper-server command line, so a whisper-server that honours it later keeps dictation without fallback as intended. The justfile comment, the README start command and the hint for the outdated `[transcription] temperature_inc` setting state that whisper-server 1.9.5 ignores `-nf` and that the confirmation request sets its own retry step. The hint stops promising that `-nf` disables the fallback.

### Test the request at unit level, the behavior at E2E level

- Unit (`command-spotting/src/confirm.rs`): the multipart body holds `temperature_inc` = `0.55`, next to the existing field assertions. This is the Red test of the fix.
- E2E (`e2e-test/tests/features/local_services.feature`, `@resilience`): the two spec scenarios. They run on services of their own ports like the other `@resilience` scenarios, because the shared services on 8080–8082 may run an older build. Steps stream the synthesized Russian phrases (`say -v Milena`, cached like every E2E phrase) and then "press enter" to the command spotting service; the first scenario reads the confirmation times the service logs (`answered in <n>ms`) for that speech and asserts each is under 2 s; the second asserts "press enter" is reported.
| Scenario (local-services) | Test level |
|---|---|
| Russian speech that makes speech recognition repeat itself | E2E |
| A command right after such speech is carried out | E2E |

- The E2E Red run must show the stall on today's request: on the measured phrases it takes 5.3–5.5 s. If a phrase does not reproduce it with the cached synthesis, pick phrases from the measured set ("раз, два, три, четыре…", "эээ, подожди, сейчас, сейчас, да, да, да, вот так") until one does.

## Risks / Trade-offs

- [The 0.55 attempt samples randomly, so its text can differ between runs] → it is used only for clips the first attempt failed on; on the user's 40 command clips one text changed ("Start. Activation." → "Start actuation."), and the E2E and `voice_samples_it` runs in `just full-check` show whether command recognition holds.
- [A 2 s bound on a GPU shared with other runs can be flaky] → the scenario requires the speech recognition service to be otherwise idle, runs under the machine lock, and the margin is wide (fixed ≤ 1.93 s measured, today ≥ 5.3 s); if it proves flaky, raise the bound toward the 3 s confirmation deadline rather than drop the scenario.
- [A later whisper-server could change the schedule or start honouring `-nf`] → the field is explicit and the unit test pins it; honouring `-nf` would only affect dictation.

## Migration Plan

None: the field is added to the request; no setting, protocol or data changes. Rolling back is reverting the commit.
