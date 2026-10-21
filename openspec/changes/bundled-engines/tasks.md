## 1. Resolver and engine logging

- [ ] 1.1 Red: unit tests in `app/src/adapters/secondary/localservices/catalog/tests.rs` — the bundle's `engines/whisper|llama|command-spotting/` program comes before `$PATH`; outside a bundle `$PATH`; origin and version of a bundled engine (`VERSION`), a Homebrew engine (Cellar version folder) and an unknown one
- [ ] 1.2 Green: engine folders and `engine_origin` in `catalog/mod.rs`; the log line per service in `app/src/runtime/services.rs`
- [ ] 1.3 `just test`; commit

## 2. Engine lock and staging

- [ ] 2.1 `app/packaging/engines.lock` with whisper (Homebrew whisper.cpp, ggml, libomp), llama (release b11669) and command spotting, written from this machine; `app/packaging/licenses/onnxruntime-LICENSE`; `app/packaging/engines/` in `.gitignore`
- [ ] 2.2 Red: scenarios "A different Homebrew version is refused", "A release archive with another checksum is refused" and "The pinned engines are staged" in `e2e-test/tests/features/app_distribution.feature` with steps in `e2e-test/tests/steps/app_distribution.rs`
- [ ] 2.3 Green: `engines` and `_engines` recipes in `justfile` (versions, download, checksums, copy, relocation, backend folder blanked, ad-hoc signature, reference sweep, clean-environment runs, `VERSION` files, licenses, table)
- [ ] 2.4 `just test`; commit

## 3. Bundle

- [ ] 3.1 Red: scenarios "Each engine runs on its own from the bundle", "The engine folders hold what the services need", "No bundled file points outside the bundle", "Every bundled file carries the application's signature" and "The bundle holds the licenses" in `app_distribution.feature` with steps
- [ ] 3.2 Green: `bundle` copies the engines and licenses and signs them inside-out; `_check-bundle` checks references, signatures, clean runs and licenses
- [ ] 3.3 `just test`; commit

## 4. Bundle engine stage

- [ ] 4.1 Red: scenarios "A downloaded bundle serves dictation and refinement on Metal" (`@bundle_engines`), "The full check runs the bundled engines" and the manual "A user without Homebrew installs the disk image" (`@wip`) in `app_distribution.feature` with steps
- [ ] 4.2 Green: `_e2e-bundle`, `bundle-smoke`, the `e2e-bundle` stage in `full-check`, `_model-stages` without shared services for it, the `e2e-bundle` nextest profile and limit in `.config/nextest.toml`, `@bundle_engines` left out of the bundle profile, `stage_inputs` for `app/packaging/*`
- [ ] 4.3 `just test`; commit

## 5. Docs

- [ ] 5.1 `README.md` § Install, § Developer (engines, bumping an engine), § Licenses, bundle size; `AGENTS.md` (`full-check` stages)
- [ ] 5.2 `docs/architecture/overview.md`, `docs/project-map.md`, `docs/ubiquitous-language.md` (Engine, Engine folder, Engine lock)
- [ ] 5.3 `just test`; commit

## 6. Release

- [ ] 6.1 Bump `version` in `app/Cargo.toml` to 0.33.0 (the next minor above main's 0.32.0); `just test`; commit
- [ ] 6.2 Manual check by the user: install the disk image with Homebrew's folders hidden from `$PATH`, launch, dictate, paste the log
