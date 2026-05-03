#!/usr/bin/env python3
"""Download benchmark corpus for voicepuncher E2E accuracy tests.

Usage:
    pip install datasets soundfile numpy
    python e2e-test/scripts/download_corpus.py --out e2e-test/corpus
    python e2e-test/scripts/download_corpus.py --out e2e-test/corpus --datasets fleurs_ru podlodka --max-samples 10
"""
import argparse
import json
from pathlib import Path

TARGET_SAMPLE_RATE = 16_000


def resample_to_16k(audio, orig_rate: int):
    import numpy as np
    if orig_rate == TARGET_SAMPLE_RATE:
        return audio
    duration = len(audio) / orig_rate
    target_len = int(duration * TARGET_SAMPLE_RATE)
    indices = np.linspace(0, len(audio) - 1, target_len)
    return np.interp(indices, np.arange(len(audio)), audio).astype(np.float32)


def to_mono(audio):
    import numpy as np
    if audio.ndim == 1:
        return audio
    return audio.mean(axis=1).astype(np.float32)


def save_wav(path: Path, audio, sample_rate: int = TARGET_SAMPLE_RATE) -> None:
    import soundfile as sf
    import numpy as np
    audio_int16 = (audio * 32767).clip(-32768, 32767).astype(np.int16)
    sf.write(str(path), audio_int16, sample_rate, subtype="PCM_16")


def load_existing_ids(manifest_path: Path) -> set:
    ids = set()
    if not manifest_path.exists():
        return ids
    with open(manifest_path, encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if line:
                try:
                    ids.add(json.loads(line)["id"])
                except (json.JSONDecodeError, KeyError):
                    pass
    return ids


def append_manifest(manifest_path: Path, entry: dict) -> None:
    with open(manifest_path, "a", encoding="utf-8") as f:
        f.write(json.dumps(entry, ensure_ascii=False) + "\n")


def download_fleurs(out_dir: Path, lang_code: str, lang_label: str, max_samples: int) -> None:
    import numpy as np
    from datasets import load_dataset

    print(f"Downloading FLEURS {lang_code} (up to {max_samples} samples)...")
    ds = load_dataset(
        "google/fleurs",
        lang_code,
        split="test",
        streaming=True,
        trust_remote_code=True,
    )

    audio_dir = out_dir / "audio"
    manifest_path = out_dir / "manifest.jsonl"
    existing_ids = load_existing_ids(manifest_path)
    count = 0

    for item in ds:
        if count >= max_samples:
            break
        audio_array = np.array(item["audio"]["array"], dtype=np.float32)
        orig_rate = item["audio"]["sampling_rate"]
        audio_array = to_mono(audio_array)
        audio_array = resample_to_16k(audio_array, orig_rate)

        sample_id = f"fleurs_{lang_label}_{item['id']}"
        if sample_id in existing_ids:
            continue

        wav_path = audio_dir / f"{sample_id}.wav"
        save_wav(wav_path, audio_array)

        duration_s = len(audio_array) / TARGET_SAMPLE_RATE
        reference = item.get("transcription", item.get("raw_transcription", ""))
        if not reference:
            print(f"  WARNING: no reference text for {sample_id}, skipping")
            continue
        append_manifest(manifest_path, {
            "id": sample_id,
            "path": f"audio/{sample_id}.wav",
            "reference": reference,
            "lang": lang_label,
            "duration_s": round(duration_s, 3),
            "source": f"fleurs_{lang_code}",
        })
        count += 1
        print(f"  [{count}/{max_samples}] {sample_id} ({duration_s:.1f}s)")

    print(f"FLEURS {lang_code}: saved {count} samples.")


def download_librispeech(out_dir: Path, max_samples: int) -> None:
    import numpy as np
    from datasets import load_dataset

    print(f"Downloading LibriSpeech test-clean (up to {max_samples} samples)...")
    ds = load_dataset(
        "openslr/librispeech_asr",
        "clean",
        split="test",
        streaming=True,
        trust_remote_code=True,
    )

    audio_dir = out_dir / "audio"
    manifest_path = out_dir / "manifest.jsonl"
    existing_ids = load_existing_ids(manifest_path)
    count = 0

    for item in ds:
        if count >= max_samples:
            break
        audio_array = np.array(item["audio"]["array"], dtype=np.float32)
        orig_rate = item["audio"]["sampling_rate"]
        audio_array = to_mono(audio_array)
        audio_array = resample_to_16k(audio_array, orig_rate)

        sample_id = f"librispeech_{item['id']}"
        if sample_id in existing_ids:
            continue

        wav_path = audio_dir / f"{sample_id}.wav"
        save_wav(wav_path, audio_array)

        duration_s = len(audio_array) / TARGET_SAMPLE_RATE
        reference = item.get("text", "")
        if not reference:
            print(f"  WARNING: no reference text for {sample_id}, skipping")
            continue
        append_manifest(manifest_path, {
            "id": sample_id,
            "path": f"audio/{sample_id}.wav",
            "reference": reference,
            "lang": "en",
            "duration_s": round(duration_s, 3),
            "source": "librispeech",
        })
        count += 1
        print(f"  [{count}/{max_samples}] {sample_id} ({duration_s:.1f}s)")

    print(f"LibriSpeech: saved {count} samples.")


PODLODKA_SPLITS = ["train", "validation", "test"]


def download_podlodka(out_dir: Path, max_samples: int) -> None:
    try:
        import numpy as np
        from datasets import load_dataset
    except ImportError as exc:
        print(
            f"\033[31mERROR: missing dependency ({exc}).\n"
            "Install with:  pip install datasets soundfile numpy\033[0m"
        )
        raise SystemExit(1) from exc

    print(f"\033[36m\033[1m── Downloading Podlodka (up to {max_samples} samples per run) \033[0m")
    audio_dir = out_dir / "audio"
    manifest_path = out_dir / "manifest.jsonl"
    existing_ids = load_existing_ids(manifest_path)
    count = 0

    for split in PODLODKA_SPLITS:
        if count >= max_samples:
            break
        ds = load_dataset(
            "bond005/podlodka_speech",
            split=split,
            streaming=True,
            trust_remote_code=True,
        )
        for i, item in enumerate(ds):
            if count >= max_samples:
                break
            episode = item.get("episode", 0)
            sample_id = f"podlodka_{episode}_{i}"
            if sample_id in existing_ids:
                continue

            audio_field = item["audio"]
            if isinstance(audio_field, dict):
                audio_array = np.array(audio_field["array"], dtype=np.float32)
                orig_rate = audio_field["sampling_rate"]
            else:
                audio_array = np.frombuffer(audio_field, dtype=np.int16).astype(np.float32) / 32768.0
                orig_rate = TARGET_SAMPLE_RATE

            audio_array = to_mono(audio_array)
            audio_array = resample_to_16k(audio_array, orig_rate)

            wav_path = audio_dir / f"{sample_id}.wav"
            save_wav(wav_path, audio_array)

            duration_s = len(audio_array) / TARGET_SAMPLE_RATE
            reference = item.get("transcription", "")
            if not reference:
                print(f"  \033[33mWARNING: no reference text for {sample_id}, skipping\033[0m")
                continue

            append_manifest(manifest_path, {
                "id": sample_id,
                "path": f"audio/{sample_id}.wav",
                "reference": reference,
                "lang": "ru",
                "duration_s": round(duration_s, 3),
                "source": "podlodka",
            })
            count += 1
            print(f"  \033[2m[{count}/{max_samples}]\033[0m {sample_id} ({duration_s:.1f}s)")

    print(f"\033[32mPodlodka: saved {count} samples.\033[0m")


def main() -> None:
    parser = argparse.ArgumentParser(
        description=__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument(
        "--out",
        default="e2e-test/corpus",
        help="Output directory for WAV files and manifest (default: e2e-test/corpus)",
    )
    parser.add_argument(
        "--max-samples",
        type=int,
        default=50,
        help="Maximum number of samples per dataset (default: 50)",
    )
    parser.add_argument(
        "--datasets",
        nargs="+",
        choices=["fleurs_ru", "fleurs_en", "librispeech", "podlodka"],
        default=["fleurs_ru", "fleurs_en", "librispeech", "podlodka"],
        help="Which datasets to download (default: all four)",
    )
    args = parser.parse_args()

    out_dir = Path(args.out)
    (out_dir / "audio").mkdir(parents=True, exist_ok=True)

    manifest_path = out_dir / "manifest.jsonl"
    if manifest_path.exists():
        print(f"Note: {manifest_path} already exists — existing IDs will be skipped (idempotent).")

    if "fleurs_ru" in args.datasets:
        download_fleurs(out_dir, "ru_ru", "ru", args.max_samples)
    if "fleurs_en" in args.datasets:
        download_fleurs(out_dir, "en_us", "en", args.max_samples)
    if "librispeech" in args.datasets:
        download_librispeech(out_dir, args.max_samples)
    if "podlodka" in args.datasets:
        download_podlodka(out_dir, args.max_samples)

    print(f"\nDone. Manifest: {manifest_path}")


if __name__ == "__main__":
    main()
