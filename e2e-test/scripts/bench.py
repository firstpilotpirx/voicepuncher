#!/usr/bin/env python3
"""
Automated E2E accuracy benchmark for voicepuncher.

Downloads models and corpus automatically on first run.
Runs all Whisper × LLM combinations and prints a Score/RTF comparison table.
For each Whisper model it starts its own whisper-server (Homebrew whisper-cpp),
and for each LLM its own llama-server (Homebrew llama.cpp), on separate ports,
and stops them after that model's combinations.

Metrics:
  Score = 1 - (WER + CER) / 2   quality, 0..1, higher is better
  RTF   = processing_s / audio_s  speed,   lower is better (< 1 = faster than real-time)

Usage:
    python e2e-test/scripts/bench.py
    python e2e-test/scripts/bench.py --limit 10
    python e2e-test/scripts/bench.py --models-dir /custom/path
"""
import argparse
import contextlib
import json
import math
import os
import re
import shutil
import socket
import subprocess
import sys
import time
import urllib.request
from pathlib import Path
from typing import Iterator, NamedTuple, Optional

ROOT = Path(__file__).resolve().parents[2]

# Separate from the 8081 port `just start-services` uses, so a running service is left alone
BENCH_STT_PORT = 8091
STT_READY_TIMEOUT_S = 120
STT_STOP_TIMEOUT_S = 10
# Same decoding flags as `just start-services`
STT_DECODING_FLAGS = ["-l", "auto", "-bo", "1", "-nf", "-sns", "-mc", "0", "-nt"]
# Separate from the 8080 port `just start-services` uses
BENCH_LLM_PORT = 8090
LLM_READY_TIMEOUT_S = 120
# Same flags as `just start-services`
LLM_SERVER_FLAGS = ["-c", "4096", "-np", "1"]

# ── Model matrix ── add entries here to include new models in the benchmark ────

WHISPER_MODELS = [
    {
        "name": "large-v3-turbo-q5",
        "file": "ggml-large-v3-turbo-q5_0.bin",
        "url": "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo-q5_0.bin",
    },
    {
        "name": "medium",
        "file": "ggml-medium.bin",
        "url": "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.bin",
    },
    {
        "name": "small",
        "file": "ggml-small.bin",
        "url": "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin",
    },
]

LLM_MODELS = [
    {
        "name": "Qwen2.5-1.5B-Q4",
        "file": "qwen2.5-1.5b-instruct-q4_k_m.gguf",
        "url": "https://huggingface.co/Qwen/Qwen2.5-1.5B-Instruct-GGUF/resolve/main/qwen2.5-1.5b-instruct-q4_k_m.gguf",
    },
    {
        "name": "Qwen2.5-3B-Q4",
        "file": "qwen2.5-3b-instruct-q4_k_m.gguf",
        "url": "https://huggingface.co/Qwen/Qwen2.5-3B-Instruct-GGUF/resolve/main/qwen2.5-3b-instruct-q4_k_m.gguf",
    },
    {
        "name": "Qwen2.5-7B-Q4",
        "file": "Qwen2.5-7B-Instruct-Q4_K_M.gguf",
        "url": "https://huggingface.co/bartowski/Qwen2.5-7B-Instruct-GGUF/resolve/main/Qwen2.5-7B-Instruct-Q4_K_M.gguf",
    },
]


# ── ANSI colours ───────────────────────────────────────────────────────────────

RESET   = "\033[0m"
BOLD    = "\033[1m"
DIM     = "\033[2m"
GREEN   = "\033[32m"
YELLOW  = "\033[33m"
RED     = "\033[31m"
CYAN    = "\033[36m"
MAGENTA = "\033[35m"


def _score_color(score: float) -> str:
    if math.isnan(score):
        return DIM
    if score >= 0.80:
        return GREEN
    if score >= 0.50:
        return YELLOW
    return RED


def _rtf_color(rtf: float) -> str:
    if math.isnan(rtf):
        return DIM
    if rtf < 1.0:
        return GREEN
    if rtf < 3.0:
        return YELLOW
    return RED


def _fmt_pct(val: float, color: str) -> str:
    if math.isnan(val):
        return f"{DIM}   n/a{RESET}"
    return f"{color}{val:6.1%}{RESET}"


def _fmt_rtf(val: float, color: str) -> str:
    if math.isnan(val):
        return f"{DIM}  n/a{RESET}"
    return f"{color}{val:5.2f}x{RESET}"


def _banner(title: str) -> str:
    return f"{CYAN}{BOLD}── {title} {('─' * (65 - len(title)))}{RESET}"


# ── WER / CER ──────────────────────────────────────────────────────────────────

def _normalize_words(text: str) -> list:
    t = text.lower().replace("ё", "е")
    t = re.sub(r"[^\w\s]", " ", t, flags=re.UNICODE)
    return t.split()


def _normalize_chars(text: str) -> list:
    t = text.lower().replace("ё", "е")
    return [c for c in t if c.isalnum()]


def _edit_distance(a: list, b: list) -> int:
    n, m = len(a), len(b)
    dp = list(range(m + 1))
    for i in range(1, n + 1):
        prev, dp[0] = dp[0], i
        for j in range(1, m + 1):
            tmp = dp[j]
            dp[j] = prev if a[i - 1] == b[j - 1] else 1 + min(prev, dp[j], dp[j - 1])
            prev = tmp
    return dp[m]


def wer(reference: str, hypothesis: str) -> float:
    r, h = _normalize_words(reference), _normalize_words(hypothesis)
    if not r and not h:
        return 0.0
    if not r:
        return 1.0
    return _edit_distance(r, h) / len(r)


def cer(reference: str, hypothesis: str) -> float:
    r, h = _normalize_chars(reference), _normalize_chars(hypothesis)
    if not r and not h:
        return 0.0
    if not r:
        return 1.0
    return _edit_distance(r, h) / len(r)


# ── Download ───────────────────────────────────────────────────────────────────

def _download(url: str, dest: Path) -> None:
    dest.parent.mkdir(parents=True, exist_ok=True)
    tmp = dest.with_suffix(dest.suffix + ".tmp")
    req = urllib.request.Request(url, headers={"User-Agent": "voicepuncher-bench/1.0"})
    try:
        with urllib.request.urlopen(req) as resp:
            total = int(resp.headers.get("Content-Length", 0))
            done = 0
            last_pct = -1
            with open(tmp, "wb") as f:
                while True:
                    chunk = resp.read(1 << 20)
                    if not chunk:
                        break
                    f.write(chunk)
                    done += len(chunk)
                    if total:
                        pct = int(done / total * 100)
                        if pct >= last_pct + 5:
                            last_pct = pct
                            bar = pct * 30 // 100
                            print(
                                f"  {DIM}[{'█' * bar}{'░' * (30 - bar)}]"
                                f" {done / 1e6:6.1f}/{total / 1e6:.1f} MB  {dest.name}{RESET}",
                                flush=True,
                            )
        tmp.rename(dest)
        print(f"  {GREEN}✓ {dest.name}{RESET}")
    except Exception:
        tmp.unlink(missing_ok=True)
        raise


def ensure_models(models_dir: Path) -> None:
    print(_banner("Models"))
    for m in WHISPER_MODELS + LLM_MODELS:
        path = models_dir / m["file"]
        if path.exists():
            print(f"  {GREEN}✓ {m['file']}{RESET}")
        else:
            print(f"  {YELLOW}↓ {m['file']}{RESET}")
            _download(m["url"], path)


def ensure_corpus(corpus_dir: Path, max_samples: int) -> None:
    print(_banner("Corpus"))
    subprocess.run(
        [
            sys.executable,
            str(ROOT / "e2e-test/scripts/download_corpus.py"),
            "--out", str(corpus_dir),
            "--max-samples", str(max_samples),
        ],
        check=True,
    )
    manifest = corpus_dir / "manifest.jsonl"
    n = sum(1 for line in manifest.read_text().splitlines() if line.strip())
    print(f"  {GREEN}✓ {manifest} ({n} samples){RESET}")


def build_binary() -> Path:
    print(_banner("Build"))
    subprocess.run(["cargo", "build", "--release"], cwd=ROOT, check=True)
    binary = ROOT / "target/release/voicepuncher"
    print(f"  {GREEN}✓ {binary}{RESET}")
    return binary


# ── Run ────────────────────────────────────────────────────────────────────────

class Sample(NamedTuple):
    id: str
    path: Path
    reference: str
    lang: str
    duration_s: float


def load_manifest(manifest_path: Path) -> list:
    samples = []
    for line in manifest_path.read_text().splitlines():
        line = line.strip()
        if not line:
            continue
        d = json.loads(line)
        samples.append(Sample(
            id=d["id"],
            path=manifest_path.parent / d["path"],
            reference=d["reference"],
            lang=d.get("lang", ""),
            duration_s=float(d.get("duration_s", 0.0)),
        ))
    return samples


# ── Speech recognition service ─────────────────────────────────────────────────

def _port_open(port: int) -> bool:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.settimeout(0.5)
        return sock.connect_ex(("127.0.0.1", port)) == 0


def _health_ok(port: int) -> bool:
    try:
        with urllib.request.urlopen(f"http://127.0.0.1:{port}/health", timeout=1) as response:
            return response.status == 200
    except OSError:
        return False


@contextlib.contextmanager
def _local_server(cmd: list, port: int, ready, ready_timeout_s: int, model_path: Path) -> Iterator[str]:
    """Runs one local service for the duration of the block; yields its base URL."""
    name = cmd[0]
    if _port_open(port):
        raise SystemExit(f"{RED}✗ port {port} is already in use — stop whatever listens there{RESET}")
    proc = subprocess.Popen(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        deadline = time.monotonic() + ready_timeout_s
        while not ready(port):
            if proc.poll() is not None:
                raise SystemExit(f"{RED}✗ {name} exited while loading {model_path.name}{RESET}")
            if time.monotonic() > deadline:
                raise SystemExit(f"{RED}✗ {name} did not start within {ready_timeout_s}s{RESET}")
            time.sleep(0.25)
        yield f"http://127.0.0.1:{port}/v1"
    finally:
        proc.terminate()
        try:
            proc.wait(timeout=STT_STOP_TIMEOUT_S)
        except subprocess.TimeoutExpired:
            proc.kill()


def whisper_server(model_path: Path, port: int = BENCH_STT_PORT):
    """Speech recognition service with one Whisper model."""
    cmd = [
        "whisper-server", "-m", str(model_path), "--host", "127.0.0.1", "--port", str(port),
        "--inference-path", "/v1/audio/transcriptions", "-t", str(os.cpu_count() or 4),
        *STT_DECODING_FLAGS,
    ]
    return _local_server(cmd, port, _port_open, STT_READY_TIMEOUT_S, model_path)


def llama_server(model_path: Path, port: int = BENCH_LLM_PORT):
    """Text refinement service with one LLM."""
    cmd = [
        "llama-server", "-m", str(model_path), "--host", "127.0.0.1", "--port", str(port),
        *LLM_SERVER_FLAGS,
    ]
    return _local_server(cmd, port, _health_ok, LLM_READY_TIMEOUT_S, model_path)


def ensure_servers() -> None:
    for binary, formula in [("whisper-server", "whisper-cpp"), ("llama-server", "llama.cpp")]:
        if shutil.which(binary) is None:
            raise SystemExit(f"{RED}✗ {binary} not found — run 'brew install {formula}'{RESET}")


def _run_one(
    binary: Path, transcription_url: str, refinement_url: str, wav: Path
) -> tuple[Optional[str], float]:
    t0 = time.monotonic()
    try:
        r = subprocess.run(
            [
                str(binary),
                "--config", "/dev/null",
                "--input-file", str(wav),
                "--transcription-url", transcription_url,
                "--refinement-url", refinement_url,
            ],
            capture_output=True,
            text=True,
            timeout=120,
        )
        elapsed = time.monotonic() - t0
        return (r.stdout.strip() if r.returncode == 0 else None), elapsed
    except subprocess.TimeoutExpired:
        return None, time.monotonic() - t0


class ComboResult(NamedTuple):
    whisper_name: str
    llm_name: str
    mean_score: float
    mean_wer: float
    mean_cer: float
    mean_time_s: float
    mean_rtf: float
    n: int
    total: int


def run_combo(
    binary: Path,
    transcription_url: str,
    refinement_url: str,
    whisper: dict,
    llm: dict,
    samples: list,
) -> ComboResult:
    wers, cers, times, rtfs = [], [], [], []
    for i, s in enumerate(samples, 1):
        print(f"  {DIM}[{i:>3}/{len(samples)}]{RESET} {s.id:<50}", end=" ", flush=True)
        hyp, elapsed = _run_one(binary, transcription_url, refinement_url, s.path)
        if hyp is None:
            print(f"{DIM}—{RESET}")
            continue
        w, c = wer(s.reference, hyp), cer(s.reference, hyp)
        score = 1.0 - (w + c) / 2.0
        rtf = elapsed / s.duration_s if s.duration_s > 0 else math.nan
        wers.append(w)
        cers.append(c)
        times.append(elapsed)
        rtfs.append(rtf)
        sc = _score_color(score)
        rc = _rtf_color(rtf)
        print(
            f"Score {_fmt_pct(score, sc)}  "
            f"RTF {_fmt_rtf(rtf, rc)}  "
            f"{DIM}(WER {w:.1%}  CER {c:.1%}  {elapsed:.1f}s){RESET}"
        )
    mean_w = sum(wers) / len(wers) if wers else math.nan
    mean_c = sum(cers) / len(cers) if cers else math.nan
    mean_t = sum(times) / len(times) if times else math.nan
    mean_rtf = sum(rtfs) / len(rtfs) if rtfs else math.nan
    mean_score = 1.0 - (mean_w + mean_c) / 2.0 if not (math.isnan(mean_w) or math.isnan(mean_c)) else math.nan
    return ComboResult(whisper["name"], llm["name"], mean_score, mean_w, mean_c, mean_t, mean_rtf, len(wers), len(samples))


# ── Table ──────────────────────────────────────────────────────────────────────

def print_table(results: list) -> None:
    wn = max((len(r.whisper_name) for r in results), default=7)
    ln = max((len(r.llm_name) for r in results), default=3)
    wn, ln = max(wn, 7), max(ln, 3)
    div = f"+{'-' * (wn + 2)}+{'-' * (ln + 2)}+--------+--------+----------+--------+"
    print(f"\n{_banner('Results')}")
    print(div)
    print(
        f"| {BOLD}{'Whisper':<{wn}}{RESET} | {BOLD}{'LLM':<{ln}}{RESET}"
        f" | {BOLD}{' Score'}{RESET} | {BOLD}{'  RTF'}{RESET}"
        f" | {BOLD}{'Samples ':>8}{RESET} | {BOLD}{'s/clip'}{RESET} |"
    )
    print(div)
    best_score = max((r.mean_score for r in results if not math.isnan(r.mean_score)), default=math.nan)
    for r in results:
        sc = _score_color(r.mean_score)
        rc = _rtf_color(r.mean_rtf)
        is_best = not math.isnan(r.mean_score) and r.mean_score == best_score
        row_fmt = BOLD if is_best else ""
        score_s = _fmt_pct(r.mean_score, sc)
        rtf_s = _fmt_rtf(r.mean_rtf, rc)
        time_s = f"{DIM}{r.mean_time_s:5.1f}s{RESET}" if not math.isnan(r.mean_time_s) else f"{DIM}  n/a {RESET}"
        samples_s = f"{r.n:>3}/{r.total:<4}"
        print(
            f"| {row_fmt}{r.whisper_name:<{wn}}{RESET} | {row_fmt}{r.llm_name:<{ln}}{RESET}"
            f" | {score_s} | {rtf_s} | {samples_s} | {time_s} |"
        )
    print(div)


# ── Main ───────────────────────────────────────────────────────────────────────

def main() -> None:
    ap = argparse.ArgumentParser(
        description=__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    ap.add_argument(
        "--models-dir",
        default=str(Path.home() / ".voicepuncher/models"),
        help="Directory for model files (default: ~/.voicepuncher/models)",
    )
    ap.add_argument(
        "--corpus-dir",
        default=str(ROOT / "e2e-test/corpus"),
        help="Corpus directory (default: e2e-test/corpus)",
    )
    ap.add_argument(
        "--max-samples",
        type=int,
        default=50,
        help="Corpus samples per dataset (default: 50)",
    )
    ap.add_argument(
        "--limit",
        type=int,
        default=None,
        help="Cap total samples used in the benchmark run (default: all)",
    )
    args = ap.parse_args()

    models_dir = Path(args.models_dir)
    corpus_dir = Path(args.corpus_dir)

    ensure_servers()
    ensure_models(models_dir)
    print()
    ensure_corpus(corpus_dir, args.max_samples)
    print()
    binary = build_binary()

    samples = load_manifest(corpus_dir / "manifest.jsonl")
    if args.limit is not None:
        samples = samples[: args.limit]
    print(f"\n{BOLD}Corpus: {len(samples)} samples{RESET}")

    results = []
    for whisper in WHISPER_MODELS:
        with whisper_server(models_dir / whisper["file"]) as transcription_url:
            for llm in LLM_MODELS:
                print(f"\n{MAGENTA}{BOLD}── {whisper['name']} × {llm['name']} {'─' * 40}{RESET}")
                with llama_server(models_dir / llm["file"]) as refinement_url:
                    results.append(run_combo(binary, transcription_url, refinement_url, whisper, llm, samples))

    print_table(results)


if __name__ == "__main__":
    main()
