# Proposal

## Why

Checks and E2E recipes reuse any service already answering on its port (8081, 8080, 8082, later 8085). The user's own services, perhaps a day old with another model, or another session's, hold those ports, so the check silently tests them instead of the branch's models. On 2026-10-17 a full check on large-v3-turbo reused the user's ggml-small dictation and failed 7 `@seams` scenarios that pass on turbo.

With many sessions in parallel, a running service must never make a check fail or test the wrong thing.

## What Changes

- Every check, E2E and benchmark recipe tests only services it started itself, from the checkout's justfile and models, on ports of its own, apart from the ports the user's services use. It never reuses a service it did not start.
- It never stops or restarts a service it did not start.
- When a port it needs is held by something it cannot account for, it stops before any stage with a message naming the port and the process holding it, instead of testing that process.
- Each run names the services, ports and models it started.

## Capabilities

None. This is check tooling, not app behavior: `skip_specs: true`.

## Scope

- **Current stage:** PoC. **Target stage:** PoC (check tooling only).
- **Acceptance bar:**
  - With the user's services running on the default ports with another model, `just check` and `just full-check` test the checkout's models and pass, running as many tests as before; the user's services keep their PIDs.
  - A port the check needs, held by an unknown process, fails the run before its first stage, naming that process.
- **Manual run:** none.

### Out of scope

- The app's default ports and the service scripts as the user runs them.
- Checking a reused service's model or flags.

## Release

No release: no app behavior changes; `version` in `app/Cargo.toml` stays as it is.
