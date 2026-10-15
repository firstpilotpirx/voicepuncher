# Tasks

## 1. Project values

- [x] 1.1 The agent-kit change `orchestrate-changes` is merged into agent-kit `main` and `kit install` linked it (`kit backlog` prints a state column: ready, blocked, running).
- [x] 1.2 `AGENTS.md` § Workflow: a paragraph on `/orchestrate`, the per-change `orchestration.yaml` (`depends_on`, `autonomy`), the machine-wide test lock serializing E2E, version conflicts resolved at merge, and implicit overlaps declared by hand (design decisions 1–4).
- [x] 1.3 `just test` passes; commit `docs: orchestrate changes in parallel sessions`.

## 2. Acceptance

- [x] 2.1 In a scratch checkout, two scratch changes without dependencies show as `ready` in `kit backlog` and a third whose `orchestration.yaml` names one of them shows as `blocked`; remove the scratch changes, nothing committed.

## 3. Version and manual checks

- [x] 3.1 No version bump: the proposal says no release; `app/Cargo.toml` stays as it is.
- [x] 3.2 Manual checks by the user: none (no hotkey, recording, overlay, paste or permissions change).
