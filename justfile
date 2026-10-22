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
# The stages of the smoke set, one per capability (_e2e-smoke-part)
e2e_smoke_stages := "e2e-smoke-voice e2e-smoke-refinement e2e-smoke-speech e2e-smoke-services"
# The model stages of check and full-check in the order that shows a failure soonest (fail-fast-stage-order): the
# light phase first, as voice control held the most failures; in each phase the short stages first and the long
# Gherkin runs last. `accuracy` stands for the accuracy stage full-check picks (_accuracy-stage)
check_model_stages := "e2e-smoke-voice e2e-smoke-refinement refine-eval e2e-smoke-speech e2e-quick e2e-smoke-services"
full_check_model_stages := "e2e-smoke-voice e2e-regression e2e-resilience e2e-smoke-refinement refine-eval e2e-bundle e2e-smoke-speech accuracy e2e-smoke-services e2e-seams"
# The test binaries of the voice control smoke part, the most failing first; placed_commands_it is the command
# spotting service's, the others the app's
voice_smoke_binaries := "placed_commands_it voice_forms_it voice_samples_it commandspotting_websocket_it"
# The app's service binaries of the smoke set
e2e_smoke_binaries := "transcription_openai_it textrefinement_openai_it commandspotting_websocket_it voice_samples_it voice_forms_it"
# Every test binary of the smoke parts in one cargo selection: each part's one nextest call builds exactly what
# _build-e2e built before the machine lock, and picks its own binaries with a filter (_smoke-part-filter)
smoke_parts_tests := "-p voicepuncher -p voicepuncher-command-spotting -p voicepuncher-e2e-test " + prepend("--test ", e2e_smoke_binaries + " placed_commands_it cucumber_e2e")
e2e_seams_tests := "-p voicepuncher --test chunked_transcription_it"
placed_commands_tests := "-p voicepuncher-command-spotting --test placed_commands_it"
cucumber_e2e_tests := "-p voicepuncher-e2e-test --test cucumber_e2e"
# What `test` runs: every test target of the workspace but the E2E harness (one cargo call cannot pick targets
# per package; a binary the filter excludes is not even listed)
test_filter := "-E 'not binary_id(voicepuncher-e2e-test::cucumber_e2e)'"
test_selection := "--workspace " + test_filter
# A rerun's selection (just rerun-failed): added to every nextest call of the E2E recipes, whose calls may each
# hold none of the failed tests; _test narrows its own selection with it
rerun_filter := '${VOICEPUNCHER_RERUN_FILTER:+-E "$VOICEPUNCHER_RERUN_FILTER" --no-tests=pass}'
# check and full-check set VOICEPUNCHER_FAIL_FAST=1, so a model stage stops at its first failure: nextest runs no
# further test of the binary and the E2E harness no further scenario (harness_args::with_fail_fast). Standalone
# recipes and rerun-failed report every failure. A volatile variable: it changes no stage key (_stage-keys)
fail_fast := '${VOICEPUNCHER_FAIL_FAST:+--fail-fast}'

# nextest prints captured test output unindented, so libtest's `test … FAILED` and `thread '…' panicked at
# <file:line:col>:` lines start at column 0, where `kit run` finds the failed tests and their locations
export NEXTEST_NO_OUTPUT_INDENT := "1"
services_log_dir := home_directory() / ".voicepuncher/logs"
# The stage journal: one JSON line per run of _stages (check, full-check, test, the E2E recipes), shared by
# every checkout and kept when a worktree goes; `just stage-times` summarizes it
export VOICEPUNCHER_STAGE_JOURNAL := env_var_or_default("VOICEPUNCHER_STAGE_JOURNAL", home_directory() / ".cache/voicepuncher/stage-times.jsonl")
# The pass records of _stages: one file per stage and key, shared by every worktree of this machine, written
# only by a passing stage (VOICEPUNCHER_STAGE_PASSES elsewhere, e.g. for a scratch run)
stage_passes := env_var_or_default("VOICEPUNCHER_STAGE_PASSES", home_directory() / ".cache/voicepuncher/stage-passes")

# The machine-wide test lock (agent-kit `with-machine-lock`): one run at a time on this machine, a hung run is
# killed after its time limit. Every check, build, E2E, benchmark and real-keyboard recipe run on its own runs its
# private `_<name>` body under it, except `test` and `release`, which use no shared service, port or GPU (see
# their comments). `check` and `full-check` take it only for their model stages (see _stages)
machine_lock := env_var_or_default("MACHINE_LOCK", home_directory() / ".claude/bin/with-machine-lock")
# How long a locked run waits for the lock before it gives up (exit 75): 6 h unless the caller sets it, so a run
# queued behind several sessions' checks waits out the queue instead of giving up after an hour and running nothing
export MACHINE_LOCK_WAIT := env_var_or_default("MACHINE_LOCK_WAIT", "21600")
# The service ports of every run under the lock: apart from the defaults above, which the user's own services
# hold, and from their copies 10000 above (18082–18089); these copies listen on 38082–38089, below the free
# ports (49152+) the @local_services and @resilience steps take. A locked run starts every service it tests
# here and never reuses one it did not start (_start-model-servers-for-e2e). The heavy lock class and every
# locked recipe outside it use these ports
test_stt_port := env_var_or_default("VOICEPUNCHER_TEST_STT_PORT", "28081")
test_command_stt_port := env_var_or_default("VOICEPUNCHER_TEST_COMMAND_STT_PORT", "28085")
test_llm_port := env_var_or_default("VOICEPUNCHER_TEST_LLM_PORT", "28080")
test_kws_port := env_var_or_default("VOICEPUNCHER_TEST_KWS_PORT", "28082")
# The ports of the light lock class, which runs beside a heavy one, so neither touches the other's services;
# copies on 37082–37089
test_light_stt_port := env_var_or_default("VOICEPUNCHER_TEST_LIGHT_STT_PORT", "27081")
test_light_command_stt_port := env_var_or_default("VOICEPUNCHER_TEST_LIGHT_COMMAND_STT_PORT", "27085")
test_light_llm_port := env_var_or_default("VOICEPUNCHER_TEST_LIGHT_LLM_PORT", "27080")
test_light_kws_port := env_var_or_default("VOICEPUNCHER_TEST_LIGHT_KWS_PORT", "27082")
# The model stages of the light lock class: voice control, which streams audio in real time and runs the small
# command model, so it runs beside another run's heavy stages (dictation, refinement, seams, accuracy); every other
# model stage is heavy (_lock-phases, _class-lock)
light_stages := "e2e-smoke-voice e2e-regression e2e-resilience"
# The environment of a run on one class's test ports; VOICEPUNCHER_TEST_PORTS names the prefix of the variables
# that move them, for the messages of _start-model-servers-for-e2e
heavy_ports := "env VOICEPUNCHER_STT_PORT=" + test_stt_port + " VOICEPUNCHER_COMMAND_STT_PORT=" + test_command_stt_port + " VOICEPUNCHER_LLM_PORT=" + test_llm_port + " VOICEPUNCHER_KWS_PORT=" + test_kws_port + " VOICEPUNCHER_TEST_PORTS=VOICEPUNCHER_TEST"
light_ports := "env VOICEPUNCHER_STT_PORT=" + test_light_stt_port + " VOICEPUNCHER_COMMAND_STT_PORT=" + test_light_command_stt_port + " VOICEPUNCHER_LLM_PORT=" + test_light_llm_port + " VOICEPUNCHER_KWS_PORT=" + test_light_kws_port + " VOICEPUNCHER_TEST_PORTS=VOICEPUNCHER_TEST_LIGHT"
locked := heavy_ports + " " + quote(machine_lock) + " " + quote(just_executable()) + " --justfile " + quote(justfile())

# List all recipes
default:
    @just --list --unsorted

# ── Checks ── `check` while developing, `full-check` before merging ─────────

# Only the model stages take the machine lock, held ≈ 7.3 min (measured), after the code checks, the build and
# build-e2e ran without it (see _stages). Incremental, no `cargo clean`
# FAST (~8.5 min), run often while developing: code checks + E2E smoke + accuracy on 10 samples + refinement evaluation
check:
    @VOICEPUNCHER_FAIL_FAST=1 VOICEPUNCHER_STAGES_RECIPE=check {{just_executable()}} _stages fmt lint test build build-e2e {{check_model_stages}}

# Only the model stages take the machine lock, held ≈ 16.3 min with 100 recordings and ≈ 14.5 min with 10 (measured)
# (see _stages). Incremental like `check`, no `cargo clean`: the clean build is `release`'s. Accuracy runs on 100
# recordings when the branch changes the recognition path (recognition_paths), on main itself or when that cannot
# be told, else on 10 (_accuracy-stage); VOICEPUNCHER_ACCURACY_SAMPLE=100 forces 100
# FULL (~15.5–17.5 min), run before merging: code checks + the bundle + E2E (the whole voice control set, seams, outages and the bundled engines included) + accuracy on 100 or 10 samples + refinement evaluation
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
    VOICEPUNCHER_FAIL_FAST=1 VOICEPUNCHER_STAGES_RECIPE=full-check VOICEPUNCHER_STAGES_ACCURACY="$accuracy" \
        {{just_executable()}} _stages fmt lint test build bundle build-e2e {{replace(full_check_model_stages, "accuracy", '"$stage"')}}
    echo "✓ full-check passed — $accuracy; before a release also walk through README.md § Release checklist (manual run)"

# Summary of the stage journal for the runs since a date (2026-10-01) or a span (7d): per recipe and stage the
# runs, median, total and failures, the runs that reused an earlier pass (left out of the times) and the time
# they saved (the seconds of the passes they reused), with the total saved; the lock
# wait; the failed tests, most first; runs and time per branch (a change's branch is <type>/<change>, so per change)
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
    my (@keys, %stages, @waits, %tests, @branches, %branches, $saved, $reused);
    for my $run (@runs) {
        for my $stage (@{ $run->{stages} }) {
            next if $stage->{result} eq "skipped";
            my $key = "$run->{recipe}\t$stage->{name}";
            push @keys, $key unless $stages{$key};
            my $row = $stages{$key} ||= { seconds => [], failed => 0, reused => 0, saved => 0 };
            if ($stage->{result} eq "reused") {
                my $seconds = ($stage->{from} || {})->{seconds} || 0;
                $row->{reused}++;
                $row->{saved} += $seconds;
                $saved += $seconds;
                $reused++;
                next;
            }
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
    printf "\n%-16s %-20s %5s %9s %10s %7s %7s %10s\n", qw(recipe stage runs median total failed reused saved);
    for my $key (@keys) {
        my ($recipe, $stage) = split /\t/, $key;
        my $row = $stages{$key};
        my @seconds = @{ $row->{seconds} };
        my $total = 0;
        $total += $_ for @seconds;
        my $median_text = @seconds ? $span->($median->(@seconds)) : "-";
        printf "%-16s %-20s %5d %9s %10s %7d %7d %10s\n", $recipe, $stage, scalar @seconds, $median_text, $span->($total), $row->{failed}, $row->{reused},
            $row->{reused} ? $span->($row->{saved}) : "-";
    }
    printf "\nreuse saved: %d stages, %s\n", $reused, $span->($saved) if $reused;
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

# Which files each stage reads, for its key (_stage-keys): `<glob> <stages>` per line, the stages comma-separated,
# `*` spanning `/`. A file feeds the code stages (fmt, lint, test, build, bundle, build-e2e) and the model stages
# its matching lines name, all of them together; `code` names no model stage, `accuracy` both accuracy stages
# (e2e-quick, e2e-bench), and `none` makes the file feed no stage at all. A file no line names feeds every stage,
# so a new file is never left out: name a file here only when the stages it does not feed are certain. The app's
# core, its wiring, the transcription adapter, the macOS adapters (the E2E scenarios run the app binary), the
# shared E2E code, the Cargo files, .config and this justfile stay unnamed
stage_inputs := '''
    openspec/*                                         none
    docs/*                                             none
    .claude/*                                          none
    .github/*                                          none
    *.md                                               none
    app/packaging/*                                    e2e-bundle
    app/tests/architecture.rs                          code
    app/tests/size_budgets/*                           code
    app/tests/size_baseline.toml                       code
    clippy.toml                                        code
    rustfmt.toml                                       code
    app/tests/cucumber_integration.rs                  code
    app/tests/features/*                               code
    app/tests/steps/*                                  code
    app/tests/*_it.rs                                  code
    app/tests/transcription_openai_it.rs               e2e-smoke-speech
    app/tests/textrefinement_openai_it.rs              e2e-smoke-refinement
    app/tests/commandspotting_websocket_it.rs          e2e-smoke-voice
    app/tests/voice_samples_it.rs                      e2e-smoke-voice
    app/tests/voice_forms_it.rs                        e2e-smoke-voice
    app/tests/chunked_transcription_it.rs              e2e-seams
    app/src/core/application/chunking_seam_cases.rs    e2e-seams
    app/src/adapters/secondary/textrefinement/*        e2e-smoke-refinement,refine-eval,e2e-smoke-speech,accuracy
    app/src/core/application/text_refiner.rs          e2e-smoke-refinement,refine-eval,e2e-smoke-speech,accuracy
    app/src/core/domain/cleaned_text.rs                e2e-smoke-refinement,refine-eval,e2e-smoke-speech,accuracy
    app/src/core/domain/filler_sounds.rs               e2e-smoke-refinement,refine-eval,e2e-smoke-speech,accuracy
    app/src/core/domain/normalization_check.rs         e2e-smoke-refinement,refine-eval,e2e-smoke-speech,accuracy
    e2e-test/src/refinement_eval.rs                    e2e-smoke-refinement,refine-eval
    e2e-test/refinement/*                              e2e-smoke-refinement,refine-eval
    e2e-test/tests/features/text_refinement.feature    e2e-smoke-refinement,refine-eval
    e2e-test/tests/steps/text_refinement/*             e2e-smoke-refinement,refine-eval
    e2e-test/tests/steps/refinement_service_stub.rs    e2e-smoke-refinement,refine-eval
    command-spotting/*                                 e2e-smoke-voice,e2e-regression,e2e-resilience,e2e-seams,e2e-smoke-services,accuracy
    app/src/adapters/secondary/commandspotting/*       e2e-smoke-voice,e2e-regression,e2e-resilience,e2e-seams,e2e-smoke-services,accuracy
    app/src/adapters/secondary/keypressing/*           e2e-smoke-voice,e2e-regression,e2e-resilience,e2e-seams,e2e-smoke-services,accuracy
    app/src/core/application/pipeline/voice_control.rs e2e-smoke-voice,e2e-regression,e2e-resilience,e2e-seams,e2e-smoke-services,accuracy
    app/src/core/application/command_spotter.rs        e2e-smoke-voice,e2e-regression,e2e-resilience,e2e-seams,e2e-smoke-services,accuracy
    app/src/core/application/key_presser.rs            e2e-smoke-voice,e2e-regression,e2e-resilience,e2e-seams,e2e-smoke-services,accuracy
    app/src/core/domain/voice_command*.rs              e2e-smoke-voice,e2e-regression,e2e-resilience,e2e-seams,e2e-smoke-services,accuracy
    app/src/core/domain/command_words.rs               e2e-smoke-voice,e2e-regression,e2e-resilience,e2e-seams,e2e-smoke-services,accuracy
    app/src/core/domain/voice_start_timing.rs          e2e-smoke-voice,e2e-regression,e2e-resilience,e2e-seams,e2e-smoke-services,accuracy
    e2e-test/src/voice_commands.rs                     e2e-smoke-voice,e2e-regression,e2e-resilience,e2e-seams,accuracy
    e2e-test/tests/features/voice_control.feature      e2e-smoke-voice,e2e-regression,e2e-resilience,e2e-seams,accuracy
    e2e-test/tests/steps/voice_control.rs              e2e-smoke-voice,e2e-regression,e2e-resilience,e2e-seams,accuracy
    e2e-test/tests/steps/voice_control/*               e2e-smoke-voice,e2e-regression,e2e-resilience,e2e-seams,accuracy
    e2e-test/tests/features/speech_recognition.feature e2e-smoke-speech,e2e-seams
    e2e-test/tests/steps/speech_recognition.rs         e2e-smoke-speech,e2e-seams
    e2e-test/tests/steps/speech_recognition/*          e2e-smoke-speech,e2e-seams
    e2e-test/tests/features/file_transcription.feature e2e-smoke-speech,accuracy
    e2e-test/tests/steps/file_transcription.rs         e2e-smoke-speech,accuracy
    e2e-test/src/wer.rs                                accuracy
    e2e-test/src/corpus.rs                             accuracy
    e2e-test/scripts/*                                 accuracy
    e2e-test/tests/features/local_services.feature     e2e-smoke-services,e2e-resilience
    e2e-test/tests/steps/local_services.rs             e2e-smoke-services,e2e-resilience
    e2e-test/tests/steps/local_services/*              e2e-smoke-services,e2e-resilience
    e2e-test/tests/features/dictation_archive.feature  e2e-smoke-speech
    e2e-test/tests/steps/dictation_archive.rs          e2e-smoke-speech
'''

# Prints `<stage> <key>` for each named stage: a sha256 over what the stage reads, so a stage whose key has a
# pass record (_stages) need not run again. Over the stage's name; the path and content of every file it reads
# (stage_inputs), tracked or untracked but not ignored, as the working tree holds it, so uncommitted edits count
# and a deleted file changes the key; the toolchain (rustc, cargo, nextest, just, the macOS build, which fixes
# the `say` voices); the VOICEPUNCHER_*, CUCUMBER_*, Rust and Cargo build variables but the volatile ones (journal,
# reports, ports, the lock, colours), so a probe or a forced sample never reuses a pass. A model stage also hashes
# what it runs on outside the repository: the size and mtime of every file or directory such a variable names
# (models, corpus, voice samples) and of the whisper-server and llama-server binaries; for it a Cargo file's `version = "…"` lines of the workspace's own
# packages do not count (no model stage reads the version). An input this misses (another program on the PATH,
# a system library) is a reason to run with VOICEPUNCHER_REUSE_STAGES=0. The test stage's parts (_test) have keys
# too: `test@<member>` over the code inputs but the files of members its tests do not build (`test@members` names
# one per workspace member), `test@justfile` over the justfile alone. VOICEPUNCHER_STAGE_ROOT names another
# repository (_test-stage-keys)
_stage-keys +stages:
    #!/usr/bin/env perl
    use strict;
    use warnings;
    use Digest::SHA qw(sha256_hex);
    use File::Find qw(find);
    use IPC::Open2 qw(open2);
    my @stages = split " ", '{{stages}}';
    my $root = $ENV{VOICEPUNCHER_STAGE_ROOT} || {{quote(justfile_directory())}};
    my $table = <<'TABLE';
    {{stage_inputs}}
    TABLE
    my $model = sub { $_[0] =~ /^e2e/ || $_[0] eq "refine-eval" };
    my %alias = (accuracy => ["e2e-quick", "e2e-bench"]);
    my @rules;
    for my $line (split /\n/, $table) {
        my ($glob, $names) = split " ", $line;
        next unless defined $names;
        (my $re = quotemeta $glob) =~ s/\\\*/.*/g;
        push @rules, [qr/^$re$/, [map { @{ $alias{$_} || [$_] } } split /,/, $names]];
    }
    # The files: tracked and untracked but not ignored, as the working tree holds them
    my @files = do {
        local $/ = "\0";
        open my $ls, "-|", "git", "-C", $root, "ls-files", "-z", "-c", "-o", "--exclude-standard" or die "git ls-files: $!";
        my %seen;
        grep { !$seen{$_}++ && -f "$root/$_" } map { chomp; $_ } <$ls>;
    };
    @files = sort @files;
    my %hash;
    if (@files) {
        my $pid = open2(my $out, my $in, "git", "-C", $root, "hash-object", "--stdin-paths");
        print $in "$_\n" for @files;
        close $in;
        my @hashes = map { chomp; $_ } <$out>;
        waitpid $pid, 0;
        die "git hash-object: " . scalar(@hashes) . " hashes for " . scalar(@files) . " files\n" unless @hashes == @files;
        @hash{@files} = @hashes;
    }
    # For a model stage a Cargo file counts without its workspace packages' version lines
    my $unversioned = sub {
        my ($path) = @_;
        open my $fh, "<", "$root/$path" or die "$path: $!";
        my ($text, $own) = ("", 0);
        while (my $line = <$fh>) {
            if ($path =~ /(^|\/)Cargo\.lock$/) {
                $own = $line =~ /^name = "voicepuncher(-[a-z-]+)?"$/ ? 1 : $line =~ /^name = / ? 0 : $own;
                next if $own && $line =~ /^version = "[^"]*"$/;
            }
            else { next if $line =~ /^version = "[^"]*"$/ }
            $text .= $line;
        }
        sha256_hex($text);
    };
    my $run = sub { my $out = `@_ 2>/dev/null`; defined $out && length $out ? $out : "none\n" };
    my $toolchain = join "", map { $run->($_) } (
        "rustc -vV", "cargo -V", "cargo nextest --version", {{quote(just_executable())}} . " --version", "sw_vers -buildVersion");
    # Outside the repository: size and mtime of a path, a directory's files included
    my $described = sub {
        my ($path) = @_;
        my $text = "";
        find({ no_chdir => 1, wanted => sub {
            return unless -f $_;
            my @st = stat _;
            (my $rel = $_) =~ s/^\Q$path\E//;
            $text .= "$rel $st[7] $st[9]\n";
        } }, $path);
        $text;
    };
    my $volatile = qr/^(VOICEPUNCHER_STAGE.*|VOICEPUNCHER_REUSE_STAGES|VOICEPUNCHER_TEST_PORTS|VOICEPUNCHER_.*_PORT|VOICEPUNCHER_E2E_SERVICES_HELD|VOICEPUNCHER_FAIL_FAST|CARGO_TERM_.*)$/;
    my ($variables, $runtime) = ("", "");
    for my $name (sort grep { /^(VOICEPUNCHER_|CUCUMBER_|RUSTFLAGS$|RUSTDOCFLAGS$|RUSTC_WRAPPER$|CARGO_BUILD_|CARGO_PROFILE_|CARGO_TARGET_|CARGO_ENCODED_RUSTFLAGS$|NEXTEST_PROFILE$|NEXTEST_RETRIES$|NEXTEST_TEST_THREADS$)/ && !/$volatile/ } keys %ENV) {
        my $value = $ENV{$name};
        $variables .= "$name=$value\n";
        $runtime .= $described->($value) if $value =~ m{^/} && -e $value;
    }
    for my $binary (qw(whisper-server llama-server)) {
        chomp(my $path = `command -v $binary 2>/dev/null` // "");
        $runtime .= "$binary " . ($path ? $described->($path) : "none\n");
    }
    # The workspace members (root Cargo.toml `members`) and the scope of each one's part test@<member>, as Cargo
    # builds its tests: the member, its `path = "../<member>"` dependencies and dev-dependencies, and from those on
    # their normal dependencies only (a dependency's dev-dependencies are not built for its dependents); every member
    # for one whose Cargo.toml sets `tests-read-every-member = true` under [package.metadata.stage-keys] (app: its
    # size_budgets test measures every member's source)
    my $read = sub { open my $fh, "<", "$root/$_[0]" or return ""; local $/; <$fh> };
    my @members = ($read->("Cargo.toml") =~ /^members\s*=\s*\[([^\]]*)\]/m ? $1 =~ /"([^"]+)"/g : ());
    my %member = map { $_ => 1 } @members;
    my (%deps, %dev, %every);
    for my $m (@members) {
        my $section = "";
        for my $line (split /\n/, $read->("$m/Cargo.toml")) {
            $section = $1 if $line =~ /^\s*\[([^\]]+)\]/;
            $every{$m} = 1 if $section eq "package.metadata.stage-keys" && $line =~ /^\s*tests-read-every-member\s*=\s*true/;
            my ($dep) = $line =~ /path\s*=\s*"\.\.\/([^"\/]+)\/?"/ or next;
            next unless $member{$dep};
            push @{ ($section =~ /dev-dependencies/ ? \%dev : \%deps)->{$m} }, $dep;
        }
    }
    my %scope;
    for my $m (@members) {
        my @todo = ($m, @{ $dev{$m} || [] }, $every{$m} ? @members : ());
        while (defined(my $next = shift @todo)) {
            next if $scope{$m}{$next}++;
            push @todo, @{ $deps{$next} || [] };
        }
    }
    @stages = map { $_ eq "test\@members" ? map { "test\@$_" } @members : $_ } @stages;
    # The member a path lies under, if any
    my $owner = sub { for my $m (@members) { return $m if index($_[0], "$m/") == 0 } undef };
    for my $stage (@stages) {
        my ($part) = $stage =~ /^test\@(.+)$/;
        if (defined $part && $part eq "justfile") {
            my $sha = Digest::SHA->new(256);
            $sha->add("stage $stage\n", $toolchain, $variables, "justfile " . ($hash{justfile} // "none") . "\n");
            print "$stage ", $sha->hexdigest, "\n";
            next;
        }
        die "✗ $stage: no workspace member $part\n" if defined $part && !$member{$part};
        my $is_model = $model->($stage);
        my $sha = Digest::SHA->new(256);
        $sha->add("stage $stage\n", $toolchain, $variables);
        $sha->add($runtime) if $is_model;
        for my $path (@files) {
            my ($named, $none, $feeds) = (0, 0, 0);
            for my $rule (@rules) {
                next unless $path =~ $rule->[0];
                $named = 1;
                for my $name (@{ $rule->[1] }) {
                    $none = 1 if $name eq "none";
                    $feeds = 1 if $name eq $stage;
                }
            }
            next if $none;
            next if $named && $is_model && !$feeds;
            if (defined $part) {
                my $of = $owner->($path);
                next if defined $of && !$scope{$part}{$of};
            }
            my $content = $is_model && $path =~ /(^|\/)Cargo\.(toml|lock)$/ ? $unversioned->($path) : $hash{$path};
            $sha->add("$path $content\n");
        }
        print "$stage ", $sha->hexdigest, "\n";
    }

# Runs the named stages (private recipes without their `_`) in two phases. The stages before the first model
# stage (`e2e*` or `refine-eval`) run one by one without the machine lock, each through the wrapper with a lock
# directory of the run's own — the device `test` uses, borrowed: it only gives the stage the time limit
# (MACHINE_LOCK_TIMEOUT, exit 124; the timeout message says `[machine-lock]`, though no machine lock is held; a
# run killed with KILL leaves that directory, holding an empty tests.lock, in $TMPDIR, which macOS clears); fmt,
# lint, build and build-e2e run at the utility QoS clamp. The rest runs in _model-stages, one phase per lock
# class (_lock-phases): each under its class lock and the shared machine lock (_class-lock), on its class's test
# ports, so a light phase runs beside another run's heavy one; when the caller already holds the machine lock,
# in one phase at once. A failing stage ends the run before the next lock is requested.
# Ends with one table: each stage's time, each class's lock wait and hold, and adds the run to the stage journal
# (_stage-journal-append; the caller names itself in VOICEPUNCHER_STAGES_RECIPE, full-check its accuracy choice in
# VOICEPUNCHER_STAGES_ACCURACY); every stage's output is also copied to a log of the run, where the journal finds
# the failed tests. Two checks of one worktree must not overlap: the lock no longer keeps their builds apart.
# A stage that passed before on the same inputs — its key (_stage-keys) has a pass record in stage_passes, from
# any worktree of this machine — is reused (↺) and does not run; build and build-e2e run whenever a model stage
# runs, as they fill this worktree's target/; with every model stage reused no lock is requested. After the run
# each stage that passed and whose key still holds gets its pass record. VOICEPUNCHER_REUSE_STAGES=0 runs every
# stage (and still records the passes)
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
    # The stages to reuse: `<stage> <pass record>` per line in reuse
    passes="${VOICEPUNCHER_STAGE_PASSES:-{{stage_passes}}}"
    keys=$(mktemp)
    reuse=$(mktemp)
    if ! {{just_executable()}} --justfile {{quote(justfile())}} _stage-keys {{stages}} >"$keys"; then
        echo "✗ the stage keys could not be computed: every stage runs and no pass is recorded" >&2
        : >"$keys"
    fi
    # A rerun of failed tests (rerun-failed) runs part of a stage: it neither reuses a pass nor records one
    rerun="${VOICEPUNCHER_RERUN_FILTER:-}${VOICEPUNCHER_RERUN_SCENARIOS:-}"
    if [[ "${VOICEPUNCHER_REUSE_STAGES:-1}" != 0 && -z "$rerun" ]]; then
        while read -r stage key; do
            if [[ -f "$passes/$stage/$key" ]]; then echo "$stage $passes/$stage/$key" >>"$reuse"; fi
        done <"$keys"
    fi
    reused_from() { awk -v s="$1" '$1 == s { print $2 }' "$reuse"; }
    # One line on the pass a reused stage came from
    reused_line() { {{just_executable()}} --justfile {{quote(justfile())}} _pass-line "$1"; }
    reuse_stage() { # <stage>: reuses it when it has a pass record; fails otherwise
        local record
        record=$(reused_from "$1")
        [[ -n "$record" ]] || return 1
        echo "↺ $(date +%H:%M:%S) $1 reused: $(reused_line "$record")"
        echo "reused $1 $record" >>"$report"
    }
    unlocked="" model=""
    for stage in {{stages}}; do
        if [[ -n "$model" || "$stage" == e2e* || "$stage" == refine-eval ]]; then
            model+=" $stage"
        else
            unlocked+=" $stage"
        fi
    done
    # The model stages to run; build and build-e2e are not reused when there is one
    to_run=""
    for stage in $model; do [[ -n "$(reused_from "$stage")" ]] || to_run+=" $stage"; done
    if [[ -n "$to_run" ]]; then
        grep -v -E '^(build|build-e2e) ' "$reuse" >"$reuse.kept" || true
        mv "$reuse.kept" "$reuse"
    fi
    # The pass records of the stages that passed and whose key held through the run
    record_passes() {
        local passed now_keys stage key seconds record
        passed=$(awk '$1 == "stage" && $2 == "✓" { print $3 }' "$report")
        [[ -n "$passed" && -s "$keys" && -z "$rerun" ]] || return 0
        now_keys=$({{just_executable()}} --justfile {{quote(justfile())}} _stage-keys $passed) || return 0
        while read -r stage key; do
            if ! grep -qx "$stage $key" "$keys"; then
                echo "● $stage passed, but what it reads changed during the run: no pass recorded"
                continue
            fi
            seconds=$(awk -v s="$stage" '$1 == "stage" && $3 == s { print $4 }' "$report" | tail -1)
            {{just_executable()}} --justfile {{quote(justfile())}} _record-pass "$stage" "$key" "$seconds"
        done <<<"$now_keys"
    }
    run="" status=0 interrupted="" phase_class=""
    finish() {
        code=$?
        [[ -z "$interrupted" || $code -ne 0 ]] || code=130
        # A phase the run left without its return mark (interrupted, failed to start) returns now
        [[ -z "$phase_class" ]] || echo "returned $phase_class $(now)" >>"$report"
        printf '\nstages:\n'
        while read -r kind a b c; do
            case "$kind" in
                stage) row "$a" "$b" "$(printf '%7ss' "$c")" ;;
                reused) row "↺" "$a" "reused — $(reused_line "$b")" ;;
            esac
        done <"$report"
        # One row per lock class the run asked for: its wait and its hold, or the wait it gave up after
        for class in $(awk '$1 == "requested" { print $2 }' "$report"); do
            mark() { awk -v k="$1" -v c="$class" '$1 == k && $2 == c { print $3 }' "$report" | tail -1; }
            requested=$(mark requested) locked=$(mark locked) returned=$(mark returned)
            returned=${returned:-$(now)}
            if [[ -n "$caller" ]]; then
                row "●" "lock" "held by the caller"
            elif [[ -z "$locked" ]]; then
                row "✗" "$class lock" "wait $(printf '%7ss' "$(since "$requested" "$returned")"), not taken"
            else
                row "●" "$class lock" "wait $(printf '%7ss' "$(since "$requested" "$locked")"), held $(printf '%7ss' "$(since "$locked" "$returned")")"
            fi
        done
        ended=$(now)
        printf '  %-16s %7ss\n' total "$(since "$started" "$ended")"
        record_passes || echo "✗ the passes of this run were not recorded" >&2
        VOICEPUNCHER_STAGES_REPORT="$report" STAGES_LOG="$run_log" STAGES_LIST="{{stages}}" STAGES_STARTED="$started" \
            STAGES_ENDED="$ended" STAGES_EXIT="$code" STAGES_INTERRUPTED="$interrupted" \
            STAGES_CALLER="$caller" STAGES_WORKTREE={{quote(justfile_directory())}} \
            {{just_executable()}} --justfile {{quote(justfile())}} _stage-journal-append \
            || echo "✗ the run was not added to the stage journal" >&2
        rm -rf "$report" "$run_log" "$stage_lock_dir" "$keys" "$reuse"
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
        ! reuse_stage "$stage" || continue
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
    for stage in $model; do ! reuse_stage "$stage" || true; done
    [[ -n "$to_run" ]] || exit 0
    # phase <class> <stage>…: runs the stages in _model-stages under the lock of their class (the caller's
    # machine lock as it is), marking in the report when it asked for the lock and when it returned
    phase() {
        local class=$1 ports lock
        shift
        case "$class" in
            light) ports=({{light_ports}}) ;;
            *) ports=({{heavy_ports}}) ;;
        esac
        if [[ -n "$caller" ]]; then
            echo "● $(date +%H:%M:%S) the caller holds the machine lock: $*"
            lock=()
        else
            echo "→ $(date +%H:%M:%S) waiting for the $class lock: $*"
            lock=({{just_executable()}} --justfile {{quote(justfile())}} _class-lock "$class")
        fi
        phase_class=$class
        echo "requested $class $(now)" >>"$report"
        run_phase "${ports[@]}" VOICEPUNCHER_LOCK_CLASS="$class" ${lock[@]+"${lock[@]}"} \
            {{just_executable()}} --justfile {{quote(justfile())}} _model-stages "$@"
        echo "returned $class $(now)" >>"$report"
        phase_class=""
        [[ $status -eq 0 ]] || exit "$status"
        [[ -z "$interrupted" ]] || exit 130
    }
    # The caller's machine lock already keeps every other run out: one phase on the heavy ports, as before
    if [[ -n "$caller" ]]; then
        phase machine $to_run
    else
        # Read first: a stage must not read the phase list as its input
        phases=$({{just_executable()}} --justfile {{quote(justfile())}} _lock-phases $to_run)
        while read -r class stages <&3; do
            phase "$class" $stages
        done 3<<<"$phases"
    fi

# The model stages split into one phase per lock class, `<class> <stage>…` per line: each phase keeps the order of
# its stages, the phases come in the order of their first stage, a class without a stage prints nothing. `light`
# are the stages light_stages names, every other model stage is `heavy` (see _class-lock)
_lock-phases *stages:
    #!/usr/bin/env bash
    set -euo pipefail
    heavy="" light="" first=""
    for stage in {{stages}}; do
        if [[ " {{light_stages}} " == *" $stage "* ]]; then light+=" $stage"; first=${first:-light}
        else heavy+=" $stage"; first=${first:-heavy}; fi
    done
    print() { [[ -z "$2" ]] || echo "$1$2"; }
    if [[ "$first" == light ]]; then print light "$light"; print heavy "$heavy"
    else print heavy "$heavy"; print light "$light"; fi

# Runs a command under the lock of its class (_lock-phases), then under the machine lock in the shared mode:
# two runs of one class take turns, a light and a heavy run overlap, and an exclusive run of the machine lock
# (another project, a standalone recipe) still has the machine alone, as the kit lets a waiting exclusive run go
# before new shared runs. The class lock comes first, so a run never holds the shared machine lock while it
# queues for its class. A class lock is a lock directory of its own beside the machine lock's; its deadline
# covers the wait for the machine lock (MACHINE_LOCK_WAIT) plus the machine lock's deadline, which stays on the
# command
[positional-arguments]
_class-lock class +command:
    #!/usr/bin/env bash
    set -euo pipefail
    class=$1
    shift
    machine_dir="${MACHINE_LOCK_DIR:-$HOME/.cache/machine-lock}"
    deadline="${MACHINE_LOCK_TIMEOUT:-3600}"
    exec env MACHINE_LOCK_DIR="$machine_dir-voicepuncher-$class" MACHINE_LOCK_TIMEOUT="$(( MACHINE_LOCK_WAIT + deadline ))" \
        {{quote(machine_lock)}} env MACHINE_LOCK_DIR="$machine_dir" MACHINE_LOCK_TIMEOUT="$deadline" \
        {{quote(machine_lock)}} --shared "$@"

# The model stages of one phase of _stages, run under its lock (VOICEPUNCHER_LOCK_CLASS): starts the services on
# the test ports of that class, runs the stages with VOICEPUNCHER_E2E_SERVICES_HELD=1, so no E2E recipe starts or stops one, and stops them
# on exit, also when the start or a stage is interrupted; appends its rows to VOICEPUNCHER_STAGES_REPORT
_model-stages +stages:
    #!/usr/bin/env bash
    set -euo pipefail
    now() { perl -MTime::HiRes=time -e 'printf "%.1f", time'; }
    since() { perl -e "printf '%.1f', $(now) - $1"; }
    report="${VOICEPUNCHER_STAGES_REPORT:-/dev/null}"
    class="${VOICEPUNCHER_LOCK_CLASS:-machine}"
    taken=$(now)
    echo "locked $class $taken" >>"$report"
    requested=$(awk -v c="$class" '$1 == "requested" && $2 == c { print $3 }' "$report" | tail -1)
    if [[ -n "$requested" && -z "${VOICEPUNCHER_STAGES_CALLER_LOCK:-}" ]]; then
        echo "● $(date +%H:%M:%S) $class lock taken after $(perl -e "printf '%.0f', $taken - $requested") s"
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
    # e2e-resilience and e2e-bundle start services of their own on ports of their own: alone, they need none of these
    own_services=1
    for stage in {{stages}}; do [[ " e2e-resilience e2e-bundle " == *" $stage "* ]] || own_services=""; done
    if [[ -z "$own_services" ]]; then
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
# report: when, where, which recipe, each requested stage (passed, failed, reused with the pass it came from, or
# skipped when the run ended before it), the lock wait and hold, the accuracy choice, and the failed tests found in the stages' output — nextest's
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
    my %env = map { $_ => $ENV{"STAGES_$_"} // "" } qw(LOG LIST STARTED ENDED EXIT INTERRUPTED CALLER WORKTREE);
    my (%rows, %marks, @classes);
    open my $report, "<", $ENV{VOICEPUNCHER_STAGES_REPORT} or die "report: $!";
    while (<$report>) {
        my ($kind, @fields) = split;
        next unless defined $kind;
        if ($kind eq "stage") { $rows{$fields[1]} = [$fields[0] eq "✓" ? "passed" : "failed", $fields[2] + 0] }
        elsif ($kind eq "reused") { $rows{$fields[0]} = ["reused", undef, $fields[1]] }
        elsif ($kind =~ /^(?:requested|locked|returned)$/) {
            push @classes, $fields[0] if $kind eq "requested" && !exists $marks{$fields[0]};
            $marks{$fields[0]}{$kind} = $fields[1];
        }
    }
    my @stages = map { my $row = $rows{$_}; $row ? [$_, @$row] : [$_, "skipped", undef] } split " ", $env{LIST};
    my $round = sub { defined $_[0] ? sprintf("%.1f", $_[0]) + 0 : undef };
    # Per lock class its wait and hold; lock_wait and lock_held are their sums (a hold is missing when the lock
    # was never taken)
    my ($wait, $held, @locks);
    if ($env{CALLER} && grep { defined $marks{$_}{locked} } @classes) { ($wait, $held) = ("caller", "caller") }
    else {
        for my $class (@classes) {
            my $m = $marks{$class};
            my $end = $m->{returned} // $env{ENDED};
            my $w = $round->(($m->{locked} // $end) - $m->{requested});
            my $h = defined $m->{locked} ? $round->($end - $m->{locked}) : undef;
            push @locks, { class => $class, wait => $w, held => $h };
            $wait = $round->(($wait // 0) + $w);
            $held = $round->(($held // 0) + $h) if defined $h;
        }
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
    # A reused stage names the pass it came from: that pass record's commit, branch, worktree, finish time and the
    # seconds it took, the time the reuse saved
    my $from = sub {
        my $record = eval { open my $fh, "<", $_[0] or die; local $/; decode_json(<$fh>) } || {};
        +{ map { $_ => $record->{$_} } qw(commit branch worktree finished seconds) };
    };
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
        stages => [map { +{ name => $_->[0], result => $_->[1], seconds => $_->[2], defined $_->[3] ? (from => $from->($_->[3])) : () } } @stages],
        lock_wait => $wait,
        lock_held => $held,
        locks => \@locks,
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
    cargo nextest run {{smoke_parts_tests}} --no-run
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

# Rewrites app/tests/size_baseline.toml from the code (AGENTS.md § Architecture rules) by running `test` with
# VOICEPUNCHER_WRITE_SIZE_BASELINE=1: to drop the entries of units back within budget, or to list the units a merge
# of branches written before the budgets brought in; a rewrite that adds or raises an entry is reviewed like code
size-baseline:
    VOICEPUNCHER_WRITE_SIZE_BASELINE=1 {{just_executable()}} test

# Runs without the machine-wide lock, as an unlocked stage of _stages (which records it in the stage journal): it
# uses no shared service, port, GPU or exclusive device. A lock directory of the run's own only gives it the wrapper's time limit (MACHINE_LOCK_TIMEOUT, exit 124; the timeout message
# says `[machine-lock]`). Departs from agent-kit `machine-lock.md` on purpose until the kit's exception lands.
# Every test of the app, the command spotting service and the E2E harness under cargo-nextest, each within its time limit, unlocked (~1 min)
test:
    @VOICEPUNCHER_STAGES_RECIPE=test {{just_executable()}} _stages test

# The rerun takes the worktree's last record of the stage journal that ran a stage, its failed stage and its failed tests (or the
# names given, in the journal's form: `<package>::<binary> <test>`, `scenario: <name>`), and runs that one stage
# with them alone — same build, services, lock class, test ports and time limits, never reused and never recorded as a
# pass — then fails naming any of
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
# clamp; the tests keep the default QoS (clamped timers fire late under load). Only the packages whose part key
# (_stage-keys test@<member>) has no pass record run, the self-tests only when test@justfile has none; the others
# are reused (↺), and each part that passed gets its record (VOICEPUNCHER_REUSE_STAGES=0 runs every part). A
# narrower package set may rebuild shared dependencies once (Cargo unifies features per selection). A rerun
# (VOICEPUNCHER_RERUN_FILTER) runs only the tests of that selection over the workspace, reusing and recording nothing
_test: _require-nextest
    #!/usr/bin/env bash
    set -euo pipefail
    me=({{just_executable()}} --justfile {{quote(justfile())}})
    now() { perl -MTime::HiRes=time -e 'printf "%.1f", time'; }
    # A rerun runs only its failed tests over the workspace, not the self-tests, and neither reuses nor records
    if [[ -n "${VOICEPUNCHER_RERUN_FILTER:-}" ]]; then
        selection=(--workspace -E "not binary_id(voicepuncher-e2e-test::cucumber_e2e) & ($VOICEPUNCHER_RERUN_FILTER)" --no-tests=pass)
        taskpolicy -c utility cargo nextest run "${selection[@]}" --no-run
        cargo nextest run "${selection[@]}"
        exit 0
    fi
    passes="${VOICEPUNCHER_STAGE_PASSES:-{{stage_passes}}}"
    keys=$(mktemp)
    trap 'rm -f "$keys"' EXIT
    if ! "${me[@]}" _stage-keys test@members test@justfile >"$keys"; then
        echo "✗ the part keys could not be computed: every package runs and no part pass is recorded" >&2
        : >"$keys"
    fi
    # The parts to run: every one without a pass record (all of them without keys or with VOICEPUNCHER_REUSE_STAGES=0)
    parts="" packages=() self=1
    if [[ -s "$keys" ]]; then
        while read -r part key; do
            if [[ "${VOICEPUNCHER_REUSE_STAGES:-1}" != 0 && -f "$passes/$part/$key" ]]; then
                echo "↺ $(date +%H:%M:%S) $part reused: $("${me[@]}" _pass-line "$passes/$part/$key")"
                [[ "$part" != test@justfile ]] || self=""
                continue
            fi
            parts+=" $part"
            [[ "$part" == test@justfile ]] && continue
            name=$(perl -ne 'if (/^name\s*=\s*"([^"]+)"/) { print $1; exit }' "{{justfile_directory()}}/${part#test@}/Cargo.toml")
            packages+=(-p "$name")
        done <"$keys"
    else
        packages=(--workspace)
    fi
    started=$(now)
    if [[ ${#packages[@]} -gt 0 ]]; then
        taskpolicy -c utility cargo nextest run "${packages[@]}" {{test_filter}} --no-run
        cargo nextest run "${packages[@]}" {{test_filter}}
    else
        echo "↺ every package reused: no test runs"
    fi
    if [[ -n "$self" ]]; then
        "${me[@]}" _test-stage-keys
        "${me[@]}" _test-lock-phases
        "${me[@]}" _test-class-locks
    fi
    # The pass records of the parts that ran, when what they read held through the run
    [[ -n "$parts" ]] || exit 0
    seconds=$(perl -e "printf '%.1f', $(now) - $started")
    "${me[@]}" _stage-keys $parts | while read -r part key; do
        if grep -qx "$part $key" "$keys"; then
            "${me[@]}" _record-pass "$part" "$key" "$seconds"
        else
            echo "● $part passed, but what it reads changed during the run: no pass recorded"
        fi
    done

# One line on a pass record: when, on which branch and commit it passed
_pass-line record:
    @perl -MJSON::PP -e 'my $r = eval { local $/; open my $f, "<", $ARGV[0] or die; decode_json(<$f>) } || {}; \
        my $at = ($r->{finished} // "") =~ s/^(\d{4}-\d\d-\d\d)T(\d\d:\d\d).*/$1 $2/r; \
        printf "passed %s on %s at %s%s", $at || "?", $r->{branch} // "?", $r->{commit} // "?", $r->{dirty} ? " (uncommitted edits)" : ""' {{quote(record)}}

# Writes the pass record of a stage (or a part of one, test@<member>) under its key in stage_passes: commit,
# branch, whether the tree had uncommitted edits, worktree, finish time and the seconds it took; by temp file and
# rename, so a reader never sees half a record
_record-pass stage key seconds:
    #!/usr/bin/env bash
    set -euo pipefail
    passes="${VOICEPUNCHER_STAGE_PASSES:-{{stage_passes}}}"
    mkdir -p "$passes/{{stage}}"
    record=$(mktemp "$passes/{{stage}}/.record.XXXXXX")
    STAGE={{quote(stage)}} KEY={{quote(key)}} SECONDS_TAKEN={{quote(seconds)}} perl -MJSON::PP -MPOSIX=strftime -e '
        my $git = sub { my $out = `git -C "$ARGV[0]" @_ 2>/dev/null`; chomp $out; length $out ? $out : undef };
        (my $finished = strftime("%Y-%m-%dT%H:%M:%S%z", localtime)) =~ s/(\d\d)(\d\d)$/$1:$2/;
        print JSON::PP->new->canonical->encode({ stage => $ENV{STAGE}, key => $ENV{KEY},
            commit => $git->("rev-parse --short HEAD"), branch => $git->("rev-parse --abbrev-ref HEAD"),
            dirty => length(`git -C "$ARGV[0]" status --porcelain 2>/dev/null`) ? JSON::PP::true : JSON::PP::false,
            worktree => $ARGV[0], finished => $finished, seconds => ($ENV{SECONDS_TAKEN} || 0) + 0 }), "\n"' \
        {{quote(justfile_directory())}} >"$record"
    mv "$record" "$passes/{{stage}}/{{key}}"

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
    @VOICEPUNCHER_STAGES_RECIPE=e2e {{just_executable()}} _stages {{e2e_smoke_stages}} e2e-seams e2e-bench refine-eval

# Smoke test — services and models load, file mode runs and voice commands are spotted (@smoke scenarios), the adapters talk to the real services
e2e-smoke:
    @VOICEPUNCHER_STAGES_RECIPE=e2e-smoke {{just_executable()}} _stages {{e2e_smoke_stages}}

# The smoke set split by capability, one stage each, so a stage whose inputs did not change is reused on its
# own (stage_inputs): speech recognition, file transcription and every capability without a part of its own
# (the dictation archive: a part of 6 s cost more than it saved), text refinement, voice control and local
# services. Each part's scenario selection and limits are its profile's (.config/nextest.toml)
_e2e-smoke-speech: (_e2e-smoke-part "e2e-smoke-speech" "transcription_openai_it")
_e2e-smoke-refinement: (_e2e-smoke-part "e2e-smoke-refinement" "textrefinement_openai_it")
_e2e-smoke-voice: (_e2e-smoke-part "e2e-smoke-voice" voice_smoke_binaries)
_e2e-smoke-services: (_e2e-smoke-part "e2e-smoke-services")

# One nextest call for the part's service binaries and its Gherkin scenarios, so the binaries run beside the
# harness instead of before it (speed-up-smoke-and-test); the command-spotting-models test group still keeps the
# binaries that drive the shared command spotting service to one test at a time, and the harness keeps its own
# scenario concurrency. nextest runs nothing in parallel under --no-capture, so the harness's output is captured
# and printed whole when it ends (the profiles' success-output). placed_commands_it is the command spotting
# service's binary, every other one the app's
_e2e-smoke-part profile binaries="": _require-nextest _require-models _build
    #!/usr/bin/env bash
    set -euo pipefail
    trap '{{just_executable()}} _stop-model-servers-for-e2e' EXIT
    {{just_executable()}} _start-model-servers-for-e2e
    export VOICEPUNCHER_TRANSCRIPTION_URL="{{stt_url}}" VOICEPUNCHER_REFINEMENT_URL="{{llm_url}}" VOICEPUNCHER_COMMAND_SPOTTING_URL="{{kws_url}}"
    export VOICEPUNCHER_COMMAND_TRANSCRIPTION_URL="{{command_stt_url}}"
    filter=$({{just_executable()}} --justfile {{quote(justfile())}} _smoke-part-filter "{{binaries}}")
    cargo nextest run --profile {{profile}} {{smoke_parts_tests}} -E "$filter" ${VOICEPUNCHER_RERUN_FILTER:+--no-tests=pass} {{fail_fast}}

# The nextest filter of one smoke part: its service binaries and the E2E harness, narrowed to a rerun's selection
# when there is one (a second -E would widen it: nextest joins several with "or")
_smoke-part-filter binaries="":
    #!/usr/bin/env bash
    set -euo pipefail
    filter=""
    for binary in {{binaries}}; do
        filter+="binary(=$binary) | "
    done
    filter+="binary(=cucumber_e2e)"
    if [[ -n "${VOICEPUNCHER_RERUN_FILTER:-}" ]]; then
        filter="($filter) & ($VOICEPUNCHER_RERUN_FILTER)"
    fi
    printf '%s\n' "$filter"

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
    cargo nextest run --profile e2e-regression {{cucumber_e2e_tests}} --no-capture {{rerun_filter}} {{fail_fast}}

# WER accuracy on 10 samples (5 per language) — fails if mean WER > 30 %, and no command in 10 recordings of ordinary speech (~1 min)
e2e-quick:
    @VOICEPUNCHER_STAGES_RECIPE=e2e-quick {{just_executable()}} _stages e2e-quick

_e2e-quick: _require-nextest _require-models _require-corpus _build
    #!/usr/bin/env bash
    set -euo pipefail
    trap '{{just_executable()}} _stop-model-servers-for-e2e' EXIT
    {{just_executable()}} _start-model-servers-for-e2e
    VOICEPUNCHER_TRANSCRIPTION_URL="{{stt_url}}" VOICEPUNCHER_REFINEMENT_URL="{{llm_url}}" VOICEPUNCHER_COMMAND_SPOTTING_URL="{{kws_url}}" VOICEPUNCHER_BENCH_LIMIT=10 \
        cargo nextest run --profile e2e-quick {{cucumber_e2e_tests}} --no-capture {{rerun_filter}} {{fail_fast}}

# Text refinement on the live services: dictated commands, normalization, evaluation per category (@refinement) (~30 s)
refine-eval:
    @VOICEPUNCHER_STAGES_RECIPE=refine-eval {{just_executable()}} _stages refine-eval

_refine-eval: _require-nextest _require-models _build
    #!/usr/bin/env bash
    set -euo pipefail
    trap '{{just_executable()}} _stop-model-servers-for-e2e' EXIT
    {{just_executable()}} _start-model-servers-for-e2e
    VOICEPUNCHER_TRANSCRIPTION_URL="{{stt_url}}" VOICEPUNCHER_REFINEMENT_URL="{{llm_url}}" \
        cargo nextest run --profile refine-eval {{cucumber_e2e_tests}} --no-capture {{rerun_filter}} {{fail_fast}}

# Outages: services that hang, die and come back while voice control runs (@resilience), each scenario on service ports of its own, so the shared services are never touched (~4 min)
e2e-resilience:
    @VOICEPUNCHER_STAGES_RECIPE=e2e-resilience {{just_executable()}} _stages e2e-resilience

_e2e-resilience: _require-nextest _require-models _build
    cargo nextest run --profile e2e-resilience {{cucumber_e2e_tests}} --no-capture {{rerun_filter}} {{fail_fast}}

# The bundled engines: a copy of dist/voicepuncher.app launched outside the build folder with nothing but the system
# folders on $PATH starts every service from its own engines, on ports of its own; a transcription, a refinement and
# the command spotting service's health through them, and Metal in the engines' logs (@bundle_engines, ~1 min)
bundle-smoke:
    @VOICEPUNCHER_STAGES_RECIPE=bundle-smoke {{just_executable()}} _stages bundle e2e-bundle

# Builds the bundle first (incremental: seconds after the bundle stage), so a reused bundle stage never leaves it a
# missing or stale one; starts no shared service (_model-stages)
_e2e-bundle: _require-nextest _require-models _require-corpus
    {{just_executable()}} --justfile {{quote(justfile())}} bundle
    cargo nextest run --profile e2e-bundle {{cucumber_e2e_tests}} --no-capture {{rerun_filter}} {{fail_fast}}

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
    cargo nextest run --profile e2e-seams {{e2e_seams_tests}} {{rerun_filter}} {{fail_fast}}
    cargo nextest run --profile e2e-seams {{cucumber_e2e_tests}} --no-capture {{rerun_filter}} {{fail_fast}}

# WER accuracy on a fixed sample of 100 recordings (50 per language), two at a time — fails if mean WER > 30 %, and no command in 20 recordings of ordinary speech (~3 min)
e2e-bench:
    @VOICEPUNCHER_STAGES_RECIPE=e2e-bench {{just_executable()}} _stages e2e-bench

_e2e-bench: _require-nextest _require-models _require-corpus _build
    #!/usr/bin/env bash
    set -euo pipefail
    trap '{{just_executable()}} _stop-model-servers-for-e2e' EXIT
    {{just_executable()}} _start-model-servers-for-e2e
    VOICEPUNCHER_TRANSCRIPTION_URL="{{stt_url}}" VOICEPUNCHER_REFINEMENT_URL="{{llm_url}}" VOICEPUNCHER_COMMAND_SPOTTING_URL="{{kws_url}}" \
        cargo nextest run --profile e2e-bench {{cucumber_e2e_tests}} --no-capture {{rerun_filter}} {{fail_fast}}

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
    for engine in whisper llama command-spotting; do cp -R "$engines/$engine" "$app/Contents/MacOS/engines/"; done
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
    cargo nextest run --profile bundle {{cucumber_e2e_tests}} --no-capture {{rerun_filter}} {{fail_fast}}

# A bundle's Info.plist checked key by key, then its signature verified strictly, and its DMG mounted and found
# to hold the app and the Applications shortcut; the first failure names what is wrong
_check-bundle app dmg:
    #!/usr/bin/env bash
    set -euo pipefail
    app=$(cd {{quote(app)}} && pwd)
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
    # The engines: one folder each with its program and VERSION, every reference inside its folder, each file
    # signed by the app's signer, each engine running with nothing but the system folders on $PATH; the licenses
    signer() { codesign -dvv "$1" 2>&1 | grep -E '^(Authority|TeamIdentifier)=|Signature=adhoc' || true; }
    app_signer=$(signer "$app/Contents/MacOS/voicepuncher")
    for engine in whisper:whisper-server:--help llama:llama-server:--version command-spotting:voicepuncher-command-spotting:--version; do
        IFS=: read -r folder program flag <<< "$engine"
        dir="$app/Contents/MacOS/engines/$folder"
        [[ -x "$dir/$program" && -s "$dir/VERSION" ]] || { echo "✗ bundle lacks engines/$folder/$program or its VERSION" >&2; exit 1; }
        for file in "$dir"/*; do
            [[ "$(basename "$file")" == VERSION ]] && continue
            while read -r ref; do
                [[ "$ref" == /opt/homebrew/* || "$ref" == /usr/local/* ]] && { echo "✗ $file references $ref, outside the bundle" >&2; exit 1; }
                [[ "$ref" == @rpath/* && ! -f "$dir/${ref#@rpath/}" ]] && { echo "✗ $file references $ref, which engines/$folder lacks" >&2; exit 1; }
            done < <(otool -L "$file" | tail -n +2 | awk '{print $1}')
            codesign --verify --strict "$file" 2>/dev/null || { echo "✗ $file: signature does not verify" >&2; exit 1; }
            [[ "$(signer "$file")" == "$app_signer" ]] || { echo "✗ $file is not signed by the app's signer" >&2; exit 1; }
        done
        (cd / && env -i PATH=/usr/bin:/bin "$dir/$program" "$flag" >/dev/null 2>&1) \
            || { echo "✗ engines/$folder/$program $flag fails with nothing but the system folders on \$PATH" >&2; exit 1; }
    done
    for license in whisper.cpp ggml llama.cpp libomp sherpa-onnx onnxruntime; do
        [[ -s "$app/Contents/Resources/licenses/$license-LICENSE" ]] || { echo "✗ bundle lacks the license $license-LICENSE" >&2; exit 1; }
    done
    mount=$(mktemp -d)
    hdiutil attach -nobrowse -readonly -mountpoint "$mount" "$dmg" >/dev/null 2>&1
    trap 'hdiutil detach "$mount" -quiet 2>/dev/null || true; rmdir "$mount" 2>/dev/null || true' EXIT
    [[ -d "$mount/voicepuncher.app" ]] || { echo "✗ $dmg holds no voicepuncher.app" >&2; exit 1; }
    [[ -L "$mount/Applications" ]] || { echo "✗ $dmg holds no Applications shortcut" >&2; exit 1; }
    echo "✓ bundle stage: $app signed and verified with its engines and licenses, $dmg holds the app and the Applications shortcut"

# Self-test of _class-lock, run by the `test` stage, on lock directories of its own: the lock wait is 6 h unless
# the caller sets it, a light and a heavy run overlap, two runs of one class take turns, and an exclusive run of
# the machine lock keeps both out
_test-class-locks:
    #!/usr/bin/env bash
    set -euo pipefail
    dir=$(mktemp -d)
    trap 'rm -rf "$dir"' EXIT
    export MACHINE_LOCK_DIR="$dir/machine-lock" MACHINE_LOCK_PRIORITY=normal
    now() { perl -MTime::HiRes=time -e 'printf "%.2f", time'; }
    # hold <name> <lock…>: holds the lock 1.5 s, writing when it got it and when it let go to <name>
    hold() {
        local name=$1
        shift
        "$@" sh -c "perl -MTime::HiRes=time -e 'printf \"%.2f \", time' >$dir/$name; sleep 1.5; perl -MTime::HiRes=time -e 'printf \"%.2f\n\", time' >>$dir/$name"
    }
    class() { {{just_executable()}} --justfile {{quote(justfile())}} _class-lock "$@"; }
    # overlap <a> <b>: whether the two holds overlapped in time
    overlap() { perl -e 'my ($a0, $a1) = split " ", `cat $ARGV[0]`; my ($b0, $b1) = split " ", `cat $ARGV[1]`; exit(($a0 < $b1 && $b0 < $a1) ? 0 : 1)' "$dir/$1" "$dir/$2"; }
    fail() { echo "✗ $1" >&2; for f in "$dir"/[a-z]*; do [[ -f "$f" ]] && echo "  $(basename "$f"): $(cat "$f")" >&2; done; exit 1; }
    wait_default=$(env -u MACHINE_LOCK_WAIT {{just_executable()}} --justfile {{quote(justfile())}} --evaluate MACHINE_LOCK_WAIT 2>/dev/null || true)
    [[ "$wait_default" == 21600 ]] || fail "the lock wait defaults to '$wait_default', not 21600 s (6 h)"
    wait_set=$(MACHINE_LOCK_WAIT=5 {{just_executable()}} --justfile {{quote(justfile())}} --evaluate MACHINE_LOCK_WAIT 2>/dev/null || true)
    [[ "$wait_set" == 5 ]] || fail "a caller's lock wait 5 became '$wait_set'"
    echo "  ✓ the lock wait is 6 h unless the caller sets it"
    hold light1 class light & hold heavy1 class heavy & wait
    overlap light1 heavy1 || fail "a light and a heavy run took turns"
    echo "  ✓ a light and a heavy run overlap"
    hold light2 class light & hold light3 class light & wait
    ! overlap light2 light3 || fail "two light runs overlapped"
    hold heavy2 class heavy & hold heavy3 class heavy & wait
    ! overlap heavy2 heavy3 || fail "two heavy runs overlapped"
    echo "  ✓ two runs of one class take turns"
    hold machine {{quote(machine_lock)}} & sleep 0.5
    hold light4 class light & hold heavy4 class heavy & wait
    ! overlap machine light4 || fail "a light run overlapped an exclusive run"
    ! overlap machine heavy4 || fail "a heavy run overlapped an exclusive run"
    echo "  ✓ an exclusive run keeps both classes out"

# Self-test of _lock-phases, run by the `test` stage: the model stages split into one phase per lock class, each
# keeping the stages' order, the phases in the order of their first stage; a stage light_stages does not name is
# heavy; a class without a stage prints no phase
_test-lock-phases:
    #!/usr/bin/env bash
    set -euo pipefail
    phases() { {{just_executable()}} --justfile {{quote(justfile())}} _lock-phases "$@"; }
    expect() {
        local what=$1 want=$2 got
        shift 2
        got=$(phases "$@")
        [[ "$got" == "$want" ]] || { printf '✗ %s\nwant:\n%s\ngot:\n%s\n' "$what" "$want" "$got" >&2; exit 1; }
        echo "  ✓ $what"
    }
    # check and full-check (fail-fast-stage-order): the light phase first, voice control holding the most failures;
    # in each phase the short stages first, the long Gherkin runs last; the same stages as before the reorder
    expect "check: voice control first, then the short heavy stages" \
        $'light e2e-smoke-voice\nheavy e2e-smoke-refinement refine-eval e2e-smoke-speech e2e-quick e2e-smoke-services' \
        {{check_model_stages}}
    expect "full-check: every voice control stage first, the seams last" \
        $'light e2e-smoke-voice e2e-regression e2e-resilience\nheavy e2e-smoke-refinement refine-eval e2e-bundle e2e-smoke-speech accuracy e2e-smoke-services e2e-seams' \
        {{full_check_model_stages}}
    sorted() { tr ' ' '\n' <<<"$*" | grep . | sort | tr '\n' ' '; }
    [[ $(sorted {{check_model_stages}}) == $(sorted e2e-smoke-speech e2e-smoke-refinement e2e-smoke-voice e2e-smoke-services e2e-quick refine-eval) ]] \
        || { echo "✗ check runs other stages than before: {{check_model_stages}}" >&2; exit 1; }
    [[ $(sorted {{full_check_model_stages}}) == $(sorted e2e-smoke-speech e2e-smoke-refinement e2e-smoke-voice e2e-smoke-services e2e-regression e2e-seams e2e-resilience e2e-bundle accuracy refine-eval) ]] \
        || { echo "✗ full-check runs other stages than before: {{full_check_model_stages}}" >&2; exit 1; }
    [[ $(sorted {{e2e_smoke_stages}}) == $(sorted e2e-smoke-speech e2e-smoke-refinement e2e-smoke-voice e2e-smoke-services) && "{{e2e_smoke_stages}}" == e2e-smoke-voice* ]] \
        || { echo "✗ the smoke parts are not voice control first: {{e2e_smoke_stages}}" >&2; exit 1; }
    echo "  ✓ check and full-check run the same stages as before"
    [[ "{{voice_smoke_binaries}}" == "placed_commands_it voice_forms_it "* ]] \
        || { echo "✗ the voice control part does not run placed_commands_it and voice_forms_it first: {{voice_smoke_binaries}}" >&2; exit 1; }
    echo "  ✓ the voice control part runs its most failing binaries first"
    part() { {{just_executable()}} --justfile {{quote(justfile())}} _smoke-part-filter "$@"; }
    want='binary(=placed_commands_it) | binary(=voice_forms_it) | binary(=cucumber_e2e)'
    [[ $(VOICEPUNCHER_RERUN_FILTER= part "placed_commands_it voice_forms_it") == "$want" ]] \
        || { echo "✗ a smoke part does not select its binaries and the harness in one call: $(part "placed_commands_it voice_forms_it")" >&2; exit 1; }
    [[ $(VOICEPUNCHER_RERUN_FILTER= part "") == 'binary(=cucumber_e2e)' ]] \
        || { echo "✗ a smoke part without binaries does not select the harness alone: $(part "")" >&2; exit 1; }
    [[ $(VOICEPUNCHER_RERUN_FILTER='test(=x)' part "voice_forms_it") == '(binary(=voice_forms_it) | binary(=cucumber_e2e)) & (test(=x))' ]] \
        || { echo "✗ a rerun's filter widens a smoke part instead of narrowing it" >&2; exit 1; }
    echo "  ✓ a smoke part runs its binaries beside the harness in one call, narrowed by a rerun"
    expect "a light first stage runs its phase first" $'light e2e-resilience\nheavy e2e-seams' e2e-resilience e2e-seams
    expect "an unknown stage is heavy" "heavy e2e-something-new" e2e-something-new
    expect "only light stages give one phase" "light e2e-regression" e2e-regression
    expect "no stage, no phase" ""

# The stage keys' expected behaviour (run by the `test` stage): in a scratch git repository laid out like this
# one, a stage's key changes with a file it reads (tracked, untracked or deleted, committed or not) and with its
# name, and not with a file of no stage, a file only another model stage reads, or a version-only Cargo edit
# for a model stage; an input variable changes every key, a model file only the model stages' keys
_test-stage-keys:
    #!/usr/bin/env bash
    set -euo pipefail
    repo=$(mktemp -d)
    trap 'rm -rf "$repo"' EXIT
    stages="fmt test e2e-smoke-voice e2e-smoke-refinement refine-eval"
    keys() { VOICEPUNCHER_STAGE_ROOT="$repo" {{just_executable()}} --justfile {{quote(justfile())}} _stage-keys $stages; }
    key() { awk -v s="$1" '$1 == s { print $2 }' <<<"$2"; }
    put() { mkdir -p "$repo/$(dirname "$1")"; printf '%s\n' "$2" >"$repo/$1"; }
    put Cargo.toml $'[package]\nname = "voicepuncher"\nversion = "1.0.0"'
    put Cargo.lock $'[[package]]\nname = "voicepuncher"\nversion = "1.0.0"\n\n[[package]]\nname = "serde"\nversion = "1.0.0"'
    put docs/notes.md "notes"
    put app/src/core/domain/text.rs "pub struct Text;"
    put app/src/adapters/secondary/textrefinement/openai/client.rs "fn refine() {}"
    put command-spotting/src/main.rs "fn main() {}"
    git -C "$repo" init -q
    git -C "$repo" add -A
    git -C "$repo" -c user.name=t -c user.email=t@t commit -qm init
    before=$(keys)
    fail() { echo "✗ $1" >&2; printf 'before:\n%s\nafter:\n%s\n' "$before" "$after" >&2; exit 1; }
    for stage in $stages; do
        [[ $(key "$stage" "$before") =~ ^[0-9a-f]{64}$ ]] || { after=""; fail "no key for $stage"; }
    done
    [[ $(key fmt "$before") != $(key test "$before") ]] || { after=""; fail "two stages share a key"; }
    # <description> <changed stages> <unchanged stages>: compares `after` with `before`, then makes it the base
    expect() {
        local what=$1 changed=$2 same=$3 stage
        after=$(keys)
        for stage in $changed; do [[ $(key "$stage" "$after") != $(key "$stage" "$before") ]] || fail "$what: $stage kept its key"; done
        for stage in $same; do [[ $(key "$stage" "$after") == $(key "$stage" "$before") ]] || fail "$what: $stage changed its key"; done
        echo "  ✓ $what"
        before=$after
    }
    expect "the same inputs give the same keys" "" "$stages"
    put docs/notes.md "more notes"
    expect "a docs edit changes no key" "" "$stages"
    put app/src/core/domain/text.rs "pub struct Text(String);"
    expect "an uncommitted edit of shared code changes every key" "$stages" ""
    put app/src/adapters/secondary/textrefinement/openai/client.rs "fn refine() { todo!() }"
    expect "a refinement edit changes the refinement stages, not voice control" "fmt test e2e-smoke-refinement refine-eval" "e2e-smoke-voice"
    put command-spotting/src/engine.rs "fn spot() {}"
    expect "an untracked command spotting file changes voice control, not refinement" "fmt test e2e-smoke-voice" "e2e-smoke-refinement refine-eval"
    rm "$repo/command-spotting/src/main.rs"
    expect "a deleted file changes the stages that read it" "fmt e2e-smoke-voice" "e2e-smoke-refinement"
    put app/src/new_module.rs "pub fn new() {}"
    expect "a file no line names changes every key" "$stages" ""
    put Cargo.toml $'[package]\nname = "voicepuncher"\nversion = "1.1.0"'
    expect "a version-only Cargo edit changes the code stages only" "fmt test" "e2e-smoke-voice e2e-smoke-refinement refine-eval"
    put Cargo.lock $'[[package]]\nname = "voicepuncher"\nversion = "1.1.0"\n\n[[package]]\nname = "serde"\nversion = "1.0.0"'
    expect "a version-only Cargo.lock edit changes the code stages only" "fmt test" "e2e-smoke-voice e2e-smoke-refinement refine-eval"
    put Cargo.lock $'[[package]]\nname = "voicepuncher"\nversion = "1.1.0"\n\n[[package]]\nname = "serde"\nversion = "1.0.1"'
    expect "a dependency bump in Cargo.lock changes every key" "$stages" ""
    after=$(VOICEPUNCHER_HANG_PROBE=1 keys)
    for stage in $stages; do [[ $(key "$stage" "$after") != $(key "$stage" "$before") ]] || fail "an input variable: $stage kept its key"; done
    echo "  ✓ an input variable changes every key"
    model="$repo.model"
    trap 'rm -rf "$repo" "$model"' EXIT
    printf 'weights\n' >"$model"
    before=$(VOICEPUNCHER_LLM_MODEL="$model" keys)
    printf 'other weights\n' >"$model"
    after=$(VOICEPUNCHER_LLM_MODEL="$model" keys)
    for stage in e2e-smoke-voice refine-eval; do [[ $(key "$stage" "$after") != $(key "$stage" "$before") ]] || fail "a model file: $stage kept its key"; done
    for stage in fmt test; do [[ $(key "$stage" "$after") == $(key "$stage" "$before") ]] || fail "a model file: $stage changed its key"; done
    echo "  ✓ a changed model file changes the model stages only"
    before=$(keys)
    after=$(VOICEPUNCHER_STAGE_JOURNAL=/elsewhere VOICEPUNCHER_TEST_STT_PORT=1 VOICEPUNCHER_FAIL_FAST=1 keys)
    [[ "$after" == "$before" ]] || fail "a volatile variable changed a key"
    echo "  ✓ volatile variables change no key"
    # The test stage's parts: one key per workspace member, over its own files, the members it depends on and
    # every file outside the members; one over the justfile alone
    put Cargo.toml $'[workspace]\nmembers = ["app", "command-spotting", "copy-pool", "e2e-test"]'
    put app/Cargo.toml $'[package]\nname = "voicepuncher"\n\n[dependencies]\nvoicepuncher-copy-pool = { path = "../copy-pool" }\n\n[dev-dependencies]\nvoicepuncher-e2e-test = { path = "../e2e-test" }'
    put command-spotting/Cargo.toml $'[package]\nname = "voicepuncher-command-spotting"\n\n[dependencies]\nvoicepuncher-copy-pool = { path = "../copy-pool" }'
    put copy-pool/Cargo.toml $'[package]\nname = "voicepuncher-copy-pool"'
    put e2e-test/Cargo.toml $'[package]\nname = "voicepuncher-e2e-test"\n\n[dependencies]\nvoicepuncher = { path = "../app" }\n\n[dev-dependencies]\nvoicepuncher-command-spotting = { path = "../command-spotting" }'
    put justfile "test:"
    stages="test@members test@justfile"
    parts="test@app test@command-spotting test@copy-pool test@e2e-test test@justfile"
    before=$(keys)
    for part in $parts; do
        [[ $(key "$part" "$before") =~ ^[0-9a-f]{64}$ ]] || { after=""; fail "no key for $part"; }
    done
    expect "the same inputs give the same part keys" "" "$parts"
    put command-spotting/src/engine.rs "fn spot() { todo!() }"
    expect "a command spotting edit changes its part and its dependents' only" "test@command-spotting test@e2e-test" "test@app test@copy-pool test@justfile"
    put copy-pool/src/lib.rs "pub fn pool() {}"
    expect "a copy pool edit changes every package's part" "test@app test@command-spotting test@copy-pool test@e2e-test" "test@justfile"
    put app/src/core/domain/text.rs "pub struct Text(u8);"
    expect "an app edit reaches app and e2e-test, its dev-dependency cycle" "test@app test@e2e-test" "test@command-spotting test@copy-pool test@justfile"
    put .config/nextest.toml "[profile.default]"
    expect "a file outside the members changes every package's part" "test@app test@command-spotting test@copy-pool test@e2e-test" "test@justfile"
    put docs/notes.md "even more notes"
    expect "a docs edit changes no part" "" "$parts"
    put app/Cargo.toml $'[package]\nname = "voicepuncher"\n\n[package.metadata.stage-keys]\ntests-read-every-member = true\n\n[dependencies]\nvoicepuncher-copy-pool = { path = "../copy-pool" }\n\n[dev-dependencies]\nvoicepuncher-e2e-test = { path = "../e2e-test" }'
    expect "a member whose tests read every member declares it" "test@app test@e2e-test" "test@command-spotting test@copy-pool test@justfile"
    put command-spotting/src/engine.rs "fn spot() { unimplemented!() }"
    expect "a command spotting edit reaches a member whose tests read every member" "test@command-spotting test@e2e-test test@app" "test@copy-pool test@justfile"
    put justfile "test: _test"
    expect "a justfile edit changes the self-tests' part" "$parts" ""

# ── Engines ── the pinned engines the bundle carries (app/packaging/engines.lock) ─────────────

engines_lock := env_var_or_default("VOICEPUNCHER_ENGINES_LOCK", packaging_dir / "engines.lock")
# Where `engines` stages them (gitignored): one folder per engine, its libraries beside it, and the licenses
engines_dir := packaging_dir / "engines"
engine_cache := env_var_or_default("VOICEPUNCHER_ENGINE_CACHE", home_directory() / ".cache/voicepuncher/engines")

# Stage the pinned engines of engines.lock into app/packaging/engines/, relocated and checked to run on their own
engines: (_engines engines_lock engines_dir)

# Stages the engines of a lock into a folder: checks every pinned version and checksum first and writes nothing
# when one differs; then copies each engine's files, relocates them (every Homebrew path → @rpath, @loader_path the
# only search path, ggml's compiled-in backend folder blanked so it loads its backends from its own folder), signs
# them ad-hoc (the bundle signs them again), proves no reference leaves the folder and each engine runs with
# nothing but the system folders on $PATH, and writes each folder's VERSION and the licenses. Whisper and llama
# are kept when the folder was staged from the same lock; the command spotting service is rebuilt every time
_engines lock out:
    #!/usr/bin/env perl
    use strict;
    use warnings;
    use Digest::SHA;
    use File::Basename qw(basename dirname);
    use File::Copy qw(copy);
    use File::Path qw(make_path remove_tree);
    use File::Temp qw(tempdir);
    $| = 1;
    my ($lock, $out) = ({{quote(lock)}}, {{quote(out)}});
    my $root = {{quote(justfile_directory())}};
    my $cache = {{quote(engine_cache)}};
    my @engines = ("whisper", "llama", "command-spotting");
    my (%t, $section);
    open my $fh, "<", $lock or die "✗ engine lock $lock: $!\n";
    while (<$fh>) {
        next if /^\s*(#|$)/;
        if (/^\[([^\]]+)\]/) { $section = $1; next }
        die "✗ $lock line $.: not a key = \"value\" line\n" unless /^"?([^"=]+?)"?\s*=\s*"([^"]*)"/;
        $t{$section}{$1} = $2;
    }
    my $sha = sub { my $d = Digest::SHA->new(256); $d->addfile($_[0]); $d->hexdigest };
    my $run = sub { system(@_) == 0 or die "✗ @_ failed\n" };
    my $capture = sub { my $cmd = join " ", map { "'$_'" } @_; my $o = `$cmd 2>&1`; $? == 0 or die "✗ @_ failed:\n$o"; $o };
    my $lock_sha = $sha->($lock);
    my $stamp = "$out/.lock-sha256";
    my $kept = -f $stamp && do { open my $s, "<", $stamp; my $v = <$s> // ""; chomp $v; $v eq $lock_sha }
        && !grep { !-d "$out/$_" } @engines;
    my (@problems, %source);
    my $cellar;
    for my $engine ("whisper", "llama") {
        my $e = $t{$engine} or die "✗ $lock has no [$engine]\n";
        my $from = $t{"$engine.from"} or die "✗ $lock has no [$engine.from]\n";
        my $sums = $t{"$engine.sha256"} || {};
        my $base;
        if ($e->{source} eq "homebrew") {
            $cellar //= do { my $c = `brew --cellar 2>/dev/null`; chomp $c;
                $c or die "✗ Homebrew is needed to stage the engines (developers only): https://brew.sh\n" };
            my %formulae = %{ $t{"$engine.formulae"} || {} };
            for my $formula (sort keys %formulae) {
                next if -d "$cellar/$formula/$formulae{$formula}";
                my @found = map { basename $_ } grep { -d } glob "$cellar/$formula/*";
                push @problems, "$formula $formulae{$formula} is not installed (found: "
                    . (@found ? join(", ", @found) : "none") . ") — engines.lock pins $formula $formulae{$formula}:"
                    . " install that version or update the lock, and keep it with 'brew pin "
                    . join(" ", sort keys %formulae) . "'";
            }
            $source{$engine} = sub { my ($formula, $path) = split m{/}, $_[0], 2; "$cellar/$formula/$formulae{$formula}/$path" };
        } elsif ($e->{source} eq "github-release") {
            my $dir = "$cache/$engine/$e->{version}";
            my $archive = "$dir/$e->{asset}";
            unless (-f $archive) {
                make_path($dir);
                print "→ downloading $e->{url}\n";
                $run->("curl", "-sSL", "--fail", "-o", "$archive.part", $e->{url});
                rename "$archive.part", $archive or die "✗ $archive: $!\n";
            }
            my $got = $sha->($archive);
            if ($got ne $e->{asset_sha256}) {
                push @problems, "$e->{asset}: sha256 is $got, engines.lock expects $e->{asset_sha256} ($archive;"
                    . " delete it to download it again, or correct the lock)";
                next;
            }
            $base = tempdir(CLEANUP => 1);
            $run->("tar", "-xzf", $archive, "-C", $base);
            $source{$engine} = sub { "$base/$_[0]" };
        } else {
            die "✗ [$engine] source '$e->{source}' is neither homebrew nor github-release\n";
        }
        next if @problems;
        for my $file (sort keys %$from) {
            my $path = $source{$engine}->($from->{$file});
            unless (-f $path) { push @problems, "$engine: $path ($file) does not exist"; next }
            my $got = $sha->($path);
            my $want = $sums->{$file} // "";
            push @problems, "$engine: $file ($path) has sha256 $got, engines.lock expects " . ($want || "none")
                if $got ne $want;
        }
    }
    if (@problems) {
        print STDERR "✗ $_\n" for @problems;
        print STDERR "✗ no engine staged: the engines are taken only as $lock pins them (README § Engines)\n";
        exit 1;
    }
    make_path($out);
    my $stage = tempdir("engines.XXXXXX", DIR => $out, CLEANUP => 1);
    my @fresh = $kept ? ("command-spotting") : @engines;
    for my $engine (grep { $_ ne "command-spotting" } @fresh) {
        my $dir = "$stage/$engine";
        make_path($dir);
        my $from = $t{"$engine.from"};
        for my $file (sort keys %$from) {
            copy($source{$engine}->($from->{$file}), "$dir/$file") or die "✗ copy $file: $!\n";
            chmod 0755, "$dir/$file";
            system("xattr", "-c", "$dir/$file");
        }
        # The compiled-in folders of the formulae (ggml's GGML_BACKEND_DIR), so a backend is never loaded from them
        my @blank = map { "$cellar/$_/" . $t{"$engine.formulae"}{$_} . "/libexec" } keys %{ $t{"$engine.formulae"} || {} };
        for my $file (sort keys %$from) {
            my $path = "$dir/$file";
            my ($type) = $capture->("otool", "-hv", $path) =~ /\b(EXECUTE|DYLIB|BUNDLE)\b/ or die "✗ $path is no Mach-O file\n";
            my @args;
            for my $ref ($capture->("otool", "-L", $path) =~ /^\s+(\S+) \(/mg) {
                push @args, "-change", $ref, "\@rpath/" . basename($ref) if $ref =~ m{^(/opt/homebrew|/usr/local)/};
            }
            push @args, "-id", "\@rpath/$file" if $type eq "DYLIB";
            my @rpaths = $capture->("otool", "-l", $path) =~ /cmd LC_RPATH\n.*\n\s+path (\S+)/g;
            push @args, map { ("-delete_rpath", $_) } grep { $_ ne "\@loader_path" } @rpaths;
            push @args, "-add_rpath", "\@loader_path" unless grep { $_ eq "\@loader_path" } @rpaths;
            $capture->("install_name_tool", @args, $path) if @args;
            if (@blank) {
                open my $in, "<:raw", $path or die "✗ $path: $!\n";
                my $bytes = do { local $/; <$in> };
                close $in;
                my $count = 0;
                for my $folder (@blank) { $count += $bytes =~ s/\Q$folder\E/"\0" x length($folder)/ge }
                if ($count) { open my $o, ">:raw", $path or die "✗ $path: $!\n"; print $o $bytes; close $o }
            }
            $capture->("codesign", "--force", "--sign", "-", "--timestamp=none", $path);
        }
        open my $v, ">", "$dir/VERSION" or die; print $v "$t{$engine}{version}\n"; close $v;
    }
    my $spotting = $t{"command-spotting"} or die "✗ $lock has no [command-spotting]\n";
    $run->("cargo", "build", "--quiet", "--release", "-p", $spotting->{package});
    make_path("$stage/command-spotting");
    copy("$root/target/release/$spotting->{program}", "$stage/command-spotting/$spotting->{program}") or die "✗ copy $spotting->{program}: $!\n";
    chmod 0755, "$stage/command-spotting/$spotting->{program}";
    my ($crate_version) = do { local $/; open my $c, "<", "$root/command-spotting/Cargo.toml" or die; <$c> } =~ /^version = "([^"]+)"/m;
    open my $v, ">", "$stage/command-spotting/VERSION" or die; print $v "$crate_version\n"; close $v;
    $t{"command-spotting"}{version} = $crate_version;
    # Every reference stays in the engine's folder, and each engine runs with a clean environment
    for my $engine (@fresh) {
        my $dir = "$stage/$engine";
        for my $path (grep { -f && !/VERSION$/ } glob "$dir/*") {
            for my $ref ($capture->("otool", "-L", $path) =~ /^\s+(\S+) \(/mg) {
                die "✗ $path references $ref, outside the bundle\n" if $ref =~ m{^(/opt/homebrew|/usr/local)/};
                die "✗ $path references $ref, which $dir lacks\n" if $ref =~ m{^\@rpath/(.+)$} && !-f "$dir/$1";
            }
        }
        my $program = $t{$engine}{program};
        my $flag = $engine eq "whisper" ? "--help" : "--version";
        my $said = `env -i PATH=/usr/bin:/bin '$dir/$program' $flag 2>&1`;
        die "✗ $program $flag failed with nothing but the system folders on \$PATH:\n$said" if $?;
    }
    # The licenses of every bundled engine and library
    make_path("$stage/licenses");
    my %crates = do { local $/; open my $c, "<", "$root/Cargo.lock" or die; <$c> } =~ /name = "([^"]+)"\nversion = "([^"]+)"/g;
    my $registry = (glob("$ENV{HOME}/.cargo/registry/src/*"))[0] // "";
    for my $name (sort keys %{ $t{licenses} || {} }) {
        my ($kind, $path) = split /:/, $t{licenses}{$name}, 2;
        my $file = $kind eq "homebrew" ? do { my ($f, $p) = split m{/}, $path, 2; "$cellar/$f/" . $t{"whisper.formulae"}{$f} . "/$p" }
            : $kind eq "llama" ? $source{llama} ? $source{llama}->($path) : "$out/licenses/$name"
            : $kind eq "cargo" ? do { my ($c, $p) = split m{/}, $path, 2; "$registry/$c-$crates{$c}/$p" }
            : "$root/$path";
        copy($file, "$stage/licenses/$name") or die "✗ license $name ($file): $!\n";
    }
    for my $engine (@fresh, "licenses") {
        remove_tree("$out/$engine");
        rename "$stage/$engine", "$out/$engine" or die "✗ $out/$engine: $!\n";
    }
    open my $s, ">", $stamp or die; print $s "$lock_sha\n"; close $s;
    printf "  %-17s %-10s %s\n", "engine", "version", "size";
    for my $engine (@engines) {
        my ($kb) = `du -sk '$out/$engine'` =~ /^(\d+)/;
        my $version = do { open my $f, "<", "$out/$engine/VERSION" or die; my $l = <$f>; chomp $l; $l };
        printf "  %-17s %-10s %.1f MB%s\n", $engine, $version, $kb / 1024, (grep { $_ eq $engine } @fresh) ? "" : " (kept)";
    }
    print "✓ engines staged in $out\n";

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
    check_port {{stt_port}} whisper-server "${VOICEPUNCHER_TEST_PORTS}_STT_PORT"
    for port in $(copy_ports {{stt_port}} {{stt_instances}}); do
        check_port "$port" "whisper-server copy" "${VOICEPUNCHER_TEST_PORTS}_STT_PORT"
    done
    check_port {{command_stt_port}} whisper-server-commands "${VOICEPUNCHER_TEST_PORTS}_COMMAND_STT_PORT"
    for port in $(copy_ports {{command_stt_port}} {{command_stt_instances}}); do
        check_port "$port" "whisper-server-commands copy" "${VOICEPUNCHER_TEST_PORTS}_COMMAND_STT_PORT"
    done
    check_port {{llm_port}} llama-server "${VOICEPUNCHER_TEST_PORTS}_LLM_PORT"
    check_port {{kws_port}} voicepuncher-command-spotting "${VOICEPUNCHER_TEST_PORTS}_KWS_PORT"
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
    # The app's own download: every file of its model catalog, pinned and checked by sha256,
    # resumed from a part left by an interrupted run; the refinement model is skipped with no
    cargo build --release -p voicepuncher
    args=(--download-models --models-dir ~/.voicepuncher/models)
    [[ "{{refinement}}" == no ]] && args+=(--no-llm)
    target/release/voicepuncher "${args[@]}"

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
