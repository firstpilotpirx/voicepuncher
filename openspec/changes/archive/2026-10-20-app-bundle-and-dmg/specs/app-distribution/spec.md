# Spec Delta

## Purpose

How the application is packaged, signed and distributed: a signed application bundle with an identity that stays the same across updates, a disk image as the only release artifact, the first-launch flow without a Developer ID, and the application icon.

## ADDED Requirements

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
The full check SHALL build the bundle and the disk image, verify the bundle's information and signature, mount the disk image and confirm it holds the application bundle and the Applications shortcut, without taking the machine-wide test lock; the quick check SHALL not build the bundle.

#### Scenario: The full check verifies the bundle
- **GIVEN** a checkout with the bundle tools installed
- **WHEN** the full check runs
- **THEN** its stage table lists a passed bundle stage before the stages that wait for the machine-wide lock, and that stage mounted the disk image and found the application bundle and the Applications shortcut

#### Scenario: The full check fails on a broken bundle
- **GIVEN** a checkout whose bundle information file lacks the microphone usage description
- **WHEN** the full check runs
- **THEN** the bundle stage fails naming the missing key, and the model stages do not run

#### Scenario: The quick check stays without the bundle
- **GIVEN** a checkout with the bundle tools installed
- **WHEN** the quick check runs
- **THEN** its stage table lists no bundle stage
