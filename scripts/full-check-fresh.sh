#!/usr/bin/env bash
set -euo pipefail

# Color setup — auto-disable when not a TTY, NO_COLOR is set, or TERM=dumb
if [[ -t 1 ]] && [[ -z "${NO_COLOR:-}" ]] && [[ "${TERM:-}" != "dumb" ]]; then
    RED='\033[0;31m'
    GREEN='\033[0;32m'
    YELLOW='\033[0;33m'
    CYAN='\033[0;36m'
    BOLD='\033[1m'
    DIM='\033[2m'
    RESET='\033[0m'
else
    RED='' GREEN='' YELLOW='' CYAN='' BOLD='' DIM='' RESET=''
fi

err()  { printf "${RED}${BOLD}✗ ERROR:${RESET}${RED} %s${RESET}\n" "$*" >&2; }
warn() { printf "${YELLOW}⚠${RESET}  ${YELLOW}%s${RESET}\n" "$*"; }
step() { printf "\n${CYAN}●${RESET} ${BOLD}%s${RESET}\n" "$*"; }
ok()   { printf "${GREEN}✓${RESET} ${DIM}%s${RESET}\n" "$*"; }

MODELS_DIR="${HOME}/.voicepuncher/models"

# Default models used when no model is present (smallest / fastest for dev)
WHISPER_DEFAULT_URL="https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin"
WHISPER_DEFAULT_FILE="ggml-small.bin"
LLM_DEFAULT_URL="https://huggingface.co/bartowski/Qwen2.5-1.5B-Instruct-GGUF/resolve/main/Qwen2.5-1.5B-Instruct-Q4_K_M.gguf"
LLM_DEFAULT_FILE="Qwen2.5-1.5B-Instruct-Q4_K_M.gguf"

download_model() {
    local url="$1"
    local dest="$2"
    local label="$3"
    local tmp="${dest}.download"

    printf "${CYAN}  ↓${RESET} Downloading %s…\n" "${label}"
    mkdir -p "$(dirname "${dest}")"
    if curl -fL --progress-bar -o "${tmp}" "${url}"; then
        mv "${tmp}" "${dest}"
        ok "downloaded ${label}: ${dest}"
    else
        rm -f "${tmp}"
        err "Failed to download ${label} from ${url}"
        exit 1
    fi
}

# Resolve whisper model — env var → auto-discover → download default
if [[ -z "${VOICEPUNCHER_WHISPER_MODEL:-}" ]]; then
    found=$(find "${MODELS_DIR}" -maxdepth 1 -name "ggml-*.bin" 2>/dev/null | sort | tail -1)
    if [[ -n "${found}" ]]; then
        export VOICEPUNCHER_WHISPER_MODEL="${found}"
        ok "whisper model: ${found}"
    else
        warn "No whisper model found — downloading ${WHISPER_DEFAULT_FILE} (460 MB)"
        download_model "${WHISPER_DEFAULT_URL}" "${MODELS_DIR}/${WHISPER_DEFAULT_FILE}" "whisper/${WHISPER_DEFAULT_FILE}"
        export VOICEPUNCHER_WHISPER_MODEL="${MODELS_DIR}/${WHISPER_DEFAULT_FILE}"
    fi
fi

# Resolve LLM model — env var → auto-discover → download default
if [[ -z "${VOICEPUNCHER_LLM_MODEL:-}" ]]; then
    found=$(find "${MODELS_DIR}" -maxdepth 1 -name "*.gguf" 2>/dev/null | sort | tail -1)
    if [[ -n "${found}" ]]; then
        export VOICEPUNCHER_LLM_MODEL="${found}"
        ok "LLM model: ${found}"
    else
        warn "No LLM model found — downloading ${LLM_DEFAULT_FILE} (~1 GB)"
        download_model "${LLM_DEFAULT_URL}" "${MODELS_DIR}/${LLM_DEFAULT_FILE}" "llm/${LLM_DEFAULT_FILE}"
        export VOICEPUNCHER_LLM_MODEL="${MODELS_DIR}/${LLM_DEFAULT_FILE}"
    fi
fi

cd "$(dirname "$0")/../app"

step "cargo clean"
cargo clean
ok "clean"

step "cargo fmt --check"
cargo fmt --check
ok "fmt"

step "cargo clippy --all-targets -- -D warnings"
cargo clippy --all-targets -- -D warnings
ok "clippy"

step "cargo test --test architecture"
cargo test --test architecture
ok "architecture tests"

step "cargo test --lib"
cargo test --lib
ok "unit tests"

step "cargo test --tests"
cargo test --tests
ok "integration tests"

step "cargo build --release"
cargo build --release
ok "release build"

printf "\n${GREEN}${BOLD}══════════════════════════════════════════${RESET}\n"
printf "${GREEN}${BOLD}  ✓  full-check-fresh: ALL CHECKS PASSED  ${RESET}\n"
printf "${GREEN}${BOLD}══════════════════════════════════════════${RESET}\n\n"
