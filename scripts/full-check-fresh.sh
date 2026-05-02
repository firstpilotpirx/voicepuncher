#!/usr/bin/env bash
set -euo pipefail

if [[ -z "${VOICEPUNCHER_WHISPER_MODEL:-}" ]]; then
    echo "ERROR: VOICEPUNCHER_WHISPER_MODEL is not set." >&2
    echo "Usage: VOICEPUNCHER_WHISPER_MODEL=<path> VOICEPUNCHER_LLM_MODEL=<path> $0" >&2
    exit 2
fi

if [[ -z "${VOICEPUNCHER_LLM_MODEL:-}" ]]; then
    echo "ERROR: VOICEPUNCHER_LLM_MODEL is not set." >&2
    echo "Usage: VOICEPUNCHER_WHISPER_MODEL=<path> VOICEPUNCHER_LLM_MODEL=<path> $0" >&2
    exit 2
fi

cd "$(dirname "$0")/../app"

cargo clean
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --test architecture
cargo test --lib
cargo test --tests
cargo build --release

echo "========================================"
echo " full-check-fresh: ALL CHECKS PASSED"
echo "========================================"
