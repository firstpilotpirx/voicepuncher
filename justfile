set dotenv-load

# ── Settings ── override any of these via env or .env ─────────────────────────

# Models: explicit env var wins; otherwise the model `just models` downloads for each speech recognition
# service, found by its exact file name in ~/.voicepuncher or ~/.voicepuncher/models (empty when not on disk:
# the start and _require-models then fail with "Run 'just models' or set it in .env")
dictation_model_file := "ggml-large-v3-turbo-q5_0.bin"
command_model_file := "ggml-small.bin"
find_model := 'find ~/.voicepuncher -maxdepth 2 -name "$1" 2>/dev/null | head -1'
# Speech recognition service for dictation (what the app uses)
export VOICEPUNCHER_WHISPER_MODEL := env_var_or_default("VOICEPUNCHER_WHISPER_MODEL", shell(find_model, dictation_model_file))
# Speech recognition service for commands (the command spotting service's confirmations)
export VOICEPUNCHER_COMMAND_WHISPER_MODEL := env_var_or_default("VOICEPUNCHER_COMMAND_WHISPER_MODEL", shell(find_model, command_model_file))
# Voice activity detection model of the speech recognition service: the newest Silero model found
export VOICEPUNCHER_VAD_MODEL := env_var_or_default("VOICEPUNCHER_VAD_MODEL", `find ~/.voicepuncher -maxdepth 2 -name 'ggml-silero-*.bin' -exec ls -t {} + 2>/dev/null | head -1`)
# Command spotting service (voice control): sherpa-onnx keyword spotting model directory and its Silero VAD
export VOICEPUNCHER_KWS_MODEL_DIR := env_var_or_default("VOICEPUNCHER_KWS_MODEL_DIR", `find ~/.voicepuncher -maxdepth 2 -type d -name 'sherpa-onnx-kws-*' -exec ls -td {} + 2>/dev/null | head -1`)
# The user's own recorded voice commands (never committed): tests run on them when they exist
export VOICEPUNCHER_VOICE_SAMPLES := env_var_or_default("VOICEPUNCHER_VOICE_SAMPLES", `if test -d ~/.voicepuncher/voice-samples; then echo ~/.voicepuncher/voice-samples; fi`)
export VOICEPUNCHER_KWS_VAD_MODEL := env_var_or_default("VOICEPUNCHER_KWS_VAD_MODEL", `find ~/.voicepuncher -maxdepth 2 -name 'silero_vad.onnx' 2>/dev/null | head -1`)

# Corpus directory containing manifest.jsonl — absolute, because cargo runs tests from e2e-test/.
# The corpus is gitignored, so a worktree without one uses the main checkout's (first of `git worktree list`)
export VOICEPUNCHER_BENCH_CORPUS := env_var_or_default("VOICEPUNCHER_BENCH_CORPUS", `main=$(git worktree list 2>/dev/null | head -1 | cut -d' ' -f1); if [ ! -f e2e-test/corpus/manifest.jsonl ] && [ -f "$main/e2e-test/corpus/manifest.jsonl" ]; then echo "$main/e2e-test/corpus"; else echo "$PWD/e2e-test/corpus"; fi`)

# The services' ports (dev-tools reads the same variables): their URLs are the app's defaults, and the E2E recipes
# point the tests at them; each speech recognition service runs as copies on the ports 10000 above its own
stt_url := "http://127.0.0.1:" + env_var_or_default("VOICEPUNCHER_STT_PORT", "8081") + "/v1"
command_stt_url := "http://127.0.0.1:" + env_var_or_default("VOICEPUNCHER_COMMAND_STT_PORT", "8085") + "/v1"
kws_url := "ws://127.0.0.1:" + env_var_or_default("VOICEPUNCHER_KWS_PORT", "8082")

# The cargo selections the model stages build, shared by their recipes and `_build-e2e`, which builds them
# before the machine lock: another package or feature selection would build other artifacts
command_spotting_build := "-p voicepuncher-command-spotting"
# The stages of the smoke set, one per capability (_e2e-smoke-part)
e2e_smoke_stages := "e2e-smoke-voice e2e-smoke-speech e2e-smoke-services"
# The model stages of check and full-check in the order that shows a failure soonest (fail-fast-stage-order):
# voice control first, as it held the most failures; then the short stages and the long Gherkin runs last, the
# start cues at the very end. `accuracy` stands for the accuracy stage full-check picks (dev-tools accuracy-stage)
check_model_stages := "e2e-smoke-voice e2e-smoke-speech e2e-quick e2e-smoke-services"
full_check_model_stages := "e2e-smoke-voice e2e-regression e2e-resilience e2e-bundle e2e-smoke-speech accuracy e2e-smoke-services e2e-seams it-real e2e-cues"
# The test modules of the voice control smoke part, the most failing first: every member's integration tests
# build into its one `it` binary (tests/it/<module>.rs), so a part names modules; placed_commands is the command
# spotting service's, the others the app's
voice_smoke_modules := "placed_commands voice_forms mishearings voice_samples commandspotting_websocket"
# Every test binary of the smoke parts in one cargo selection: each part's one nextest call builds exactly what
# _build-e2e built before the machine lock, and picks its own modules with a filter (dev-tools filter smoke)
smoke_parts_tests := "-p voicepuncher -p voicepuncher-command-spotting -p voicepuncher-e2e-test --test it --test cucumber_e2e"
e2e_seams_tests := "-p voicepuncher --test it"
e2e_seams_modules := "chunked_transcription"
cucumber_e2e_tests := "-p voicepuncher-e2e-test --test cucumber_e2e"
# A rerun's selection (just rerun-failed): added to every nextest call of the E2E recipes, whose calls may each
# hold none of the failed tests; _test narrows its own selection with it
rerun_filter := '${VOICEPUNCHER_RERUN_FILTER:+-E "$VOICEPUNCHER_RERUN_FILTER" --no-tests=pass}'
# check and full-check set VOICEPUNCHER_FAIL_FAST=1, so a model stage stops at its first failure: nextest runs no
# further test of the binary and the E2E harness no further scenario (harness_args::with_fail_fast). Standalone
# recipes and rerun-failed report every failure. A volatile variable: it changes no stage key (dev-tools keys)
fail_fast := '${VOICEPUNCHER_FAIL_FAST:+--fail-fast}'

# nextest prints captured test output unindented, so libtest's `test … FAILED` and `thread '…' panicked at
# <file:line:col>:` lines start at column 0, where `kit run` finds the failed tests and their locations
export NEXTEST_NO_OUTPUT_INDENT := "1"
# The stage journal: one JSON line per run of _stages (check, full-check, test, the E2E recipes), shared by
# every checkout and kept when a worktree goes; `just stage-times` summarizes it
export VOICEPUNCHER_STAGE_JOURNAL := env_var_or_default("VOICEPUNCHER_STAGE_JOURNAL", home_directory() / ".cache/voicepuncher/stage-times.jsonl")
# How long a locked run waits for the machine lock before it gives up (exit 75): 6 h unless the caller sets it,
# so a run queued behind several sessions' checks waits out the queue instead of giving up and running nothing
export MACHINE_LOCK_WAIT := env_var_or_default("MACHINE_LOCK_WAIT", "21600")

# dev-tools (crate dev-tools/): the stage runner — keys, pass records, the machine lock's phases, the stage table
# and journal — the frozen tests, the named runs, the filters, the engines and the local services. Built once per
# recipe by _dev-tools (seconds when warm) and run prebuilt, never through `cargo run`, whose variables would leak
# into the stages; it calls the private recipes back through the same just and justfile. The machine-wide test
# lock (agent-kit `with-machine-lock`, MACHINE_LOCK; none with VOICEPUNCHER_NO_MACHINE_LOCK=1, the hosted CI) is
# its business: a run without it fails naming the install command (_require-machine-lock)
dev_tools := "DEV_TOOLS_JUST=" + quote(just_executable()) + " DEV_TOOLS_JUSTFILE=" + quote(justfile()) + " " + quote(env_var_or_default("CARGO_TARGET_DIR", justfile_directory() / "target") / "debug/dev-tools")
# A recipe run on its own under the machine lock exclusively, on the test ports (28081, 28085, 28080, 28082 unless
# VOICEPUNCHER_TEST_*_PORT moves them): every check, build, E2E, benchmark and real-keyboard recipe runs its private
# `_<name>` body so, except `test` and `release`, which use no shared service, port or GPU
locked := dev_tools + " locked"

# Builds dev-tools for the recipes that call it
_dev-tools:
    @cargo build --quiet -p voicepuncher-dev-tools

# Fails naming the command that installs the machine lock when it is missing
_require-machine-lock: _dev-tools
    @{{dev_tools}} require-lock

# List all recipes
default:
    @just --list --unsorted

# ── Checks ── `check` while developing, `full-check` before merging ─────────

# Only the model stages take the machine lock, held ≈ 7.3 min (measured), after the code checks, the build and
# build-e2e ran without it (see _stages). Incremental, no `cargo clean`
# FAST (~8.5 min), run often while developing: code checks + E2E smoke + accuracy on 10 samples
check:
    @VOICEPUNCHER_FAIL_FAST=1 VOICEPUNCHER_STAGES_RECIPE=check {{just_executable()}} _stages static lint test metrics build build-e2e {{check_model_stages}}

# Only the model stages take the machine lock, ≈ 16.3 min with 100 recordings, ≈ 14.5 min with 10 (measured); incremental, no `cargo clean`.
# Accuracy on 100 recordings and the whole seam matrix when the branch changes their paths, on main itself or when that cannot be told (dev-tools accuracy-stage, seams-stage), else 10 and the seam sample; VOICEPUNCHER_ACCURACY_SAMPLE=100 forces 100
# FULL (~15.5–17.5 min), run before merging: code checks + the bundle + E2E (the whole voice control set, seams, outages and the bundled engines included) + accuracy on 100 or 10 samples
full-check: _require-models _require-corpus _dev-tools
    #!/usr/bin/env bash
    set -euo pipefail
    choice=$({{dev_tools}} accuracy-stage)
    stage=${choice%% *}
    reason=${choice#* — }
    case "$stage" in
        e2e-bench) count=100 ;;
        e2e-quick) count=10 ;;
        *) echo "✗ dev-tools accuracy-stage printed no stage: '$choice'" >&2; exit 1 ;;
    esac
    accuracy="accuracy: $count recordings ($stage) — $reason"
    echo "● $accuracy"
    seams=$({{dev_tools}} seams-stage)
    echo "● seams: $seams"
    VOICEPUNCHER_FAIL_FAST=1 VOICEPUNCHER_STAGES_RECIPE=full-check VOICEPUNCHER_STAGES_ACCURACY="$accuracy" \
        {{just_executable()}} _stages static lint test metrics build bundle build-e2e {{replace(replace(full_check_model_stages, "accuracy", '"$stage"'), "e2e-seams", '"${seams%% *}"')}}
    echo "✓ full-check passed — $accuracy; before a release also walk through README.md § Release checklist (manual run)"

# Summary of the stage journal since a date (2026-10-01) or a span (7d): per recipe and stage the runs, median, total, failures, reused runs and the time they saved; the lock wait;
# overlapping e2e lock holds; the failed tests, most first; runs and time per branch (so per change); the mean WER of the last accuracy runs per sample size
stage-times since="30d": _dev-tools
    @{{dev_tools}} stage-times {{quote(since)}}

# Accepts this checkout's last journaled mean WER per sample size as the accuracy baseline
# (e2e-test/accuracy_baseline.json, with its commit); a baseline that gets worse is reviewed like code
accuracy-baseline: _dev-tools
    @{{dev_tools}} accuracy-baseline

# Prints the accuracy stage full-check runs and why: `e2e-bench — <reason>` (100 recordings) or `e2e-quick —
# <reason>` (10). 100 when VOICEPUNCHER_ACCURACY_SAMPLE=100 (any other value fails), when the changed paths

# Prints `<stage> <key>` for each named stage: a sha256 over what the stage reads, so a stage whose key has a
# pass record (_stages) need not run again. Over the stage's name; the path and content of every file it reads
# (stage_inputs), tracked or untracked but not ignored, as the working tree holds it, so uncommitted edits count
# and a deleted file changes the key; the toolchain (rustc, cargo, nextest, just, the macOS build, which fixes
# the `say` voices); the VOICEPUNCHER_*, CUCUMBER_*, Rust and Cargo build variables but the volatile ones (journal,
# reports, ports, the lock, colours), so a probe or a forced sample never reuses a pass. A model stage also hashes
# what it runs on outside the repository: the size and mtime of every file or directory such a variable names
# (models, corpus, voice samples) and of the whisper-server binary; for it a Cargo file's `version = "…"` lines of the workspace's own
# packages do not count (no model stage reads the version). An input this misses (another program on the PATH,
# a system library) is a reason to run with VOICEPUNCHER_REUSE_STAGES=0. The test stage's parts (_test) have keys
# too: `test@<member>` over the code inputs but the files of members its tests do not build (`test@members` names
# one per workspace member). VOICEPUNCHER_STAGE_ROOT names another repository

# Runs the named stages (private recipes without their `_`) in phases (dev-tools stages): the stages before the
# first model stage (`e2e*` or `it-real`) one by one under the quiet lock in the shared mode, which
# gives each its time limit and keeps it out of another run's model stages; the rest in one phase under the e2e
# lock (the machine lock, then the quiet lock, both exclusively; or the caller's machine lock), on the test
# ports. A stage that passed before on the same inputs (its key has a pass record) is reused (↺); build and
# build-e2e run whenever a model stage runs. Ends with the stage table and one line in the stage journal; each
# stage that passed and whose key still holds gets its pass record. VOICEPUNCHER_REUSE_STAGES=0 runs every stage
_stages +stages: _dev-tools
    @{{dev_tools}} stages {{stages}}

# Builds what the model stages run — the command spotting service and their test binaries — so nothing
# compiles under the machine lock; warm, five no-op cargo calls. The test binaries are built as the E2E
# recipes' nextest runs build them
_build-e2e: _require-nextest
    cargo build --release {{command_spotting_build}}
    cargo nextest run {{smoke_parts_tests}} --no-run
    cargo nextest run {{e2e_seams_tests}} --no-run
    cargo nextest run {{cucumber_e2e_tests}} --no-run

# The static analyzers' pinned versions (`tools`, `_require-tools`)
machete_version := "0.9.2"
deny_version := "0.20.2"
semver_checks_version := "0.51.0"
typos_version := "1.51.1"
rca_version := "0.0.25"
jscpd_version := "4.0.5"
shellcheck_version := "0.11.0"
gitleaks_version := "8.30.1"
trufflehog_version := "3.99.6"
# The ratcheted analyzers (analyzers/), run from the workspace root
analyzers := "cargo run --quiet --package voicepuncher-analyzers --"

# Formatting check
fmt: _dev-tools
    @{{locked}} _fmt

_fmt:
    cargo fmt --all -- --check

# Clippy on the whole workspace (warnings are errors) and every static analyzer against its baseline (AGENTS.md
# § How to verify a change): the clippy groups, unused dependencies, the dependency policy, copy-pool's semver,
# spelling, shellcheck over the recipes, and the secret scanners over the history; runs every analyzer, then fails on any that failed
lint: _dev-tools
    @{{locked}} _lint

_lint: _require-tools
    #!/usr/bin/env bash
    set -euo pipefail
    failed=()
    analyzer() { # <name> <command>…: runs one analyzer, noting its failure
        local name=$1
        shift
        echo "● $name"
        "$@" || failed+=("$name")
    }
    analyzer clippy cargo clippy --workspace --all-targets -- -D warnings
    analyzer clippy-groups {{analyzers}} clippy
    analyzer machete cargo machete
    analyzer deny cargo deny --log-level error check
    analyzer semver cargo +stable semver-checks --package voicepuncher-copy-pool --baseline-rev origin/main
    analyzer typos typos
    analyzer recipes {{analyzers}} recipes
    # Secrets in any commit of any ref: the history, not only the tree, is what a push publishes
    analyzer gitleaks gitleaks git --no-banner --redact --log-opts=--all .
    analyzer trufflehog trufflehog git file://. --no-update --results=verified,unknown --fail
    if (( ${#failed[@]} > 0 )); then
        echo "✗ lint: ${failed[*]} failed — baselines in .config/analyzers/, rewritten by 'just analyzers-baseline'" >&2
        exit 1
    fi

# The fast static check after an edit (~10 s): formatting, clippy, the size budgets and the metrics of the changed files, stopping at the first that fails
static: _dev-tools
    @VOICEPUNCHER_STAGES_RECIPE=static {{just_executable()}} _stages static

_static:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "● fmt"
    cargo fmt --all -- --check
    echo "● clippy"
    cargo clippy --workspace --all-targets -- -D warnings
    {{analyzers}} size
    {{analyzers}} metrics --changed origin/main

# Code metrics (AGENTS.md § How to verify a change): the 25 most complex functions, one row per module (functions,
# lengths, complexity, unsafe/unwrap/clone, fan-in/out), import cycles and duplicated lines, ratcheted against
# .config/analyzers/metrics.toml; a stage of `test`, unlocked like it
metrics: _dev-tools
    @{{locked}} _metrics

_metrics: _require-tools
    {{analyzers}} metrics

# Rewrites the baselines of .config/analyzers/ from the code: to drop the findings fixed, or to list what a merge
# of branches brought in; a rewrite that adds or raises an entry is reviewed like code
analyzers-baseline: _require-tools
    #!/usr/bin/env bash
    set -euo pipefail
    for tool in clippy recipes metrics; do {{analyzers}} "$tool" --write-baseline; done

# Installs the pinned static analyzers `lint` and `metrics` need (shellcheck comes from Homebrew, checked only)
tools:
    cargo +stable install --locked cargo-machete@{{machete_version}} cargo-deny@{{deny_version}} cargo-semver-checks@{{semver_checks_version}} typos-cli@{{typos_version}} rust-code-analysis-cli@{{rca_version}}
    npm install --global jscpd@{{jscpd_version}}

# Each pinned analyzer at its version; names the command that installs it and installs nothing
_require-tools:
    #!/usr/bin/env bash
    set -euo pipefail
    missing=0
    want() { # <name> <version> <install command> <version command>…
        local name=$1 version=$2 install=$3 got
        shift 3
        # A tool that is missing or fails prints nothing: the comparison below names it
        got=$("$@" 2>/dev/null | head -2 | tr '\n' ' ' || true)
        if [[ " $got " != *[\ :v]"$version"[\ ]* ]]; then
            echo "✗ $name $version not found (got '${got% }') — run '$install'" >&2
            missing=1
        fi
    }
    want cargo-machete {{machete_version}} "just tools" cargo machete --version
    want cargo-deny {{deny_version}} "just tools" cargo deny --version
    want cargo-semver-checks {{semver_checks_version}} "just tools" cargo +stable semver-checks --version
    want typos {{typos_version}} "just tools" typos --version
    want rust-code-analysis-cli {{rca_version}} "just tools" rust-code-analysis-cli --version
    want jscpd {{jscpd_version}} "just tools" jscpd --version
    want shellcheck {{shellcheck_version}} "brew install shellcheck" shellcheck --version
    want gitleaks {{gitleaks_version}} "brew install gitleaks" gitleaks version
    want trufflehog {{trufflehog_version}} "brew install trufflehog" trufflehog --version
    exit "$missing"

# Rewrites app/tests/size_baseline.toml from the code (AGENTS.md § Architecture rules) with the size analyzer: to
# drop the entries of units back within budget, or to list the units a merge of branches written before the
# budgets brought in; a rewrite that adds or raises an entry is reviewed like code
size-baseline:
    {{analyzers}} size --write-baseline

# Runs without the machine-wide lock, as an unlocked stage of _stages: it uses no shared service, port, GPU or
# exclusive device; the quiet lock in the shared mode gives it the time limit and keeps it out of another run's
# model stages (dev-tools stages)
# Every test of the app, the command spotting service and the E2E harness under cargo-nextest, each within its time limit, without the machine lock (~1 min)
test:
    @VOICEPUNCHER_STAGES_RECIPE=test {{just_executable()}} _stages static test metrics

# Runs only the named tests and slice/component scenarios in the `test` stage (dev-tools named-run): the TDD Red
# and Green steps (AGENTS.md § TDD), with the test stage's build, nextest config and time limits, unlocked;
# journaled as `test-one`, never reused and never a pass. A name is a bare test name (every test whose name holds
# it), `<package>::<binary> <test>` or `scenario: <name>`; one that never ran fails the run
# TDD Red/Green: run only the named tests or scenarios (e.g. `just test-one should_reject_empty_chunk`), in seconds
[positional-arguments]
test-one +names: _dev-tools
    @{{dev_tools}} named-run test-one test "$@"

# Reruns this worktree's last run in the stage journal that ran a stage: its failed stage with its failed tests
# (or the names given, in the journal's form) alone, with that stage's build, services, lock, test ports and time
# limits, never reused and never a pass; fails naming any of them that never ran. Journaled as `rerun-failed`
# After a failure: rerun only the failed tests and scenarios of the last run, in their stage; once green, run the stage's recipe or the check whole once
[positional-arguments]
rerun-failed *names: _dev-tools
    @{{dev_tools}} rerun-failed "$@"

# Adds a test that failed outside the change to .config/frozen-tests.toml on the branch and on main at once (a
# scratch worktree of origin/main), so no recipe in any worktree runs it until a fix removes its entry. `test` is
# the stage journal's name, `recipe` and `stage` where it failed, `beside` what ran beside it
# Freeze a test that failed outside your change: it runs nowhere until a fix removes its entry; no change is filed
[positional-arguments]
freeze-test test recipe stage beside: _dev-tools
    @{{dev_tools}} freeze-test "$@"

# Lists the frozen tests (.config/frozen-tests.toml merged with main's); `run` runs only them, each in the stage it
# failed in, like a rerun: nothing is reused or recorded as a pass. Journaled as `frozen-tests`
# List the frozen tests, or `run` them alone to see whether a fix holds
frozen-tests action="": _dev-tools
    @{{dev_tools}} frozen-tests {{action}}

# The `test` stage (dev-tools test-stage): one nextest run over the workspace's test targets except the E2E
# harness; only the packages whose part key (test@<member>) has no pass record run, each part that passed gets
# its record; a rerun (VOICEPUNCHER_RERUN_FILTER) runs only its selection, reusing and recording nothing
_test: _require-nextest _dev-tools
    @{{dev_tools}} test-stage

# Run the app for real use with one command: download missing models, restart the services (the command spotting service is rebuilt), build and start the app with voice control; extra arguments go to the app
run *args:
    #!/usr/bin/env bash
    set -euo pipefail
    just models
    just stop-model-servers
    just start-model-servers
    cargo run --release -p voicepuncher -- --voice-control {{args}}

# Build the release binary
build: _dev-tools
    @{{locked}} _build

_build:
    cargo build --release

# E2E on real models: smoke + chunk seams + WER accuracy on 100 samples (~13 min)
e2e:
    @VOICEPUNCHER_STAGES_RECIPE=e2e {{just_executable()}} _stages {{e2e_smoke_stages}} e2e-seams e2e-cues e2e-bench

# Smoke test — services and models load, file mode runs and voice commands are spotted (@smoke scenarios), the adapters talk to the real services
e2e-smoke:
    @VOICEPUNCHER_STAGES_RECIPE=e2e-smoke {{just_executable()}} _stages {{e2e_smoke_stages}}

# The smoke set split by capability, one stage each, so a stage whose inputs did not change is reused on its
# own (stage_inputs): speech recognition, file transcription and every capability without a part of its own
# (the dictation archive: a part of 6 s cost more than it saved), voice control and local
# services. Each part's scenario selection and limits are its profile's (.config/nextest.toml)
_e2e-smoke-speech: (_e2e-smoke-part "e2e-smoke-speech" "transcription_openai")
_e2e-smoke-voice: (_e2e-smoke-part "e2e-smoke-voice" voice_smoke_modules)
_e2e-smoke-services: (_e2e-smoke-part "e2e-smoke-services")

# One nextest call for the part's service test modules and its Gherkin scenarios, so the modules run beside the
# harness instead of before it (speed-up-smoke-and-test); the command-spotting-models test group still keeps the
# tests that drive the shared command spotting service to one at a time, and the harness keeps its own
# scenario concurrency. nextest runs nothing in parallel under --no-capture, so the harness's output is captured
# and printed whole when it ends (the profiles' success-output). placed_commands is the command spotting
# service's module, every other one the app's
_e2e-smoke-part profile modules="": _require-nextest _require-models _build _dev-tools
    #!/usr/bin/env bash
    set -euo pipefail
    trap '{{just_executable()}} _stop-model-servers-for-e2e' EXIT
    {{just_executable()}} _start-model-servers-for-e2e
    export VOICEPUNCHER_TRANSCRIPTION_URL="{{stt_url}}" VOICEPUNCHER_COMMAND_SPOTTING_URL="{{kws_url}}"
    export VOICEPUNCHER_COMMAND_TRANSCRIPTION_URL="{{command_stt_url}}"
    filter=$({{dev_tools}} filter smoke "{{modules}}")
    cargo nextest run --profile {{profile}} {{smoke_parts_tests}} -E "$filter" ${VOICEPUNCHER_RERUN_FILTER:+--no-tests=pass} {{fail_fast}}

# The closed needs of this environment (test_needs), one `<need> <reason>` per line

# The scenarios beyond the smoke set (@regression): voice control under load, run by full-check right after e2e-smoke so it still runs the whole set
e2e-regression:
    @VOICEPUNCHER_STAGES_RECIPE=e2e-regression {{just_executable()}} _stages e2e-regression

_e2e-regression: _require-nextest _require-models _build
    #!/usr/bin/env bash
    set -euo pipefail
    trap '{{just_executable()}} _stop-model-servers-for-e2e' EXIT
    {{just_executable()}} _start-model-servers-for-e2e
    export VOICEPUNCHER_TRANSCRIPTION_URL="{{stt_url}}" VOICEPUNCHER_COMMAND_SPOTTING_URL="{{kws_url}}"
    export VOICEPUNCHER_COMMAND_TRANSCRIPTION_URL="{{command_stt_url}}"
    cargo nextest run --profile e2e-regression {{cucumber_e2e_tests}} --no-capture {{rerun_filter}} {{fail_fast}}

# WER accuracy on 10 samples (5 per language) — fails if mean WER is worse than the accepted baseline (e2e-test/accuracy_baseline.json) by more than the margin, and no command in 10 recordings of ordinary speech (~1 min)
e2e-quick:
    @VOICEPUNCHER_STAGES_RECIPE=e2e-quick {{just_executable()}} _stages e2e-quick

_e2e-quick: _require-nextest _require-models _require-corpus _build
    #!/usr/bin/env bash
    set -euo pipefail
    trap '{{just_executable()}} _stop-model-servers-for-e2e' EXIT
    {{just_executable()}} _start-model-servers-for-e2e
    VOICEPUNCHER_TRANSCRIPTION_URL="{{stt_url}}" VOICEPUNCHER_COMMAND_SPOTTING_URL="{{kws_url}}" VOICEPUNCHER_BENCH_LIMIT=10 \
        cargo nextest run --profile e2e-quick {{cucumber_e2e_tests}} --no-capture {{rerun_filter}} {{fail_fast}}

# Outages: services that hang, die and come back while voice control runs (@resilience), each scenario on service ports of its own, so the shared services are never touched (~4 min)
e2e-resilience:
    @VOICEPUNCHER_STAGES_RECIPE=e2e-resilience {{just_executable()}} _stages e2e-resilience

_e2e-resilience: _require-nextest _require-models _build
    cargo nextest run --profile e2e-resilience {{cucumber_e2e_tests}} --no-capture {{rerun_filter}} {{fail_fast}}

# The bundled engines: a copy of dist/voicepuncher.app launched outside the build folder with nothing but the system
# folders on $PATH starts every service from its own engines, on ports of its own; a transcription and
# the command spotting service's health through them, and Metal in the engines' logs (@bundle_engines, ~1 min)
bundle-smoke:
    @VOICEPUNCHER_STAGES_RECIPE=bundle-smoke {{just_executable()}} _stages bundle e2e-bundle

# Builds the bundle first (incremental: seconds after the bundle stage), so a reused bundle stage never leaves it a
# missing or stale one; starts no shared service (dev-tools model-stages)
_e2e-bundle: _require-nextest _require-models _require-corpus
    {{just_executable()}} --justfile {{quote(justfile())}} bundle
    cargo nextest run --profile e2e-bundle {{cucumber_e2e_tests}} --no-capture {{rerun_filter}} {{fail_fast}}

# Chunk seams: the seam sweep (speech after different lead-ins) and Russian and English speech under 11 recording conditions, transcribed in chunks — fails on a phrase repeated at a seam or > 10 % missing words (~4 min); its sample, `_e2e-seams-sample` (VOICEPUNCHER_SEAM_SAMPLE=1): the sweep at lead-in 0 and the @seams_sample rows
e2e-seams:
    @VOICEPUNCHER_STAGES_RECIPE=e2e-seams {{just_executable()}} _stages e2e-seams

_e2e-seams profile="e2e-seams": _require-nextest _require-models _build _dev-tools
    #!/usr/bin/env bash
    set -euo pipefail
    trap '{{just_executable()}} _stop-model-servers-for-e2e' EXIT
    {{just_executable()}} _start-model-servers-for-e2e
    export VOICEPUNCHER_TRANSCRIPTION_URL="{{stt_url}}"
    filter=$({{dev_tools}} filter it "{{e2e_seams_modules}}")
    cargo nextest run --profile {{profile}} {{e2e_seams_tests}} -E "$filter" ${VOICEPUNCHER_RERUN_FILTER:+--no-tests=pass} {{fail_fast}}
    cargo nextest run --profile {{profile}} {{cucumber_e2e_tests}} --no-capture {{rerun_filter}} {{fail_fast}}

_e2e-seams-sample:
    @VOICEPUNCHER_SEAM_SAMPLE=1 {{just_executable()}} --justfile {{quote(justfile())}} _e2e-seams e2e-seams-sample

# Start cues: the start cue of each voice start, timed from the end of the command's words, on an idle machine — the stage holds the machine lock exclusively and keeps the other checks' build stages out (~2.5 min)
e2e-cues:
    @VOICEPUNCHER_STAGES_RECIPE=e2e-cues {{just_executable()}} _stages e2e-cues

_e2e-cues: _require-nextest _require-models _build
    #!/usr/bin/env bash
    set -euo pipefail
    trap '{{just_executable()}} _stop-model-servers-for-e2e' EXIT
    {{just_executable()}} _start-model-servers-for-e2e
    # The command spotting services for the voice start timing scenarios (@cues)
    export VOICEPUNCHER_TRANSCRIPTION_URL="{{stt_url}}" VOICEPUNCHER_COMMAND_SPOTTING_URL="{{kws_url}}"
    export VOICEPUNCHER_COMMAND_TRANSCRIPTION_URL="{{command_stt_url}}"
    cargo nextest run --profile e2e-cues {{cucumber_e2e_tests}} --no-capture {{rerun_filter}} {{fail_fast}}

# WER accuracy on a fixed sample of 100 recordings (50 per language), two at a time — fails if mean WER is worse than the accepted baseline (e2e-test/accuracy_baseline.json) by more than the margin, and no command in 20 recordings of ordinary speech (~3 min)
e2e-bench:
    @VOICEPUNCHER_STAGES_RECIPE=e2e-bench {{just_executable()}} _stages e2e-bench

_e2e-bench: _require-nextest _require-models _require-corpus _build
    #!/usr/bin/env bash
    set -euo pipefail
    trap '{{just_executable()}} _stop-model-servers-for-e2e' EXIT
    {{just_executable()}} _start-model-servers-for-e2e
    VOICEPUNCHER_TRANSCRIPTION_URL="{{stt_url}}" VOICEPUNCHER_COMMAND_SPOTTING_URL="{{kws_url}}" \
        cargo nextest run --profile e2e-bench {{cucumber_e2e_tests}} --no-capture {{rerun_filter}} {{fail_fast}}

# The desktop tests (`needs_desktop`): the real microphone, the Accessibility API and windows on screen, alone on
# the machine like a model stage (full-check runs it before e2e-cues); without Microphone or Accessibility for the
# app running it, it names the missing permission, counts the tests it leaves out and skips (exit 77). Key presses
# into the focused app stay in test-real-keyboard
it-real:
    @VOICEPUNCHER_STAGES_RECIPE=it-real {{just_executable()}} _stages it-real

_it-real: _require-nextest _dev-tools
    @{{dev_tools}} it-real

# Manual: @real_keyboard scenarios drive the real hotkey + paste, and the key presser's test (`needs_real_keyboard`) presses Return, Cmd+A and Backspace in the focused app — needs Accessibility and Input Monitoring for the terminal, a GUI session, hands off the keyboard
test-real-keyboard: _dev-tools
    @{{locked}} _test-real-keyboard

_test-real-keyboard: _require-nextest
    VOICEPUNCHER_REAL_KEYBOARD_IT=1 cargo nextest run --profile real-keyboard -p voicepuncher --test cucumber_integration --test it \
        -E 'binary(cucumber_integration) | test(/::needs_real_keyboard::/)' --no-capture

# ── Release ── after the merge, on a clean checkout of origin/main ──────────

# Runs without the machine-wide lock, like `test`: it uses no shared service, port or GPU; a lock directory of the
# run's own only gives it the wrapper's time limit (MACHINE_LOCK_TIMEOUT, exit 124; the timeout message says
# `[machine-lock]`; a run killed with KILL leaves that directory, holding an empty tests.lock, in $TMPDIR).
# `cargo clean` deletes target/release/voicepuncher, which the user may be running: in the main checkout, ask first
# Tag v<app version>, clean build, publish a GitHub Release with the signed DMG + sha256 (needs the signing identity)
release: _require-release-identity _dev-tools
    #!/usr/bin/env bash
    set -euo pipefail
    lock_dir=$(mktemp -d)
    trap 'rm -rf "$lock_dir"' EXIT
    # The wrapper runs in the background, so a signal to this script reaches it and stops the whole run
    MACHINE_LOCK_DIR="$lock_dir" {{locked}} _release &
    run=$!
    # The run may have ended already: nothing is left to stop then
    trap 'kill -TERM "$run" 2>/dev/null || true' TERM INT HUP
    while true; do
        wait "$run" && code=0 || code=$?
        kill -0 "$run" 2>/dev/null || exit "$code"
    done

# The checks of the checkout, the clean build and package, then the only steps that publish: tag, push, release
_release: _require-release-head _require-release-identity _release-package
    #!/usr/bin/env bash
    set -euo pipefail
    tag="v{{app_version}}"
    dmg={{quote(dist_dir)}}/voicepuncher-$tag.dmg
    git tag -a "$tag" -m "voicepuncher $tag"
    git push origin "$tag"
    gh release create "$tag" "$dmg" "$dmg.sha256" --title "voicepuncher $tag" --generate-notes --verify-tag
    echo "✓ released $tag"

# Everything of a release that publishes nothing, safe to run alone: `cargo clean`, every test (_test: its build
# at the utility QoS clamp, its test binaries at the default QoS), the release build from scratch at the utility
# clamp, then the signed bundle in its DMG and the DMG's .sha256 in dist/
_release-package: _require-release-identity _require-create-dmg
    #!/usr/bin/env bash
    set -euo pipefail
    cargo clean
    rm -rf {{quote(dist_dir)}}
    {{just_executable()}} --justfile {{quote(justfile())}} _test
    taskpolicy -c utility cargo build --release
    {{just_executable()}} --justfile {{quote(justfile())}} dmg
    echo "✓ packaged {{dist_dir}}/voicepuncher-v{{app_version}}.dmg"

# ── Bundle ── the macOS application bundle and its disk image ────────────────

# Where `bundle` and `dmg` write (gitignored): the signed voicepuncher.app, the DMG and its .sha256
dist_dir := justfile_directory() / "dist"
app_bundle := dist_dir / "voicepuncher.app"
packaging_dir := justfile_directory() / "app/packaging"
app_version := shell("grep -m1 '^version = ' \"$1\" | cut -d '\"' -f2", justfile_directory() / "app/Cargo.toml")
# The self-signed code-signing certificate of README § Release checklist; empty signs ad-hoc (no stable identity)
signing_identity := env_var_or_default("VOICEPUNCHER_SIGNING_IDENTITY", "")

# Build the signed application bundle into dist/voicepuncher.app (release build, no `cargo clean`), with the pinned
# engines in Contents/MacOS/engines/ and their licenses in Contents/Resources/licenses/: `open dist/voicepuncher.app`
bundle: _require-signing-identity engines
    #!/usr/bin/env bash
    set -euo pipefail
    cargo build --release
    app={{quote(app_bundle)}}
    engines={{quote(engines_dir)}}
    rm -rf "$app" && mkdir -p "$app/Contents/MacOS/engines" "$app/Contents/Resources"
    cp target/release/voicepuncher "$app/Contents/MacOS/voicepuncher"
    for engine in whisper command-spotting; do cp -R "$engines/$engine" "$app/Contents/MacOS/engines/"; done
    cp -R "$engines/licenses" "$app/Contents/Resources/licenses"
    iconutil -c icns {{quote(packaging_dir / "icons/voicepuncher.iconset")}} -o "$app/Contents/Resources/voicepuncher.icns"
    cp {{quote(packaging_dir / "Info.plist")}} "$app/Contents/Info.plist"
    plutil -replace CFBundleShortVersionString -string {{quote(app_version)}} "$app/Contents/Info.plist"
    plutil -replace CFBundleVersion -string {{quote(app_version)}} "$app/Contents/Info.plist"
    {{just_executable()}} --justfile {{quote(justfile())}} _sign "$app"
    echo "✓ bundle $app"

# Build the release disk image dist/voicepuncher-vX.Y.Z.dmg (the bundle, an Applications shortcut, the background
# picture) and its .sha256; create-dmg lays the window out through the Finder
dmg: _require-create-dmg bundle
    #!/usr/bin/env bash
    set -euo pipefail
    dist={{quote(dist_dir)}}
    name="voicepuncher-v{{app_version}}"
    stage="$dist/stage"
    rm -rf "$stage" "$dist/$name.dmg" "$dist/$name.dmg.sha256" && mkdir -p "$stage"
    cp -R {{quote(app_bundle)}} "$stage/"
    create-dmg --volname "voicepuncher {{app_version}}" --background {{quote(packaging_dir / "dmg-background.png")}} \
        --window-pos 200 120 --window-size 600 400 --icon-size 128 --icon voicepuncher.app 150 200 \
        --hide-extension voicepuncher.app --app-drop-link 450 200 --no-internet-enable \
        "$dist/$name.dmg" "$stage"
    (cd "$dist" && shasum -a 256 "$name.dmg" > "$name.dmg.sha256")
    rm -rf "$stage"
    echo "✓ dmg $dist/$name.dmg"

# Sign a bundle with the signing identity, or ad-hoc with a notice when none is named — every engine file first,
# then the app (inside-out) — then verify it strictly and print its designated requirement (what the permission
# panes identify the app by). No hardened runtime: it needs a Developer ID and entitlements, and without them it
# blocks the microphone and the key listener
_sign path:
    #!/usr/bin/env bash
    set -euo pipefail
    identity={{quote(signing_identity)}}
    if [[ -z "$identity" ]]; then
        echo "● VOICEPUNCHER_SIGNING_IDENTITY is unset: signing ad-hoc — the bundle has no stable identity, so macOS asks for every permission again after each build (README § Release checklist)" >&2
        identity="-"
    fi
    if [[ -d {{quote(path)}}/Contents/MacOS/engines ]]; then
        # Every file, VERSION too: a data file under Contents/MacOS is a nested component, signed in its extended attributes
        find {{quote(path)}}/Contents/MacOS/engines -type f -print0 \
            | xargs -0 codesign --force --sign "$identity" --timestamp=none
    fi
    codesign --force --sign "$identity" --timestamp=none {{quote(path)}}
    codesign --verify --deep --strict --verbose=2 {{quote(path)}}
    codesign -d -r- {{quote(path)}} 2>&1 | sed -n 's/^# designated => /  identity: /p'

# A named signing identity must be in the keychain; none named is allowed (ad-hoc)
_require-signing-identity:
    #!/usr/bin/env bash
    set -euo pipefail
    identity={{quote(signing_identity)}}
    [[ -n "$identity" ]] || exit 0
    if ! security find-identity -v -p codesigning 2>/dev/null | grep -q "\"$identity\""; then
        echo "✗ signing identity '$identity' (VOICEPUNCHER_SIGNING_IDENTITY) is not in the keychain" >&2
        {{just_executable()}} --justfile {{quote(justfile())}} _signing-identity-steps >&2
        exit 1
    fi

# A release is always signed with the certificate: the identity must be named and in the keychain
_require-release-identity:
    #!/usr/bin/env bash
    set -euo pipefail
    if [[ -z {{quote(signing_identity)}} ]]; then
        echo "✗ VOICEPUNCHER_SIGNING_IDENTITY is unset — a release is signed with the self-signed certificate, so that updates keep their permissions" >&2
        {{just_executable()}} --justfile {{quote(justfile())}} _signing-identity-steps >&2
        exit 1
    fi
    {{just_executable()}} --justfile {{quote(justfile())}} _require-signing-identity

_signing-identity-steps:
    @echo "  Create the certificate once: 'just signing-identity' (macOS asks for your password to trust it),"
    @echo "  then add to .env: VOICEPUNCHER_SIGNING_IDENTITY=voicepuncher (README § Release checklist)"

# Creates the self-signed code-signing certificate `voicepuncher` (10 years) in the login keychain, once per
# machine, and trusts it for code signing; macOS asks for the user's password. Does nothing when it exists
signing-identity name="voicepuncher":
    #!/usr/bin/env bash
    set -euo pipefail
    if security find-identity -v -p codesigning | grep -q "\"{{name}}\""; then
        echo "✓ code-signing identity \"{{name}}\" already in the keychain"; exit 0
    fi
    work=$(mktemp -d); trap 'rm -rf "$work"' EXIT
    # /usr/bin/openssl (LibreSSL) writes a PKCS#12 file that `security import` reads
    /usr/bin/openssl req -x509 -newkey rsa:2048 -nodes -days 3650 -subj "/CN={{name}}" \
        -keyout "$work/key.pem" -out "$work/cert.pem" \
        -addext "keyUsage=critical,digitalSignature" -addext "extendedKeyUsage=critical,codeSigning" \
        -addext "basicConstraints=critical,CA:false" 2>/dev/null
    /usr/bin/openssl pkcs12 -export -inkey "$work/key.pem" -in "$work/cert.pem" -out "$work/id.p12" -passout pass:voicepuncher
    keychain=$(security default-keychain | tr -d ' "')
    security import "$work/id.p12" -k "$keychain" -P voicepuncher -T /usr/bin/codesign
    echo "  macOS now asks for your password to trust the certificate for code signing"
    security add-trusted-cert -r trustRoot -p codeSign -k "$keychain" "$work/cert.pem"
    security find-identity -v -p codesigning | grep "\"{{name}}\""
    echo "✓ created \"{{name}}\" — add to .env: VOICEPUNCHER_SIGNING_IDENTITY={{name}}"

_require-create-dmg:
    #!/usr/bin/env bash
    set -euo pipefail
    wanted=1.3.0
    # Missing create-dmg prints nothing: the comparison below names it
    got=$(create-dmg --version 2>/dev/null | head -1 || true)
    if [[ "$got" != *"$wanted"* ]]; then
        echo "✗ create-dmg $wanted not found (got '${got}') — run 'brew install create-dmg'" >&2
        exit 1
    fi

# The `bundle` stage of full-check: the DMG built, the bundle and the DMG checked (dev-tools check-bundle), then the
# app-distribution scenarios (@bundle) on them: the bundle's information, its signature, the DMG as an
# installer and the signing guards. No shared service, port or GPU: it runs before the model stages, outside
# the machine lock
_bundle: _require-nextest _require-create-dmg _dev-tools
    #!/usr/bin/env bash
    set -euo pipefail
    just="{{just_executable()}} --justfile {{justfile()}}"
    $just dmg
    $just _check-bundle {{quote(app_bundle)}} {{quote(dist_dir)}}/voicepuncher-v{{app_version}}.dmg
    cargo nextest run --profile bundle {{cucumber_e2e_tests}} --no-capture {{rerun_filter}} {{fail_fast}}

# A bundle and its disk image checked: its information, signature and the image as an installer (dev-tools)
_check-bundle app dmg: _dev-tools
    @{{dev_tools}} check-bundle {{quote(app)}} {{quote(dmg)}}

# ── Engines ── the pinned engines the bundle carries (app/packaging/engines.lock) ─────────────

engines_lock := env_var_or_default("VOICEPUNCHER_ENGINES_LOCK", packaging_dir / "engines.lock")
# Where `engines` stages them (gitignored): one folder per engine, its libraries beside it, and the licenses
engines_dir := packaging_dir / "engines"

# Stage the pinned engines of engines.lock into app/packaging/engines/, relocated and checked to run on their own
# (dev-tools engines: every pinned version and checksum checked first, nothing written when one differs; each
# engine's references kept in its folder, each run with nothing but the system folders on $PATH; the licenses)
engines lock=engines_lock out=engines_dir: _dev-tools
    @{{dev_tools}} engines {{quote(lock)}} {{quote(out)}}

# ── Services ── local processes the app talks to over loopback ────────────────

# Start every service an E2E recipe tests, on the test ports the machine lock gives it, never reusing one it did
# not start; a service a killed locked run left there is stopped and started again; any other process on a test
# port fails the run. Inside check and full-check, which hold the services for all their stages, it does nothing
_start-model-servers-for-e2e: _dev-tools
    @{{dev_tools}} model-servers-for-e2e start

# Stop the services that _start-model-servers-for-e2e started (nothing while check or full-check holds them)
_stop-model-servers-for-e2e: _dev-tools
    @{{dev_tools}} model-servers-for-e2e stop

# Start the speech recognition services for dictation (whisper-server) and for commands (whisper-server-commands) and command spotting (voicepuncher-command-spotting), or only the named ones; one that already answers on its port is skipped
start-model-servers *services: _dev-tools
    @{{dev_tools}} start-model-servers {{services}}

# Stop the services that start-model-servers launched (all, or the named ones: whisper-server, whisper-server-commands, voicepuncher-command-spotting); services started by hand keep running
stop-model-servers *services: _dev-tools
    @{{dev_tools}} stop-model-servers {{services}}

# ── Setup ─────────────────────────────────────────────────────────────────────

# Download default models into ~/.voicepuncher/models (whisper large-v3-turbo q5_0 ~550 MB for dictation, whisper small ~460 MB for commands, Silero VAD ~1 MB, keyword spotting ~20 MB, Silero VAD for sherpa-onnx ~1 MB)
models:
    #!/usr/bin/env bash
    set -euo pipefail
    # The app's own download: every file of its model catalog, pinned and checked by sha256,
    # resumed from a part left by an interrupted run
    cargo build --release -p voicepuncher
    args=(--download-models --models-dir ~/.voicepuncher/models)
    target/release/voicepuncher "${args[@]}"

# Download benchmark corpus — idempotent, ~500 MB, one-time setup
corpus max_samples="50":
    uv run --python 3.11 --with "datasets<3.0.0" --with soundfile --with numpy --with librosa \
        python3 e2e-test/scripts/download_corpus.py \
        --out e2e-test/corpus \
        --max-samples {{max_samples}}

# ── Model comparison ── not a gate; only when changing a model, decoding params or the prompt

# Full benchmark — every Whisper model on the full corpus
bench: _dev-tools
    @MACHINE_LOCK_TIMEOUT=14400 {{locked}} _bench

_bench:
    uv run --python 3.11 --with "datasets<3.0.0" --with soundfile --with numpy --with librosa \
        python3 e2e-test/scripts/bench.py

# Quick benchmark — 10 samples, ~2 min
bench-smoke: _dev-tools
    @{{locked}} _bench-smoke

_bench-smoke:
    uv run --python 3.11 --with "datasets<3.0.0" --with soundfile --with numpy --with librosa \
        python3 e2e-test/scripts/bench.py --limit 10

# ── Guards ── fail fast instead of letting E2E tests skip silently (GATED) ────

_require-models:
    #!/usr/bin/env bash
    set -euo pipefail
    # Defaults are found by exact file name (see the settings at the top); a default that is not on disk is empty
    for entry in VOICEPUNCHER_WHISPER_MODEL={{dictation_model_file}} VOICEPUNCHER_COMMAND_WHISPER_MODEL={{command_model_file}}; do
        var=${entry%%=*} path="${!var:-}"
        if [[ -z "$path" || ! -f "$path" ]]; then
            echo "✗ $var: no model file (got '${path}', default ${entry#*=} in ~/.voicepuncher or ~/.voicepuncher/models). Run 'just models' or set it in .env" >&2
            exit 1
        fi
    done
    echo "whisper (dictation): $VOICEPUNCHER_WHISPER_MODEL"
    echo "whisper (commands):  $VOICEPUNCHER_COMMAND_WHISPER_MODEL"

# The test runner every test recipe uses, pinned like .config/nextest.toml's nextest-version; installs nothing
_require-nextest:
    #!/usr/bin/env bash
    set -euo pipefail
    wanted=0.9.146
    # `cargo-nextest 0.9.146 (<commit> <date>)`; empty when cargo has no nextest subcommand
    got=$(cargo nextest --version 2>/dev/null | head -1 || true)
    # Empty when nextest is missing: the comparison below names it
    read -r _ version _ <<<"$got" || true
    if [[ "${version:-}" != "$wanted" ]]; then
        echo "✗ cargo-nextest $wanted not found (got '${got}') — run 'cargo +stable install --locked cargo-nextest@$wanted'" >&2
        exit 1
    fi

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
