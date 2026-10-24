## MODIFIED Requirements

### Requirement: A failed action shows a failure notice
The system SHALL show a failure notice in the overlay when an action the user started cannot be completed because a local service, a device or a permission failed. The notice SHALL name the failing part in plain words (speech recognition, command spotting, microphone, accessibility) and, when a remedy is known, say it. It SHALL appear within 2 s of the failure, take no keyboard focus, and the log SHALL keep the technical detail.

#### Scenario: The notice names the failing service and its remedy
- **GIVEN** the speech recognition service is not running
- **WHEN** the user records a dictation with the hotkey and stops it
- **THEN** within 2 s of the stop the overlay shows a notice that names speech recognition and says to start the local services
- **AND** the app the user dictates into keeps the keyboard focus

## REMOVED Requirements

### Requirement: Text pasted without refinement is announced

**Reason**: Text refinement is removed; every dictation is pasted as its transcript, so there is no refinement that can fail.
**Migration**: None.
