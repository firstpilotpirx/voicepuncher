set dotenv-load

# ── Settings ── override any of these via env or .env ─────────────────────────

# Models: explicit env var wins; otherwise the largest model found in ~/.voicepuncher or ~/.voicepuncher/models
export VOICEPUNCHER_WHISPER_MODEL := env_var_or_default("VOICEPUNCHER_WHISPER_MODEL", `find ~/.voicepuncher -maxdepth 2 -name 'ggml-*.bin' ! -name 'ggml-silero-*' -exec ls -S {} + 2>/dev/null | head -1 || true`)
# Voice activity detection model of the speech recognition service: the newest Silero model found
export VOICEPUNCHER_VAD_MODEL := env_var_or_default("VOICEPUNCHER_VAD_MODEL", `find ~/.voicepuncher -maxdepth 2 -name 'ggml-silero-*.bin' -exec ls -t {} + 2>/dev/null | head -1 || true`)
export VOICEPUNCHER_LLM_MODEL := env_var_or_default("VOICEPUNCHER_LLM_MODEL", `find ~/.voicepuncher -maxdepth 2 -name '*.gguf' -exec ls -S {} + 2>/dev/null | head -1 || true`)
# Command spotting service (voice control): sherpa-onnx keyword spotting model directory and its Silero VAD
export VOICEPUNCHER_KWS_MODEL_DIR := env_var_or_default("VOICEPUNCHER_KWS_MODEL_DIR", `find ~/.voicepuncher -maxdepth 2 -type d -name 'sherpa-onnx-kws-*' 2>/dev/null | head -1 || true`)
# The user's own recorded voice commands (never committed): tests run on them when they exist
export VOICEPUNCHER_VOICE_SAMPLES := env_var_or_default("VOICEPUNCHER_VOICE_SAMPLES", `test -d ~/.voicepuncher/voice-samples && echo ~/.voicepuncher/voice-samples || true`)
export VOICEPUNCHER_KWS_VAD_MODEL := env_var_or_default("VOICEPUNCHER_KWS_VAD_MODEL", `find ~/.voicepuncher -maxdepth 2 -name 'silero_vad.onnx' 2>/dev/null | head -1 || true`)

# Corpus directory containing manifest.jsonl — absolute, because cargo runs tests from e2e-test/
export VOICEPUNCHER_BENCH_CORPUS := env_var_or_default("VOICEPUNCHER_BENCH_CORPUS", justfile_directory() / "e2e-test/corpus")

# Local services started by `start-model-servers`; their URLs are the app's default [transcription].base_url and [refinement].base_url
stt_port := env_var_or_default("VOICEPUNCHER_STT_PORT", "8081")
stt_url := "http://127.0.0.1:" + stt_port + "/v1"
# Copies of the speech recognition service behind HAProxy on its port: whisper-server serves one
# request at a time, and two to four copies on one GPU are twice as fast on long speech
stt_instances := env_var_or_default("VOICEPUNCHER_STT_INSTANCES", "3")
llm_port := env_var_or_default("VOICEPUNCHER_LLM_PORT", "8080")
llm_url := "http://127.0.0.1:" + llm_port + "/v1"
kws_port := env_var_or_default("VOICEPUNCHER_KWS_PORT", "8082")
kws_url := "ws://127.0.0.1:" + kws_port
kws_binary := justfile_directory() / "target/release/voicepuncher-command-spotting"
services_run_dir := home_directory() / ".voicepuncher/run"
services_log_dir := home_directory() / ".voicepuncher/logs"

# Every check, test and benchmark recipe runs its private `_<name>` body under the machine-wide test lock
# (agent-kit `with-machine-lock`): one run at a time on this machine, a hung run is killed after its time limit
machine_lock := env_var_or_default("MACHINE_LOCK", home_directory() / ".claude/bin/with-machine-lock")
locked := quote(machine_lock) + " " + quote(just_executable()) + " --justfile " + quote(justfile())

# List all recipes
default:
    @just --list --unsorted

# ── Checks ── `check` while developing, `full-check` before merging ─────────

# FAST (~6 min), run often while developing: code checks + E2E smoke + accuracy on 10 samples + refinement evaluation
check:
    @{{locked}} _check

_check:
    {{just_executable()}} _stages fmt lint test build e2e-smoke e2e-quick refine-eval

# FULL (~10 min), run before merging: clean build + code checks + E2E (seams included) + accuracy on 100 samples
full-check:
    @{{locked}} _full-check

_full-check: _require-models _require-corpus
    cargo clean
    {{just_executable()}} _stages fmt lint test build e2e-smoke e2e-seams e2e-bench refine-eval
    @echo "✓ full-check passed — before a release also walk through README.md § Release checklist (manual run)"

# Runs the named stages (private recipes without their `_`) one by one: the services start once,
# before the first stage that needs them, and stop at the end; ends with each stage's time
_stages +stages:
    #!/usr/bin/env bash
    set -euo pipefail
    now() { perl -MTime::HiRes=time -e 'printf "%.1f", time'; }
    started=$(now)
    report=""
    services=""
    finish() {
        code=$?
        if [[ -n "$services" ]]; then
            unset VOICEPUNCHER_E2E_SERVICES_HELD
            {{just_executable()}} _stop-model-servers-for-e2e || true
        fi
        printf '\nstages:\n%b' "$report"
        printf '  %-12s %7ss\n' total "$(perl -e "printf '%.1f', $(now) - $started")"
        exit "$code"
    }
    trap finish EXIT
    for stage in {{stages}}; do
        if [[ -z "$services" && ( "$stage" == e2e* || "$stage" == refine-eval ) ]]; then
            {{just_executable()}} _start-model-servers-for-e2e
            services=up
            export VOICEPUNCHER_E2E_SERVICES_HELD=1
        fi
        t0=$(now)
        status=0
        {{just_executable()}} "_$stage" || status=$?
        mark="✓"
        [[ $status -eq 0 ]] || mark="✗"
        report+="  $mark $(printf '%-10s %7ss' "$stage" "$(perl -e "printf '%.1f', $(now) - $t0")")\n"
        [[ $status -eq 0 ]] || exit "$status"
    done

# Formatting check
fmt:
    @{{locked}} _fmt

_fmt:
    cargo fmt --all -- --check

# Clippy on the whole workspace, warnings are errors
lint:
    @{{locked}} _lint

_lint:
    cargo clippy --workspace --all-targets -- -D warnings

# Unit + architecture + integration tests (plain and .feature) of the app, tests of the command spotting service, unit tests of the E2E harness, every test binary at once (~20 s; ~1 min on a cold build)
test:
    @{{locked}} _test

_test:
    #!/usr/bin/env bash
    set -euo pipefail
    # One build per selection, then every test binary at once, each in its package directory
    # as cargo runs it; a binary's output is printed whole when it ends.
    root=$PWD
    tab=$'\t'
    runs=$(mktemp -d)
    trap 'kill $(jobs -p) 2>/dev/null || true; rm -rf "$runs"' EXIT
    now() { perl -MTime::HiRes=time -e 'printf "%.1f", time'; }
    started=$(now)
    for selection in "app -p voicepuncher --tests" "command-spotting -p voicepuncher-command-spotting" "e2e-test -p voicepuncher-e2e-test --lib"; do
        set -- $selection
        dir=$1
        shift
        cargo test "$@" --no-run 2>"$runs/build.log" || { cat "$runs/build.log" >&2; exit 101; }
        grep -v '^ *Executable ' "$runs/build.log" >&2 || true
        sed -n "s|^ *Executable \(.*\) (\(.*\))$|$dir$tab\1$tab\2|p" "$runs/build.log" >>"$runs/binaries"
    done
    total=0
    while IFS=$'\t' read -r dir source executable; do
        total=$((total + 1))
        (
            cd "$dir"
            t0=$(now)
            code=0
            "$root/$executable" >"$runs/$total.out" 2>&1 || code=$?
            echo "$code $(perl -e "printf '%.1f', $(now) - $t0")" >"$runs/$total.tmp"
            mv "$runs/$total.tmp" "$runs/$total.done"
        ) &
    done <"$runs/binaries"
    finished=0
    failed=()
    while [ "$finished" -lt "$total" ]; do
        for n in $(seq 1 "$total"); do
            [ -f "$runs/$n.done" ] && [ ! -f "$runs/$n.shown" ] || continue
            touch "$runs/$n.shown"
            finished=$((finished + 1))
            read -r code secs <"$runs/$n.done"
            IFS=$'\t' read -r dir source executable <<<"$(sed -n "${n}p" "$runs/binaries")"
            printf '\n     Running %s (%s)\n' "$source" "$executable"
            cat "$runs/$n.out"
            echo "$secs $dir/$source" >>"$runs/times"
            [ "$code" = 0 ] || failed+=("$dir/$source")
        done
        sleep 0.2
    done
    printf '\nslowest test binaries:\n'
    sort -rn "$runs/times" | head -5 | while read -r secs name; do printf '  %6ss  %s\n' "$secs" "$name"; done
    elapsed=$(perl -e "printf '%.0f', $(now) - $started")
    if [ "${#failed[@]}" -gt 0 ]; then
        echo "error: ${#failed[@]} target(s) failed:"
        printf '    `%s`\n' "${failed[@]}"
        exit 101
    fi
    echo "✓ $total test binaries passed in ${elapsed} s"

# Run the app for real use with one command: download missing models, restart the three services (the command spotting service is rebuilt), build and start the app with voice control; extra arguments go to the app (`just run --no-llm`)
run *args: models
    just stop-model-servers
    just start-model-servers
    cargo run --release -p voicepuncher -- --voice-control {{args}}

# Build the release binary
build:
    @{{locked}} _build

_build:
    cargo build --release

# E2E on real models: smoke + chunk seams + WER accuracy on 100 samples + refinement evaluation (~8 min)
e2e:
    @{{locked}} _e2e

_e2e: _e2e-smoke _e2e-seams _e2e-bench _refine-eval

# Smoke test — services and models load, file mode runs and voice commands are spotted (@smoke scenarios), the adapters talk to the real services
e2e-smoke:
    @{{locked}} _e2e-smoke

_e2e-smoke: _require-models _build
    #!/usr/bin/env bash
    set -euo pipefail
    trap '{{just_executable()}} _stop-model-servers-for-e2e' EXIT
    {{just_executable()}} _start-model-servers-for-e2e
    export VOICEPUNCHER_TRANSCRIPTION_URL="{{stt_url}}" VOICEPUNCHER_REFINEMENT_URL="{{llm_url}}" VOICEPUNCHER_COMMAND_SPOTTING_URL="{{kws_url}}"
    cargo test -p voicepuncher --test transcription_openai_it --test textrefinement_openai_it --test commandspotting_websocket_it --test voice_samples_it --test voice_forms_it -- --nocapture
    cargo test -p voicepuncher-command-spotting --test placed_commands_it
    cargo test -p voicepuncher-e2e-test --test cucumber_e2e -- --tags '@smoke and not @wip'

# WER accuracy on 10 samples (5 per language) — fails if mean WER > 30 %, and no command in 20 recordings of ordinary speech (~1 min)
e2e-quick:
    @{{locked}} _e2e-quick

_e2e-quick: _require-models _require-corpus _build
    #!/usr/bin/env bash
    set -euo pipefail
    trap '{{just_executable()}} _stop-model-servers-for-e2e' EXIT
    {{just_executable()}} _start-model-servers-for-e2e
    VOICEPUNCHER_TRANSCRIPTION_URL="{{stt_url}}" VOICEPUNCHER_REFINEMENT_URL="{{llm_url}}" VOICEPUNCHER_COMMAND_SPOTTING_URL="{{kws_url}}" VOICEPUNCHER_BENCH_LIMIT=10 \
        cargo test -p voicepuncher-e2e-test --test cucumber_e2e -- --tags '@accuracy and not @wip'

# Text refinement on the live services: dictated commands, normalization, evaluation per category (@refinement) (~30 s)
refine-eval:
    @{{locked}} _refine-eval

_refine-eval: _require-models _build
    #!/usr/bin/env bash
    set -euo pipefail
    trap '{{just_executable()}} _stop-model-servers-for-e2e' EXIT
    {{just_executable()}} _start-model-servers-for-e2e
    VOICEPUNCHER_TRANSCRIPTION_URL="{{stt_url}}" VOICEPUNCHER_REFINEMENT_URL="{{llm_url}}" \
        cargo test -p voicepuncher-e2e-test --test cucumber_e2e -- --tags '@refinement and not @wip'

# Chunk seams: the seam sweep (speech after different lead-ins) and Russian and English speech under 11 recording conditions, transcribed in chunks — fails on a phrase repeated at a seam or > 10 % missing words (~2.5 min)
e2e-seams:
    @{{locked}} _e2e-seams

_e2e-seams: _require-models _build
    #!/usr/bin/env bash
    set -euo pipefail
    trap '{{just_executable()}} _stop-model-servers-for-e2e' EXIT
    {{just_executable()}} _start-model-servers-for-e2e
    export VOICEPUNCHER_TRANSCRIPTION_URL="{{stt_url}}"
    cargo test -p voicepuncher --test chunked_transcription_it -- --nocapture
    cargo test -p voicepuncher-e2e-test --test cucumber_e2e -- --tags '@seams and not @wip'

# WER accuracy on a fixed sample of 100 recordings (50 per language), two at a time — fails if mean WER > 30 %, and no command in 20 recordings of ordinary speech (~2.5 min)
e2e-bench:
    @{{locked}} _e2e-bench

_e2e-bench: _require-models _require-corpus _build
    #!/usr/bin/env bash
    set -euo pipefail
    trap '{{just_executable()}} _stop-model-servers-for-e2e' EXIT
    {{just_executable()}} _start-model-servers-for-e2e
    VOICEPUNCHER_TRANSCRIPTION_URL="{{stt_url}}" VOICEPUNCHER_REFINEMENT_URL="{{llm_url}}" VOICEPUNCHER_COMMAND_SPOTTING_URL="{{kws_url}}" \
        cargo test -p voicepuncher-e2e-test --test cucumber_e2e -- --tags '@accuracy and not @wip'

# Manual: @real_keyboard scenarios drive the real hotkey + paste — needs Accessibility and Input Monitoring for the terminal, a GUI session, hands off the keyboard
test-real-keyboard:
    @{{locked}} _test-real-keyboard

_test-real-keyboard:
    VOICEPUNCHER_REAL_KEYBOARD_IT=1 cargo test -p voicepuncher --test cucumber_integration -- --tags @real_keyboard

# ── Release ── after the merge, on a clean checkout of origin/main ──────────

# Tag v<app version>, build, publish a GitHub Release with the binary tarball + sha256
release:
    @{{locked}} _release

_release: _require-release-head _test _build
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

# Start the services for an E2E recipe; a service that already runs (yours) is reused and left running.
# Inside check and full-check, which hold the services for all their stages, it does nothing
_start-model-servers-for-e2e:
    #!/usr/bin/env bash
    set -euo pipefail
    [[ -z "${VOICEPUNCHER_E2E_SERVICES_HELD:-}" ]] || exit 0
    mkdir -p "{{services_run_dir}}"
    for service in whisper-server:{{stt_port}} llama-server:{{llm_port}} voicepuncher-command-spotting:{{kws_port}}; do
        if nc -z 127.0.0.1 "${service#*:}" >/dev/null 2>&1; then
            echo "✓ reusing ${service%%:*} already running on port ${service#*:}; it stays running"
        else
            touch "{{services_run_dir}}/${service%%:*}-${service#*:}.e2e"
        fi
    done
    {{just_executable()}} start-model-servers

# Stop the services that _start-model-servers-for-e2e started in this run (nothing while check or full-check holds them)
_stop-model-servers-for-e2e:
    #!/usr/bin/env bash
    set -euo pipefail
    [[ -z "${VOICEPUNCHER_E2E_SERVICES_HELD:-}" ]] || exit 0
    for service in whisper-server:{{stt_port}} llama-server:{{llm_port}} voicepuncher-command-spotting:{{kws_port}}; do
        marker="{{services_run_dir}}/${service%%:*}-${service#*:}.e2e"
        if [[ -f "$marker" ]]; then
            rm -f "$marker"
            {{just_executable()}} stop-model-servers "${service%%:*}"
        fi
    done

# Start the speech recognition (whisper-server), text refinement (llama-server) and command spotting (voicepuncher-command-spotting) services, or only the named ones; one that already answers on its port is skipped
start-model-servers *services:
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
    require_model_dir() {
        local path="${!1:-}"
        if [[ -z "$path" || ! -d "$path" ]]; then
            echo "✗ $1: no model directory (got '$path'). Run 'just models' or set it in .env" >&2
            exit 1
        fi
    }
    # supervise <log file> <command…>: runs the service and starts it again whenever it exits,
    # after 1 s, doubling up to 30 s while it keeps exiting within 30 s of its start.
    # TERM or INT stops the service and the supervisor, without a restart
    supervise() {
        local log_file=$1 delay=1 started code
        shift
        child=""
        trap '[[ -z "$child" ]] || kill "$child" 2>/dev/null; wait "$child" 2>/dev/null; exit 0' TERM INT
        while true; do
            started=$SECONDS
            "$@" >>"$log_file" 2>&1 &
            child=$!
            wait "$child" && code=0 || code=$?
            (( SECONDS - started < 30 )) || delay=1
            echo "… $1 exited with code $code — restarting in $delay s" >>"$log_file"
            sleep "$delay" &
            child=$!
            wait "$child"
            delay=$(( delay * 2 > 30 ? 30 : delay * 2 ))
        done
    }
    # start_service <binary> <port> <label> <readiness check> <command…>: starts the service
    # under its supervisor; the PID file holds the supervisor's PID
    start_service() {
        local binary=$1 port=$2 label=$3 ready=$4
        shift 4
        local pid_file="$run_dir/$binary-$port.pid" log_file="$log_dir/$binary-$port.log"
        : >"$log_file"
        nohup bash -c "$(declare -f supervise); supervise \"\$@\"" "supervise-$binary" "$log_file" "$@" >/dev/null 2>&1 &
        local pid=$!
        echo "$pid" >"$pid_file"
        echo "… starting $binary (pid $pid), log: $log_file"
        for _ in $(seq 1 240); do
            if $ready; then
                echo "✓ $label ready on port $port"
                return 0
            fi
            # A service that exits before it was ever ready cannot start: stop its supervisor
            # instead of letting it retry
            if ! kill -0 "$pid" 2>/dev/null || grep -q " exited with code " "$log_file"; then
                kill "$pid" 2>/dev/null || true
                echo "✗ $binary exited during startup (log: $log_file) — last log lines:" >&2
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
    # whisper_args <port> <threads>: the whisper-server command line, in whisper_cmd.
    # Greedy, no temperature fallback, no cross-request text context, non-speech tokens
    # suppressed; timestamps stay on, so whisper resumes after the last decoded segment
    # instead of skipping the rest of a 30 s window when the text ends early at a pause.
    # VAD removes only silences longer than 2 s, so silence yields no text ("Thank you.")
    # while the short pauses between words stay. No language probabilities (-nlp): with them,
    # whisper-server 1.9.5 crashes on a verbose_json request whose audio VAD leaves empty,
    # as the command spotting service's confirmation of an utterance without speech
    whisper_args() {
        whisper_cmd=(whisper-server -m "$VOICEPUNCHER_WHISPER_MODEL" --host 127.0.0.1 --port "$1"
            --inference-path /v1/audio/transcriptions -l auto -t "$2"
            -bo 1 -nf -sns -mc 0 --vad -vm "$VOICEPUNCHER_VAD_MODEL" -vsd 2000 -vp 500 -nlp)
    }
    # haproxy_config <copy port…>: one address for the copies. A request goes to a free copy
    # (leastconn, one request per copy) or waits in the queue; short audio (under 160 kB,
    # about 5 s: a command confirmation) waits ahead of long dictation chunks. A copy whose
    # port does not answer — at the first refused connection, or by the health check — gets no
    # requests while its supervisor restarts it, and a request that could not reach a copy is
    # retried on another one
    haproxy_config() {
        printf '%s\n' "global" "    maxconn 256" "defaults" "    mode http" \
            "    timeout connect 2s" "    timeout client 300s" "    timeout server 300s" \
            "    timeout queue 300s" "    option http-server-close" "    retries 3" "    option redispatch 1" \
            "frontend speech_recognition" "    bind 127.0.0.1:{{stt_port}}" \
            "    http-request set-priority-class int(-1) if { req.hdr_val(content-length) -m int lt 160000 }" \
            "    default_backend copies" "backend copies" "    balance leastconn"
        local i=0 port
        for port in "$@"; do
            i=$((i + 1))
            echo "    server copy$i 127.0.0.1:$port maxconn 1 check inter 1s fall 2 rise 1 observe layer4 error-limit 1 on-error mark-down"
        done
    }
    start_whisper() {
        require_binary whisper-server whisper-cpp
        if port_open "{{stt_port}}"; then
            echo "✓ speech recognition service already running at {{stt_url}}"
            return 0
        fi
        require_model VOICEPUNCHER_WHISPER_MODEL
        require_model VOICEPUNCHER_VAD_MODEL
        local copies={{stt_instances}} cores
        cores=$(sysctl -n hw.ncpu)
        if (( copies <= 1 )); then
            # A copy list left from an earlier start would make the stop look for copies
            rm -f "$run_dir/whisper-server-{{stt_port}}.copies" "$run_dir/haproxy-{{stt_port}}.cfg"
            whisper_args "{{stt_port}}" "$cores"
            start_service whisper-server "{{stt_port}}" "speech recognition service" whisper_ready "${whisper_cmd[@]}"
            return 0
        fi
        require_binary haproxy haproxy
        local threads=$(( cores / copies > 0 ? cores / copies : 1 ))
        local base=$(( {{stt_port}} + 10000 + copies <= 65535 ? {{stt_port}} + 10000 : {{stt_port}} - 10000 ))
        local list="$run_dir/whisper-server-{{stt_port}}.copies" config="$run_dir/haproxy-{{stt_port}}.cfg"
        local ports=() i
        for i in $(seq 1 "$copies"); do ports+=($(( base + i ))); done
        echo "${ports[*]}" >"$list"
        # A copy or HAProxy that cannot start stops what this start already started
        trap 'code=$?; (( code == 0 )) || {{quote(just_executable())}} --justfile {{quote(justfile())}} stop-model-servers whisper-server >/dev/null 2>&1 || true' EXIT
        for i in "${!ports[@]}"; do
            whisper_args "${ports[$i]}" "$threads"
            start_service whisper-server "${ports[$i]}" "speech recognition copy $((i + 1)) of $copies" \
                "port_open ${ports[$i]}" "${whisper_cmd[@]}"
        done
        haproxy_config "${ports[@]}" >"$config"
        haproxy -c -q -f "$config"
        start_service haproxy "{{stt_port}}" "speech recognition service ($copies copies)" whisper_ready \
            haproxy -db -f "$config"
        trap - EXIT
    }
    start_llama() {
        require_binary llama-server llama.cpp
        if port_open "{{llm_port}}"; then
            echo "✓ text refinement service already running at {{llm_url}}"
            return 0
        fi
        require_model VOICEPUNCHER_LLM_MODEL
        # One slot, so a request gets the whole context, as the former in-process engine did
        start_service llama-server "{{llm_port}}" "text refinement service" llama_ready \
            llama-server -m "$VOICEPUNCHER_LLM_MODEL" --host 127.0.0.1 --port "{{llm_port}}" -c 4096 -np 1
    }
    kws_ready() { port_open "{{kws_port}}"; }
    start_command_spotting() {
        if port_open "{{kws_port}}"; then
            echo "✓ command spotting service already running at {{kws_url}}"
            return 0
        fi
        require_model_dir VOICEPUNCHER_KWS_MODEL_DIR
        require_model VOICEPUNCHER_KWS_VAD_MODEL
        cargo build --release -p voicepuncher-command-spotting
        start_service voicepuncher-command-spotting "{{kws_port}}" "command spotting service" kws_ready \
            "{{kws_binary}}" --port "{{kws_port}}" \
            --model-dir "$VOICEPUNCHER_KWS_MODEL_DIR" --vad-model "$VOICEPUNCHER_KWS_VAD_MODEL" \
            --transcription-url "{{stt_url}}" \
            ${VOICEPUNCHER_KWS_SAVE_SPEECH_DIR:+--save-speech-dir "$VOICEPUNCHER_KWS_SAVE_SPEECH_DIR"}
    }
    selected="{{services}}"
    for service in ${selected:-whisper-server llama-server voicepuncher-command-spotting}; do
        case "$service" in
            whisper-server) start_whisper ;;
            llama-server) start_llama ;;
            voicepuncher-command-spotting) start_command_spotting ;;
            *) echo "✗ unknown service '$service' — use whisper-server, llama-server or voicepuncher-command-spotting" >&2; exit 1 ;;
        esac
    done

# Stop the services that start-model-servers launched (all, or the named ones: whisper-server, llama-server, voicepuncher-command-spotting); services started by hand keep running
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
        # The PID is the service's supervisor, or the service itself when an older justfile
        # started it; either has the binary in its command line
        if ps -p "$pid" -o command= 2>/dev/null | grep -q -- "$binary"; then
            kill "$pid"
            for _ in $(seq 1 40); do
                kill -0 "$pid" 2>/dev/null || break
                sleep 0.25
            done
            if kill -0 "$pid" 2>/dev/null; then
                pkill -9 -P "$pid" || true
                kill -9 "$pid"
            fi
            echo "✓ stopped $binary (pid $pid)"
        else
            echo "· $binary (pid $pid) is no longer running"
        fi
        rm -f "$pid_file"
    }
    # The speech recognition service: HAProxy and its copies when it runs as several, or the
    # one whisper-server on its port
    stop_speech_recognition() {
        local list="{{services_run_dir}}/whisper-server-{{stt_port}}.copies" port
        if [[ ! -f "$list" ]]; then
            stop_service whisper-server "{{stt_port}}" "speech recognition service"
            return 0
        fi
        stop_service haproxy "{{stt_port}}" "speech recognition load balancer"
        for port in $(cat "$list"); do
            stop_service whisper-server "$port" "speech recognition copy on port $port"
        done
        rm -f "$list" "{{services_run_dir}}/haproxy-{{stt_port}}.cfg"
    }
    selected="{{services}}"
    for service in ${selected:-whisper-server llama-server voicepuncher-command-spotting}; do
        case "$service" in
            whisper-server) stop_speech_recognition ;;
            llama-server) stop_service llama-server "{{llm_port}}" "text refinement service" ;;
            voicepuncher-command-spotting) stop_service voicepuncher-command-spotting "{{kws_port}}" "command spotting service" ;;
            *) echo "✗ unknown service '$service' — use whisper-server, llama-server or voicepuncher-command-spotting" >&2; exit 1 ;;
        esac
    done

# ── Setup ─────────────────────────────────────────────────────────────────────

# Download default models into ~/.voicepuncher/models (whisper small ~460 MB, Silero VAD ~1 MB, Qwen2.5-1.5B ~1 GB, keyword spotting ~20 MB, Silero VAD for sherpa-onnx ~1 MB)
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
    fetch https://huggingface.co/ggml-org/whisper-vad/resolve/main/ggml-silero-v5.1.2.bin ggml-silero-v5.1.2.bin
    fetch https://huggingface.co/bartowski/Qwen2.5-1.5B-Instruct-GGUF/resolve/main/Qwen2.5-1.5B-Instruct-Q4_K_M.gguf Qwen2.5-1.5B-Instruct-Q4_K_M.gguf
    fetch https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/silero_vad.onnx silero_vad.onnx
    kws=sherpa-onnx-kws-zipformer-gigaspeech-3.3M-2024-01-01
    if [[ -d "$dir/$kws" ]]; then
        echo "✓ $kws already present"
    else
        fetch "https://github.com/k2-fsa/sherpa-onnx/releases/download/kws-models/$kws.tar.bz2" "$kws.tar.bz2"
        tar -xjf "$dir/$kws.tar.bz2" -C "$dir" && rm "$dir/$kws.tar.bz2"
    fi

# Download benchmark corpus — idempotent, ~500 MB, one-time setup
corpus max_samples="50":
    uv run --python 3.11 --with "datasets<3.0.0" --with soundfile --with numpy --with librosa \
        python3 e2e-test/scripts/download_corpus.py \
        --out e2e-test/corpus \
        --max-samples {{max_samples}}

# ── Model comparison ── not a gate; only when changing a model, decoding params or the prompt

# Full benchmark — all Whisper × LLM combos on the full corpus
bench:
    @MACHINE_LOCK_TIMEOUT=14400 {{locked}} _bench

_bench:
    uv run --python 3.11 --with "datasets<3.0.0" --with soundfile --with numpy --with librosa \
        python3 e2e-test/scripts/bench.py

# Quick benchmark — 10 samples, ~2 min
bench-smoke:
    @{{locked}} _bench-smoke

_bench-smoke:
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
