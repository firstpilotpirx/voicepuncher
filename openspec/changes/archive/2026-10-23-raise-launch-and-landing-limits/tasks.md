## 1. Limits

- [x] 1.1 `.claude/kit.yaml`: add `launch: {max_running: 7}` and set `landing.max_batch: 7`
- [x] 1.2 Check: `kit launchable` runs without a config error, and kit's config loader reads both values (a refused value fails it)
- [x] 1.3 `AGENTS.md` § Workflow stage 11: "at most 4 per group" → "at most 7 per group"; `AGENTS.md` § Workflow, several changes at once: name the 7-change launch limit
- [x] 1.4 `just test`, commit

## 2. Superseded PR

- [x] 2.1 Close PR #302 with a comment pointing to this change (closed 2026-10-23)
- [x] 2.2 Record the closed PR in this file, commit

## 3. Version

- [x] 3.1 No version bump: tooling configuration only, no behavior of the app changes (proposal § Release)
