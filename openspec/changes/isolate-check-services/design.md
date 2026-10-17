# Design

## D1. Test ports of their own, never reuse

Recipes that run under the machine lock (the model stages of `check` and `full-check`, every E2E and benchmark recipe run on its own) start the services on a fixed test port set apart from the defaults — e.g. 18081 dictation, 18085 commands, 18080 text refinement, 18082 command spotting, and their whisper copies on ports derived from them — and point the tests at them through the URLs the tests already read (`VOICEPUNCHER_TRANSCRIPTION_URL` and siblings). `_start-model-servers-for-e2e` no longer reuses: it starts every service it needs and marks each as its own; `_stop-model-servers-for-e2e` stops exactly those. The user's ports, and so the user's services, are never touched.

Why fixed ports: the machine lock already runs one such recipe at a time, so one set cannot collide with another run, and fixed ports keep logs and messages readable. Random ports were weighed and add nothing under the lock.

Rejected: keep reusing but compare a reused service's model and flags with the checkout's. whisper-server behind HAProxy does not report its model, llama-server reports the model path but not every flag, and the command spotting service has no such endpoint; the comparison would be partial, and on a mismatch the check still could not run. Starting its own services is both simpler and complete.

## D2. A held test port fails the run, naming its holder

Before starting, the recipe checks each test port. A port held by a service a previous locked run started and left behind (its pid file in `~/.voicepuncher/run` names a supervisor whose command line holds the binary — a run killed with KILL) is stopped and started fresh. Any other holder fails the run before its first stage with `✗ port <p> (<service>) is held by pid <pid>: <command> — stop it or set <env var>`, from `lsof -nP -iTCP:<p> -sTCP:LISTEN`. Nothing it cannot account for is stopped.

## D3. The run states what it tests

On start the recipe prints one line per service: port, model file and the started PID, so a log shows which models a run tested.

## D4. Cost

Each locked run now loads the models itself even when the user's services run (seconds for whisper copies and llama-server). The check's lock time is measured before and after on the same machine and reported in the PR; no bar is set beyond reporting it, since correctness outweighs it.

## Test levels

No spec scenario (`skip_specs: true`) and no app code change. Each behavior is proven by running the recipes in stage 5 (tasks.md): with the user's services on the default ports with another model, with a planted unknown listener on a test port, and with a leftover test service from a killed run. No manual run: hotkey, recording, overlay, paste and permissions are untouched.
