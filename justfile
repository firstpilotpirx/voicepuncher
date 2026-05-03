set dotenv-load

corpus_manifest := env_var_or_default("VOICEPUNCHER_BENCH_CORPUS", "e2e-test/corpus/manifest.jsonl")

# List all recipes
default:
    @just --list

# Build the release binary
build:
    cargo build --release

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
    VOICEPUNCHER_BENCH_CORPUS="{{corpus_manifest}}" \
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
