set dotenv-load

# ── Settings ── override any of these via env or .env ─────────────────────────

# Models: explicit env var wins; otherwise the largest model found in ~/.voicepuncher or ~/.voicepuncher/models
export VOICEPUNCHER_WHISPER_MODEL := env_var_or_default("VOICEPUNCHER_WHISPER_MODEL", `find ~/.voicepuncher -maxdepth 2 -name 'ggml-*.bin' -exec ls -S {} + 2>/dev/null | head -1 || true`)
export VOICEPUNCHER_LLM_MODEL := env_var_or_default("VOICEPUNCHER_LLM_MODEL", `find ~/.voicepuncher -maxdepth 2 -name '*.gguf' -exec ls -S {} + 2>/dev/null | head -1 || true`)

# Corpus directory containing manifest.jsonl — absolute, because cargo runs tests from e2e-test/
export VOICEPUNCHER_BENCH_CORPUS := env_var_or_default("VOICEPUNCHER_BENCH_CORPUS", justfile_directory() / "e2e-test/corpus")

# Local services started by `start-model-servers`; their URLs are the app's default [transcription].base_url and [refinement].base_url
stt_port := env_var_or_default("VOICEPUNCHER_STT_PORT", "8081")
stt_url := "http://127.0.0.1:" + stt_port + "/v1"
llm_port := env_var_or_default("VOICEPUNCHER_LLM_PORT", "8080")
llm_url := "http://127.0.0.1:" + llm_port + "/v1"
services_run_dir := home_directory() / ".voicepuncher/run"
services_log_dir := home_directory() / ".voicepuncher/logs"

# List all recipes
default:
    @just --list --unsorted

# ── Checks ── `check` while developing, `full-check` before merging ─────────

# FAST (~6 min), run often while developing: code checks + E2E smoke + accuracy on 10 samples
check: fmt lint test build e2e-smoke e2e-quick

# FULL (~35 min), run before merging: clean build + code checks + E2E (seams included) on the whole corpus
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

# E2E on real models: smoke + chunk seams + WER accuracy on the whole corpus (~25 min)
e2e: e2e-smoke e2e-seams e2e-bench

# Smoke test — services and models load, file mode runs (@smoke scenarios), the adapters talk to the real services
e2e-smoke: _require-models build
    #!/usr/bin/env bash
    set -euo pipefail
    trap '{{just_executable()}} _stop-model-servers-for-e2e' EXIT
    {{just_executable()}} _start-model-servers-for-e2e
    export VOICEPUNCHER_TRANSCRIPTION_URL="{{stt_url}}" VOICEPUNCHER_REFINEMENT_URL="{{llm_url}}"
    cargo test -p voicepuncher --test transcription_openai_it --test textrefinement_openai_it --test chunked_transcription_it --release -- --nocapture
    cargo test -p voicepuncher-e2e-test --test cucumber_e2e --release -- --tags @smoke

# WER accuracy on 10 samples (5 per language) — fails if mean WER > 30 % (~1 min)
e2e-quick: _require-models _require-corpus build
    #!/usr/bin/env bash
    set -euo pipefail
    trap '{{just_executable()}} _stop-model-servers-for-e2e' EXIT
    {{just_executable()}} _start-model-servers-for-e2e
    VOICEPUNCHER_TRANSCRIPTION_URL="{{stt_url}}" VOICEPUNCHER_REFINEMENT_URL="{{llm_url}}" VOICEPUNCHER_BENCH_LIMIT=10 \
        cargo test -p voicepuncher-e2e-test --test cucumber_e2e --release -- --tags @accuracy

# Chunk seams: Russian and English speech under 11 recording conditions, transcribed in chunks — fails on a phrase repeated at a seam or > 10 % missing words (~3 min)
e2e-seams: _require-models build
    #!/usr/bin/env bash
    set -euo pipefail
    trap '{{just_executable()}} _stop-model-servers-for-e2e' EXIT
    {{just_executable()}} _start-model-servers-for-e2e
    VOICEPUNCHER_TRANSCRIPTION_URL="{{stt_url}}" \
        cargo test -p voicepuncher-e2e-test --test cucumber_e2e --release -- --tags @seams

# WER accuracy on the whole corpus — fails if mean WER > 30 % (~20 min)
e2e-bench: _require-models _require-corpus build
    #!/usr/bin/env bash
    set -euo pipefail
    trap '{{just_executable()}} _stop-model-servers-for-e2e' EXIT
    {{just_executable()}} _start-model-servers-for-e2e
    VOICEPUNCHER_TRANSCRIPTION_URL="{{stt_url}}" VOICEPUNCHER_REFINEMENT_URL="{{llm_url}}" \
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

# Start the services for an E2E recipe; a service that already runs (yours) is reused and left running
_start-model-servers-for-e2e:
    #!/usr/bin/env bash
    set -euo pipefail
    mkdir -p "{{services_run_dir}}"
    for service in whisper-server:{{stt_port}} llama-server:{{llm_port}}; do
        if nc -z 127.0.0.1 "${service#*:}" >/dev/null 2>&1; then
            echo "✓ reusing ${service%%:*} already running on port ${service#*:}; it stays running"
        else
            touch "{{services_run_dir}}/${service%%:*}-${service#*:}.e2e"
        fi
    done
    {{just_executable()}} start-model-servers

# Stop the services that _start-model-servers-for-e2e started in this run
_stop-model-servers-for-e2e:
    #!/usr/bin/env bash
    set -euo pipefail
    for service in whisper-server:{{stt_port}} llama-server:{{llm_port}}; do
        marker="{{services_run_dir}}/${service%%:*}-${service#*:}.e2e"
        if [[ -f "$marker" ]]; then
            rm -f "$marker"
            {{just_executable()}} stop-model-servers "${service%%:*}"
        fi
    done

# Start the speech recognition (whisper-server) and text refinement (llama-server) services; one that already answers on its port is skipped
start-model-servers:
    #!/usr/bin/env bash
    set -euo pipefail
    run_dir="{{services_run_dir}}"
    log_dir="{{services_log_dir}}"
    mkdir -p "$run_dir" "$log_dir"
    port_open() { nc -z 127.0.0.1 "$1" >/dev/null 2>&1; }
    whisper_ready() { port_open "{{stt_port}}"; }
    llama_ready() { curl -sf -o /dev/null --max-time 1 "http://127.0.0.1:{{llm_port}}/health"; }
    require_binary() {
        local binary=$1 formula=$2
        command -v "$binary" >/dev/null 2>&1 && return 0
        if [[ -t 0 ]]; then
            read -r -p "$binary not found. Install $formula with Homebrew? [y/N] " answer
            if [[ "$answer" =~ ^[Yy]$ ]]; then
                brew install "$formula"
                return 0
            fi
        fi
        echo "✗ $binary not found — run 'brew install $formula'" >&2
        exit 1
    }
    require_model() {
        local path="${!1:-}"
        if [[ -z "$path" || ! -f "$path" ]]; then
            echo "✗ $1: no model file (got '$path'). Run 'just models' or set it in .env" >&2
            exit 1
        fi
    }
    # start_service <binary> <port> <label> <readiness check> <command…>
    start_service() {
        local binary=$1 port=$2 label=$3 ready=$4
        shift 4
        local pid_file="$run_dir/$binary-$port.pid" log_file="$log_dir/$binary-$port.log"
        nohup "$@" >"$log_file" 2>&1 &
        local pid=$!
        echo "$pid" >"$pid_file"
        echo "… starting $binary (pid $pid), log: $log_file"
        for _ in $(seq 1 240); do
            if "$ready"; then
                echo "✓ $label ready at http://127.0.0.1:$port/v1"
                return 0
            fi
            if ! kill -0 "$pid" 2>/dev/null; then
                echo "✗ $binary exited during startup — last log lines:" >&2
                tail -n 20 "$log_file" >&2
                rm -f "$pid_file"
                exit 1
            fi
            sleep 0.25
        done
        echo "✗ $binary was not ready on port $port within 60 s — see $log_file" >&2
        kill "$pid" 2>/dev/null || true
        rm -f "$pid_file"
        exit 1
    }
    require_binary whisper-server whisper-cpp
    require_binary llama-server llama.cpp
    if port_open "{{stt_port}}"; then
        echo "✓ speech recognition service already running at {{stt_url}}"
    else
        require_model VOICEPUNCHER_WHISPER_MODEL
        # Same decoding as the former in-process engine: greedy, no temperature fallback,
        # no cross-request text context, non-speech tokens suppressed
        start_service whisper-server "{{stt_port}}" "speech recognition service" whisper_ready \
            whisper-server -m "$VOICEPUNCHER_WHISPER_MODEL" --host 127.0.0.1 --port "{{stt_port}}" \
            --inference-path /v1/audio/transcriptions -l auto -t "$(sysctl -n hw.ncpu)" \
            -bo 1 -nf -sns -mc 0 -nt
    fi
    if port_open "{{llm_port}}"; then
        echo "✓ text refinement service already running at {{llm_url}}"
    else
        require_model VOICEPUNCHER_LLM_MODEL
        # One slot, so a request gets the whole context, as the former in-process engine did
        start_service llama-server "{{llm_port}}" "text refinement service" llama_ready \
            llama-server -m "$VOICEPUNCHER_LLM_MODEL" --host 127.0.0.1 --port "{{llm_port}}" -c 4096 -np 1
    fi

# Stop the services that start-model-servers launched (all, or the named ones: whisper-server, llama-server); services started by hand keep running
stop-model-servers *services:
    #!/usr/bin/env bash
    set -euo pipefail
    stop_service() {
        local binary=$1 port=$2 label=$3
        local pid_file="{{services_run_dir}}/$binary-$port.pid"
        if [[ ! -f "$pid_file" ]]; then
            echo "· no $label started by start-model-servers"
            return 0
        fi
        local pid
        pid="$(cat "$pid_file")"
        if ps -p "$pid" -o comm= 2>/dev/null | grep -q "$binary"; then
            kill "$pid"
            for _ in $(seq 1 40); do
                kill -0 "$pid" 2>/dev/null || break
                sleep 0.25
            done
            kill -0 "$pid" 2>/dev/null && kill -9 "$pid"
            echo "✓ stopped $binary (pid $pid)"
        else
            echo "· $binary (pid $pid) is no longer running"
        fi
        rm -f "$pid_file"
    }
    selected="{{services}}"
    for service in ${selected:-whisper-server llama-server}; do
        case "$service" in
            whisper-server) stop_service whisper-server "{{stt_port}}" "speech recognition service" ;;
            llama-server) stop_service llama-server "{{llm_port}}" "text refinement service" ;;
            *) echo "✗ unknown service '$service' — use whisper-server or llama-server" >&2; exit 1 ;;
        esac
    done

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
