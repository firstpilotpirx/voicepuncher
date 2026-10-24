# app-distribution Specification

## Purpose
How the application is packaged, signed and distributed: a signed application bundle with an identity that stays the same across updates, a disk image as the only release artifact, the first-launch flow without a Developer ID, and the application icon.

## Requirements

### Requirement: The release is one disk image
A release build SHALL produce exactly one disk image named after the application and the release version, holding the application bundle and a shortcut to the Applications folder, plus a checksum file of that image; the published release SHALL hold exactly these two files and nothing else.

#### Scenario: A release is published
- **GIVEN** the checkout is a clean release commit with a bumped version
- **WHEN** the release is run
- **THEN** the published release holds the disk image named with the version and its checksum file, and no other file

#### Scenario: The disk image opens as an installer
- **GIVEN** a disk image made by the release build
- **WHEN** the user opens it
- **THEN** its window shows the application bundle on the left and the Applications shortcut on the right over the background picture that says to drag the application to Applications, and the volume is named after the application and the version

### Requirement: The bundle identifies the application
The application bundle SHALL carry the bundle identifier, the display name, the version of the crate, the flag that keeps it out of the Dock, the description of why it needs the microphone and the application icon, so that the permission panes list the application by its own name with its icon.

#### Scenario: The bundle's information matches the crate
- **GIVEN** a bundle made by the bundle build
- **WHEN** its information file is read
- **THEN** it names the bundle identifier, the display name, the crate's version as both the short and the build version, the no-Dock flag, the microphone usage description and the icon file

#### Scenario: Permissions are listed under the application's name
- **GIVEN** the bundle is installed in Applications and launched for the first time
- **WHEN** the user grants Microphone, Accessibility and Input Monitoring
- **THEN** each permission pane lists the application by its name with its icon, not a terminal

#### Scenario: No Dock icon
- **GIVEN** the bundle is installed in Applications
- **WHEN** the user launches it
- **THEN** no icon appears in the Dock and no application menu replaces the menu of the frontmost application

### Requirement: The signature keeps the identity across updates
The bundle SHALL be signed with the self-signed code-signing certificate named by the signing identity, so that its designated requirement names that certificate and every build signed with it shares one identity for the permission system.

#### Scenario: Two builds share one identity
- **GIVEN** two bundles built from different commits and signed with the same signing identity
- **WHEN** their designated requirements are read
- **THEN** both name the certificate and are identical

#### Scenario: Permissions survive an update
- **GIVEN** the user granted Microphone, Accessibility and Input Monitoring to an installed bundle
- **WHEN** a later build signed with the same signing identity replaces it in Applications and is launched
- **THEN** the application dictates without any permission prompt

#### Scenario: The signature verifies strictly
- **GIVEN** a bundle made by the bundle build
- **WHEN** its signature is verified strictly, every nested file included
- **THEN** the verification passes

### Requirement: The release refuses to run unsigned
The release SHALL stop before building when no signing identity is configured or the configured one is not in the keychain, naming the steps to create the certificate; the bundle build SHALL stop the same way when the configured identity is not in the keychain, and SHALL sign ad-hoc with a visible notice when no identity is configured, so that the checks run on a machine without the certificate.

#### Scenario: The release without a signing identity
- **GIVEN** no signing identity is configured
- **WHEN** the release is run
- **THEN** it stops before building anything, naming the command that creates the certificate

#### Scenario: The bundle build with an identity the keychain lacks
- **GIVEN** the configured signing identity is not in the keychain
- **WHEN** the bundle build is run
- **THEN** it stops before building, naming the identity and the steps to create the certificate

#### Scenario: The bundle build without a signing identity
- **GIVEN** no signing identity is configured
- **WHEN** the bundle build is run
- **THEN** it produces a bundle whose signature verifies, and its output says the bundle is signed ad-hoc and has no stable identity

### Requirement: The first launch is documented
The README SHALL tell the user how to install from the disk image and how to pass Gatekeeper on the first launch of each version without a Developer ID, and the release checklist SHALL tell the maintainer how to create the signing certificate and how to check that permissions survive an update.

#### Scenario: A user installs a downloaded release
- **GIVEN** the user downloaded the disk image from a release
- **WHEN** they follow the README's install section
- **THEN** they drag the application to Applications, allow it once in Privacy & Security after the first launch is refused, and the application runs

#### Scenario: A maintainer prepares a release machine
- **GIVEN** a machine without the signing certificate
- **WHEN** the maintainer follows the release checklist
- **THEN** the certificate exists in the keychain, is trusted for code signing, is named in the environment, and the release no longer refuses

### Requirement: The bundle is verified before a merge
The full check SHALL build the bundle and the disk image, verify the bundle's information, its engines and its signature, mount the disk image and confirm it holds the application bundle and the Applications shortcut, without taking the machine-wide test lock; it SHALL then run the bundled engines from a copy of the bundle in a model stage under the machine-wide test lock; the quick check SHALL not build the bundle.

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
- **THEN** the bundle stage's check fails naming the missing key

#### Scenario: The quick check stays without the bundle
- **GIVEN** a checkout with the bundle tools installed
- **WHEN** the quick check runs
- **THEN** its stage table lists no bundle stage

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

### Requirement: The engines are relocated
No bundled program or library SHALL reference a library by a path outside the bundle, every library reference SHALL be found in the engine's own folder, and the speech recognition engine SHALL load its compute backends from its own folder, not from a folder compiled into it.

#### Scenario: No bundled file points outside the bundle
- **GIVEN** the bundle the bundle build made
- **WHEN** the library references of every bundled program and library are read
- **THEN** none names a Homebrew or `/usr/local` path, and each library it names relative to its search path is a file in the same engine folder
- **AND** no bundled file holds a Homebrew installation folder compiled into it

### Requirement: The engines are signed with the application
Every bundled program and library SHALL be signed with the application's signing identity before the application itself, and the bundle's strict verification of every nested file SHALL pass.

#### Scenario: Every bundled file carries the application's signature
- **GIVEN** the bundle the bundle build made
- **WHEN** the signature of every bundled program and library is read
- **THEN** each verifies and names the same signer as the application

### Requirement: Licenses ship with the bundle
The bundle SHALL carry the license text of every bundled engine and library it links in its resources, and the README SHALL name each of them with its license.

#### Scenario: The bundle holds the licenses
- **GIVEN** the bundle the bundle build made
- **WHEN** its licenses folder is listed
- **THEN** it holds the licenses of whisper.cpp, ggml, the OpenMP runtime, sherpa-onnx and ONNX Runtime
- **AND** it holds no license of llama.cpp

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
