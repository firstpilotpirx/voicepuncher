## ADDED Requirements

### Requirement: Settings open the settings window from the menu
The menu SHALL offer Settings with the standard shortcut. Choosing it SHALL open the settings window.

#### Scenario: Settings opens the settings window
- **GIVEN** the app runs with its configuration file
- **WHEN** the user chooses Settings
- **THEN** the settings window opens in front

## REMOVED Requirements

### Requirement: Settings open from the menu
**Reason**: The app now has a settings window; Settings opens it instead of the configuration file in a text editor.
**Migration**: Nothing to do: Settings stays in the same place with the same shortcut.
