## 1. Dual license in the repository and the bundle

- [x] 1.1 Red: add the scenario "The bundle holds the application's own license" and its steps; it fails on a bundle without the texts
- [x] 1.2 Green: copy `LICENSE-MIT` and `LICENSE-APACHE` into the bundle in `just bundle`
- [x] 1.3 Add `LICENSE-MIT` and the official `LICENSE-APACHE` at the root
- [x] 1.4 Set `license = "MIT OR Apache-2.0"` in every member's `Cargo.toml`
- [x] 1.5 Add the README `## License` section; group `.gitignore` and ignore `.claude/worktrees/`
- [x] 1.6 Bump the version to 0.49.0
