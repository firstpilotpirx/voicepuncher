## REMOVED Requirements

### Requirement: Text refinement shows the raw transcript dimmed

**Reason**: The text window under the pill now shows only during a recording (visual-feedback: "The text window shows only during a recording"); the user found text under the pill after the recording distracting.
**Migration**: None. The refinement stage shows its icon and "Refining text" in the pill; the raw transcript is no longer shown.

## ADDED Requirements

### Requirement: The processing stages show no text window

While the speech recognition stage or the text refinement stage shows, the overlay SHALL show the stage in the pill alone, with no text window under it.

#### Scenario: Refinement shows no raw transcript
- **GIVEN** text refinement is enabled
- **WHEN** text refinement of a dictation with speech in it starts
- **THEN** the overlay shows "Refining text" in the pill and no text window under it
