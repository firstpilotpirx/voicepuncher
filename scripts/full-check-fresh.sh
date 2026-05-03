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
step() { printf "\n${CYAN}●${RESET} ${BOLD}%s${RESET}\n" "$*"; }
ok()   { printf "${GREEN}✓${RESET} ${DIM}%s${RESET}\n" "$*"; }

if [[ -z "${VOICEPUNCHER_WHISPER_MODEL:-}" ]]; then
    err "VOICEPUNCHER_WHISPER_MODEL is not set."
    printf "${DIM}Usage: VOICEPUNCHER_WHISPER_MODEL=<path> VOICEPUNCHER_LLM_MODEL=<path> %s${RESET}\n" "$0" >&2
    exit 2
fi

if [[ -z "${VOICEPUNCHER_LLM_MODEL:-}" ]]; then
    err "VOICEPUNCHER_LLM_MODEL is not set."
    printf "${DIM}Usage: VOICEPUNCHER_WHISPER_MODEL=<path> VOICEPUNCHER_LLM_MODEL=<path> %s${RESET}\n" "$0" >&2
    exit 2
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
