## ADDED Requirements

### Requirement: The bundle carries its engines
The application bundle SHALL carry every engine the services run — the speech recognition engine, the text refinement engine and the command spotting service — each in a folder of its own inside the bundle with every library it loads beside it, so that an installed application needs no Homebrew and nothing on `$PATH`.

#### Scenario: Each engine runs on its own from the bundle
- **GIVEN** the bundle the bundle build made
- **WHEN** each bundled engine is run with nothing but the system folders on `$PATH` and no library path variable
- **THEN** each one answers with its usage or version and exits successfully

#### Scenario: The engine folders hold what the services need
- **GIVEN** the bundle the bundle build made
- **WHEN** its engine folders are listed
- **THEN** there is one folder for the speech recognition engine, one for the text refinement engine and one for the command spotting service, each holding its program and a version file

### Requirement: The engines are pinned
The engines SHALL be taken only from the source and version the engine lock names — the speech recognition engine from the Homebrew installation, the text refinement engine from the official release archive, the command spotting service from this workspace — and never built from source; the engine staging SHALL compare every file it takes with the checksum the lock records and SHALL refuse a different version or checksum, naming what it found and what the lock expects.

#### Scenario: A different Homebrew version is refused
- **GIVEN** an engine lock that names a Homebrew formula version that is not installed
- **WHEN** the engines are staged
- **THEN** the staging fails naming the formula, the version the lock expects, the versions found, and how to pin the formula
- **AND** no engine folder is written

#### Scenario: A release archive with another checksum is refused
- **GIVEN** an engine lock whose release archive checksum differs from the archive's
- **WHEN** the engines are staged
- **THEN** the staging fails naming the archive, the checksum found and the checksum the lock expects
- **AND** no engine folder is written

#### Scenario: The pinned engines are staged
- **GIVEN** the engine lock of the checkout and its pinned sources on the machine
- **WHEN** the engines are staged
- **THEN** each engine folder holds exactly the files the lock lists for it, and each engine's version file holds the lock's version

### Requirement: The engines are relocated
No bundled program or library SHALL reference a library by a path outside the bundle, every library reference SHALL be found in the engine's own folder, and the speech recognition engine SHALL load its compute backends from its own folder, not from a folder compiled into it.

#### Scenario: No bundled file points outside the bundle
- **GIVEN** the bundle the bundle build made
- **WHEN** the library references of every bundled program and library are read
- **THEN** none names a Homebrew or `/usr/local` path, and each library it names relative to its search path is a file in the same engine folder

### Requirement: The engines are signed with the application
Every bundled program and library SHALL be signed with the application's signing identity before the application itself, and the bundle's strict verification of every nested file SHALL pass.

#### Scenario: Every bundled file carries the application's signature
- **GIVEN** the bundle the bundle build made
- **WHEN** the signature of every bundled program and library is read
- **THEN** each verifies and names the same signer as the application

### Requirement: The bundled engines use the GPU
Started from a downloaded copy of the bundle, the bundled speech recognition and text refinement engines SHALL run on the Metal GPU, the application SHALL start every service from its own engine folders with nothing but the system folders on `$PATH`, and a transcription, a refinement and the command spotting service's health SHALL work on them.

#### Scenario: A downloaded bundle serves dictation and refinement on Metal
- **GIVEN** a copy of the bundle marked as downloaded from the internet, the models on disk and nothing but the system folders on `$PATH`
- **WHEN** the copy's application starts its services with refinement and voice control on
- **THEN** its log names each engine as taken from the bundle
- **AND** a recording is transcribed and a sentence is refined through the started services
- **AND** the command spotting service answers its health check
- **AND** the logs of the speech recognition and text refinement engines show the Metal GPU in use

#### Scenario: A user without Homebrew installs the disk image
- **GIVEN** a Mac on which Homebrew's folders are hidden from `$PATH` and the disk image of the release
- **WHEN** the user installs the application from it, launches it and dictates
- **THEN** the services start from the bundle, the dictated text is pasted and the engine logs show the Metal GPU in use

### Requirement: Licenses ship with the bundle
The bundle SHALL carry the license text of every bundled engine and library it links in its resources, and the README SHALL name each of them with its license.

#### Scenario: The bundle holds the licenses
- **GIVEN** the bundle the bundle build made
- **WHEN** its licenses folder is listed
- **THEN** it holds the licenses of whisper.cpp, ggml, llama.cpp, the OpenMP runtime, sherpa-onnx and ONNX Runtime

## MODIFIED Requirements

### Requirement: The bundle is verified before a merge
The full check SHALL build the bundle and the disk image, verify the bundle's information, its engines and its signature, mount the disk image and confirm it holds the application bundle and the Applications shortcut, without taking the machine-wide test lock; it SHALL then run the bundled engines from a downloaded copy of the bundle in a model stage under the machine-wide test lock; the quick check SHALL not build the bundle.

#### Scenario: The full check verifies the bundle
- **GIVEN** a checkout with the bundle tools installed
- **WHEN** the full check runs
- **THEN** its stage table lists a passed bundle stage before the stages that wait for the machine-wide lock, and that stage mounted the disk image and found the application bundle and the Applications shortcut

#### Scenario: The full check runs the bundled engines
- **GIVEN** a checkout with the bundle tools installed and the models on disk
- **WHEN** the full check runs
- **THEN** its stage table lists a passed bundle engine stage among the stages that hold the machine-wide lock

#### Scenario: The full check fails on a broken bundle
- **GIVEN** a checkout whose bundle information file lacks the microphone usage description
- **WHEN** the full check runs
- **THEN** the bundle stage fails naming the missing key, and the model stages do not run

#### Scenario: The quick check stays without the bundle
- **GIVEN** a checkout with the bundle tools installed
- **WHEN** the quick check runs
- **THEN** its stage table lists no bundle stage
