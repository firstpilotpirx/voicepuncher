# Design

## Context

The orchestrator itself is generic and lives in agent-kit (its change `orchestrate-changes`: `orchestration.yaml`, readiness in `kit backlog`, the `/orchestrate` skill, the start answers `/proceed-change` takes from it). This change supplies only what is specific to voicepuncher: where `AGENTS.md` tells an agent about it, and which limits of this project make parallel changes collide.

## Decisions

1. **One `AGENTS.md` paragraph under § Workflow, no new file.** It names `/orchestrate`, the per-change `orchestration.yaml` (`depends_on`, `autonomy`) and links the kit skill instead of restating it (`kit-placement.md`: project values here, workflow knowledge in the kit).
2. **Shared E2E ports need nothing new.** Every check, test and benchmark recipe already queues on the machine-wide test lock (`with-machine-lock`), so changes run their `just check` one at a time while coding goes on in parallel. The paragraph says so.
3. **Parallel version bumps are resolved at merge, not prevented.** Two changes that both bump `app/Cargo.toml` from the same version conflict when the second merges `origin/main` (`kit sync-base`). The second change then takes the next version above `main` (PATCH on PATCH → the next patch) and fixes the target version in its proposal; `kit version-check` checks it. Serializing version-bumping changes would throw away most of the parallelism: every other active change bumps the version.
4. **Implicit overlaps are declared by hand.** Two changes touching the same code (chunk seams, the command spotting protocol) depend on each other even when nothing says so. The paragraph asks `/create-change` to declare such a dependency in `orchestration.yaml`; the orchestrator does not guess overlaps.
5. **No `orchestration.yaml` for the current backlog in this change.** No active proposal names another active change as a prerequisite (`fix-dropped-phrase-at-word-pauses` names `commands-without-pauses`, already archived); a change without the file has no dependencies.

## Test level

No spec scenarios (`skip_specs: true`). The acceptance bar is checked by hand in this repo after the agent-kit change is on its `main`: two scratch changes with no dependency show as ready in `kit backlog`, a third whose `orchestration.yaml` names one of them shows as blocked; the scratch changes are removed afterwards and never committed.

## Risks

- The app may refuse to start a session on a turn that a side-session event started. The kit skill falls back to a proposal card and a push notification; covered by the agent-kit change.
