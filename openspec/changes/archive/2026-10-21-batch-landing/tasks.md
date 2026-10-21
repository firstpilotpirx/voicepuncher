## 1. Configuration

- [x] 1.1 Add `landing: {batch: true, max_batch: 4}` to `.claude/kit.yaml`
- [x] 1.2 Commit: `chore(kit): turn on batch landing`

## 2. Workflow docs

- [x] 2.1 Rewrite `AGENTS.md` § Workflow stages 8–11: the branch's own check is `just check`, `kit land` queues it, the group's `full-check` is the merge gate, the session waits for the queue's verdict without polling
- [x] 2.2 Update `AGENTS.md` § How to verify a change (merge gate is the group's `full-check`) and `openspec/config.yaml` context to match
- [x] 2.3 Run `just test`
- [x] 2.4 Version unchanged: no behavior change, no release
- [x] 2.5 Commit: `docs: land branches in groups through the queue`
