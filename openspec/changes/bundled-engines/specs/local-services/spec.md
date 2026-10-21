## ADDED Requirements

### Requirement: A bundled app starts its services from its own engines
When the app runs from its application bundle, it SHALL start each service from the engine folder of that service inside the bundle before looking on `$PATH`; a program named by the environment SHALL still come first, and an app that does not run from a bundle SHALL find its engines on `$PATH` as before.

#### Scenario: The bundle's engine comes before `$PATH`
- **GIVEN** the app runs from a bundle whose engine folders hold every engine, and `$PATH` holds them too
- **WHEN** the services it needs are looked up
- **THEN** each service's program is the one in its engine folder inside the bundle

#### Scenario: An app outside a bundle uses `$PATH`
- **GIVEN** the app runs from a build folder and `$PATH` holds every engine
- **WHEN** the services it needs are looked up
- **THEN** each service's program is the one on `$PATH`

### Requirement: The engine in use is logged
At every launch that manages services, the app SHALL log for each service the engine it runs, where the engine came from — the bundle, a program the environment names, `$PATH` or the build folder — and the engine's version when it is known: the version file of a bundled engine, or the version folder of a Homebrew installation.

#### Scenario: A bundled engine is logged with its version
- **GIVEN** the app runs from a bundle whose speech recognition engine folder has the version file 1.9.5
- **WHEN** the app launches and looks up its services
- **THEN** its log says the speech recognition service runs whisper-server 1.9.5 from the bundle

#### Scenario: A Homebrew engine is logged with its version
- **GIVEN** the app runs outside a bundle and `$PATH` leads to llama-server in Homebrew's folder of version 0.7.0
- **WHEN** the app launches with refinement on and looks up its services
- **THEN** its log says the text refinement service runs llama-server 0.7.0 from `$PATH`
