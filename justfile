set dotenv-load

# ── Settings ── override any of these via env or .env ─────────────────────────

# Models: explicit env var wins; otherwise the model `just models` downloads for each speech recognition
# service, found by its exact file name in ~/.voicepuncher or ~/.voicepuncher/models (empty when not on disk:
# the start and _require-models then fail with "Run 'just models' or set it in .env")
dictation_model_file := "ggml-large-v3-turbo-q5_0.bin"
command_model_file := "ggml-small.bin"
# Text refinement model (llama-server)
refinement_model_file := "Qwen_Qwen3-4B-Instruct-2507-Q4_K_M.gguf"
find_model := 'find ~/.voicepuncher -maxdepth 2 -name "$1" 2>/dev/null | head -1 || true'
# Speech recognition service for dictation (what the app uses)
export VOICEPUNCHER_WHISPER_MODEL := env_var_or_default("VOICEPUNCHER_WHISPER_MODEL", shell(find_model, dictation_model_file))
# Speech recognition service for commands (the command spotting service's confirmations)
export VOICEPUNCHER_COMMAND_WHISPER_MODEL := env_var_or_default("VOICEPUNCHER_COMMAND_WHISPER_MODEL", shell(find_model, command_model_file))
# Voice activity detection model of the speech recognition service: the newest Silero model found
export VOICEPUNCHER_VAD_MODEL := env_var_or_default("VOICEPUNCHER_VAD_MODEL", `find ~/.voicepuncher -maxdepth 2 -name 'ggml-silero-*.bin' -exec ls -t {} + 2>/dev/null | head -1 || true`)
export VOICEPUNCHER_LLM_MODEL := env_var_or_default("VOICEPUNCHER_LLM_MODEL", shell(find_model, refinement_model_file))
# Command spotting service (voice control): sherpa-onnx keyword spotting model directory and its Silero VAD
export VOICEPUNCHER_KWS_MODEL_DIR := env_var_or_default("VOICEPUNCHER_KWS_MODEL_DIR", `find ~/.voicepuncher -maxdepth 2 -type d -name 'sherpa-onnx-kws-*' 2>/dev/null | head -1 || true`)
# The user's own recorded voice commands (never committed): tests run on them when they exist
export VOICEPUNCHER_VOICE_SAMPLES := env_var_or_default("VOICEPUNCHER_VOICE_SAMPLES", `test -d ~/.voicepuncher/voice-samples && echo ~/.voicepuncher/voice-samples || true`)
export VOICEPUNCHER_KWS_VAD_MODEL := env_var_or_default("VOICEPUNCHER_KWS_VAD_MODEL", `find ~/.voicepuncher -maxdepth 2 -name 'silero_vad.onnx' 2>/dev/null | head -1 || true`)

# Corpus directory containing manifest.jsonl — absolute, because cargo runs tests from e2e-test/.
# The corpus is gitignored, so a worktree without one uses the main checkout's (first of `git worktree list`)
export VOICEPUNCHER_BENCH_CORPUS := env_var_or_default("VOICEPUNCHER_BENCH_CORPUS", `main=$(git worktree list 2>/dev/null | head -1 | cut -d' ' -f1); if [ ! -f e2e-test/corpus/manifest.jsonl ] && [ -f "$main/e2e-test/corpus/manifest.jsonl" ]; then echo "$main/e2e-test/corpus"; else echo "$PWD/e2e-test/corpus"; fi`)

# Local services started by `start-model-servers`; their URLs are the app's default [transcription].base_url and [refinement].base_url
stt_port := env_var_or_default("VOICEPUNCHER_STT_PORT", "8081")
stt_url := "http://127.0.0.1:" + stt_port + "/v1"
# Copies of the speech recognition service, 1 to max_copies: whisper-server serves one request at a time, and
# two to four copies on one GPU are twice as fast on long speech. One copy listens on the service's own port;
# several listen on the first ports of the ones its clients look at (copy_ports) and nothing on its own port
stt_instances := env_var_or_default("VOICEPUNCHER_STT_INSTANCES", "3")
# The speech recognition service for commands: only the command spotting service confirms utterances with it.
# Its copies listen 10000 above its port (18086–18089), clear of the copies of the one for dictation (18082–18085)
command_stt_port := env_var_or_default("VOICEPUNCHER_COMMAND_STT_PORT", "8085")
command_stt_url := "http://127.0.0.1:" + command_stt_port + "/v1"
command_stt_instances := env_var_or_default("VOICEPUNCHER_COMMAND_STT_INSTANCES", "3")
# The most copies a speech recognition service runs: the clients look for its copies on the four ports
# 10001–10004 above its own (10000 below when they would pass 65535), crate copy-pool, address.rs MAX_COPIES
max_copies := "4"
llm_port := env_var_or_default("VOICEPUNCHER_LLM_PORT", "8080")
llm_url := "http://127.0.0.1:" + llm_port + "/v1"
kws_port := env_var_or_default("VOICEPUNCHER_KWS_PORT", "8082")
kws_url := "ws://127.0.0.1:" + kws_port
kws_binary := justfile_directory() / "target/release/voicepuncher-command-spotting"
services_run_dir := home_directory() / ".voicepuncher/run"

# The cargo selections the model stages build, shared by their recipes and `_build-e2e`, which builds them
# before the machine lock: another package or feature selection would build other artifacts
command_spotting_build := "-p voicepuncher-command-spotting"
# The app's service binaries of e2e-smoke, run one after another in this order
e2e_smoke_binaries := "transcription_openai_it textrefinement_openai_it commandspotting_websocket_it voice_samples_it voice_forms_it"
e2e_smoke_tests := "-p voicepuncher " + prepend("--test ", e2e_smoke_binaries)
e2e_seams_tests := "-p voicepuncher --test chunked_transcription_it"
placed_commands_tests := "-p voicepuncher-command-spotting --test placed_commands_it"
cucumber_e2e_tests := "-p voicepuncher-e2e-test --test cucumber_e2e"
# What `test` runs: every test target of the workspace but the E2E harness (one cargo call cannot pick targets
# per package; a binary the filter excludes is not even listed)
test_selection := "--workspace -E 'not binary_id(voicepuncher-e2e-test::cucumber_e2e)'"
# A rerun's selection (just rerun-failed): added to every nextest call of the E2E recipes, whose calls may each
# hold none of the failed tests; _test narrows its own selection with it
rerun_filter := '${VOICEPUNCHER_RERUN_FILTER:+-E "$VOICEPUNCHER_RERUN_FILTER" --no-tests=pass}'

# nextest prints captured test output unindented, so libtest's `test … FAILED` and `thread '…' panicked at
# <file:line:col>:` lines start at column 0, where `kit run` finds the failed tests and their locations
export NEXTEST_NO_OUTPUT_INDENT := "1"
services_log_dir := home_directory() / ".voicepuncher/logs"
# The stage journal: one JSON line per run of _stages (check, full-check, test, the E2E recipes), shared by
# every checkout and kept when a worktree goes; `just stage-times` summarizes it
export VOICEPUNCHER_STAGE_JOURNAL := env_var_or_default("VOICEPUNCHER_STAGE_JOURNAL", home_directory() / ".cache/voicepuncher/stage-times.jsonl")

# The machine-wide test lock (agent-kit `with-machine-lock`): one run at a time on this machine, a hung run is
# killed after its time limit. Every check, build, E2E, benchmark and real-keyboard recipe run on its own runs its
# private `_<name>` body under it, except `test` and `release`, which use no shared service, port or GPU (see
# their comments). `check` and `full-check` take it only for their model stages (see _stages)
machine_lock := env_var_or_default("MACHINE_LOCK", home_directory() / ".claude/bin/with-machine-lock")
# The service ports of every run under the lock: apart from the defaults above, which the user's own services
# hold, and from their copies 10000 above (18082–18089); these copies listen on 38082–38089, below the free
# ports (49152+) the @local_services and @resilience steps take. A locked run starts every service it tests
# here and never reuses one it did not start (_start-model-servers-for-e2e)
test_stt_port := env_var_or_default("VOICEPUNCHER_TEST_STT_PORT", "28081")
test_command_stt_port := env_var_or_default("VOICEPUNCHER_TEST_COMMAND_STT_PORT", "28085")
test_llm_port := env_var_or_default("VOICEPUNCHER_TEST_LLM_PORT", "28080")
test_kws_port := env_var_or_default("VOICEPUNCHER_TEST_KWS_PORT", "28082")
locked := "env VOICEPUNCHER_STT_PORT=" + test_stt_port + " VOICEPUNCHER_COMMAND_STT_PORT=" + test_command_stt_port + " VOICEPUNCHER_LLM_PORT=" + test_llm_port + " VOICEPUNCHER_KWS_PORT=" + test_kws_port + " VOICEPUNCHER_TEST_PORTS=1 " + quote(machine_lock) + " " + quote(just_executable()) + " --justfile " + quote(justfile())

# List all recipes
default:
    @just --list --unsorted

# ── Checks ── `check` while developing, `full-check` before merging ─────────

# Only the model stages take the machine lock, held ≈ 7.3 min (measured), after the code checks, the build and
# build-e2e ran without it (see _stages). Incremental, no `cargo clean`
# FAST (~8.5 min), run often while developing: code checks + E2E smoke + accuracy on 10 samples + refinement evaluation
check:
    @VOICEPUNCHER_STAGES_RECIPE=check {{just_executable()}} _stages fmt lint test build build-e2e e2e-smoke e2e-quick refine-eval

# Only the model stages take the machine lock, held ≈ 16.3 min with 100 recordings and ≈ 14.5 min with 10 (measured)
# (see _stages). Incremental like `check`, no `cargo clean`: the clean build is `release`'s. Accuracy runs on 100
# recordings when the branch changes the recognition path (recognition_paths), on main itself or when that cannot
# be told, else on 10 (_accuracy-stage); VOICEPUNCHER_ACCURACY_SAMPLE=100 forces 100
# FULL (~15.5–17.5 min), run before merging: code checks + E2E (the whole voice control set, seams and outages included) + accuracy on 100 or 10 samples + refinement evaluation
full-check: _require-models _require-corpus
    #!/usr/bin/env bash
    set -euo pipefail
    choice=$({{just_executable()}} _accuracy-stage)
    stage=${choice%% *}
    reason=${choice#* — }
    case "$stage" in
        e2e-bench) count=100 ;;
        e2e-quick) count=10 ;;
        *) echo "✗ _accuracy-stage printed no stage: '$choice'" >&2; exit 1 ;;
    esac
    accuracy="accuracy: $count recordings ($stage) — $reason"
    echo "● $accuracy"
    VOICEPUNCHER_STAGES_RECIPE=full-check VOICEPUNCHER_STAGES_ACCURACY="$accuracy" \
        {{just_executable()}} _stages fmt lint test build bundle build-e2e e2e-smoke e2e-regression e2e-seams e2e-resilience "$stage" refine-eval
    echo "✓ full-check passed — $accuracy; before a release also walk through README.md § Release checklist (manual run)"

# Summary of the stage journal for the runs since a date (2026-10-01) or a span (7d): per recipe and stage the
# runs, median, total and failures; the lock wait; the failed tests, most first; runs and time per branch
# (a change's branch is <type>/<change>, so per change)
stage-times $since="30d":
    #!/usr/bin/env perl
    use strict;
    use warnings;
    use JSON::PP;
    use POSIX qw(strftime);
    use Time::Local qw(timegm timelocal);
    my $path = $ENV{VOICEPUNCHER_STAGE_JOURNAL};
    my $since = $ENV{since};
    my $from;
    if ($since =~ /^(\d+)d$/) { $from = time - $1 * 86400 }
    elsif ($since =~ /^(\d{4})-(\d\d)-(\d\d)$/) { $from = timelocal(0, 0, 0, $3, $2 - 1, $1) }
    else { die "✗ since '$since': give a span like 7d or a date like 2026-10-01\n" }
    my $period = strftime("%Y-%m-%d %H:%M", localtime $from) . ($since =~ /d$/ ? " ($since)" : "");
    my $epoch = sub {
        my ($y, $mo, $d, $h, $mi, $s, $sign, $oh, $om) = $_[0] =~ /^(\d+)-(\d+)-(\d+)T(\d+):(\d+):(\d+)([+-])(\d\d):(\d\d)$/
            or return;
        timegm($s, $mi, $h, $d, $mo - 1, $y) - ($sign eq "-" ? -1 : 1) * ($oh * 3600 + $om * 60);
    };
    my $span = sub {
        my $s = shift;
        return sprintf("%.1fs", $s) if $s < 60;
        return sprintf("%dm %02ds", $s / 60, $s % 60) if $s < 3600;
        sprintf("%dh %02dm", $s / 3600, ($s % 3600) / 60);
    };
    my $median = sub {
        my @sorted = sort { $a <=> $b } @_;
        my $mid = int(@sorted / 2);
        @sorted % 2 ? $sorted[$mid] : ($sorted[$mid - 1] + $sorted[$mid]) / 2;
    };
    my (@runs, %results);
    if (open my $journal, "<", $path) {
        while (my $line = <$journal>) {
            my $run = eval { decode_json($line) } or next;
            my $at = $epoch->($run->{started} // "");
            push @runs, $run if defined $at && $at >= $from;
        }
    }
    unless (@runs) { print "no runs since $period in $path\n"; exit 0 }
    $results{$_->{result}}++ for @runs;
    my (@keys, %stages, @waits, %tests, @branches, %branches);
    for my $run (@runs) {
        for my $stage (@{ $run->{stages} }) {
            next if $stage->{result} eq "skipped";
            my $key = "$run->{recipe}\t$stage->{name}";
            push @keys, $key unless $stages{$key};
            my $row = $stages{$key} ||= { seconds => [], failed => 0 };
            push @{ $row->{seconds} }, $stage->{seconds};
            $row->{failed}++ if $stage->{result} eq "failed";
        }
        push @waits, $run->{lock_wait} if defined $run->{lock_wait} && $run->{lock_wait} =~ /^[\d.]+$/;
        $tests{$_}++ for @{ $run->{failed_tests} };
        my $branch = $run->{branch} // "(none)";
        push @branches, $branch unless $branches{$branch};
        my $row = $branches{$branch} ||= { runs => 0, failed => 0, seconds => 0 };
        $row->{runs}++;
        $row->{failed}++ if $run->{result} eq "failed";
        $row->{seconds} += $run->{seconds};
    }
    printf "stage times since %s: %d runs — %s\n", $period, scalar @runs,
        join(", ", map { "$results{$_} $_" } grep { $results{$_} } qw(passed failed interrupted));
    printf "\n%-16s %-16s %5s %9s %10s %7s\n", qw(recipe stage runs median total failed);
    for my $key (@keys) {
        my ($recipe, $stage) = split /\t/, $key;
        my $row = $stages{$key};
        my @seconds = @{ $row->{seconds} };
        my $total = 0;
        $total += $_ for @seconds;
        printf "%-16s %-16s %5d %9s %10s %7d\n", $recipe, $stage, scalar @seconds, $span->($median->(@seconds)), $span->($total), $row->{failed};
    }
    if (@waits) {
        my $total = 0;
        $total += $_ for @waits;
        printf "\nlock wait: %d runs, median %s, total %s\n", scalar @waits, $span->($median->(@waits)), $span->($total);
    }
    if (%tests) {
        print "\nfailed tests:\n";
        printf "%6d  %s\n", $tests{$_}, $_ for sort { $tests{$b} <=> $tests{$a} || $a cmp $b } keys %tests;
    }
    print "\nruns per branch:\n";
    for my $branch (sort { $branches{$b}{runs} <=> $branches{$a}{runs} } @branches) {
        my $row = $branches{$branch};
        printf "  %-40s %4d runs %4d failed %10s\n", $branch, $row->{runs}, $row->{failed}, $span->($row->{seconds});
    }

# The paths whose change makes full-check run the accuracy stage on 100 recordings: what the two @accuracy
# scenarios run — file mode, transcription and chunking; text refinement; command spotting (keywords.txt is
# compiled in) and voice-command matching; the WER and corpus test code; the justfile whole (models, service
# settings); the Cargo files when a line other than `version = "…"` changed. One glob per line, matched with
# bash `[[ == ]]` (`*` spans `/`). A branch that adds a file on this path adds it here
recognition_paths := '''
    copy-pool/src/*
    app/src/main.rs
    app/src/adapters/primary/cli/*
    app/src/config.rs
    app/src/config/*
    app/src/adapters/secondary/recording/filesystem/*
    app/src/adapters/secondary/transcription/*
    app/src/adapters/secondary/http/*
    app/src/adapters/secondary/textpasting/stdio/*
    app/src/core/application/pipeline.rs
    app/src/core/application/chunking.rs
    app/src/core/application/transcriber.rs
    app/src/core/application/transcript_filter.rs
    app/src/core/domain/audio_buffer.rs
    app/src/core/domain/raw_transcript.rs
    app/src/adapters/secondary/textrefinement/*
    app/src/core/application/text_refiner.rs
    app/src/core/domain/cleaned_text.rs
    app/src/core/domain/filler_sounds.rs
    app/src/core/domain/normalization_check.rs
    app/src/core/domain/command_words.rs
    command-spotting/src/*
    command-spotting/keywords.txt
    app/src/adapters/secondary/commandspotting/websocket/client.rs
    app/src/core/domain/voice_command.rs
    app/src/core/domain/voice_command_recognizer.rs
    e2e-test/src/corpus.rs
    e2e-test/src/wer.rs
    e2e-test/src/concurrency.rs
    e2e-test/src/speech.rs
    e2e-test/src/voice_commands.rs
    e2e-test/src/scenario_filter.rs
    e2e-test/tests/cucumber_e2e.rs
    e2e-test/tests/steps/mod.rs
    e2e-test/tests/steps/file_transcription.rs
    e2e-test/tests/steps/voice_control.rs
    e2e-test/tests/features/file_transcription.feature
    e2e-test/tests/features/voice_control.feature
    justfile
'''

# Prints the accuracy stage full-check runs and why: `e2e-bench — <reason>` (100 recordings) or `e2e-quick —
# <reason>` (10). 100 when VOICEPUNCHER_ACCURACY_SAMPLE=100 (any other value fails), when the changed paths
# against the merge base with <base> (local ref, no fetch; committed, uncommitted and untracked, renames as
# delete + add) cannot be read, when there are none (main itself), or when one matches recognition_paths or
# changes a Cargo file beyond its `version = "…"` lines; else 10
_accuracy-stage base="origin/main":
    #!/usr/bin/env bash
    set -euo pipefail
    choose() { echo "$1 — $2"; exit 0; }
    case "${VOICEPUNCHER_ACCURACY_SAMPLE:-}" in
        "") ;;
        100) choose e2e-bench "forced by VOICEPUNCHER_ACCURACY_SAMPLE=100" ;;
        *) echo "✗ VOICEPUNCHER_ACCURACY_SAMPLE=${VOICEPUNCHER_ACCURACY_SAMPLE}: only 100 is allowed (the switch can only add recordings)" >&2; exit 1 ;;
    esac
    unreadable="cannot tell what the branch changed against {{base}}"
    merge_base=$(git merge-base HEAD "{{base}}" 2>/dev/null) || choose e2e-bench "$unreadable"
    changed=$(git diff --name-only --no-renames "$merge_base" 2>/dev/null) || choose e2e-bench "$unreadable"
    untracked=$(git ls-files --others --exclude-standard 2>/dev/null) || choose e2e-bench "$unreadable"
    changed=$(printf '%s\n%s\n' "$changed" "$untracked" | sed '/^$/d' | sort -u)
    [[ -n "$changed" ]] || choose e2e-bench "no change against {{base}} (main itself)"
    # A Cargo file counts when a line other than `version = "…"` changed (an untracked one counts whole)
    beyond_version() {
        local diff line
        git ls-files --error-unmatch -- "$1" >/dev/null 2>&1 || return 0
        diff=$(git diff --no-renames -U0 "$merge_base" -- "$1") || return 0
        while IFS= read -r line; do
            case "$line" in
                "+++ "* | "--- "*) ;;
                [+-]'version = "'*'"') ;;
                [+-]*) return 0 ;;
            esac
        done <<<"$diff"
        return 1
    }
    patterns='{{recognition_paths}}'
    set -f
    total=0 matches="" count=0
    while IFS= read -r path; do
        total=$((total + 1))
        hit=""
        case "$path" in
            Cargo.lock | Cargo.toml | */Cargo.toml) ! beyond_version "$path" || hit=1 ;;
            *) for pattern in $patterns; do [[ "$path" != $pattern ]] || { hit=1; break; }; done ;;
        esac
        [[ -n "$hit" ]] || continue
        count=$((count + 1))
        (( count > 3 )) || matches+="${matches:+, }$path"
    done <<<"$changed"
    if (( count > 3 )); then
        choose e2e-bench "recognition paths changed: $matches and $((count - 3)) more"
    elif (( count > 0 )); then
        choose e2e-bench "recognition paths changed: $matches"
    fi
    choose e2e-quick "no recognition path among the $total changed files"

# Runs the named stages (private recipes without their `_`) in two phases. The stages before the first model
# stage (`e2e*` or `refine-eval`) run one by one without the machine lock, each through the wrapper with a lock
# directory of the run's own — the device `test` uses, borrowed: it only gives the stage the time limit
# (MACHINE_LOCK_TIMEOUT, exit 124; the timeout message says `[machine-lock]`, though no machine lock is held; a
# run killed with KILL leaves that directory, holding an empty tests.lock, in $TMPDIR, which macOS clears); fmt,
# lint, build and build-e2e run at the utility QoS clamp. The rest runs in _model-stages under the machine
# lock (at once when the caller already holds it). A failing stage ends the run before the lock is requested.
# Ends with one table: each stage's time, the lock wait and the lock hold, and adds the run to the stage journal
# (_stage-journal-append; the caller names itself in VOICEPUNCHER_STAGES_RECIPE, full-check its accuracy choice in
# VOICEPUNCHER_STAGES_ACCURACY); every stage's output is also copied to a log of the run, where the journal finds
# the failed tests. Two checks of one worktree must not overlap: the lock no longer keeps their builds apart
_stages +stages:
    #!/usr/bin/env bash
    set -euo pipefail
    now() { perl -MTime::HiRes=time -e 'printf "%.1f", time'; }
    since() { perl -e "printf '%.1f', ${2:-$(now)} - $1"; }
    row() { printf '  %s %-14s %s\n' "$1" "$2" "$3"; }
    started=$(now)
    report=$(mktemp)
    run_log=$(mktemp)
    stage_lock_dir=$(mktemp -d)
    export VOICEPUNCHER_STAGES_REPORT="$report"
    # The stages' output also goes to run_log, for the failed tests of the journal: behind that copy cargo and
    # nextest see no terminal, so they keep their colors only when this run's output is one
    [[ ! -t 1 || -n "${CARGO_TERM_COLOR:-}" ]] || export CARGO_TERM_COLOR=always
    caller=""
    if [[ "${MACHINE_LOCK_HELD:-}" == "${MACHINE_LOCK_DIR:-$HOME/.cache/machine-lock}/tests.lock" ]]; then
        caller=1
        export VOICEPUNCHER_STAGES_CALLER_LOCK=1
    fi
    unlocked="" model=""
    for stage in {{stages}}; do
        if [[ -n "$model" || "$stage" == e2e* || "$stage" == refine-eval ]]; then
            model+=" $stage"
        else
            unlocked+=" $stage"
        fi
    done
    run="" status=0 interrupted="" requested="" returned=""
    finish() {
        code=$?
        [[ -z "$interrupted" || $code -ne 0 ]] || code=130
        [[ -z "$requested" || -n "$returned" ]] || returned=$(now)
        printf '\nstages:\n'
        locked=""
        while read -r kind a b c; do
            case "$kind" in
                stage) row "$a" "$b" "$(printf '%7ss' "$c")" ;;
                locked)
                    locked=$a
                    if [[ -n "$caller" ]]; then
                        row "●" "lock wait" "held by the caller"
                    else
                        row "●" "lock wait" "$(printf '%7ss' "$(since "$requested" "$locked")")"
                    fi ;;
            esac
        done <"$report"
        if [[ -n "$requested" && -z "$locked" ]]; then
            row "✗" "lock wait" "$(printf '%7ss' "$(since "$requested" "$returned")")"
        elif [[ -n "$locked" && -n "$caller" ]]; then
            row "●" "lock held" "held by the caller"
        elif [[ -n "$locked" ]]; then
            row "●" "lock held" "$(printf '%7ss' "$(since "$locked" "$returned")")"
        fi
        ended=$(now)
        printf '  %-16s %7ss\n' total "$(since "$started" "$ended")"
        VOICEPUNCHER_STAGES_REPORT="$report" STAGES_LOG="$run_log" STAGES_LIST="{{stages}}" STAGES_STARTED="$started" \
            STAGES_RETURNED="$returned" STAGES_ENDED="$ended" STAGES_EXIT="$code" STAGES_INTERRUPTED="$interrupted" \
            STAGES_CALLER="$caller" STAGES_WORKTREE={{quote(justfile_directory())}} \
            {{just_executable()}} --justfile {{quote(justfile())}} _stage-journal-append \
            || echo "✗ the run was not added to the stage journal" >&2
        rm -rf "$report" "$run_log" "$stage_lock_dir"
        exit "$code"
    }
    trap finish EXIT
    # Each phase runs in the background, so a signal to this script reaches its wrapper and stops it
    trap 'interrupted=1; [[ -z "$run" ]] || kill -TERM "$run" 2>/dev/null || true' TERM INT HUP
    run_phase() {
        "$@" > >(tee -a "$run_log") 2> >(tee -a "$run_log" >&2) &
        run=$!
        while true; do
            wait "$run" && status=0 || status=$?
            kill -0 "$run" 2>/dev/null || break
        done
        run=""
    }
    for stage in $unlocked; do
        t0=$(now)
        # Builds run at the utility QoS clamp, so they yield the CPU to another run's model stages; the
        # `test` stage clamps only its own build phase (see _test)
        clamp=()
        case "$stage" in fmt|lint|build|bundle|build-e2e) clamp=(taskpolicy -c utility) ;; esac
        run_phase ${clamp[@]+"${clamp[@]}"} env MACHINE_LOCK_DIR="$stage_lock_dir" {{locked}} "_$stage"
        mark="✓"
        [[ $status -eq 0 && -z "$interrupted" ]] || mark="✗"
        echo "stage $mark $stage $(since "$t0")" >>"$report"
        [[ $status -eq 0 ]] || exit "$status"
        [[ -z "$interrupted" ]] || exit 130
    done
    [[ -n "$model" ]] || exit 0
    requested=$(now)
    echo "requested $requested" >>"$report"
    if [[ -n "$caller" ]]; then
        echo "● $(date +%H:%M:%S) the caller holds the machine lock:$model"
    else
        echo "→ $(date +%H:%M:%S) waiting for the machine lock:$model"
    fi
    run_phase {{locked}} _model-stages $model
    returned=$(now)
    [[ $status -eq 0 ]] || exit "$status"
    [[ -z "$interrupted" ]] || exit 130

# The model stages of _stages, run under the machine lock: starts the services on the test ports, runs
# the stages with VOICEPUNCHER_E2E_SERVICES_HELD=1, so no E2E recipe starts or stops one, and stops them
# on exit, also when the start or a stage is interrupted; appends its rows to VOICEPUNCHER_STAGES_REPORT
_model-stages +stages:
    #!/usr/bin/env bash
    set -euo pipefail
    now() { perl -MTime::HiRes=time -e 'printf "%.1f", time'; }
    since() { perl -e "printf '%.1f', $(now) - $1"; }
    report="${VOICEPUNCHER_STAGES_REPORT:-/dev/null}"
    taken=$(now)
    echo "locked $taken" >>"$report"
    requested=$(sed -n 's/^requested //p' "$report" | tail -1)
    if [[ -n "$requested" && -z "${VOICEPUNCHER_STAGES_CALLER_LOCK:-}" ]]; then
        echo "● $(date +%H:%M:%S) machine lock taken after $(perl -e "printf '%.0f', $taken - $requested") s"
    fi
    services="" current="" t0=""
    finish() {
        code=$?
        [[ -z "$current" ]] || echo "stage ✗ $current $(since "$t0")" >>"$report"
        if [[ -n "$services" ]]; then
            unset VOICEPUNCHER_E2E_SERVICES_HELD
            {{just_executable()}} _stop-model-servers-for-e2e || true
        fi
        exit "$code"
    }
    trap finish EXIT
    trap 'exit 143' TERM
    trap 'exit 130' INT
    trap 'exit 129' HUP
    # e2e-resilience starts services of its own on ports of its own: alone, it needs none of these
    if [[ " {{stages}} " != " e2e-resilience " ]]; then
        # Set before the start: the markers of the services it starts exist before they do
        services=up
        {{just_executable()}} _start-model-servers-for-e2e
        export VOICEPUNCHER_E2E_SERVICES_HELD=1
    fi
    for stage in {{stages}}; do
        current=$stage
        t0=$(now)
        status=0
        {{just_executable()}} "_$stage" || status=$?
        mark="✓"
        [[ $status -eq 0 ]] || mark="✗"
        echo "stage $mark $stage $(since "$t0")" >>"$report"
        current=""
        [[ $status -eq 0 ]] || exit "$status"
    done

# Appends the record of one _stages run to the stage journal, from what _stages hands over in STAGES_* and the
# report: when, where, which recipe, each requested stage (passed, failed, or skipped when the run ended before
# it), the lock wait and hold, the accuracy choice, and the failed tests found in the stages' output — nextest's
# FAIL, TIMEOUT and SIG* result lines and each Gherkin scenario with a failed step. Under flock, so the lines of
# parallel runs never mix
_stage-journal-append:
    #!/usr/bin/env perl
    use strict;
    use warnings;
    use Fcntl qw(:flock);
    use File::Basename qw(dirname);
    use File::Path qw(make_path);
    use JSON::PP;
    use POSIX qw(strftime);
    my $json = JSON::PP->new->allow_nonref->canonical;
    my %env = map { $_ => $ENV{"STAGES_$_"} // "" } qw(LOG LIST STARTED RETURNED ENDED EXIT INTERRUPTED CALLER WORKTREE);
    my ($requested, $locked, %rows);
    open my $report, "<", $ENV{VOICEPUNCHER_STAGES_REPORT} or die "report: $!";
    while (<$report>) {
        my ($kind, @fields) = split;
        next unless defined $kind;
        if ($kind eq "stage") { $rows{$fields[1]} = [$fields[0] eq "✓" ? "passed" : "failed", $fields[2] + 0] }
        elsif ($kind eq "requested") { $requested = $fields[0] }
        elsif ($kind eq "locked") { $locked = $fields[0] }
    }
    my @stages = map { my $row = $rows{$_}; $row ? [$_, @$row] : [$_, "skipped", undef] } split " ", $env{LIST};
    my $round = sub { defined $_[0] ? sprintf("%.1f", $_[0]) + 0 : undef };
    my $end = length $env{RETURNED} ? $env{RETURNED} : $env{ENDED};
    my ($wait, $held);
    if ($env{CALLER} && defined $locked) { ($wait, $held) = ("caller", "caller") }
    elsif (defined $requested) {
        $wait = $round->((defined $locked ? $locked : $end) - $requested);
        $held = defined $locked ? $round->($end - $locked) : undef;
    }
    my (@failed, %seen, $scenario);
    if (open my $log, "<", $env{LOG}) {
        while (my $line = <$log>) {
            $line =~ s/\e\[[0-9;]*[A-Za-z]//g;
            $line =~ s/\s+$//;
            my $name;
            if ($line =~ /^\s*(FAIL|TIMEOUT|SIG\w+|ABORT)\s+\[\s*[\d.]+s\]\s+(?:\(\s*\d+\/\d+\)\s+)?(.+)$/) {
                my ($outcome, $test) = ($1, $2);
                # The tests an interrupt stopped are not failures of their own
                $name = $test unless $env{INTERRUPTED} && $outcome =~ /^SIG(?:INT|TERM|HUP|KILL)$/;
            }
            elsif ($line =~ /^\s*(?:Scenario|Scenario Outline|Example):\s*(.+)$/) { $scenario = $1 }
            elsif ($line =~ /^\s*✘/ && defined $scenario) { $name = "scenario: $scenario" }
            push @failed, $name if defined $name && !$seen{$name}++;
        }
    }
    my $git = sub { my $out = `git -C "$env{WORKTREE}" @_ 2>/dev/null`; chomp $out; length $out ? $out : undef };
    my $started = strftime("%Y-%m-%dT%H:%M:%S%z", localtime $env{STARTED});
    $started =~ s/(\d\d)(\d\d)$/$1:$2/;
    my $result = $env{EXIT} eq "0" ? "passed" : $env{INTERRUPTED} ? "interrupted" : "failed";
    my @record = (
        started => $started,
        seconds => $round->($env{ENDED} - $env{STARTED}),
        worktree => $env{WORKTREE},
        branch => $git->("rev-parse --abbrev-ref HEAD"),
        commit => $git->("rev-parse --short HEAD"),
        recipe => $ENV{VOICEPUNCHER_STAGES_RECIPE} || "stages",
        result => $result,
        exit => $env{EXIT} + 0,
        stages => [map { +{ name => $_->[0], result => $_->[1], seconds => $_->[2] } } @stages],
        lock_wait => $wait,
        lock_held => $held,
        accuracy => $ENV{VOICEPUNCHER_STAGES_ACCURACY} || undef,
        failed_tests => \@failed,
    );
    my @pairs;
    while (my ($key, $value) = splice @record, 0, 2) { push @pairs, $json->encode($key) . ":" . $json->encode($value) }
    my $path = $ENV{VOICEPUNCHER_STAGE_JOURNAL};
    make_path(dirname($path));
    open my $journal, ">>", $path or die "$path: $!";
    flock $journal, LOCK_EX or die "flock $path: $!";
    print $journal "{" . join(",", @pairs) . "}\n";
    close $journal or die "$path: $!";

# Builds what the model stages run — the command spotting service and their test binaries — so nothing
# compiles under the machine lock; warm, five no-op cargo calls. The test binaries are built as the E2E
# recipes' nextest runs build them
_build-e2e: _require-nextest
    cargo build --release {{command_spotting_build}}
    cargo nextest run {{e2e_smoke_tests}} --no-run
    cargo nextest run {{e2e_seams_tests}} --no-run
    cargo nextest run {{placed_commands_tests}} --no-run
    cargo nextest run {{cucumber_e2e_tests}} --no-run

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

# Runs without the machine-wide lock, as an unlocked stage of _stages (which records it in the stage journal): it
# uses no shared service, port, GPU or exclusive device. A lock directory of the run's own only gives it the wrapper's time limit (MACHINE_LOCK_TIMEOUT, exit 124; the timeout message
# says `[machine-lock]`). Departs from agent-kit `machine-lock.md` on purpose until the kit's exception lands.
# Every test of the app, the command spotting service and the E2E harness under cargo-nextest, each within its time limit, unlocked (~1 min)
test:
    @VOICEPUNCHER_STAGES_RECIPE=test {{just_executable()}} _stages test

# The rerun takes the worktree's last record of the stage journal that ran a stage, its failed stage and its failed tests (or the
# names given, in the journal's form: `<package>::<binary> <test>`, `scenario: <name>`), and runs that one stage
# with them alone — same build, services, test ports, machine lock and time limits — then fails naming any of
# them that never ran. Journaled as `rerun-failed`, never the change's passing check
# After a failure: rerun only the failed tests and scenarios of the last run, in their stage; once green, run the stage's recipe or the check whole once
[positional-arguments]
rerun-failed *names:
    #!/usr/bin/env bash
    set -euo pipefail
    plan=$(mktemp) log=$(mktemp)
    trap 'rm -f "$plan" "$log"' EXIT
    STAGES_WORKTREE={{quote(justfile_directory())}} perl -MJSON::PP -e '
        my ($journal, $worktree, @names) = ($ENV{VOICEPUNCHER_STAGE_JOURNAL}, $ENV{STAGES_WORKTREE}, @ARGV);
        my $last;
        if (open my $fh, "<", $journal) {
            while (<$fh>) {
                my $run = eval { decode_json($_) } or next;
                next unless ($run->{worktree} // "") eq $worktree;
                # A run that ended before its first stage (a lock wait given up or interrupted, a failed setup)
                # tested nothing: the run before it still holds the failures
                next if $run->{result} ne "passed" && !grep { $_->{result} ne "skipped" } @{ $run->{stages} };
                $last = $run;
            }
        }
        die "✗ no run of this worktree in the stage journal ($journal)\n" unless $last;
        my $what = "the last run ($last->{recipe}, $last->{started})";
        die "✗ $what was interrupted: run its recipe again\n" if $last->{result} eq "interrupted";
        my @stages = @{ $last->{stages} };
        my $stage;
        if ($last->{result} eq "passed") {
            unless (@names) { print "● nothing to rerun: $what passed\n"; exit }
            # Names given after a passed run: the stage is the one of that run, when it ran only one
            die "✗ $what passed and ran several stages: run the recipe of the stage your names belong to first\n" if @stages != 1;
            $stage = $stages[0]{name};
        } else {
            ($stage) = map { $_->{name} } grep { $_->{result} eq "failed" } @stages;
        }
        @names = @{ $last->{failed_tests} } unless @names;
        my (@tests, @scenarios, $harness);
        for my $name (@names) {
            if ($name =~ /^scenario:\s*(.+)$/) { push @scenarios, $1; next }
            my ($binary, $test) = $name =~ /^(\S+)\s+(.+)$/ or die "✗ \"$name\" is neither `<package>::<binary> <test>` nor `scenario: <name>`\n";
            $harness = 1 if $binary =~ /::cucumber_(?:integration|e2e)$/;
            push @tests, "(binary_id(=$binary) & test(=$test))";
        }
        # Scenarios named by hand, without their harness test: every harness test of the stage runs, each only them
        push @tests, "binary(=cucumber_integration)", "binary(=cucumber_e2e)" if @scenarios && !$harness;
        print "run $stage\n";
        print "filter ", join(" | ", @tests), "\n";
        print "scenario $_\n" for @scenarios;
        print "name $_\n" for @names;
        printf "● rerun of %s: stage %s, %s\n", $what, $stage, @names ? scalar(@names) . " test(s)" : "no named test, so the whole stage";
    ' -- "$@" >"$plan"
    grep -v '^\(run\|filter\|scenario\|name\) ' "$plan" || true
    stage=$(sed -n 's/^run //p' "$plan")
    [[ -n "$stage" ]] || exit 0
    filter=$(sed -n 's/^filter //p' "$plan")
    scenarios=$(sed -n 's/^scenario //p' "$plan")
    status=0
    VOICEPUNCHER_STAGES_RECIPE=rerun-failed VOICEPUNCHER_RERUN_FILTER="$filter" VOICEPUNCHER_RERUN_SCENARIOS="$scenarios" \
        NEXTEST_SUCCESS_OUTPUT=immediate {{just_executable()}} --justfile {{quote(justfile())}} _stages "$stage" \
        > >(tee "$log") 2> >(tee -a "$log" >&2) || status=$?
    wait
    # Every name must have run: nextest's result line of the test, or the scenario's line in the harness output
    missing=$(sed -n 's/^name //p' "$plan" | RERUN_LOG="$log" perl -ne '
        BEGIN { open my $fh, "<", $ENV{RERUN_LOG} or die; local $/; ($log = <$fh>) =~ s/\e\[[0-9;]*[A-Za-z]//g }
        chomp; my $name = $_;
        my $seen = $name =~ /^scenario:\s*(.+)$/
            ? $log =~ /^\s*(?:Scenario|Scenario Outline|Example):\s*\Q$1\E\s*$/m
            : $log =~ /^\s*(?:PASS|FAIL|FLAKY|TIMEOUT|SIG\w+|ABORT)\s+\[[^\]]*\]\s+(?:\(\s*\d+\/\d+\)\s+)?\Q$name\E\s*$/m;
        print "  $name\n" unless $seen;
    ')
    if [[ -n "$missing" ]]; then
        printf '✗ never ran in this rerun:\n%s\n' "$missing" >&2
        [[ $status -ne 0 ]] || status=1
    fi
    exit "$status"

# One nextest run over the workspace's test targets (libs, bins, integration tests and the slice/component
# harness; no doctests, as before) except the E2E harness, whose suites are the E2E recipes'. nextest runs up
# to one test per core over all binaries, each test in a process of its own in its package directory, with
# the limits of .config/nextest.toml: a test over its limit is stopped and reported TIMEOUT, and the run ends
# listing the failed, timed-out and slow tests after its `Summary` line. The build runs at the utility QoS
# clamp; the tests keep the default QoS (clamped timers fire late under load). A rerun (VOICEPUNCHER_RERUN_FILTER)
# runs only the tests of that selection within it
_test: _require-nextest
    #!/usr/bin/env bash
    set -euo pipefail
    selection=({{test_selection}})
    if [[ -n "${VOICEPUNCHER_RERUN_FILTER:-}" ]]; then
        selection=(--workspace -E "not binary_id(voicepuncher-e2e-test::cucumber_e2e) & ($VOICEPUNCHER_RERUN_FILTER)" --no-tests=pass)
    fi
    taskpolicy -c utility cargo nextest run "${selection[@]}" --no-run
    cargo nextest run "${selection[@]}"

# Run the app for real use with one command: download missing models, restart the services (the command spotting service is rebuilt), build and start the app with voice control; extra arguments go to the app; text refinement is off unless they hold `--llm`, and only then are its model and llama-server needed (`just run --llm`)
run *args:
    #!/usr/bin/env bash
    set -euo pipefail
    services="whisper-server whisper-server-commands voicepuncher-command-spotting"
    refinement=no
    for arg in {{args}}; do
        [[ "$arg" == --llm ]] && refinement=yes
    done
    [[ "$refinement" == yes ]] && services="$services llama-server"
    just models "$refinement"
    just stop-model-servers
    just start-model-servers $services
    cargo run --release -p voicepuncher -- --voice-control {{args}}

# Build the release binary
build:
    @{{locked}} _build

_build:
    cargo build --release

# E2E on real models: smoke + chunk seams + WER accuracy on 100 samples + refinement evaluation (~13 min)
e2e:
    @VOICEPUNCHER_STAGES_RECIPE=e2e {{just_executable()}} _stages e2e-smoke e2e-seams e2e-bench refine-eval

# Smoke test — services and models load, file mode runs and voice commands are spotted (@smoke scenarios), the adapters talk to the real services
e2e-smoke:
    @VOICEPUNCHER_STAGES_RECIPE=e2e-smoke {{just_executable()}} _stages e2e-smoke

# One nextest run per test binary, one after another as cargo test ran them, so the services see the
# load of one binary's tests at a time; the first failing binary ends the suite. The scenario selection and
# the limits are the e2e-smoke profile's (.config/nextest.toml); the Gherkin run streams (--no-capture)
_e2e-smoke: _require-nextest _require-models _build
    #!/usr/bin/env bash
    set -euo pipefail
    trap '{{just_executable()}} _stop-model-servers-for-e2e' EXIT
    {{just_executable()}} _start-model-servers-for-e2e
    export VOICEPUNCHER_TRANSCRIPTION_URL="{{stt_url}}" VOICEPUNCHER_REFINEMENT_URL="{{llm_url}}" VOICEPUNCHER_COMMAND_SPOTTING_URL="{{kws_url}}"
    export VOICEPUNCHER_COMMAND_TRANSCRIPTION_URL="{{command_stt_url}}"
    for binary in {{e2e_smoke_binaries}}; do
        cargo nextest run --profile e2e-smoke -p voicepuncher --test "$binary" {{rerun_filter}}
    done
    cargo nextest run --profile e2e-smoke {{placed_commands_tests}} {{rerun_filter}}
    cargo nextest run --profile e2e-smoke {{cucumber_e2e_tests}} --no-capture {{rerun_filter}}

# The voice control scenarios beyond the smoke set (@regression), run by full-check right after e2e-smoke so it still runs the whole set
e2e-regression:
    @VOICEPUNCHER_STAGES_RECIPE=e2e-regression {{just_executable()}} _stages e2e-regression

_e2e-regression: _require-nextest _require-models _build
    #!/usr/bin/env bash
    set -euo pipefail
    trap '{{just_executable()}} _stop-model-servers-for-e2e' EXIT
    {{just_executable()}} _start-model-servers-for-e2e
    export VOICEPUNCHER_TRANSCRIPTION_URL="{{stt_url}}" VOICEPUNCHER_REFINEMENT_URL="{{llm_url}}" VOICEPUNCHER_COMMAND_SPOTTING_URL="{{kws_url}}"
    export VOICEPUNCHER_COMMAND_TRANSCRIPTION_URL="{{command_stt_url}}"
    cargo nextest run --profile e2e-regression {{cucumber_e2e_tests}} --no-capture {{rerun_filter}}

# WER accuracy on 10 samples (5 per language) — fails if mean WER > 30 %, and no command in 10 recordings of ordinary speech (~1 min)
e2e-quick:
    @VOICEPUNCHER_STAGES_RECIPE=e2e-quick {{just_executable()}} _stages e2e-quick

_e2e-quick: _require-nextest _require-models _require-corpus _build
    #!/usr/bin/env bash
    set -euo pipefail
    trap '{{just_executable()}} _stop-model-servers-for-e2e' EXIT
    {{just_executable()}} _start-model-servers-for-e2e
    VOICEPUNCHER_TRANSCRIPTION_URL="{{stt_url}}" VOICEPUNCHER_REFINEMENT_URL="{{llm_url}}" VOICEPUNCHER_COMMAND_SPOTTING_URL="{{kws_url}}" VOICEPUNCHER_BENCH_LIMIT=10 \
        cargo nextest run --profile e2e-quick {{cucumber_e2e_tests}} --no-capture {{rerun_filter}}

# Text refinement on the live services: dictated commands, normalization, evaluation per category (@refinement) (~30 s)
refine-eval:
    @VOICEPUNCHER_STAGES_RECIPE=refine-eval {{just_executable()}} _stages refine-eval

_refine-eval: _require-nextest _require-models _build
    #!/usr/bin/env bash
    set -euo pipefail
    trap '{{just_executable()}} _stop-model-servers-for-e2e' EXIT
    {{just_executable()}} _start-model-servers-for-e2e
    VOICEPUNCHER_TRANSCRIPTION_URL="{{stt_url}}" VOICEPUNCHER_REFINEMENT_URL="{{llm_url}}" \
        cargo nextest run --profile refine-eval {{cucumber_e2e_tests}} --no-capture {{rerun_filter}}

# Outages: services that hang, die and come back while voice control runs (@resilience), each scenario on service ports of its own, so the shared services are never touched (~4 min)
e2e-resilience:
    @VOICEPUNCHER_STAGES_RECIPE=e2e-resilience {{just_executable()}} _stages e2e-resilience

_e2e-resilience: _require-nextest _require-models _build
    cargo nextest run --profile e2e-resilience {{cucumber_e2e_tests}} --no-capture {{rerun_filter}}

# Chunk seams: the seam sweep (speech after different lead-ins) and Russian and English speech under 11 recording conditions, transcribed in chunks — fails on a phrase repeated at a seam or > 10 % missing words; and the start cue of each voice start, timed from the end of the command's words (~4 min)
e2e-seams:
    @VOICEPUNCHER_STAGES_RECIPE=e2e-seams {{just_executable()}} _stages e2e-seams

_e2e-seams: _require-nextest _require-models _build
    #!/usr/bin/env bash
    set -euo pipefail
    trap '{{just_executable()}} _stop-model-servers-for-e2e' EXIT
    {{just_executable()}} _start-model-servers-for-e2e
    # The command spotting services for the voice start timing scenarios (@seams)
    export VOICEPUNCHER_TRANSCRIPTION_URL="{{stt_url}}" VOICEPUNCHER_COMMAND_SPOTTING_URL="{{kws_url}}"
    export VOICEPUNCHER_COMMAND_TRANSCRIPTION_URL="{{command_stt_url}}"
    cargo nextest run --profile e2e-seams {{e2e_seams_tests}} {{rerun_filter}}
    cargo nextest run --profile e2e-seams {{cucumber_e2e_tests}} --no-capture {{rerun_filter}}

# WER accuracy on a fixed sample of 100 recordings (50 per language), two at a time — fails if mean WER > 30 %, and no command in 20 recordings of ordinary speech (~3 min)
e2e-bench:
    @VOICEPUNCHER_STAGES_RECIPE=e2e-bench {{just_executable()}} _stages e2e-bench

_e2e-bench: _require-nextest _require-models _require-corpus _build
    #!/usr/bin/env bash
    set -euo pipefail
    trap '{{just_executable()}} _stop-model-servers-for-e2e' EXIT
    {{just_executable()}} _start-model-servers-for-e2e
    VOICEPUNCHER_TRANSCRIPTION_URL="{{stt_url}}" VOICEPUNCHER_REFINEMENT_URL="{{llm_url}}" VOICEPUNCHER_COMMAND_SPOTTING_URL="{{kws_url}}" \
        cargo nextest run --profile e2e-bench {{cucumber_e2e_tests}} --no-capture {{rerun_filter}}

# Manual: @real_keyboard scenarios drive the real hotkey + paste — needs Accessibility and Input Monitoring for the terminal, a GUI session, hands off the keyboard
test-real-keyboard:
    @{{locked}} _test-real-keyboard

_test-real-keyboard: _require-nextest
    VOICEPUNCHER_REAL_KEYBOARD_IT=1 cargo nextest run --profile real-keyboard -p voicepuncher --test cucumber_integration --no-capture

# ── Release ── after the merge, on a clean checkout of origin/main ──────────

# Runs without the machine-wide lock, like `test`: it uses no shared service, port or GPU; a lock directory of the
# run's own only gives it the wrapper's time limit (MACHINE_LOCK_TIMEOUT, exit 124; the timeout message says
# `[machine-lock]`; a run killed with KILL leaves that directory, holding an empty tests.lock, in $TMPDIR).
# `cargo clean` deletes target/release/voicepuncher, which the user may be running: in the main checkout, ask first
# Tag v<app version>, clean build, publish a GitHub Release with the signed DMG + sha256 (needs the signing identity)
release: _require-release-identity
    #!/usr/bin/env bash
    set -euo pipefail
    lock_dir=$(mktemp -d)
    trap 'rm -rf "$lock_dir"' EXIT
    # The wrapper runs in the background, so a signal to this script reaches it and stops the whole run
    MACHINE_LOCK_DIR="$lock_dir" {{locked}} _release &
    run=$!
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

# Build the signed application bundle into dist/voicepuncher.app (release build, no `cargo clean`): `open dist/voicepuncher.app`
bundle: _require-signing-identity
    #!/usr/bin/env bash
    set -euo pipefail
    cargo build --release
    app={{quote(app_bundle)}}
    rm -rf "$app" && mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
    cp target/release/voicepuncher "$app/Contents/MacOS/voicepuncher"
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

# Sign a bundle with the signing identity, or ad-hoc with a notice when none is named, then verify it strictly
# and print its designated requirement (what the permission panes identify the app by). No hardened runtime: it
# needs a Developer ID and entitlements, and without them it blocks the microphone and the key listener
_sign path:
    #!/usr/bin/env bash
    set -euo pipefail
    identity={{quote(signing_identity)}}
    if [[ -z "$identity" ]]; then
        echo "● VOICEPUNCHER_SIGNING_IDENTITY is unset: signing ad-hoc — the bundle has no stable identity, so macOS asks for every permission again after each build (README § Release checklist)" >&2
        identity="-"
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
    got=$(create-dmg --version 2>/dev/null | head -1 || true)
    if [[ "$got" != *"$wanted"* ]]; then
        echo "✗ create-dmg $wanted not found (got '${got}') — run 'brew install create-dmg'" >&2
        exit 1
    fi

# The `bundle` stage of full-check: the DMG built, the bundle and the DMG checked (_check-bundle), then the
# app-distribution scenarios (@bundle) on them: the bundle's information, its signature, the DMG as an
# installer and the signing guards. No shared service, port or GPU: it runs before the model stages, outside
# the machine lock
_bundle: _require-nextest _require-create-dmg
    #!/usr/bin/env bash
    set -euo pipefail
    just="{{just_executable()}} --justfile {{justfile()}}"
    $just dmg
    $just _check-bundle {{quote(app_bundle)}} {{quote(dist_dir)}}/voicepuncher-v{{app_version}}.dmg
    cargo nextest run --profile bundle {{cucumber_e2e_tests}} --no-capture {{rerun_filter}}

# A bundle's Info.plist checked key by key, then its signature verified strictly, and its DMG mounted and found
# to hold the app and the Applications shortcut; the first failure names what is wrong
_check-bundle app dmg:
    #!/usr/bin/env bash
    set -euo pipefail
    app={{quote(app)}}
    plist="$app/Contents/Info.plist"
    dmg={{quote(dmg)}}
    plutil -lint "$plist"
    expect_key() { # <key> <value>: the bundle's Info.plist holds the key with that value
        local got
        got=$(plutil -extract "$1" raw -o - "$plist" 2>/dev/null) || { echo "✗ Info.plist lacks $1" >&2; exit 1; }
        [[ "$got" == "$2" ]] || { echo "✗ Info.plist: $1 is '$got', expected '$2'" >&2; exit 1; }
    }
    expect_key CFBundleIdentifier io.github.firstpilotpirx.voicepuncher
    expect_key CFBundleName voicepuncher
    expect_key CFBundleDisplayName voicepuncher
    expect_key CFBundleExecutable voicepuncher
    expect_key CFBundleIconFile voicepuncher
    expect_key CFBundleShortVersionString {{quote(app_version)}}
    expect_key CFBundleVersion {{quote(app_version)}}
    expect_key LSMinimumSystemVersion 15.0
    expect_key LSUIElement true
    expect_key LSApplicationCategoryType public.app-category.productivity
    expect_key NSMicrophoneUsageDescription "voicepuncher records your voice while you hold the hotkey, to turn it into text on this Mac."
    [[ -f "$app/Contents/Resources/voicepuncher.icns" ]] || { echo "✗ bundle lacks Resources/voicepuncher.icns" >&2; exit 1; }
    codesign --verify --deep --strict "$app"
    mount=$(mktemp -d)
    hdiutil attach -nobrowse -readonly -mountpoint "$mount" "$dmg" >/dev/null 2>&1
    trap 'hdiutil detach "$mount" -quiet 2>/dev/null || true; rmdir "$mount" 2>/dev/null || true' EXIT
    [[ -d "$mount/voicepuncher.app" ]] || { echo "✗ $dmg holds no voicepuncher.app" >&2; exit 1; }
    [[ -L "$mount/Applications" ]] || { echo "✗ $dmg holds no Applications shortcut" >&2; exit 1; }
    echo "✓ bundle stage: $app signed and verified, $dmg holds the app and the Applications shortcut"

# ── Services ── local processes the app talks to over loopback ────────────────

# Start every service an E2E recipe tests, on the test ports the machine lock gives it (see `locked`), and never
# reuse one it did not start: a service someone else runs may hold another model or other code. A service a
# killed locked run left there (its PID file names it) is stopped and started again; any other process on a
# test port fails the run before it starts anything. Inside check and full-check, which hold the services for
# all their stages, it does nothing
_start-model-servers-for-e2e:
    #!/usr/bin/env bash
    set -euo pipefail
    [[ -z "${VOICEPUNCHER_E2E_SERVICES_HELD:-}" ]] || exit 0
    if [[ -z "${VOICEPUNCHER_TEST_PORTS:-}" ]]; then
        echo "✗ _start-model-servers-for-e2e runs only under the machine lock, on the test ports — run an E2E recipe or check" >&2
        exit 1
    fi
    # Left by a killed locked run: stop-model-servers stops only what a PID file on these ports names
    {{just_executable()}} stop-model-servers | grep -v '^·' || true
    # copy_ports <port> <copies>: the ports of a speech recognition service's copies, as start-model-servers picks them
    copy_ports() {
        (( $2 > 1 )) || return 0
        local base=$(( $1 + 10000 + {{max_copies}} <= 65535 ? $1 + 10000 : $1 - 10000 )) i
        for i in $(seq 1 "$2"); do echo "$(( base + i ))"; done
    }
    held=""
    check_port() {
        local port=$1 service=$2 variable=$3 pid
        pid=$(lsof -nP -iTCP:"$port" -sTCP:LISTEN -t 2>/dev/null | head -1 || true)
        [[ -n "$pid" ]] || return 0
        echo "✗ port $port ($service) is held by pid $pid: $(ps -p "$pid" -o command= 2>/dev/null) — stop it or set $variable" >&2
        held=1
    }
    check_port {{stt_port}} whisper-server VOICEPUNCHER_TEST_STT_PORT
    for port in $(copy_ports {{stt_port}} {{stt_instances}}); do
        check_port "$port" "whisper-server copy" VOICEPUNCHER_TEST_STT_PORT
    done
    check_port {{command_stt_port}} whisper-server-commands VOICEPUNCHER_TEST_COMMAND_STT_PORT
    for port in $(copy_ports {{command_stt_port}} {{command_stt_instances}}); do
        check_port "$port" "whisper-server-commands copy" VOICEPUNCHER_TEST_COMMAND_STT_PORT
    done
    check_port {{llm_port}} llama-server VOICEPUNCHER_TEST_LLM_PORT
    check_port {{kws_port}} voicepuncher-command-spotting VOICEPUNCHER_TEST_KWS_PORT
    [[ -z "$held" ]] || exit 1
    {{just_executable()}} start-model-servers
    # What this run tests: one line per service, its port, model and the PID of what it started
    started() {
        local port=$1 label=$2 model=$3 binary=$4
        echo "● testing $label on port $port: $(basename "$model"), pid $(cat "{{services_run_dir}}/$binary-$port.pid" 2>/dev/null || echo '?')"
    }
    # started_copies <port> <copies> <label> <model>: the same for a speech recognition service, one PID per copy
    started_copies() {
        local port=$1 copies=$2 label=$3 model=$4 ports p pids=()
        ports=$(copy_ports "$port" "$copies" | xargs)
        ports=${ports:-$port}
        for p in $ports; do
            pids+=("$(cat "{{services_run_dir}}/whisper-server-$p.pid" 2>/dev/null || echo '?')")
        done
        echo "● testing $label on port $port, copies on $ports: $(basename "$model"), pids ${pids[*]}"
    }
    started_copies {{stt_port}} {{stt_instances}} whisper-server "$VOICEPUNCHER_WHISPER_MODEL"
    started_copies {{command_stt_port}} {{command_stt_instances}} whisper-server-commands "$VOICEPUNCHER_COMMAND_WHISPER_MODEL"
    started {{llm_port}} llama-server "$VOICEPUNCHER_LLM_MODEL" llama-server
    started {{kws_port}} voicepuncher-command-spotting "$VOICEPUNCHER_KWS_MODEL_DIR" voicepuncher-command-spotting

# Stop the services that _start-model-servers-for-e2e started: every one on the test ports (nothing while check or
# full-check holds them)
_stop-model-servers-for-e2e:
    #!/usr/bin/env bash
    set -euo pipefail
    [[ -z "${VOICEPUNCHER_E2E_SERVICES_HELD:-}" ]] || exit 0
    [[ -n "${VOICEPUNCHER_TEST_PORTS:-}" ]] || exit 0
    {{just_executable()}} stop-model-servers

# Start the speech recognition services for dictation (whisper-server) and for commands (whisper-server-commands), text refinement (llama-server) and command spotting (voicepuncher-command-spotting), or only the named ones; one that already answers on its port is skipped
start-model-servers *services:
    #!/usr/bin/env bash
    set -euo pipefail
    run_dir="{{services_run_dir}}"
    log_dir="{{services_log_dir}}"
    mkdir -p "$run_dir" "$log_dir"
    port_open() { nc -z 127.0.0.1 "$1" >/dev/null 2>&1; }
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
    # supervise <log file> <health URL or -> <command…>: runs the service and starts it again
    # whenever it exits, after 1 s, doubling up to 30 s while it keeps exiting within 30 s of
    # its start. With a health URL, a service that once answered it and then fails it 3 times
    # in a row, VOICEPUNCHER_HEALTH_CHECK_SECS apart (default 5 s; the @resilience steps set 1 s),
    # is killed and so restarted: a service that hangs without exiting.
    # TERM or INT stops the service and the supervisor, without a restart
    supervise() {
        local log_file=$1 health=$2 delay=1 started code
        local check_every=${VOICEPUNCHER_HEALTH_CHECK_SECS:-5}
        shift 2
        child="" watcher=""
        trap '[[ -z "$watcher" ]] || kill "$watcher" 2>/dev/null; [[ -z "$child" ]] || kill "$child" 2>/dev/null; wait "$child" 2>/dev/null; exit 0' TERM INT
        while true; do
            started=$SECONDS
            "$@" >>"$log_file" 2>&1 &
            child=$!
            watcher=""
            if [[ "$health" != "-" ]]; then
                (
                    answered="" fails=0
                    while kill -0 "$child" 2>/dev/null; do
                        sleep "$check_every"
                        if curl -fsS -o /dev/null --max-time 2 "$health" 2>/dev/null; then
                            answered=1 fails=0
                        elif [[ -n "$answered" ]]; then
                            fails=$((fails + 1))
                            if (( fails >= 3 )); then
                                echo "… $1 stopped answering its health check ($fails checks in a row) — restarting" >>"$log_file"
                                kill -9 "$child" 2>/dev/null
                                exit 0
                            fi
                        fi
                    done
                ) &
                watcher=$!
            fi
            wait "$child" && code=0 || code=$?
            [[ -z "$watcher" ]] || kill "$watcher" 2>/dev/null || true
            (( SECONDS - started < 30 )) || delay=1
            echo "… $1 exited with code $code — restarting in $delay s" >>"$log_file"
            sleep "$delay" &
            child=$!
            wait "$child"
            delay=$(( delay * 2 > 30 ? 30 : delay * 2 ))
        done
    }
    # start_service <binary> <port> <label> <readiness check> <health URL or -> <command…>:
    # starts the service under its supervisor; the PID file holds the supervisor's PID
    start_service() {
        local binary=$1 port=$2 label=$3 ready=$4 health=$5
        shift 5
        local pid_file="$run_dir/$binary-$port.pid" log_file="$log_dir/$binary-$port.log"
        : >"$log_file"
        nohup bash -c "$(declare -f supervise); supervise \"\$@\"" "supervise-$binary" "$log_file" "$health" "$@" >/dev/null 2>&1 &
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
    # whisper_args <model> <port> <threads>: the whisper-server command line, in whisper_cmd; both
    # speech recognition services run these flags, each with its own model.
    # Greedy, non-speech tokens suppressed, no text context (-mc 0): text decoded in one 30 s window
    # of a request does not steer the next (requests never share text: whisper-server clears it
    # before every request). -mc 0 drops a request's prompt too, so a dictation request with a
    # prompt sets its own context (max_context 224, carry_initial_prompt). -nf (no
    # temperature fallback) is ignored by whisper-server 1.9.5: dictation keeps the server's
    # fallback (retries at 0.2, 0.4, … 1.0), and the command spotting service's confirmations
    # turn it off per request; -nf stays for a whisper-server that honours it.
    # Timestamps stay on, so whisper resumes after the last decoded segment instead of
    # skipping the rest of a 30 s window when the text ends early at a pause.
    # VAD removes only silences longer than 2 s, so silence yields no text ("Thank you.")
    # while the short pauses between words stay. No language probabilities (-nlp): with them,
    # whisper-server 1.9.5 crashes on a verbose_json request whose audio VAD leaves empty,
    # as the command spotting service's confirmation of an utterance without speech
    whisper_args() {
        whisper_cmd=(whisper-server -m "$1" --host 127.0.0.1 --port "$2"
            --inference-path /v1/audio/transcriptions -l auto -t "$3"
            -bo 1 -nf -sns -mc 0 --vad -vm "$VOICEPUNCHER_VAD_MODEL" -vsd 2000 -vp 500 -nlp)
    }
    # copy_ports <port> <copies>: the ports of the copies of the speech recognition service on <port>,
    # the first <copies> of the ports its clients look at (crate copy-pool): 10001–10004 above its
    # port, or 10000 below when those would pass 65535
    copy_ports() {
        local base=$(( $1 + 10000 + {{max_copies}} <= 65535 ? $1 + 10000 : $1 - 10000 )) i
        for i in $(seq 1 "$2"); do echo "$(( base + i ))"; done
    }
    # start_whisper <name> <port> <copies> <model variable> <label>: one speech recognition service,
    # as one whisper-server on its port or as copies on the ports its clients find from that port,
    # with nothing on the port itself. Its PID, log and copy-list files are named by its port, so the
    # two services never share a file
    start_whisper() {
        local name=$1 port=$2 copies=$3 model_var=$4 label=$5
        require_binary whisper-server whisper-cpp
        if ! [[ "$copies" =~ ^[0-9]+$ ]] || (( copies < 1 || copies > {{max_copies}} )); then
            echo "✗ $label: $copies copies — set 1 to {{max_copies}}: its clients look for at most {{max_copies}}" >&2
            exit 1
        fi
        local p
        for p in "$port" $(copy_ports "$port" {{max_copies}}); do
            if port_open "$p"; then
                echo "✓ $label already running at http://127.0.0.1:$port/v1 (port $p answers)"
                return 0
            fi
        done
        require_model "$model_var"
        require_model VOICEPUNCHER_VAD_MODEL
        local model="${!model_var}" cores
        cores=$(sysctl -n hw.ncpu)
        if (( copies == 1 )); then
            # A copy list left from an earlier start would make the stop look for copies
            rm -f "$run_dir/whisper-server-$port.copies"
            whisper_args "$model" "$port" "$cores"
            start_service whisper-server "$port" "$label" "port_open $port" - "${whisper_cmd[@]}"
            return 0
        fi
        local threads=$(( cores / copies > 0 ? cores / copies : 1 ))
        local list="$run_dir/whisper-server-$port.copies"
        local ports=($(copy_ports "$port" "$copies")) i
        echo "${ports[*]}" >"$list"
        # A copy that cannot start stops what this start already started of this service only
        trap 'code=$?; (( code == 0 )) || {{quote(just_executable())}} --justfile {{quote(justfile())}} stop-model-servers '"$name"' >/dev/null 2>&1 || true' EXIT
        for i in "${!ports[@]}"; do
            whisper_args "$model" "${ports[$i]}" "$threads"
            start_service whisper-server "${ports[$i]}" "$label: copy $((i + 1)) of $copies" \
                "port_open ${ports[$i]}" - "${whisper_cmd[@]}"
        done
        trap - EXIT
        echo "✓ $label: $copies copies on ports ${ports[*]}, found by its clients from http://127.0.0.1:$port/v1"
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
            "http://127.0.0.1:{{llm_port}}/health" llama-server -m "$VOICEPUNCHER_LLM_MODEL" --host 127.0.0.1 --port "{{llm_port}}" -c 4096 -np 1
    }
    kws_ready() { port_open "{{kws_port}}"; }
    start_command_spotting() {
        if port_open "{{kws_port}}"; then
            echo "✓ command spotting service already running at {{kws_url}}"
            return 0
        fi
        require_model_dir VOICEPUNCHER_KWS_MODEL_DIR
        require_model VOICEPUNCHER_KWS_VAD_MODEL
        # The build drops the variables cargo sets for a test it runs, because the @local_services and
        # @resilience steps run this recipe inside `cargo test`: ring's build script reruns when CARGO_PKG_* or
        # CARGO_MANIFEST_DIR change, so the service and its dependencies would be built again under the machine
        # lock, and once more by the next plain build
        (
            for var in $(compgen -e); do
                case "$var" in
                    CARGO_MANIFEST_* | CARGO_PKG_* | CARGO_CRATE_NAME | CARGO_BIN_NAME | CARGO_PRIMARY_PACKAGE | CARGO_TARGET_TMPDIR | CARGO_RUSTC_CURRENT_DIR) unset "$var" ;;
                esac
            done
            cargo build --release {{command_spotting_build}}
        )
        start_service voicepuncher-command-spotting "{{kws_port}}" "command spotting service" kws_ready \
            "http://127.0.0.1:{{kws_port}}/health" "{{kws_binary}}" --port "{{kws_port}}" \
            --model-dir "$VOICEPUNCHER_KWS_MODEL_DIR" --vad-model "$VOICEPUNCHER_KWS_VAD_MODEL" \
            --transcription-url "{{command_stt_url}}" \
            ${VOICEPUNCHER_KWS_SAVE_SPEECH_DIR:+--save-speech-dir "$VOICEPUNCHER_KWS_SAVE_SPEECH_DIR"}
    }
    selected="{{services}}"
    # The service for commands starts before the command spotting service, so its first confirmation finds it ready
    for service in ${selected:-whisper-server whisper-server-commands llama-server voicepuncher-command-spotting}; do
        case "$service" in
            whisper-server) start_whisper whisper-server "{{stt_port}}" "{{stt_instances}}" VOICEPUNCHER_WHISPER_MODEL "speech recognition service for dictation" ;;
            whisper-server-commands) start_whisper whisper-server-commands "{{command_stt_port}}" "{{command_stt_instances}}" VOICEPUNCHER_COMMAND_WHISPER_MODEL "speech recognition service for commands" ;;
            llama-server) start_llama ;;
            voicepuncher-command-spotting) start_command_spotting ;;
            *) echo "✗ unknown service '$service' — use whisper-server, whisper-server-commands, llama-server or voicepuncher-command-spotting" >&2; exit 1 ;;
        esac
    done

# Stop the services that start-model-servers launched (all, or the named ones: whisper-server, whisper-server-commands, llama-server, voicepuncher-command-spotting); services started by hand keep running
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
    # stop_speech_recognition <port> <label>: the copies of the speech recognition service on that port
    # when it runs as several, or the one whisper-server on its port
    stop_speech_recognition() {
        local service_port=$1 label=$2
        local list="{{services_run_dir}}/whisper-server-$service_port.copies" port
        if [[ ! -f "$list" ]]; then
            stop_service whisper-server "$service_port" "$label"
            return 0
        fi
        for port in $(cat "$list"); do
            stop_service whisper-server "$port" "$label: copy on port $port"
        done
        rm -f "$list"
    }
    selected="{{services}}"
    for service in ${selected:-whisper-server whisper-server-commands llama-server voicepuncher-command-spotting}; do
        case "$service" in
            whisper-server) stop_speech_recognition "{{stt_port}}" "speech recognition service for dictation" ;;
            whisper-server-commands) stop_speech_recognition "{{command_stt_port}}" "speech recognition service for commands" ;;
            llama-server) stop_service llama-server "{{llm_port}}" "text refinement service" ;;
            voicepuncher-command-spotting) stop_service voicepuncher-command-spotting "{{kws_port}}" "command spotting service" ;;
            *) echo "✗ unknown service '$service' — use whisper-server, whisper-server-commands, llama-server or voicepuncher-command-spotting" >&2; exit 1 ;;
        esac
    done

# ── Setup ─────────────────────────────────────────────────────────────────────

# Download default models into ~/.voicepuncher/models (whisper large-v3-turbo q5_0 ~550 MB for dictation, whisper small ~460 MB for commands, Silero VAD ~1 MB, Qwen3-4B-Instruct-2507 ~2.5 GB, keyword spotting ~20 MB, Silero VAD for sherpa-onnx ~1 MB)
models refinement="yes":
    #!/usr/bin/env bash
    set -euo pipefail
    dir=~/.voicepuncher/models
    mkdir -p "$dir"
    # fetch <url> <file> [sha256]: a file already present is skipped without hashing; with a sha256, the
    # download is moved into place only when it matches, else it is removed and the recipe fails
    fetch() {
        [[ -f "$dir/$2" ]] && { echo "✓ $2 already present"; return; }
        echo "↓ $2"
        curl -fL --progress-bar -o "$dir/$2.download" "$1"
        if [[ -n "${3:-}" ]]; then
            local got
            got=$(shasum -a 256 "$dir/$2.download" | cut -d' ' -f1)
            if [[ "$got" != "$3" ]]; then
                rm -f "$dir/$2.download"
                echo "✗ $2: sha256 $got, expected $3 — download removed" >&2
                exit 1
            fi
        fi
        mv "$dir/$2.download" "$dir/$2"
    }
    # The Whisper models of the two speech recognition services, pinned to one revision and checked
    whisper=https://huggingface.co/ggerganov/whisper.cpp/resolve/5359861c739e955e79d9a303bcbc70fb988958b1
    fetch "$whisper/{{dictation_model_file}}" {{dictation_model_file}} 394221709cd5ad1f40c46e6031ca61bce88931e6e088c188294c6d5a55ffa7e2
    fetch "$whisper/{{command_model_file}}" {{command_model_file}} 1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b
    fetch https://huggingface.co/ggml-org/whisper-vad/resolve/main/ggml-silero-v5.1.2.bin ggml-silero-v5.1.2.bin
    # The text refinement model, pinned to one revision and checked; skipped by `just run` without `--llm`
    [[ "{{refinement}}" == no ]] || fetch https://huggingface.co/bartowski/Qwen_Qwen3-4B-Instruct-2507-GGUF/resolve/ae44f08e1392f39c0e474af10c3ff8355c8b6688/{{refinement_model_file}} {{refinement_model_file}} 2fde00ce69dd4899c70d020845e2638353015bba0fdf161b3eb965f2bca4464e
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

_require-models refinement="yes":
    #!/usr/bin/env bash
    set -euo pipefail
    # Defaults are found by exact file name (see the settings at the top); a default that is not on disk is empty
    for entry in VOICEPUNCHER_WHISPER_MODEL={{dictation_model_file}} VOICEPUNCHER_COMMAND_WHISPER_MODEL={{command_model_file}} \
        VOICEPUNCHER_LLM_MODEL={{refinement_model_file}}; do
        var=${entry%%=*} path="${!var:-}"
        if [[ -z "$path" || ! -f "$path" ]]; then
            echo "✗ $var: no model file (got '${path}', default ${entry#*=} in ~/.voicepuncher or ~/.voicepuncher/models). Run 'just models' or set it in .env" >&2
            exit 1
        fi
    done
    echo "whisper (dictation): $VOICEPUNCHER_WHISPER_MODEL"
    echo "whisper (commands):  $VOICEPUNCHER_COMMAND_WHISPER_MODEL"
    echo "llm:                 $VOICEPUNCHER_LLM_MODEL"

# The test runner every test recipe uses, pinned like .config/nextest.toml's nextest-version; installs nothing
_require-nextest:
    #!/usr/bin/env bash
    set -euo pipefail
    wanted=0.9.146
    # `cargo-nextest 0.9.146 (<commit> <date>)`; empty when cargo has no nextest subcommand
    got=$(cargo nextest --version 2>/dev/null | head -1 || true)
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
