set dotenv-load

# ── Settings ── override any of these via env or .env ─────────────────────────

# Models: explicit env var wins; otherwise the largest model found in ~/.voicepuncher or ~/.voicepuncher/models
export VOICEPUNCHER_WHISPER_MODEL := env_var_or_default("VOICEPUNCHER_WHISPER_MODEL", `find ~/.voicepuncher -maxdepth 2 -name 'ggml-*.bin' -exec ls -S {} + 2>/dev/null | head -1 || true`)
export VOICEPUNCHER_LLM_MODEL := env_var_or_default("VOICEPUNCHER_LLM_MODEL", `find ~/.voicepuncher -maxdepth 2 -name '*.gguf' -exec ls -S {} + 2>/dev/null | head -1 || true`)

# Corpus directory containing manifest.jsonl — absolute, because cargo runs tests from e2e-test/
export VOICEPUNCHER_BENCH_CORPUS := env_var_or_default("VOICEPUNCHER_BENCH_CORPUS", justfile_directory() / "e2e-test/corpus")

# Local speech recognition service (whisper-server) started by `start-services`; the app's default [transcription].base_url
stt_port := env_var_or_default("VOICEPUNCHER_STT_PORT", "8081")
stt_url := "http://127.0.0.1:" + stt_port + "/v1"
services_run_dir := home_directory() / ".voicepuncher/run"
services_log_dir := home_directory() / ".voicepuncher/logs"

# Homebrew OpenSSL headers — llama-cpp-sys fails with 'openssl/err.h' not found without them
export CPATH := env_var_or_default("CPATH", "/opt/homebrew/opt/openssl@3/include")
export LIBRARY_PATH := env_var_or_default("LIBRARY_PATH", "/opt/homebrew/opt/openssl@3/lib")

# List all recipes
default:
    @just --list --unsorted

# ── Checks ── `check` while developing, `full-check` before merging ─────────

# FAST (~4 min), run often while developing: code checks + E2E smoke + accuracy on 10 samples
check: fmt lint test build e2e-smoke e2e-quick

# FULL (~30 min), run before merging: clean build + code checks + E2E on the whole corpus
full-check: _require-models _require-corpus
    cargo clean
    just fmt lint test build e2e
    @echo "✓ full-check passed — before a release also walk through README.md § Release checklist (manual run)"

# Formatting check
fmt:
    cargo fmt --all -- --check

# Clippy on the whole workspace, warnings are errors
lint:
    cargo clippy --workspace --all-targets -- -D warnings

# Unit + architecture + integration tests (plain and .feature) of the app, unit tests of the E2E harness (~1 min)
test:
    cargo test -p voicepuncher --tests --no-fail-fast
    cargo test -p voicepuncher-e2e-test --lib

# Build the release binary
build:
    cargo build --release

# E2E on real models: smoke + WER accuracy on the whole corpus (~20 min)
e2e: e2e-smoke e2e-bench

# Smoke test — services and models load, file mode runs (@smoke scenarios), the adapter talks to the real service
e2e-smoke: _require-models build
    #!/usr/bin/env bash
    set -euo pipefail
    trap '{{just_executable()}} stop-services' EXIT
    {{just_executable()}} start-services
    export VOICEPUNCHER_TRANSCRIPTION_URL="{{stt_url}}"
    cargo test -p voicepuncher --test transcription_openai_it --release -- --nocapture
    cargo test -p voicepuncher-e2e-test --test cucumber_e2e --release -- --tags @smoke

# WER accuracy on 10 samples (5 per language) — fails if mean WER > 30 % (~1 min)
e2e-quick: _require-models _require-corpus build
    #!/usr/bin/env bash
    set -euo pipefail
    trap '{{just_executable()}} stop-services' EXIT
    {{just_executable()}} start-services
    VOICEPUNCHER_TRANSCRIPTION_URL="{{stt_url}}" VOICEPUNCHER_BENCH_LIMIT=10 \
        cargo test -p voicepuncher-e2e-test --test cucumber_e2e --release -- --tags @accuracy

# WER accuracy on the whole corpus — fails if mean WER > 30 % (~20 min)
e2e-bench: _require-models _require-corpus build
    #!/usr/bin/env bash
    set -euo pipefail
    trap '{{just_executable()}} stop-services' EXIT
    {{just_executable()}} start-services
    VOICEPUNCHER_TRANSCRIPTION_URL="{{stt_url}}" \
        cargo test -p voicepuncher-e2e-test --test cucumber_e2e --release -- --tags @accuracy

# Manual: @real_keyboard scenarios drive the real hotkey + paste — needs Accessibility and Input Monitoring for the terminal, a GUI session, hands off the keyboard
test-real-keyboard:
    VOICEPUNCHER_REAL_KEYBOARD_IT=1 cargo test -p voicepuncher --test cucumber_integration -- --tags @real_keyboard

# ── Release ── after the merge, on a clean checkout of origin/main ──────────

# Tag v<app version>, build, publish a GitHub Release with the binary tarball + sha256
release: _require-release-head test build
    #!/usr/bin/env bash
    set -euo pipefail
    version=$(sed -n 's/^version = "\(.*\)"/\1/p' app/Cargo.toml | head -1)
    tag="v$version"
    name="voicepuncher-$tag-aarch64-apple-darwin"
    dist=target/dist
    rm -rf "$dist" && mkdir -p "$dist/$name"
    cp target/release/voicepuncher README.md "$dist/$name/"
    tar -C "$dist" -czf "$dist/$name.tar.gz" "$name"
    (cd "$dist" && shasum -a 256 "$name.tar.gz" > "$name.tar.gz.sha256")
    git tag -a "$tag" -m "voicepuncher $tag"
    git push origin "$tag"
    gh release create "$tag" "$dist/$name.tar.gz" "$dist/$name.tar.gz.sha256" \
        --title "voicepuncher $tag" --generate-notes --verify-tag
    echo "✓ released $tag"

# ── Services ── local processes the app talks to over loopback ────────────────

# Start the speech recognition service (whisper-server) unless one already answers on its port
start-services:
    #!/usr/bin/env bash
    set -euo pipefail
    port="{{stt_port}}"
    pid_file="{{services_run_dir}}/whisper-server.pid"
    log_file="{{services_log_dir}}/whisper-server.log"
    port_open() { nc -z 127.0.0.1 "$port" >/dev/null 2>&1; }
    if ! command -v whisper-server >/dev/null 2>&1; then
        if [[ -t 0 ]]; then
            read -r -p "whisper-server not found. Install whisper-cpp with Homebrew? [y/N] " answer
            if [[ ! "$answer" =~ ^[Yy]$ ]]; then
                echo "✗ whisper-server not found — run 'brew install whisper-cpp'" >&2
                exit 1
            fi
            brew install whisper-cpp
        else
            echo "✗ whisper-server not found — run 'brew install whisper-cpp'" >&2
            exit 1
        fi
    fi
    if port_open; then
        echo "✓ speech recognition service already running at {{stt_url}}"
        exit 0
    fi
    model="${VOICEPUNCHER_WHISPER_MODEL:-}"
    if [[ -z "$model" || ! -f "$model" ]]; then
        echo "✗ VOICEPUNCHER_WHISPER_MODEL: no model file (got '$model'). Run 'just models' or set it in .env" >&2
        exit 1
    fi
    mkdir -p "{{services_run_dir}}" "{{services_log_dir}}"
    # Same decoding as the former in-process engine: greedy, no temperature fallback,
    # no cross-request text context, non-speech tokens suppressed
    nohup whisper-server -m "$model" --host 127.0.0.1 --port "$port" \
        --inference-path /v1/audio/transcriptions -l auto -t "$(sysctl -n hw.ncpu)" \
        -bo 1 -nf -sns -mc 0 -nt >"$log_file" 2>&1 &
    pid=$!
    echo "$pid" >"$pid_file"
    echo "… starting whisper-server (pid $pid, model $(basename "$model")), log: $log_file"
    for _ in $(seq 1 240); do
        if port_open; then
            echo "✓ speech recognition service ready at {{stt_url}}"
            exit 0
        fi
        if ! kill -0 "$pid" 2>/dev/null; then
            echo "✗ whisper-server exited during startup — last log lines:" >&2
            tail -n 20 "$log_file" >&2
            rm -f "$pid_file"
            exit 1
        fi
        sleep 0.25
    done
    echo "✗ whisper-server did not open port $port within 60 s — see $log_file" >&2
    kill "$pid" 2>/dev/null || true
    rm -f "$pid_file"
    exit 1

# Stop the services that start-services launched; services started by hand keep running
stop-services:
    #!/usr/bin/env bash
    set -euo pipefail
    pid_file="{{services_run_dir}}/whisper-server.pid"
    if [[ ! -f "$pid_file" ]]; then
        echo "· no speech recognition service started by start-services"
        exit 0
    fi
    pid="$(cat "$pid_file")"
    if ps -p "$pid" -o comm= 2>/dev/null | grep -q whisper-server; then
        kill "$pid"
        for _ in $(seq 1 40); do
            kill -0 "$pid" 2>/dev/null || break
            sleep 0.25
        done
        kill -0 "$pid" 2>/dev/null && kill -9 "$pid"
        echo "✓ stopped whisper-server (pid $pid)"
    else
        echo "· whisper-server (pid $pid) is no longer running"
    fi
    rm -f "$pid_file"

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

_require-release-head:
    #!/usr/bin/env bash
    set -euo pipefail
    git fetch --tags origin
    if [[ -n "$(git status --porcelain)" ]]; then
        echo "✗ working tree is dirty — release only from a clean checkout" >&2; exit 1
    fi
    if [[ "$(git rev-parse HEAD)" != "$(git rev-parse origin/main)" ]]; then
        echo "✗ HEAD is not origin/main. Run: git fetch origin && git switch --detach origin/main" >&2; exit 1
    fi
    version=$(sed -n 's/^version = "\(.*\)"/\1/p' app/Cargo.toml | head -1)
    if git rev-parse -q --verify "refs/tags/v$version" >/dev/null; then
        echo "✗ tag v$version already exists — bump version in app/Cargo.toml first" >&2; exit 1
    fi
    echo "release: v$version from $(git rev-parse --short HEAD)"

_require-corpus:
    #!/usr/bin/env bash
    set -euo pipefail
    if [[ ! -f "$VOICEPUNCHER_BENCH_CORPUS/manifest.jsonl" ]]; then
        echo "✗ no corpus at $VOICEPUNCHER_BENCH_CORPUS. Run 'just corpus' or set VOICEPUNCHER_BENCH_CORPUS" >&2
        exit 1
    fi
