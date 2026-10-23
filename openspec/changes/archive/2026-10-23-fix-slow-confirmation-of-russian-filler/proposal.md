## Why

The E2E resilience scenario "Russian speech that makes speech recognition repeat itself" (`e2e-test/tests/features/local_services.feature:133`, rule "Speech that is no command is confirmed quickly") fails without a code change touching voice control: in the `e2e-resilience` stage of `just full-check` (light lock class, test ports 27081 and siblings, services of its own) on branch `chore/fast-test-build` (a test layout change that touches no voice control or confirmation code) at 17:03 on 2026-10-22, one confirmation of the Russian counting and filler words took 2.116 s against the bound of 2 s; the other 58 confirmations of the same speech took 112 ms to 1.8 s, the other 12 scenarios of the stage passed, the light lock was taken after 1 s (no other light run beside it), the load average was 14–20 (other sessions' builds and heavy model stages beside this run's light phase, as the lock classes allow), and the run's accuracy stage used 100 recordings.

A flake is a failure (AGENTS.md § How to verify a change): the check is never rerun unchanged, so the cause of a confirmation slower than its bound under the load the lock classes allow must be found and fixed, or the bound must be shown to hold with that load by design.

## What Changes

- Every confirmation of speech that is no command is answered in under 2 seconds while another run's heavy model stage and other sessions' builds load the machine, as the light lock class allows (design of `split-machine-lock`): the slow confirmations (1.0–2.1 s among 0.2 s ones) are explained (the speech recognition service for commands repeating itself on filler words, a copy of the pool busy, GPU contention) and their cause removed, or the scenario is shown to measure something the light class cannot guarantee and is moved to the heavy class or its bound is set from the measurement, by design, not loosened to pass.

Current stage: PoC. Target stage: PoC.
Scope: the confirmation path of the command spotting service for non-command Russian speech, the `@resilience` steps that time it, and the lock class of the `e2e-resilience` stage.
Acceptance bar: the scenario passes in the `full-check` runs of three changes on this machine with other sessions' heavy model stages running beside it (the stage journal shows no failed "Russian speech that makes speech recognition repeat itself"); the design names the cause of the 2.116 s confirmation.
Out of scope: the other flaky scenarios of the audit of 2026-10-21 and `fix-clear-text-please-unrecognized`; the machine lock itself.

## Capabilities

### Modified Capabilities

- `local-services`: a confirmation of Russian speech that is no command is answered in under 2 seconds also while the machine runs another run's heavy model stage and other sessions' builds.

## Impact

- `command-spotting/src/` (the confirmation of heard speech), `e2e-test/tests/steps/` (the `@resilience` timing step), possibly `light_stages` in the `justfile`.
- `fast-test-build` waits for this change (`depends_on` in its `orchestration.yaml`), as the flake rule asks.

## Open points

- Whether the slow confirmations come from the speech recognition service for commands (whisper small repeating itself on filler words takes longer to decode) or from GPU contention with the heavy phase beside it: the proposed first step is to replay the cached synthesized speech against the service alone and beside a running `e2e-bench`, and compare the confirmation times.
- Target version: the bump proposed below; the change takes the next PATCH above `main` when it starts.

## Release

SemVer: PATCH — a bug fix of the command spotting service or its test, no new capability.
Target version: 0.37.5.
