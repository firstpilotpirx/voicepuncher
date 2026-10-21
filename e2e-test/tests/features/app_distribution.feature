@app_distribution @e2e @bundle
Feature: app-distribution
  The bundle and the disk image the release build makes, and the guards that keep a release from
  going out unsigned. Run by the bundle stage of the full check, right after it built both.

  Rule: The release is one disk image

    # Manual: running the release publishes a GitHub Release, which no test may do; the maintainer
    # checks the release page (README § Release checklist)
    @wip
    Scenario: A release is published
      Given a clean release commit with a bumped version
      When the release is run
      Then the published release holds the disk image named with the version and its checksum file, and no other file

    Scenario: The disk image opens as an installer
      Given the disk image the release build made
      When the user opens it
      Then it holds the application and a shortcut to the Applications folder
      And its window is laid out over the picture that says to drag the application to Applications
      And its volume is named after the application and the version

  Rule: The bundle identifies the application

    Scenario: The bundle's information matches the crate
      Given the bundle the bundle build made
      When its information file is read
      Then it names the bundle identifier, the display name and the icon file the bundle holds
      And it carries the crate's version as both the short and the build version
      And it keeps the application out of the Dock
      And it says why the application needs the microphone

    # Manual: the permission panes need a person and a fresh install (README § Release checklist)
    @wip
    Scenario: Permissions are listed under the application's name
      Given the bundle is installed in Applications and launched for the first time
      When the user grants the three permissions
      Then each permission pane lists the application by its name with its icon, not a terminal

    # Manual: the Dock and the menu bar of the frontmost application need a person to look
    @wip
    Scenario: No Dock icon
      Given the bundle is installed in Applications
      When the user launches it
      Then no icon appears in the Dock and the frontmost application keeps its menu

  Rule: The signature keeps the identity across updates

    @signing_identity
    Scenario: Two builds share one identity
      Given two different builds signed with the same signing identity
      When their designated requirements are read
      Then both name the signing certificate
      And they are identical

    # Manual: the permission prompts need a person and two installed versions (README § Release checklist)
    @wip
    Scenario: Permissions survive an update
      Given the user granted the three permissions to an installed bundle
      When a later build signed with the same signing identity replaces it and is launched
      Then the application dictates without any permission prompt

    Scenario: The signature verifies strictly
      Given the bundle the bundle build made
      When its signature is verified strictly, every nested file included
      Then the verification passes

  Rule: The release refuses to run unsigned

    Scenario: The release without a signing identity
      Given no signing identity is configured
      When the release is run
      Then it stops before building anything
      And it names the command that creates the certificate

    Scenario: The bundle build with an identity the keychain lacks
      Given the configured signing identity is not in the keychain
      When the bundle build is run
      Then it stops before building anything
      And it names the identity and the steps to create the certificate

    Scenario: The bundle build without a signing identity
      Given no signing identity is configured
      When the bundle build is run
      Then it produces a bundle whose signature verifies
      And it says the bundle is signed ad-hoc and has no stable identity

  Rule: The first launch is documented

    # Manual: following the README needs a person, a downloaded release and Gatekeeper's first refusal
    @wip
    Scenario: A user installs a downloaded release
      Given the user downloaded the disk image from a release
      When they follow the README's install section
      Then they drag the application to Applications, allow it once in Privacy & Security, and it runs

    # Manual: trusting the certificate asks for the user's password
    @wip
    Scenario: A maintainer prepares a release machine
      Given a machine without the signing certificate
      When the maintainer follows the release checklist
      Then the certificate is in the keychain, trusted for code signing and named in the environment
      And the release no longer refuses

  Rule: The bundle is verified before a merge

    Scenario: The full check verifies the bundle
      Given a checkout with the bundle tools installed
      When the stages of the full check are listed
      Then a bundle stage comes before the stages that wait for the machine-wide lock

    Scenario: The full check fails on a broken bundle
      Given a bundle whose information file lacks the microphone usage description
      When the full check's bundle stage checks it
      Then the check fails naming the missing key

    Scenario: The quick check stays without the bundle
      Given a checkout with the bundle tools installed
      When the stages of the quick check are listed
      Then there is no bundle stage
