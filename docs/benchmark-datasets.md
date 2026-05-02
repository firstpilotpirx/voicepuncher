# Speech Benchmark Datasets

> Reference for the N×N pipeline benchmark: which datasets to use, how to download them,
> what transcription format to expect, and how to compute WER/CER.

## Overview

The benchmark runs every combination of (whisper model × LLM correction model) against a fixed
corpus of audio files with ground-truth transcriptions, then produces a WER/CER table.
The datasets below cover the two target languages (Russian, English) and the relevant duration
buckets for voicepuncher: short clips (2–30 s), medium (30 s–3 min), and long (3–10 min).

---

## Dataset Selection

### Comparison table

| Dataset | Lang | License | Registration | Test clips | Avg duration | Speech style | Download |
|---|---|---|---|---|---|---|---|
| **FLEURS** | RU + EN | CC BY 4.0 | No | ~400 / lang | ~9 s | Read (parallel sentences) | HuggingFace API |
| **LibriSpeech test-clean** | EN | CC BY 4.0 | No | 2,620 | ~7–10 s | Read (audiobooks, clean) | Direct URL / HF |
| **LibriSpeech test-other** | EN | CC BY 4.0 | No | 2,939 | ~7–10 s | Read (audiobooks, harder) | Direct URL / HF |
| **GOLOS** (farfield) | RU | Custom (Sber) ¹ | No | 1,916 | ~4–6 s | Commands / far-field | HuggingFace API |
| **GOLOS** (crowd) | RU | Custom (Sber) ¹ | No | 9,994 | ~3–5 s | Read phrases | HuggingFace API |
| **Russian LibriSpeech** | RU | CC BY 4.0 | No | 1,350 | 2–20 s | Read (audiobooks) | HuggingFace API |
| **Common Voice v17** | RU + EN | CC0 1.0 | No (HF mirror) | EN ~16k / RU ~2k | 3–10 s | Scripted + semi-spontaneous | HuggingFace API |
| **TED-LIUM 3** | EN | CC BY-NC-ND 3.0 ² | No | 1,142 | ~7 s | Spontaneous talks | HuggingFace API |
| **OpenSTT calls** | RU | CC-BY-NC ² | No (torrent) | ~13k | 2–8 s | Phone calls (spontaneous) | torrent / Azure |
| **VoxPopuli** | EN only ³ | CC0 1.0 | No | ~1,100 | ≤20 s | Parliamentary speech | HuggingFace API |

¹ Free for research and benchmarking; not open-source compatible.  
² Non-commercial only — fine for internal benchmarking, not for redistribution.  
³ VoxPopuli has no Russian — European Parliament only.

### Recommended starter set

Covers all duration buckets, both languages, no registration required, total download under 1 GB.

| Role | Dataset | Size |
|---|---|---|
| RU + EN baseline (short clips) | FLEURS `ru_ru` + `en_us` test | ~50 MB each |
| EN canonical WER reference | LibriSpeech `test-clean` | ~346 MB |
| RU commands / short phrases | GOLOS farfield test | ~200 MB (streamed) |
| EN spontaneous (harder) | TED-LIUM 3 test | ~100 MB streamed |

Add **OpenSTT calls** when Russian spontaneous/conversational benchmarks are needed.

---

## Download Instructions

### Prerequisites

```bash
pip install datasets jiwer soundfile
```

### FLEURS (RU + EN) — quickest start

```python
from datasets import load_dataset

ru = load_dataset("google/fleurs", "ru_ru", split="test", streaming=True)
en = load_dataset("google/fleurs", "en_us", split="test", streaming=True)

for sample in ru:
    audio_array = sample["audio"]["array"]          # np.ndarray, 16 kHz, float32
    sr          = sample["audio"]["sampling_rate"]  # 16000
    reference   = sample["transcription"]           # normalized, lowercase, no punct
    raw_ref     = sample["raw_transcription"]       # original with punctuation and case
```

### LibriSpeech test-clean (EN)

```bash
# Option A: direct download (~346 MB)
wget https://www.openslr.org/resources/12/test-clean.tar.gz
tar -xzf test-clean.tar.gz
# Transcriptions: LibriSpeech/test-clean/<speaker>/<chapter>/<file>.trans.txt
# Format per line:  <FILEID> THE TRANSCRIPTION IN UPPERCASE
```

```python
# Option B: HuggingFace (streaming, no local extract)
from datasets import load_dataset

ds = load_dataset("openslr/librispeech_asr", "clean", split="test", streaming=True)
for sample in ds:
    audio_array = sample["audio"]["array"]   # 16 kHz
    reference   = sample["text"]             # uppercase, no punctuation
```

### GOLOS (RU)

```python
from datasets import load_dataset

# farfield test — 1,916 clips, ~200 MB WAV
golos_ff = load_dataset("SberDevices/Golos", name="farfield", split="test", streaming=True)

# crowd test — 9,994 clips, ~1.1 GB WAV
golos_cr = load_dataset("SberDevices/Golos", name="crowd", split="test", streaming=True)

for sample in golos_ff:
    audio_array = sample["audio"]["array"]   # 16 kHz
    reference   = sample["transcription"]    # Cyrillic, normalized
```

### Russian LibriSpeech

```python
from datasets import load_dataset

ds = load_dataset("istupakov/russian_librispeech", split="test", streaming=True)
for sample in ds:
    audio_array = sample["audio"]["array"]
    reference   = sample["text"]               # punctuated
    raw_ref     = sample["text_no_preprocessing"]
```

### Common Voice v17 (RU + EN, no registration)

```python
from datasets import load_dataset

cv_ru = load_dataset(
    "mozilla-foundation/common_voice_17_0", "ru", split="test",
    trust_remote_code=True, streaming=True
)
cv_en = load_dataset(
    "mozilla-foundation/common_voice_17_0", "en", split="test",
    trust_remote_code=True, streaming=True
)
for sample in cv_ru:
    audio_array = sample["audio"]["array"]   # 48 kHz — resample to 16 kHz before whisper
    reference   = sample["sentence"]
```

> **Note:** Common Voice audio is 48 kHz. Resample to 16 kHz before passing to whisper.cpp:
> ```python
> import librosa
> audio_16k = librosa.resample(audio_array, orig_sr=48000, target_sr=16000)
> ```

### TED-LIUM 3 (EN, spontaneous)

```python
from datasets import load_dataset

ds = load_dataset("LIUM/tedlium", "release3", split="test", streaming=True)
for sample in ds:
    audio_array = sample["audio"]["array"]   # 16 kHz
    reference   = sample["text"].replace("<unk>", "").strip()  # remove <unk> tokens
```

### OpenSTT calls (RU, spontaneous — torrent required)

```bash
# See https://github.com/snakers4/open_stt for latest Azure Blob URLs
# Validation set: asr_calls_2_val (~7.7 h, 12,950 utterances)
# Manifest format: CSV with columns: audio_filepath, duration, text
```

---

## Duration Coverage

The benchmark should sample across all target duration buckets. Clip distribution by dataset:

| Bucket | Target count | Recommended source |
|---|---|---|
| 2–10 s (short dictation) | 200–500 clips | FLEURS, GOLOS farfield, Common Voice |
| 10–30 s (sentence burst) | 100–200 clips | LibriSpeech, Russian LibriSpeech, FLEURS |
| 30 s–3 min (paragraph) | 20–50 clips | Concatenate LibriSpeech segments or use long-form TED-LIUM |
| 3–10 min (long dictation) | 5–10 clips | Concatenate multiple LibriSpeech/FLEURS segments or real recordings |

> For the 30 s–10 min buckets no ready-made dataset has clips of that length — these are
> naturally produced by concatenating consecutive same-speaker segments or by recording
> real voicepuncher sessions against a script.

---

## WER / CER Computation

### Tool: `jiwer` (Python, Apache-2.0)

```bash
pip install jiwer   # current: 3.1.0
```

```python
import jiwer

# Basic WER
wer = jiwer.wer(reference, hypothesis)

# CER — use for Russian (morphologically rich, long compound words)
cer = jiwer.cer(reference, hypothesis)

# Normalized comparison (recommended for all datasets)
transform = jiwer.Compose([
    jiwer.ToLowerCase(),
    jiwer.RemovePunctuation(),
    jiwer.RemoveMultipleSpaces(),
    jiwer.Strip(),
    jiwer.ReduceToListOfListOfWords(),
])
result = jiwer.process_words(
    reference, hypothesis,
    reference_transform=transform,
    hypothesis_transform=transform,
)
print(f"WER: {result.wer:.3f}  MER: {result.mer:.3f}")
```

### Russian-specific normalization (mandatory)

Before computing WER/CER for Russian, apply these normalizations to **both** reference and hypothesis:

```python
def normalize_ru(text: str) -> str:
    text = text.replace("ё", "е")    # whisper.cpp may or may not output ё
    text = text.lower()
    # strip punctuation
    import re
    text = re.sub(r"[^\w\s]", "", text)
    return text.strip()
```

GOLOS and OpenSTT ground-truth already replaces `ё → е`; LibriSpeech-RU and FLEURS may not.

### Rust

No mature WER crate exists (as of 2025). Options:
- Keep the benchmark runner in Python using `jiwer`.
- Implement WER manually using `strsim` crate (Levenshtein on word list).

---

## N×N Benchmark Structure

The goal is a table of (whisper_model × llm_model) → WER/CER per dataset/language.

```
bench/
├── samples/
│   ├── fleurs_ru_test.jsonl        # {audio_path, reference, lang, duration_s}
│   ├── fleurs_en_test.jsonl
│   ├── librispeech_test_clean.jsonl
│   ├── golos_farfield_test.jsonl
│   └── ...
├── models/
│   ├── whisper/                    # symlinks or paths to .bin files
│   └── llm/                        # symlinks or paths to .gguf files
├── run_bench.py                    # runs pipeline for each (whisper × llm) combo
└── results/
    └── YYYY-MM-DD_HH-MM/
        ├── raw.jsonl               # per-clip: model combo, reference, hypothesis, wer, cer
        └── summary.md              # pivot table: model combo → avg WER/CER per dataset
```

### Expected output (summary.md excerpt)

| Whisper model | LLM model | FLEURS RU WER | FLEURS EN WER | LibriSpeech WER | GOLOS WER |
|---|---|---|---|---|---|
| large-v3-turbo | Qwen2.5-1.5B | 0.082 | 0.031 | 0.028 | 0.104 |
| large-v3-turbo | Qwen2.5-3B | 0.071 | 0.027 | 0.025 | 0.091 |
| large-v3-turbo | none (raw) | 0.095 | 0.038 | 0.033 | 0.119 |
| small | Qwen2.5-1.5B | 0.143 | 0.062 | 0.054 | 0.187 |

The "none (raw)" row shows whisper output without LLM correction — this is the control that
measures LLM contribution.

---

## Licensing Summary

| Dataset | Internal benchmarking | Publication of results | Redistribution of audio |
|---|---|---|---|
| FLEURS | ✅ | ✅ | ✅ |
| LibriSpeech | ✅ | ✅ | ✅ (with attribution) |
| GOLOS | ✅ | ✅ (check Sber license PDF) | ❌ |
| Russian LibriSpeech | ✅ | ✅ | ✅ (with attribution) |
| Common Voice v17 | ✅ | ✅ | ✅ |
| TED-LIUM 3 | ✅ | ✅ | ❌ (NC-ND) |
| OpenSTT calls | ✅ | ✅ | ❌ (NC) |

All datasets are safe for internal benchmarking and publishing accuracy numbers.
Do not redistribute audio from TED-LIUM, GOLOS, or OpenSTT.
