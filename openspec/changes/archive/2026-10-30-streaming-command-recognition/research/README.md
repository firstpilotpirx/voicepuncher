# Research: one streaming model for voice commands (2026-10-29/30)

Why this exists: the keyword spotter + Whisper small confirmation missed commands said quietly or with
an accent, often until the 2nd or 3rd try, and doubled command words came back hyphenated or glued
("enter-enter", "enterenter"). The user asked for one always-on streaming recognizer instead of a
detector plus a recognizer, measured on their own recordings. Keep this folder when the models are
swapped: rerun the bench with a new candidate and compare against the tables below.

## Data

- **Voice samples** (`~/.voicepuncher/voice-samples/session-1..5`): 121 positive clips with what was said.
  session-1 has one clip per form (`manifest.jsonl`; its `start`/`end` are positions in the long
  `session-1.wav`, **not** in the clip — latency for these items is not measurable); sessions 2–5 are cut
  from the long wavs at `forms.jsonl` bounds ±0.6 s.
- **Command history** (`~/.voicepuncher/command-history/<id>/{entry.json,audio.wav}`): 413 utterances the
  running service kept. Labelled with Whisper large-v3-turbo (`language=en` and `auto`); an item counts
  ("confident") only when turbo-en agrees with the service's small transcript or with turbo-auto.
  294 confident positives, 101 negatives (no command said).
- **Bias:** history labels come from Whisper, so the history column favours Whisper-family models;
  the voice-samples column has human labels.
- The dataset (`dataset.jsonl`, user speech texts) and the raw per-model results stay on the user's
  machine only: `~/.voicepuncher/research/command-bench/`. Never commit them (private speech).

## Method (`bench/`)

- `build_dataset.py` builds `dataset.jsonl`; `bench.py <candidate>` streams each clip in 0.1 s chunks plus a
  0.8 s silent tail through the model and records the final text, compute time, the stream position at
  which each command word first appeared (`emitted`), and the per-chunk p95 compute.
- `score.py`: command accuracy = the sequence of command words (doubles collapsed, hyphens/glue split,
  project mishearings applied: "star dictation" → start, "presenter" → press enter, "capsule dictation" →
  cancel, bare "star" → start) equals the label. FA = a negative with any command. Delay = emitted − word end.
- `score_union.py`: Russian spellings of the English command words (`keywords_ru` in `common.py`, Cyrillic
  patterns such as `ст[оа]п`, `[эеи]н[тд][еэоа]?р`, fillers `дикт…`, `плиз`, `пресс`, `текст`, `ол`) and the
  pair rule: English result if it has a command, else the Russian one.
- Python 3.12, `sherpa-onnx==2.0.1` (the same version the service links), `vosk`, `soundfile`, `numpy`.
  Machine: M1 Max (8P+2E), CPU only.

## Results

Single models (`results/single_models.txt`; voice = voice samples, hist = confident history):

| model | voice | hist | FA | delay median / p90 s | CPU |
|---|---|---|---|---|---|
| Whisper turbo (non-streaming) | 94% | 100% | 2% | ~0.5 s per call | GPU |
| Whisper small en (current confirmation) | 93% | 97% | 11% | ~0.45 s (0.3 s pause + call) | GPU |
| Vosk small + grammar | 96% | 95% | **36%** | −0.15 / 0.31 | tiny |
| **Nemotron speech streaming en 0.6B 160 ms int8** | 88% | 86% | 6% | 0.27 / 0.77 (mean 0.34) | ~70% of one core |
| NeMo fast-conformer 80 ms int8 | 69% | 70% | 6% | 0.38 / 0.85 | ~11% |
| kroko zipformer en | 67% | 70% | 5% | 0.62 / 1.50 (0.78 without session-1) | ~5% |
| NeMo fast-conformer 480 ms | 68% | 66% | 7% | 0.56 / 1.08 | ~4% |
| zipformer en 70M | 47% | 37% | 7% | 0.57 / 0.91 | ~4% |
| **KWS gigaspeech 3.3M (current spotter)** | 49% | 21% | **58%** | 0.11 / 0.64 | ~2% |
| zipformer en 20M | 6% | 2% | 2% | — | ~2% |

Russian models reading the user's English commands (Cyrillic matcher, all items):

| model | commands | FA | delay median / p90 s | CPU |
|---|---|---|---|---|
| **zipformer small ru (vosk 2025-08-16)** | 82% | 11% | 0.27 / 0.68 | ~7% of one core |
| T-one ru (CTC) | 80% | 11% | 0.69 | ~6% |
| multilingual zipformer (8 langs) | 77% | 11% | 0.23 / 0.73 | ~8% |
| Whisper small ru | 77% | 6% | — | GPU |
| Vosk small ru | 61% | 7% | 0.18 | ~14% |

English + Russian pairs (`results/pairs.txt`):

| pair | commands | FA | delay mean / median / p90 s |
|---|---|---|---|
| **Nemotron + zipformer small ru** | **94%** | 13% | 0.19 / 0.22 / 0.64 |
| kroko + zipformer small ru | 90% | 13% | 0.25 / 0.29 / 0.69 (~12% of a core together) |
| NeMo fc 80 ms + zipformer small ru | 89% | 13% | — |
| Whisper small en + ru | 97% | 13% | non-streaming |

CPU was measured with `getrusage` over the 152 s `session-2.wav`. Behind a voice activity gate the
recognizers run only while the user speaks, so their load falls with the share of silence.

## Findings worth keeping

- The accent matters: the Russian model hears the user's "stop", "enter", "clear" as Cyrillic words
  (`стоп`, `энтер`, `клиар`) that the English model misses; the pair beats every single streaming model.
- Hotwords/contextual biasing on Nemotron (word lists, `modeling_unit=bpe`, scores 2–6) changed nothing;
  token-level hotword lines failed to encode (sherpa split "▁").
- CoreML provider was ~2× slower than CPU for Nemotron; 4 threads were ~30% faster than 2.
- Parakeet unified 0.6B streaming 240 ms returned empty text on most user clips (fine on its own test
  wav; gain and a longer tail did not help) — excluded, not resolved.
- Grammar-constrained Vosk is accurate but fires on anything (36% FA): unusable always-on.
- The current keyword spotter is both the least accurate and the noisiest (58% FA): it only works
  because Whisper confirms behind it.
- A voice gate with a pre-roll buffer adds no end latency: the buffer is replayed into the recognizers
  at speech onset, faster than real time.

## Models (sherpa-onnx `asr-models` release)

| model | archive | bytes | sha256 |
|---|---|---|---|
| Nemotron EN | `sherpa-onnx-nemotron-speech-streaming-en-0.6b-160ms-int8-2026-04-25.tar.bz2` | 463 945 198 | `0ae73a41cd51599dc7cac9ac083d9d35de53d762ca45923505fde47a3751814b` |
| zipformer small RU | `sherpa-onnx-streaming-zipformer-small-ru-vosk-2025-08-16.tar.bz2` | 89 653 224 | `cc2e99ed0c67cae8801170e7b7539b4cac00b716076af86f974bf5b888d9370c` |

Base URL: `https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/`. Other candidates
benched (same release unless noted): `sherpa-onnx-streaming-zipformer-en-kroko-2025-08-06`,
`sherpa-onnx-streaming-zipformer-en-20M-2023-02-17`, `sherpa-onnx-streaming-zipformer-en-2023-06-26`,
`sherpa-onnx-nemo-streaming-fast-conformer-transducer-en-{80,480}ms-int8`,
`sherpa-onnx-nemo-parakeet-unified-en-0.6b-int8-streaming-240ms`,
`sherpa-onnx-streaming-t-one-russian-2025-09-08`,
`sherpa-onnx-streaming-zipformer-ar_en_id_ja_ru_th_vi_zh-2025-02-10`; Vosk
`vosk-model-small-en-us-0.15`, `vosk-model-en-us-0.22-lgraph`, `vosk-model-small-ru-0.22`
(alphacephei.com). Downloaded copies: `~/.voicepuncher/models/research/streaming/`.

## Swapping a model later

1. Download the candidate next to the others; add it to `CANDIDATES` in `bench/bench.py`.
2. Rebuild the dataset (`build_dataset.py`, needs the dictation Whisper copies on 18082–18084) to pick up
   new command history entries — the streaming recognizer's own history now logs each model's text.
3. `bench.py <name>`, then `score.py` and `score_union.py <singles> <en+ru pairs>`; compare with the tables.
4. Point the service at the new folder (`--en-model-dir` / `--ru-model-dir`) and the catalog at its archive.
