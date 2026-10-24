## MODIFIED Requirements

### Requirement: The bundle carries its engines
The application bundle SHALL carry every engine the services run — the speech recognition engine and the command spotting service — each in a folder of its own inside the bundle with every library it loads beside it, so that an installed application needs no Homebrew and nothing on `$PATH`. The bundle SHALL carry no text refinement engine.

#### Scenario: Each engine runs on its own from the bundle
- **GIVEN** the bundle the bundle build made
- **WHEN** each bundled engine is run with nothing but the system folders on `$PATH` and no library path variable
- **THEN** each one answers with its usage or version and exits successfully

#### Scenario: The engine folders hold what the services need
- **GIVEN** the bundle the bundle build made
- **WHEN** its engine folders are listed
- **THEN** there is one folder for the speech recognition engine and one for the command spotting service, each holding its program and a version file
- **AND** there is no folder for a text refinement engine

### Requirement: Licenses ship with the bundle
The bundle SHALL carry the license text of every bundled engine and library it links in its resources, and the README SHALL name each of them with its license.

#### Scenario: The bundle holds the licenses
- **GIVEN** the bundle the bundle build made
- **WHEN** its licenses folder is listed
- **THEN** it holds the licenses of whisper.cpp, ggml, the OpenMP runtime, sherpa-onnx and ONNX Runtime
- **AND** it holds no license of llama.cpp

## ADDED Requirements

### Requirement: The engines are pinned to Homebrew and this workspace
The engines SHALL be taken only from the source and version the engine lock names — the speech recognition engine from the Homebrew installation, the command spotting service from this workspace — and never built from source; the engine staging SHALL compare every file it takes with the checksum the lock records and SHALL refuse a different version or checksum, naming what it found and what the lock expects.

#### Scenario: A different Homebrew version is refused
- **GIVEN** an engine lock that names a Homebrew formula version that is not installed
- **WHEN** the engines are staged
- **THEN** the staging fails naming the formula, the version the lock expects, the versions found, and how to pin the formula
- **AND** no engine folder is written

#### Scenario: A Homebrew file with another checksum is refused
- **GIVEN** an engine lock whose checksum of a Homebrew file differs from the installed file's
- **WHEN** the engines are staged
- **THEN** the staging fails naming the file, the checksum found and the checksum the lock expects
- **AND** no engine folder is written

#### Scenario: The pinned engines are staged
- **GIVEN** the engine lock of the checkout and its pinned sources on the machine
- **WHEN** the engines are staged
- **THEN** each engine folder holds exactly the files the lock lists for it, and each engine's version file holds the lock's version

### Requirement: The bundled speech recognition engine uses the GPU
Started from a copy of the bundle outside the build folder, the bundled speech recognition engine SHALL load its compute backends from the copy and run on the Metal GPU, the application SHALL start every service from its own engine folders with nothing but the system folders on `$PATH`, and a transcription and the command spotting service's health SHALL work on them.

#### Scenario: A bundle copy serves dictation on Metal
- **GIVEN** a copy of the bundle outside the build folder, the models on disk and nothing but the system folders on `$PATH`
- **WHEN** the copy's application starts its services with voice control on
- **THEN** its log names each engine as taken from the bundle
- **AND** a recording is transcribed through the started services
- **AND** the command spotting service answers its health check
- **AND** the speech recognition engine's log shows it loaded its compute backends from the copy and set up the Metal GPU

#### Scenario: A user without Homebrew installs the disk image
- **GIVEN** a Mac on which Homebrew's folders are hidden from `$PATH` and the disk image of the release
- **WHEN** the user installs the application from it, launches it and dictates
- **THEN** after the user allowed the application once with Open Anyway, the services start from the bundle, the dictated text is pasted and the engine logs show the Metal GPU in use

## REMOVED Requirements

### Requirement: The engines are pinned

**Reason**: Text refinement is removed; the requirement is restated without it as "The engines are pinned to Homebrew and this workspace".
**Migration**: None.

### Requirement: The bundled engines use the GPU

**Reason**: Text refinement is removed; the requirement is restated without it as "The bundled speech recognition engine uses the GPU".
**Migration**: None.
