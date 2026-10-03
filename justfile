set dotenv-load

# Corpus directory containing manifest.jsonl — absolute, because cargo runs tests from e2e-test/
corpus_dir := env_var_or_default("VOICEPUNCHER_BENCH_CORPUS", justfile_directory() / "e2e-test/corpus")

# List all recipes
default:
    @just --list

# Build the release binary
build:
    cargo build --release

# Code checks — fmt, clippy, all app tests, release build. Keeps target/ (no cargo clean)
check:
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test -p voicepuncher --tests --no-fail-fast
    cargo build --release

# Pre-release gate — clean build, code checks, E2E smoke + WER accuracy; then README § Release checklist
release-check: _require-models
    cargo clean
    just check
    just e2e

# Fail fast when model env vars are missing — otherwise E2E tests skip silently (GATED)
_require-models:
    #!/usr/bin/env bash
    set -euo pipefail
    for var in VOICEPUNCHER_WHISPER_MODEL VOICEPUNCHER_LLM_MODEL; do
        path="${!var:-}"
        if [[ -z "$path" ]]; then
            echo "✗ $var is not set — export it (or put it in .env) with the path to the model file" >&2
            exit 1
        fi
        if [[ ! -f "$path" ]]; then
            echo "✗ $var points to a missing file: $path" >&2
            exit 1
        fi
    done

# Download benchmark corpus — idempotent, ~500 MB, one-time setup
corpus max_samples="50":
    uv run --python 3.11 --with "datasets<3.0.0" --with soundfile --with numpy --with librosa \
        python3 e2e-test/scripts/download_corpus.py \
        --out e2e-test/corpus \
        --max-samples {{max_samples}}

# Smoke test — verify models load and pipeline runs end-to-end
e2e-smoke: build
    cargo test -p voicepuncher-e2e-test --test smoke --release

# WER accuracy benchmark — run pipeline on corpus, assert mean WER < 30 %
e2e-bench: build
    VOICEPUNCHER_BENCH_CORPUS="{{corpus_dir}}" \
        cargo test -p voicepuncher-e2e-test --test pipeline_accuracy --release -- --nocapture

# Run all E2E tests (smoke + accuracy benchmark)
e2e: e2e-smoke e2e-bench

# Full automated benchmark — all Whisper × LLM combos on the full corpus
bench:
    uv run --python 3.11 --with "datasets<3.0.0" --with soundfile --with numpy --with librosa \
        python3 e2e-test/scripts/bench.py

# Quick benchmark — 10 samples, runs in ~2 min; use after each change to get a fast signal
bench-smoke:
    uv run --python 3.11 --with "datasets<3.0.0" --with soundfile --with numpy --with librosa \
        python3 e2e-test/scripts/bench.py --limit 10
