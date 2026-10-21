## 1. Default configuration file

- [x] 1.1 Red: unit tests in `app/src/config.rs` — the default file parses to the configuration of an empty file; every key of `Config::default()` is set in it with a comment above, but the three exemptions (`refinement.system_prompt`, `refinement.examples`, `audio.debug_save_wav`); `ensure_exists` writes the file when missing and keeps an existing one
- [x] 1.2 Green: `app/src/config/default.toml` and `config::ensure_exists(path) -> Result<Created>` (atomic write through the temporary file, as `ConfigDocument::save`); `just test` and commit

## 2. Remove the wizard

- [x] 2.1 Red: `cli_it.rs` — `--setup` is refused with exit 2; the binary with a missing `--config` path, stdin closed and an unreadable `--input-file` writes the default file and exits 1; `--input-file` with a dead service port exits 2; an unknown flag exits 2; the help lists exactly the developer flags and each setting flag says it overrides the configuration for this run; `Cargo.lock` has no `dialoguer`
- [x] 2.2 Green: remove `app/src/adapters/primary/setup/`, `app/tests/setup_cli_it.rs`, the `--setup` flag, `dialoguer` and `console` (and the stale `app/Cargo.lock`); `main.rs` calls `ensure_exists` and logs the creation; file mode reads the recording before it connects its services; help texts of every flag; `just test` and commit

## 3. Documentation and version

- [x] 3.1 README rewritten for the installed app (Install, First launch, Using, Settings, Logs, Uninstall, Privacy, Licenses, Developer with flags, exit codes, checks and release checklist)
- [x] 3.2 `docs/architecture/{context,overview}.md`, `docs/project-map.md`, `docs/ubiquitous-language.md`, `docs/e2e-benchmark-design.md`, `AGENTS.md` lines
- [x] 3.3 Bump `version` in `app/Cargo.toml` to 0.35.0; `just test` and commit
