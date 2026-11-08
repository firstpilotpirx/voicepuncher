## Context

The dictation model is named in four places that must agree: the app's model catalog (download URL, sha256, size), the app's service catalog (the file it starts `whisper-server` with), the `justfile`'s model discovery, and the E2E steps that check which model a started service loaded. The decoding flags are one list in `dev-tools/src/services/command_lines.rs`, which the app's `whisper_args` mirrors and the `command_lines` test compares.

## Decisions

### D1 — Full large-v3 at 16 bits, from the same pinned revision

`ggml-large-v3.bin` from `ggerganov/whisper.cpp` at revision `5359861c739e955e79d9a303bcbc70fb988958b1` (the one turbo is pinned to), sha256 `64d182b440b98d5203c4f9bd541544d84c605196c4f7b845dfa11fb23594d1e2`, 3 095 033 483 bytes. The 2026-11-08 comparison measured the 5-bit large-v3 no better on FLEURS (5.9 % WER against 6.2 %) and slower on Metal (RTF 0.18 against 0.14), so the 16-bit file is both the more accurate and the faster one; it costs 3.1 GB of disk and about 3.3 GB of memory per copy. Rejected: large-v3 q5_0 (slower, no gain), turbo q8_0 (no gain over q5_0: 6.3 % WER), GigaAM v3 and Parakeet v3 (they garbled English terms, 7.1 % and 8.5 % WER).

### D2 — Beam search of 5

`-bs 5` joins the whisper-server arguments, in `dev-tools` and the app alike. On the user's 45 dictations it fixed the meaning of several hard clips greedy large-v3 missed, at RTF 0.16 against 0.14, and it stays deterministic, so "the same dictation audio gives the same text" holds. `-bo 1` stays: best-of applies to sampling, which dictation does not use.

### D3 — No other flag changes

`-mc 0` stays: the app already gives a request with a prompt its own context (`max_context` 224, `carry_initial_prompt`). Without a prompt the share of transcripts without punctuation was the same for turbo and large-v3 (8 and 9 of 45), so the default prompt stays empty, as the README explains for English dictation. Three copies stay: about 10 GB of memory on the user's 64 GB machine, and fewer copies would wait on long dictations.

### D4 — Time budgets and the accuracy baseline follow the measurement

Recognition takes about twice as long, so the model stages that transcribe (`e2e-smoke-speech`, `accuracy`, the seams) may pass their budget or their nextest limit. They are rerun once with the new model, and each budget and limit that the run passes is set from that run by the rules of AGENTS.md (budget: measured + 20 %; limit: at least twice the longest run). The accuracy baselines are rewritten with `just accuracy-baseline` only if the rate is better; a worse rate within the margin passes unchanged, beyond it fails the change.

## Risks

- First launch downloads 3.1 GB; the app's download already resumes a part left by an interrupted run.
- A copy's warm-up takes longer, so the app is ready later after start.
- The chunk seam cases were recorded from turbo; they are unit data for the merge logic and stay valid; `e2e-seams-sample` checks the seams on the new model.

## Test levels

| Scenario | Level |
|---|---|
| Dictation is recognized with the large-v3 model by default | E2E `.feature` (`speech_recognition.feature`, `@smoke`) |
| Dictation is recognized with the model the user names | E2E `.feature` (`@regression`), unchanged |
| Catalog entry, service arguments | unit (`model_catalog.rs`, `args.rs` tests, `command_lines` test) |
| A dictation in the installed app | manual run (log line `transcribed`) |
