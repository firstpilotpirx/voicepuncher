## Why

`just check` and `just full-check` reuse a `voicepuncher-command-spotting` already listening on the shared port 8082 ("reusing voicepuncher-command-spotting already running on port 8082; it stays running"), even when the branch under check changes the `command-spotting/` crate. The check then verifies the branch's app against a service built from other code and started with another configuration.

Seen on 2026-10-17 while checking `fix-chained-start-after-cancel-enter`: the service on 8082 was the main checkout's `target/release` binary, started on 2026-10-16 with `--transcription-url http://127.0.0.1:8081/v1` (the dictation model, the configuration from before 0.15). Two check runs on the branch failed against it (`should_serve_several_apps_listening_at_once`; the user's recorded "restart" not carried out) and passed once run on ports of their own. A check can therefore pass or fail on code that is not the branch's, and a broken service change can be merged green.

## What Changes

- A check never verifies a branch against a command spotting service, or a speech recognition service for commands, built from other code or started with another configuration than the branch's own.
- Possible ways (the design picks): refuse or restart a stale shared service, or run the model stages on ports of their own when the branch changes `command-spotting/`.
- Bug fix: PATCH.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None: the checks and their `justfile` recipes change; the product's behavior does not.

## Scope

- Current stage: PoC. Target stage: PoC.
- Acceptance bar: with a stale `voicepuncher-command-spotting` listening on 8082, `just check` on a branch that changes `command-spotting/` runs against the branch's own service (or stops and says why), and the user's service on 8082 keeps running; `just full-check` passes.

## Out of scope

- Changes to the command spotting service or the app themselves.

## Impact

- `justfile` (`start-model-servers`, the model stages of `check` and `full-check`), possibly the E2E harness's port selection.
- The user's own running service on 8082 must keep running.

## Open points

- Whether a stale service is detected by binary (path, build time) or by its start arguments, or avoided by always using own ports.
- Whether the speech recognition services for dictation and text refinement need the same rule.

## Absorbed by

`isolate-check-services`: it makes every check, E2E and benchmark recipe start all its services — command spotting and the speech recognition service for commands included — from the checkout's own code on test ports of its own, and never reuse a service it did not start. That removes the stale shared service on 8082 this change was filed for, so it closes without its own implementation.
