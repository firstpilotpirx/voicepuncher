## 1. Configuration

- [ ] 1.1 Add `landing: {batch: true, max_batch: 4}` to `.claude/kit.yaml`
- [ ] 1.2 Commit: `chore(kit): turn on batch landing`

## 2. Workflow docs

- [ ] 2.1 Rewrite `AGENTS.md` § Workflow stages 8–11: the branch's own check is `just check`, `kit land` queues it, the group's `full-check` is the merge gate, the session waits for the queue's verdict without polling
- [ ] 2.2 Update `AGENTS.md` § How to verify a change (merge gate is the group's `full-check`) and `openspec/config.yaml` context to match
- [ ] 2.3 Run `just test`
- [ ] 2.4 Version unchanged: no behavior change, no release
- [ ] 2.5 Commit: `docs: land branches in groups through the queue`
