## MODIFIED Requirements

### Requirement: Each processing stage has its own icon

The speech recognition stage and the text refinement stage SHALL each show an icon of its own, so the user tells them apart at a glance without reading any text. Both SHALL show that processing is under way, as the processing overlay did before. The speech recognition stage SHALL show 🎧 (headphones) and the text refinement stage ✨ (sparkles), each on the overlay's left with a spinner turning to the icon's right. Both icons SHALL show in color, whole and legible in light and dark appearance.

#### Scenario: Speech recognition shows headphones left of the spinner
- **GIVEN** the user records a dictation
- **WHEN** the user ends the recording
- **THEN** the overlay shows 🎧 (headphones) on its left
- **AND** the spinner turns to the right of the icon

#### Scenario: Text refinement shows sparkles left of the spinner
- **GIVEN** a dictation with speech in it is processed with text refinement enabled
- **WHEN** text refinement starts
- **THEN** the overlay shows ✨ (sparkles) on its left
- **AND** the spinner turns to the right of the icon

#### Scenario: The two stages look different
- **GIVEN** a dictation is processed with text refinement enabled
- **WHEN** the overlay switches from the speech recognition stage to the text refinement stage
- **THEN** the icon on the overlay changes to a different one

#### Scenario: The stage icons read in light and dark appearance
- **GIVEN** text refinement is enabled and macOS uses the light or the dark appearance
- **WHEN** the user ends a dictation with speech in it
- **THEN** the overlay shows 🎧 (headphones), then ✨ (sparkles), each in color, whole and legible
