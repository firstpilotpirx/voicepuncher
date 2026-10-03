set dotenv-load

# ── Settings ── override any of these via env or .env ─────────────────────────

# Models: explicit env var wins; otherwise the largest model found in ~/.voicepuncher or ~/.voicepuncher/models
export VOICEPUNCHER_WHISPER_MODEL := env_var_or_default("VOICEPUNCHER_WHISPER_MODEL", `find ~/.voicepuncher -maxdepth 2 -name 'ggml-*.bin' -exec ls -S {} + 2>/dev/null | head -1 || true`)
export VOICEPUNCHER_LLM_MODEL := env_var_or_default("VOICEPUNCHER_LLM_MODEL", `find ~/.voicepuncher -maxdepth 2 -name '*.gguf' -exec ls -S {} + 2>/dev/null | head -1 || true`)

# Corpus directory containing manifest.jsonl — absolute, because cargo runs tests from e2e-test/
export VOICEPUNCHER_BENCH_CORPUS := env_var_or_default("VOICEPUNCHER_BENCH_CORPUS", justfile_directory() / "e2e-test/corpus")

# Homebrew OpenSSL headers — llama-cpp-sys fails with 'openssl/err.h' not found without them
export CPATH := env_var_or_default("CPATH", "/opt/homebrew/opt/openssl@3/include")
export LIBRARY_PATH := env_var_or_default("LIBRARY_PATH", "/opt/homebrew/opt/openssl@3/lib")

# List all recipes
default:
    @just --list --unsorted

# ── Checks ── release-check runs all of them ──────────────────────────────────

# 1. Code checks: fmt, clippy, all app tests, release build (keeps target/)
check: fmt lint test build

# Formatting check
fmt:
    cargo fmt --all -- --check

# Clippy on the whole workspace, warnings are errors
lint:
    cargo clippy --workspace --all-targets -- -D warnings

# Unit + architecture + integration tests of the app
test:
    cargo test -p voicepuncher --tests --no-fail-fast

# Build the release binary
build:
    cargo build --release

# 2. E2E on real models: smoke + WER accuracy (~20 min)
e2e: e2e-smoke e2e-bench

# Smoke test — models load, file mode transcribes
e2e-smoke: _require-models build
    cargo test -p voicepuncher-e2e-test --test smoke --release -- --nocapture

# WER accuracy on the corpus — fails if mean WER > 30 %
e2e-bench: _require-models _require-corpus build
    cargo test -p voicepuncher-e2e-test --test pipeline_accuracy --release -- --nocapture

# 3. Pre-release gate: cargo clean + check + e2e (~30 min), then README § Release checklist
release-check: _require-models _require-corpus
    cargo clean
    just check
    just e2e
    @echo "✓ release-check passed — now walk through README.md § Release checklist (manual run)"

# ── Setup ─────────────────────────────────────────────────────────────────────

# Download default models into ~/.voicepuncher/models (whisper small ~460 MB, Qwen2.5-1.5B ~1 GB)
models:
    #!/usr/bin/env bash
    set -euo pipefail
    dir=~/.voicepuncher/models
    mkdir -p "$dir"
    fetch() {
        [[ -f "$dir/$2" ]] && { echo "✓ $2 already present"; return; }
        echo "↓ $2"
        curl -fL --progress-bar -o "$dir/$2.download" "$1"
        mv "$dir/$2.download" "$dir/$2"
    }
    fetch https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin ggml-small.bin
    fetch https://huggingface.co/bartowski/Qwen2.5-1.5B-Instruct-GGUF/resolve/main/Qwen2.5-1.5B-Instruct-Q4_K_M.gguf Qwen2.5-1.5B-Instruct-Q4_K_M.gguf

# Download benchmark corpus — idempotent, ~500 MB, one-time setup
corpus max_samples="50":
    uv run --python 3.11 --with "datasets<3.0.0" --with soundfile --with numpy --with librosa \
        python3 e2e-test/scripts/download_corpus.py \
        --out e2e-test/corpus \
        --max-samples {{max_samples}}

# ── Model comparison ── not a gate; only when changing a model, decoding params or the prompt

# Full benchmark — all Whisper × LLM combos on the full corpus
bench:
    uv run --python 3.11 --with "datasets<3.0.0" --with soundfile --with numpy --with librosa \
        python3 e2e-test/scripts/bench.py

# Quick benchmark — 10 samples, ~2 min
bench-smoke:
    uv run --python 3.11 --with "datasets<3.0.0" --with soundfile --with numpy --with librosa \
        python3 e2e-test/scripts/bench.py --limit 10

# ── Guards ── fail fast instead of letting E2E tests skip silently (GATED) ────

_require-models:
    #!/usr/bin/env bash
    set -euo pipefail
    for var in VOICEPUNCHER_WHISPER_MODEL VOICEPUNCHER_LLM_MODEL; do
        path="${!var:-}"
        if [[ -z "$path" || ! -f "$path" ]]; then
            echo "✗ $var: no model file (got '${path}'). Run 'just models' or set it in .env" >&2
            exit 1
        fi
    done
    echo "whisper: $VOICEPUNCHER_WHISPER_MODEL"
    echo "llm:     $VOICEPUNCHER_LLM_MODEL"

_require-corpus:
    #!/usr/bin/env bash
    set -euo pipefail
    if [[ ! -f "$VOICEPUNCHER_BENCH_CORPUS/manifest.jsonl" ]]; then
        echo "✗ no corpus at $VOICEPUNCHER_BENCH_CORPUS. Run 'just corpus' or set VOICEPUNCHER_BENCH_CORPUS" >&2
        exit 1
    fi
